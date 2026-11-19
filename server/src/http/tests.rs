use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use danfse_core::domain::enums::SituacaoNotaTipo;
use danfse_core::service::{ErroDanfse, ServicoGeracaoPdf};
use http_body_util::BodyExt;
use tower::ServiceExt as _;

use super::response::{ErroResponse, HealthStatus};
use super::*;
use crate::logging::Level;
use crate::logging::tests::{Captura, logger_capturado};

const PDF_FAKE: &[u8] = b"%PDF-fake";
const XML_MIN: &[u8] = include_bytes!("../../../core/tests/fixtures/paridade/min.xml");

#[derive(Default)]
struct Mock {
    erro: Option<ErroDanfse>,
    panica: bool,
    chamadas: std::sync::Mutex<Vec<(SituacaoNotaTipo, bool)>>,
}

impl ServicoGeracaoPdf for Mock {
    fn executar(
        &self,
        _: &[u8],
        situacao: SituacaoNotaTipo,
        canhoto: bool,
    ) -> Result<Vec<u8>, ErroDanfse> {
        assert!(!self.panica, "falha interna com dado fiscal 12345678000195");
        if let Ok(mut c) = self.chamadas.lock() {
            c.push((situacao, canhoto));
        }
        match &self.erro {
            Some(e) => Err(e.clone()),
            None => Ok(PDF_FAKE.to_vec()),
        }
    }
}

struct Resposta {
    status: StatusCode,
    headers: axum::http::HeaderMap,
    corpo: Vec<u8>,
}

impl Resposta {
    fn header(&self, nome: &str) -> Option<&str> {
        self.headers.get(nome).and_then(|v| v.to_str().ok())
    }

    fn headers_todos(&self, nome: &str) -> Vec<&str> {
        self.headers
            .get_all(nome)
            .iter()
            .filter_map(|v| v.to_str().ok())
            .collect()
    }

    fn erro(&self) -> Option<ErroResponse> {
        serde_json::from_slice(&self.corpo).ok()
    }
}

struct Cenario {
    app: App,
    captura: Captura,
    mock: Arc<Mock>,
}

fn cenario(mock: Mock) -> Cenario {
    let mock = Arc::new(mock);
    let (logger, captura) = logger_capturado(Level::Debug);
    let app = App::new(Arc::clone(&mock) as Arc<dyn ServicoGeracaoPdf>).with_logger(logger);
    Cenario { app, captura, mock }
}

async fn enviar(app: App, req: Request<Body>) -> Resposta {
    let resp = match app.router().oneshot(req).await {
        Ok(r) => r,
        Err(e) => match e {},
    };
    let status = resp.status();
    let headers = resp.headers().clone();
    let corpo = match resp.into_body().collect().await {
        Ok(c) => c.to_bytes().to_vec(),
        Err(e) => panic!("{e}"),
    };
    Resposta {
        status,
        headers,
        corpo,
    }
}

fn req(metodo: &str, uri: &str, corpo: impl Into<Body>) -> Request<Body> {
    match Request::builder()
        .method(metodo)
        .uri(uri)
        .body(corpo.into())
    {
        Ok(r) => r,
        Err(e) => panic!("{e}"),
    }
}

fn post(uri: &str, corpo: &'static [u8]) -> Request<Body> {
    req("POST", uri, corpo)
}

fn conclusoes(cap: &Captura) -> Vec<serde_json::Value> {
    cap.linhas()
        .into_iter()
        .filter(|l| l["event"] == "request_completed")
        .collect()
}

#[tokio::test]
async fn gera_pdf_com_headers() {
    let c = cenario(Mock::default());
    let r = enviar(c.app, post("/api/danfse", b"<xml>ok</xml>")).await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.header("content-type"), Some("application/pdf"));
    assert_eq!(
        r.header("content-disposition"),
        Some(r#"attachment; filename="danfse.pdf""#)
    );
    assert_eq!(r.corpo, PDF_FAKE);
    let chamadas = c
        .mock
        .chamadas
        .lock()
        .map(|c| c.clone())
        .unwrap_or_default();
    assert_eq!(chamadas, vec![(SituacaoNotaTipo::Valida, true)]);
}

#[tokio::test]
async fn repassa_stc_e_canhoto() {
    let c = cenario(Mock::default());
    let r = enviar(c.app, post("/api/danfse?stc=2&canhoto=FALSE", b"<x/>")).await;
    assert_eq!(r.status, StatusCode::OK);
    let chamadas = c
        .mock
        .chamadas
        .lock()
        .map(|c| c.clone())
        .unwrap_or_default();
    assert_eq!(chamadas, vec![(SituacaoNotaTipo::Cancelada, false)]);
}

#[tokio::test]
async fn erros_de_entrada_sao_400_estaveis() {
    for (uri, corpo, msg) in [
        ("/api/danfse", b"   ".as_slice(), "Xml não informado."),
        (
            "/api/danfse?stc=3",
            b"<x/>".as_slice(),
            "parâmetro stc inválido: deve ser 0, 1 ou 2",
        ),
        (
            "/api/danfse?stc=abc",
            b"<x/>".as_slice(),
            "parâmetro stc inválido: deve ser 0, 1 ou 2",
        ),
        (
            "/api/danfse?canhoto=1",
            b"<x/>".as_slice(),
            "parâmetro canhoto inválido: deve ser true ou false",
        ),
    ] {
        let r = enviar(cenario(Mock::default()).app, post(uri, corpo)).await;
        assert_eq!(r.status, StatusCode::BAD_REQUEST, "{uri}");
        assert_eq!(
            r.erro(),
            Some(ErroResponse {
                sucesso: false,
                mensagem: msg.into()
            }),
            "{uri}"
        );
        assert_eq!(r.header("content-type"), Some("application/json"));
    }
}

#[tokio::test]
async fn erro_do_servico_vira_400_com_mensagem() {
    let c = cenario(Mock {
        erro: Some(ErroDanfse::XmlContemDtd),
        ..Default::default()
    });
    let r = enviar(c.app, post("/api/danfse", b"<x/>")).await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);
    assert_eq!(
        r.corpo,
        b"{\"sucesso\":false,\"mensagem\":\"XML contains DTD or entity declaration\"}\n"
    );
}

#[tokio::test]
async fn servico_real_gera_pdf_e_erros_xml() {
    let servico = match danfse_core::app::new_pdf_service(None) {
        Ok(s) => Arc::new(s),
        Err(e) => panic!("{e}"),
    };
    let app = || App::new(Arc::clone(&servico) as Arc<dyn ServicoGeracaoPdf>);
    let r = enviar(app(), post("/api/danfse", XML_MIN)).await;
    assert_eq!(r.status, StatusCode::OK);
    assert!(r.corpo.starts_with(b"%PDF-1.7"));
    let r = enviar(app(), post("/api/danfse", b"<x/>")).await;
    assert_eq!(
        r.corpo,
        b"{\"sucesso\":false,\"mensagem\":\"expected element type \\u003cNFSe\\u003e but have \\u003cx\\u003e\"}\n"
    );
    let r = enviar(app(), post("/api/danfse", b"x")).await;
    assert_eq!(r.erro().map(|e| e.mensagem), Some("EOF".into()));
}

#[tokio::test]
async fn rotas_antigas_e_desconhecidas_sao_404() {
    for uri in [
        "/api/danfse/v1/pdf",
        "/api/danfse/v1/html",
        "/danfse-service/v1/pdf",
        "/health/live/",
    ] {
        let r = enviar(cenario(Mock::default()).app, post(uri, b"<x/>")).await;
        assert_eq!(r.status, StatusCode::NOT_FOUND, "{uri}");
        assert_eq!(r.erro().map(|e| e.mensagem), Some("Not Found".into()));
    }
}

#[tokio::test]
async fn metodo_errado_e_405_com_allow() {
    let r = enviar(
        cenario(Mock::default()).app,
        req("GET", "/api/danfse", Body::empty()),
    )
    .await;
    assert_eq!(r.status, StatusCode::METHOD_NOT_ALLOWED);
    assert_eq!(r.header("allow"), Some("OPTIONS, POST"));
    assert_eq!(
        r.erro().map(|e| e.mensagem),
        Some("Method Not Allowed".into())
    );
    let r = enviar(
        cenario(Mock::default()).app,
        req("HEAD", "/health/live", Body::empty()),
    )
    .await;
    assert_eq!(r.status, StatusCode::METHOD_NOT_ALLOWED);
    assert_eq!(r.header("allow"), Some("OPTIONS, GET"));
}

#[tokio::test]
async fn cors_e_vary() {
    let c = cenario(Mock::default());
    let mut r = req("POST", "/api/danfse", b"   ".as_slice());
    r.headers_mut().insert(
        header::ORIGIN,
        axum::http::HeaderValue::from_static("http://a.com"),
    );
    let resp = enviar(c.app, r).await;
    assert_eq!(resp.header("access-control-allow-origin"), Some("*"));
    assert_eq!(resp.headers_todos("vary"), vec!["Origin"]);

    let resp = enviar(
        cenario(Mock::default()).app,
        req("GET", "/health/live", Body::empty()),
    )
    .await;
    assert!(resp.header("access-control-allow-origin").is_none());
    assert_eq!(resp.headers_todos("vary"), vec!["Origin"]);
}

fn preflight(uri: &str) -> Request<Body> {
    match Request::builder()
        .method("OPTIONS")
        .uri(uri)
        .header("origin", "http://a.com")
        .header("access-control-request-method", "POST")
        .header("access-control-request-headers", "content-type,x-foo")
        .body(Body::empty())
    {
        Ok(r) => r,
        Err(e) => panic!("{e}"),
    }
}

#[tokio::test]
async fn preflight_204() {
    let r = enviar(cenario(Mock::default()).app, preflight("/api/danfse")).await;
    assert_eq!(r.status, StatusCode::NO_CONTENT);
    assert_eq!(r.header("access-control-allow-origin"), Some("*"));
    assert_eq!(
        r.header("access-control-allow-methods"),
        Some("POST,OPTIONS")
    );
    assert_eq!(
        r.header("access-control-allow-headers"),
        Some("content-type,x-foo")
    );
    assert_eq!(r.header("allow"), Some("OPTIONS, POST"));
    assert_eq!(
        r.headers_todos("vary"),
        vec![
            "Origin",
            "Access-Control-Request-Method",
            "Access-Control-Request-Headers"
        ]
    );
    assert!(r.header("x-request-id").is_some());

    let r = enviar(cenario(Mock::default()).app, preflight("/nao/existe")).await;
    assert_eq!(r.status, StatusCode::NO_CONTENT);
    assert!(r.header("allow").is_none());

    let r = enviar(
        cenario(Mock::default()).app,
        req("OPTIONS", "/api/danfse", Body::empty()),
    )
    .await;
    assert_eq!(r.status, StatusCode::NO_CONTENT);
    assert_eq!(r.header("allow"), Some("OPTIONS, POST"));
    assert!(r.header("access-control-allow-methods").is_none());
}

#[tokio::test]
async fn limite_de_corpo() {
    let grande = vec![b'a'; 2048];
    let app = || cenario(Mock::default()).app.with_body_limit(1024);
    let r = enviar(app(), req("POST", "/api/danfse", b"<x/>".as_slice())).await;
    assert_eq!(r.status, StatusCode::OK);

    let mut declarado = req("POST", "/api/danfse", grande.clone());
    declarado.headers_mut().insert(
        header::CONTENT_LENGTH,
        axum::http::HeaderValue::from(grande.len()),
    );
    let r = enviar(app(), declarado).await;
    assert_eq!(r.status, StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(
        r.erro().map(|e| e.mensagem),
        Some("Request Entity Too Large".into())
    );

    // sem Content-Length (streaming): estoura na leitura e vira 400
    let stream =
        futures_util::stream::iter(vec![Ok::<_, std::io::Error>(bytes::Bytes::from(grande))]);
    let r = enviar(app(), req("POST", "/api/danfse", Body::from_stream(stream))).await;
    assert_eq!(r.status, StatusCode::BAD_REQUEST);
    assert_eq!(
        r.erro().map(|e| e.mensagem),
        Some("Request Entity Too Large".into())
    );

    let r = enviar(
        cenario(Mock::default()).app.with_body_limit(0),
        post("/api/danfse", b"<x/>"),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);
}

#[tokio::test]
async fn health_probes() {
    let r = enviar(
        cenario(Mock::default()).app,
        req("GET", "/health/live", Body::empty()),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.corpo, b"{\"status\":\"ok\"}\n");
    assert_eq!(r.header("content-type"), Some("application/json"));

    let readiness = Readiness::new(true);
    let app = || {
        cenario(Mock::default())
            .app
            .with_readiness(readiness.clone())
    };
    let r = enviar(app(), req("GET", "/health/ready", Body::empty())).await;
    assert_eq!(r.status, StatusCode::OK);
    readiness.set_ready(false);
    let r = enviar(app(), req("GET", "/health/ready", Body::empty())).await;
    assert_eq!(r.status, StatusCode::SERVICE_UNAVAILABLE);
    let corpo: Option<HealthStatus> = serde_json::from_slice(&r.corpo).ok();
    assert_eq!(
        corpo,
        Some(HealthStatus {
            status: "unavailable".into()
        })
    );
    // live não depende de readiness
    let r = enviar(app(), req("GET", "/health/live", Body::empty())).await;
    assert_eq!(r.status, StatusCode::OK);
}

#[tokio::test]
async fn swagger_ui() {
    for uri in ["/api/swagger-ui", "/api/swagger-ui/"] {
        let r = enviar(cenario(Mock::default()).app, req("GET", uri, Body::empty())).await;
        assert_eq!(r.status, StatusCode::MOVED_PERMANENTLY, "{uri}");
        assert_eq!(r.header("location"), Some("/api/swagger-ui/index.html"));
    }
    let r = enviar(
        cenario(Mock::default()).app,
        req("GET", "/api/swagger-ui/index.html", Body::empty()),
    )
    .await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.header("content-type"), Some("text/html; charset=utf-8"));
    assert!(String::from_utf8_lossy(&r.corpo).contains("swagger-ui-bundle.js"));

    let r = enviar(
        cenario(Mock::default()).app,
        req("GET", "/api/swagger-ui/swagger-ui.css", Body::empty()),
    )
    .await;
    assert_eq!(
        (r.status, r.header("content-type")),
        (StatusCode::OK, Some("text/css; charset=utf-8"))
    );

    let r = enviar(
        cenario(Mock::default()).app,
        req("GET", "/api/swagger-ui/nada", Body::empty()),
    )
    .await;
    assert_eq!(r.status, StatusCode::NOT_FOUND);
    assert_eq!(r.header("content-type"), Some("text/plain; charset=UTF-8"));
    assert_eq!(r.corpo, b"Not Found");
}

#[test]
fn swagger_documenta_rotas_e_health() {
    let doc: serde_json::Value = match serde_json::from_slice(handlers::DOC_JSON) {
        Ok(d) => d,
        Err(e) => panic!("{e}"),
    };
    assert_eq!(doc["basePath"], "/");
    for p in ["/api/danfse", "/health/live", "/health/ready"] {
        assert!(doc["paths"][p].is_object(), "{p}");
    }
    assert_eq!(doc["paths"]["/health/live"]["get"]["tags"][0], "Health");
    assert!(doc["paths"]["/health/ready"]["get"]["responses"]["503"].is_object());
    assert!(doc["definitions"]["controller.HealthStatus"]["properties"]["status"].is_object());
}

#[tokio::test]
async fn request_id_sempre_gerado_pelo_servidor() {
    let c = cenario(Mock::default());
    let mut r = req("GET", "/health/live", Body::empty());
    r.headers_mut().insert(
        "x-request-id",
        axum::http::HeaderValue::from_static("cliente-123"),
    );
    let resp = enviar(c.app, r).await;
    let rid = resp.header("x-request-id").unwrap_or_default().to_string();
    assert_eq!(rid.len(), 32);
    assert!(rid.chars().all(|ch| ch.is_ascii_alphanumeric()));
    assert_ne!(rid, "cliente-123");
    let linhas = conclusoes(&c.captura);
    assert_eq!(linhas[0]["request_id"], rid.as_str());
}

#[tokio::test]
async fn log_de_conclusao_unico_com_campos() {
    let c = cenario(Mock::default());
    let resp = enviar(
        c.app,
        post("/api/danfse?stc=1", b"<xml>12345678000195</xml>"),
    )
    .await;
    let linhas = conclusoes(&c.captura);
    assert_eq!(linhas.len(), 1);
    let l = &linhas[0];
    assert_eq!(l["level"], "INFO");
    assert_eq!(l["msg"], "request_completed");
    assert_eq!(l["method"], "POST");
    assert_eq!(l["route"], "/api/danfse");
    assert_eq!(l["status"], 200);
    assert!(l["duration"].is_u64());
    assert!(l["timestamp"].as_str().is_some_and(|t| t.ends_with('Z')));
    assert_eq!(l["request_id"].as_str(), resp.header("x-request-id"));
    assert!(l.get("error_type").is_none() && l.get("trace_id").is_none());
    // redação: nunca corpo, query nem identificadores fiscais
    let bruto = c
        .captura
        .0
        .lock()
        .map(|v| String::from_utf8_lossy(&v).into_owned())
        .unwrap_or_default();
    assert!(
        !bruto.contains("12345678000195") && !bruto.contains("stc=") && !bruto.contains("<xml>")
    );
}

#[tokio::test]
async fn log_rota_sem_match_e_erros() {
    let c = cenario(Mock::default());
    let _ = enviar(c.app, req("GET", "/nao/existe/123", Body::empty())).await;
    let l = &conclusoes(&c.captura)[0];
    assert_eq!(l["route"], "unmatched");
    assert_eq!(l["level"], "WARN");
    assert_eq!(l["status"], 404);
    assert_eq!(l["error_type"], "http_error");
}

#[tokio::test]
async fn panic_recuperado_vira_500_generico() {
    let c = cenario(Mock {
        panica: true,
        ..Default::default()
    });
    let r = enviar(c.app, post("/api/danfse", b"<x/>")).await;
    assert_eq!(r.status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(
        r.erro().map(|e| e.mensagem),
        Some("Internal Server Error".into())
    );
    assert_eq!(r.headers_todos("vary"), vec!["Origin"]);
    let l = &conclusoes(&c.captura)[0];
    assert_eq!(l["level"], "ERROR");
    assert_eq!(l["error_type"], "panic");
    let bruto = c
        .captura
        .0
        .lock()
        .map(|v| String::from_utf8_lossy(&v).into_owned())
        .unwrap_or_default();
    assert!(!bruto.contains("12345678000195"));
}

#[tokio::test]
async fn probe_logado_em_debug_com_kind() {
    let c = cenario(Mock::default());
    let _ = enviar(c.app, req("GET", "/health/ready", Body::empty())).await;
    let l = &conclusoes(&c.captura)[0];
    assert_eq!(l["level"], "DEBUG");
    assert_eq!(l["kind"], "probe");
}

#[tokio::test]
async fn trace_id_do_contexto_recebido() {
    let c = cenario(Mock::default());
    let mut r = req("GET", "/api/swagger-ui", Body::empty());
    r.headers_mut().insert(
        "traceparent",
        axum::http::HeaderValue::from_static(
            "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01",
        ),
    );
    let _ = enviar(c.app, r).await;
    let l = &conclusoes(&c.captura)[0];
    assert_eq!(l["trace_id"], "0af7651916cd43dd8448eb211c80319c");
    assert_eq!(l["span_id"], "b7ad6b7169203331");
}

#[tokio::test]
async fn logger_padrao_e_silencioso() {
    let app = App::new(Arc::new(Mock::default()));
    let r = enviar(app, req("GET", "/health/live", Body::empty())).await;
    assert_eq!(r.status, StatusCode::OK);
}
