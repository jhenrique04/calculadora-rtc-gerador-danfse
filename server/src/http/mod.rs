//! camada HTTP: roteamento, CORS, limite de corpo, recover, logging e
//! telemetria com o mesmo contrato do serviço Go (Echo).
//!
//! Correlação: o request ID é sempre gerado pelo servidor; o X-Request-ID do
//! cliente é descartado. Privacidade: nunca são logados corpo, PDF, query
//! string nem identificadores fiscais

mod handlers;
mod readiness;
mod response;
mod transport;

#[cfg(test)]
mod tests;

use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Instant;

use axum::Router;
use axum::body::{Body, HttpBody as _};
use axum::extract::State;
use axum::http::{HeaderMap, HeaderValue, Method, Request, Response, StatusCode, Version, header};
use chrono::Utc;
use danfse_core::service::{MAX_XML_INPUT_SIZE, ServicoGeracaoPdf};
use futures_util::FutureExt as _;
use rand::Rng as _;
use rand::distr::Alphanumeric;

use crate::logging::{Level, Logger, Valor};
use crate::telemetry::{RequestInfo, ResponseInfo, Telemetry};
use handlers::Falha;
pub use readiness::Readiness;
pub use response::HttpError;

pub const HEALTH_LIVE_PATH: &str = "/health/live";
pub const HEALTH_READY_PATH: &str = "/health/ready";
const DANFSE_PATH: &str = "/api/danfse";
const SWAGGER_ROOT: &str = "/api/swagger-ui";
const SWAGGER_ROOT_SLASH: &str = "/api/swagger-ui/";
const SWAGGER_FILES: &str = "/api/swagger-ui/*";
const ROUTE_UNMATCHED: &str = "unmatched";
const REQUEST_ID_LEN: usize = 32;
const CORS_ALLOW_METHODS: &str = "POST,OPTIONS";

pub fn is_probe_route(route: &str) -> bool {
    route == HEALTH_LIVE_PATH || route == HEALTH_READY_PATH
}

/// dependências da aplicação HTTP
pub struct App {
    servico: Arc<dyn ServicoGeracaoPdf>,
    logger: Logger,
    readiness: Readiness,
    body_limit: usize,
    telemetry: Arc<Telemetry>,
}

impl App {
    pub fn new(servico: Arc<dyn ServicoGeracaoPdf>) -> Self {
        Self {
            servico,
            logger: Logger::discard(),
            readiness: Readiness::default(),
            body_limit: usize::try_from(MAX_XML_INPUT_SIZE).unwrap_or(usize::MAX),
            telemetry: Arc::new(Telemetry::disabled()),
        }
    }

    pub fn with_logger(mut self, logger: Logger) -> Self {
        self.logger = logger;
        self
    }

    pub fn with_readiness(mut self, readiness: Readiness) -> Self {
        self.readiness = readiness;
        self
    }

    /// limites não positivos mantêm o padrão de 5 MiB
    pub fn with_body_limit(mut self, bytes: i64) -> Self {
        if let Ok(b) = usize::try_from(bytes)
            && b > 0
        {
            self.body_limit = b;
        }
        self
    }

    pub fn with_telemetry(mut self, telemetry: Arc<Telemetry>) -> Self {
        self.telemetry = telemetry;
        self
    }

    pub fn router(self) -> Router {
        Router::new().fallback(handle).with_state(Arc::new(self))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Rota {
    Live,
    Ready,
    Danfse,
    SwaggerRedirect,
    SwaggerArquivo,
}

#[derive(Debug, Clone, Copy)]
struct RotaInfo {
    rota: Rota,
    pattern: &'static str,
    metodo: &'static str,
}

fn resolver_rota(path: &str) -> Option<RotaInfo> {
    let (rota, pattern, metodo) = match path {
        HEALTH_LIVE_PATH => (Rota::Live, HEALTH_LIVE_PATH, "GET"),
        HEALTH_READY_PATH => (Rota::Ready, HEALTH_READY_PATH, "GET"),
        DANFSE_PATH => (Rota::Danfse, DANFSE_PATH, "POST"),
        SWAGGER_ROOT => (Rota::SwaggerRedirect, SWAGGER_ROOT, "GET"),
        SWAGGER_ROOT_SLASH => (Rota::SwaggerRedirect, SWAGGER_ROOT_SLASH, "GET"),
        p if p.starts_with(SWAGGER_ROOT_SLASH) => (Rota::SwaggerArquivo, SWAGGER_FILES, "GET"),
        _ => return None,
    };
    Some(RotaInfo {
        rota,
        pattern,
        metodo,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TipoErro {
    Http,
    Panic,
}

impl TipoErro {
    fn as_str(self) -> &'static str {
        match self {
            Self::Http => "http_error",
            Self::Panic => "panic",
        }
    }
}

fn gerar_request_id() -> String {
    rand::rng()
        .sample_iter(&Alphanumeric)
        .take(REQUEST_ID_LEN)
        .map(char::from)
        .collect()
}

fn versao_protocolo(v: Version) -> &'static str {
    match v {
        Version::HTTP_09 => "0.9",
        Version::HTTP_10 => "1.0",
        Version::HTTP_2 => "2",
        Version::HTTP_3 => "3",
        _ => "1.1",
    }
}

/// tamanho declarado do corpo; `None` para chunked (desconhecido)
fn tamanho_declarado(headers: &HeaderMap) -> Option<u64> {
    match headers.get(header::CONTENT_LENGTH) {
        Some(v) => v.to_str().ok().and_then(|s| s.parse().ok()),
        None if headers.contains_key(header::TRANSFER_ENCODING) => None,
        None => Some(0),
    }
}

async fn handle(State(app): State<Arc<App>>, req: Request<Body>) -> Response<Body> {
    let (mut parts, body) = req.into_parts();
    parts.headers.remove("x-request-id");
    let (body, corpo_lido) = transport::CorpoRastreado::envolver(body);
    let req = Request::from_parts(parts, body);
    let request_id = gerar_request_id();
    let inicio = Instant::now();
    let start_time = Utc::now();
    let rota = resolver_rota(req.uri().path());
    let info = RequestInfo {
        method: req.method().as_str().to_string(),
        route: rota.map(|r| r.pattern),
        protocol_version: versao_protocolo(req.version()),
        body_size: tamanho_declarado(req.headers()),
    };
    let span = app.telemetry.start(req.headers(), &info);

    let (mut resp, erro) = executar(&app, req, rota).await;
    if let Ok(v) = HeaderValue::from_str(&request_id) {
        resp.headers_mut().insert("x-request-id", v);
    }

    let resp_info = ResponseInfo {
        status: resp.status().as_u16(),
        body_size: resp.body().size_hint().exact().unwrap_or(0),
    };
    let ids = span.ids.clone();
    app.telemetry
        .finish(span, &info, resp_info, inicio.elapsed());

    let mut attrs: Vec<(&str, Valor)> = vec![
        ("timestamp", Valor::Time(start_time)),
        ("event", "request_completed".into()),
        ("request_id", request_id.into()),
        ("method", info.method.clone().into()),
        ("route", info.route.unwrap_or(ROUTE_UNMATCHED).into()),
        ("status", Valor::Int(i64::from(resp_info.status))),
        ("duration", Valor::Duration(inicio.elapsed())),
    ];
    if let Some(ids) = ids {
        attrs.push(("trace_id", ids.trace_id.into()));
        attrs.push(("span_id", ids.span_id.into()));
    }
    if let Some(e) = erro {
        attrs.push(("error_type", e.as_str().into()));
    }
    let mut level = nivel_por_status(resp.status());
    if info.route.is_some_and(is_probe_route) {
        attrs.push(("kind", "probe".into()));
        // probes são ruído operacional: só em debug
        level = Level::Debug;
    }
    app.logger.log(level, "request_completed", &attrs);
    let fechar = transport::deve_fechar(info.body_size, corpo_lido.load(Ordering::Acquire));
    transport::ajustar(resp, resp_info.body_size, fechar)
}

fn nivel_por_status(status: StatusCode) -> Level {
    if status.is_server_error() {
        Level::Error
    } else if status.is_client_error() {
        Level::Warn
    } else {
        Level::Info
    }
}

/// cabeçalhos que o Echo grava antes do handler e que sobrevivem a erros:
/// `Allow` do roteador e os de CORS
fn cabecalhos_previos(req: &Request<Body>, rota: Option<RotaInfo>) -> HeaderMap {
    let mut h = HeaderMap::new();
    if let Some(r) =
        rota.filter(|r| req.method() == Method::OPTIONS || req.method().as_str() != r.metodo)
        && let Ok(v) = HeaderValue::from_str(&format!("OPTIONS, {}", r.metodo))
    {
        h.insert(header::ALLOW, v);
    }
    h.append(header::VARY, HeaderValue::from_static("Origin"));
    if tem_origin(req) {
        h.insert(
            header::ACCESS_CONTROL_ALLOW_ORIGIN,
            HeaderValue::from_static("*"),
        );
    }
    h
}

fn tem_origin(req: &Request<Body>) -> bool {
    req.headers()
        .get(header::ORIGIN)
        .is_some_and(|v| !v.is_empty())
}

fn preflight(req: &Request<Body>, mut h: HeaderMap) -> Response<Body> {
    h.append(
        header::VARY,
        HeaderValue::from_static("Access-Control-Request-Method"),
    );
    h.append(
        header::VARY,
        HeaderValue::from_static("Access-Control-Request-Headers"),
    );
    h.insert(
        header::ACCESS_CONTROL_ALLOW_METHODS,
        HeaderValue::from_static(CORS_ALLOW_METHODS),
    );
    if let Some(pedidos) = req.headers().get(header::ACCESS_CONTROL_REQUEST_HEADERS) {
        h.insert(header::ACCESS_CONTROL_ALLOW_HEADERS, pedidos.clone());
    }
    com_cabecalhos(response::vazio(StatusCode::NO_CONTENT), h)
}

fn com_cabecalhos(mut resp: Response<Body>, extra: HeaderMap) -> Response<Body> {
    let mut nome_atual = None;
    for (nome, valor) in extra {
        if let Some(n) = nome {
            nome_atual = Some(n);
        }
        if let Some(n) = &nome_atual {
            resp.headers_mut().append(n.clone(), valor);
        }
    }
    resp
}

async fn executar(
    app: &App,
    req: Request<Body>,
    rota: Option<RotaInfo>,
) -> (Response<Body>, Option<TipoErro>) {
    let previos = cabecalhos_previos(&req, rota);
    if req.method() == Method::OPTIONS && tem_origin(&req) {
        return (preflight(&req, previos), None);
    }
    let resultado = AssertUnwindSafe(despachar(app, req, rota))
        .catch_unwind()
        .await
        .unwrap_or(Err(Falha::Panic));
    let (resp, erro) = match resultado {
        Ok(r) => (r, None),
        Err(Falha::Http(e)) => (e.into_response(), Some(TipoErro::Http)),
        Err(Falha::Panic) => (
            HttpError::padrao(StatusCode::INTERNAL_SERVER_ERROR).into_response(),
            Some(TipoErro::Panic),
        ),
    };
    (com_cabecalhos(resp, previos), erro)
}

fn excede_limite(req: &Request<Body>, limite: usize) -> bool {
    tamanho_declarado(req.headers()).is_some_and(|t| t > limite as u64)
}

async fn despachar(app: &App, req: Request<Body>, rota: Option<RotaInfo>) -> handlers::Resultado {
    if excede_limite(&req, app.body_limit) {
        return Err(HttpError::padrao(StatusCode::PAYLOAD_TOO_LARGE).into());
    }
    let Some(r) = rota.filter(|r| req.method().as_str() == r.metodo) else {
        if req.method() == Method::OPTIONS {
            return Ok(response::vazio(StatusCode::NO_CONTENT));
        }
        let status = if rota.is_some() {
            StatusCode::METHOD_NOT_ALLOWED
        } else {
            StatusCode::NOT_FOUND
        };
        return Err(HttpError::padrao(status).into());
    };
    match r.rota {
        Rota::Live => Ok(handlers::live()),
        Rota::Ready => Ok(handlers::ready(&app.readiness)),
        Rota::Danfse => handlers::gerar_pdf(req, Arc::clone(&app.servico), app.body_limit).await,
        Rota::SwaggerRedirect => Ok(handlers::redirect_swagger()),
        Rota::SwaggerArquivo => Ok(handlers::swagger(&req)),
    }
}
