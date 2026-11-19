//! detalhes de transporte do `net/http` do Go reproduzidos na resposta:
//! enquadramento chunked, reason phrase e fechamento da conexão quando o
//! corpo da requisição não foi consumido

use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::task::{Context, Poll};

use axum::body::Body;
use axum::http::{HeaderValue, Response, StatusCode, header};
use bytes::Bytes;
use http_body::{Frame, SizeHint};

// bufferBeforeChunkingSize do net/http: acima disso a resposta sai chunked
const GO_CHUNKING_BUFFER: u64 = 2048;
// maxPostHandlerReadBytes: resto maior que isso não é drenado e a conexão fecha
const GO_MAX_POST_HANDLER_READ: u64 = 256 << 10;

/// corpo de requisição que sinaliza quando chegou ao fim
pub struct CorpoRastreado {
    inner: Body,
    fim: Arc<AtomicBool>,
}

impl CorpoRastreado {
    pub fn envolver(inner: Body) -> (Body, Arc<AtomicBool>) {
        let fim = Arc::new(AtomicBool::new(false));
        let corpo = Self {
            inner,
            fim: Arc::clone(&fim),
        };
        (Body::new(corpo), fim)
    }
}

impl http_body::Body for CorpoRastreado {
    type Data = Bytes;
    type Error = axum::Error;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        let frame = Pin::new(&mut self.inner).poll_frame(cx);
        if matches!(frame, Poll::Ready(None)) {
            self.fim.store(true, Ordering::Release);
        }
        frame
    }

    fn is_end_stream(&self) -> bool {
        self.inner.is_end_stream()
    }

    fn size_hint(&self) -> SizeHint {
        self.inner.size_hint()
    }
}

/// go drena até 256 KiB do corpo não lido; além disso (ou tamanho
/// desconhecido) responde com `Connection: close`
pub fn deve_fechar(declarado: Option<u64>, consumido: bool) -> bool {
    !consumido && declarado.is_none_or(|n| n > GO_MAX_POST_HANDLER_READ)
}

/// reason phrase do `http.StatusText` quando difere da do hyper
fn reason_go(status: StatusCode) -> Option<&'static str> {
    match status.as_u16() {
        413 => Some("Request Entity Too Large"),
        414 => Some("Request URI Too Long"),
        416 => Some("Requested Range Not Satisfiable"),
        _ => None,
    }
}

pub fn status_text(status: StatusCode) -> &'static str {
    reason_go(status).unwrap_or_else(|| status.canonical_reason().unwrap_or(""))
}

/// aplica enquadramento, reason phrase e `Connection: close` como o Go
pub fn ajustar(mut resp: Response<Body>, tamanho: u64, fechar: bool) -> Response<Body> {
    if let Some(reason) = reason_go(resp.status()) {
        resp.extensions_mut()
            .insert(hyper::ext::ReasonPhrase::from_static(reason.as_bytes()));
    }
    if fechar {
        resp.headers_mut()
            .insert(header::CONNECTION, HeaderValue::from_static("close"));
    }
    if tamanho <= GO_CHUNKING_BUFFER || resp.headers().contains_key(header::CONTENT_LENGTH) {
        return resp;
    }
    // sem tamanho exato o hyper omite Content-Length e usa chunked
    let (parts, body) = resp.into_parts();
    Response::from_parts(parts, Body::from_stream(body.into_data_stream()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::HttpBody as _;
    use http_body_util::BodyExt as _;

    #[test]
    fn fechamento_segue_go() {
        assert!(!deve_fechar(Some(10), true));
        assert!(!deve_fechar(Some(GO_MAX_POST_HANDLER_READ), false));
        assert!(deve_fechar(Some(GO_MAX_POST_HANDLER_READ + 1), false));
        assert!(deve_fechar(None, false));
        assert!(!deve_fechar(None, true));
    }

    #[test]
    fn status_text_como_go() {
        assert_eq!(
            status_text(StatusCode::PAYLOAD_TOO_LARGE),
            "Request Entity Too Large"
        );
        assert_eq!(status_text(StatusCode::NOT_FOUND), "Not Found");
    }

    #[test]
    fn grande_vira_chunked() {
        let pequeno = ajustar(Response::new(Body::from(vec![0u8; 2048])), 2048, false);
        assert_eq!(pequeno.body().size_hint().exact(), Some(2048));
        let grande = ajustar(Response::new(Body::from(vec![0u8; 2049])), 2049, true);
        assert_eq!(grande.body().size_hint().exact(), None);
        assert_eq!(
            grande
                .headers()
                .get(header::CONNECTION)
                .map(|v| v.as_bytes()),
            Some(b"close".as_slice())
        );
    }

    #[tokio::test]
    async fn rastreia_fim_do_corpo() {
        let (corpo, fim) = CorpoRastreado::envolver(Body::from("abc"));
        assert!(!fim.load(Ordering::Acquire));
        let lido = corpo.collect().await.map(|c| c.to_bytes());
        assert_eq!(lido.ok().as_deref(), Some(b"abc".as_slice()));
        assert!(fim.load(Ordering::Acquire));
    }
}
