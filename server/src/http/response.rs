use axum::body::Body;
use axum::http::{HeaderValue, Response, StatusCode, header};
use serde::Serialize;

pub use super::transport::status_text;

pub(crate) const MIME_JSON: &str = "application/json";
pub(crate) const MIME_TEXT: &str = "text/plain; charset=UTF-8";

/// envelope JSON de erro da API
#[derive(Debug, Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct ErroResponse {
    pub sucesso: bool,
    pub mensagem: String,
}

#[derive(Debug, Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct HealthStatus {
    pub status: String,
}

/// erro HTTP com status e mensagem de apresentação (como `echo.HTTPError`)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpError {
    pub status: StatusCode,
    pub mensagem: String,
}

impl HttpError {
    pub fn new(status: StatusCode, mensagem: impl Into<String>) -> Self {
        Self {
            status,
            mensagem: mensagem.into(),
        }
    }

    /// erro com o texto padrão do status (`http.StatusText`)
    pub fn padrao(status: StatusCode) -> Self {
        Self::new(status, status_text(status))
    }

    pub fn bad_request(mensagem: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, mensagem)
    }

    /// 5xx nunca expõe o erro interno; 4xx mantém a mensagem de validação
    pub fn into_response(self) -> Response<Body> {
        let mensagem = if self.status.is_server_error() {
            status_text(self.status).to_string()
        } else {
            self.mensagem
        };
        json(
            self.status,
            &ErroResponse {
                sucesso: false,
                mensagem,
            },
        )
    }
}

/// JSON como o `encoding/json` do Go: HTML escapado e quebra de linha final
pub fn go_json<T: Serialize>(valor: &T) -> Vec<u8> {
    let texto = serde_json::to_string(valor).unwrap_or_else(|_| "null".into());
    let mut out = String::with_capacity(texto.len() + 1);
    for c in texto.chars() {
        match c {
            '<' => out.push_str("\\u003c"),
            '>' => out.push_str("\\u003e"),
            '&' => out.push_str("\\u0026"),
            '\u{2028}' => out.push_str("\\u2028"),
            '\u{2029}' => out.push_str("\\u2029"),
            _ => out.push(c),
        }
    }
    out.push('\n');
    out.into_bytes()
}

pub fn json<T: Serialize>(status: StatusCode, valor: &T) -> Response<Body> {
    blob(status, MIME_JSON, go_json(valor))
}

pub fn blob(status: StatusCode, content_type: &str, corpo: impl Into<Body>) -> Response<Body> {
    let mut resp = Response::new(corpo.into());
    *resp.status_mut() = status;
    if let Ok(v) = HeaderValue::from_str(content_type) {
        resp.headers_mut().insert(header::CONTENT_TYPE, v);
    }
    resp
}

pub fn vazio(status: StatusCode) -> Response<Body> {
    let mut resp = Response::new(Body::empty());
    *resp.status_mut() = status;
    resp
}

pub fn redirect(destino: &'static str) -> Response<Body> {
    let mut resp = vazio(StatusCode::MOVED_PERMANENTLY);
    resp.headers_mut()
        .insert(header::LOCATION, HeaderValue::from_static(destino));
    resp
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_escapa_html_como_go() {
        let corpo = go_json(&ErroResponse {
            sucesso: false,
            mensagem: "expected element type <NFSe> but have <x> & ok".into(),
        });
        assert_eq!(
            String::from_utf8_lossy(&corpo),
            "{\"sucesso\":false,\"mensagem\":\"expected element type \\u003cNFSe\\u003e but have \\u003cx\\u003e \\u0026 ok\"}\n"
        );
    }

    #[test]
    fn erro_5xx_e_generico() {
        let resp =
            HttpError::new(StatusCode::INTERNAL_SERVER_ERROR, "segredo <xml>").into_response();
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
        let resp = HttpError::new(StatusCode::BAD_GATEWAY, "x").into_response();
        assert_eq!(resp.status(), StatusCode::BAD_GATEWAY);
    }
}
