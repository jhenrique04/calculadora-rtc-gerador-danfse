use std::sync::Arc;

use axum::body::Body;
use axum::http::{HeaderValue, Request, Response, StatusCode, header};
use danfse_core::domain::enums::SituacaoNotaTipo;
use danfse_core::service::ServicoGeracaoPdf;
use http_body_util::{BodyExt, LengthLimitError};

use super::readiness::Readiness;
use super::response::{HealthStatus, HttpError, MIME_TEXT, blob, json, redirect, status_text};

const ERRO_STC: &str = "parâmetro stc inválido: deve ser 0, 1 ou 2";
const ERRO_CANHOTO: &str = "parâmetro canhoto inválido: deve ser true ou false";
const ERRO_XML_VAZIO: &str = "Xml não informado.";
const CONTENT_DISPOSITION_PDF: &str = r#"attachment; filename="danfse.pdf""#;
pub(crate) const SWAGGER_INDEX: &str = "/api/swagger-ui/index.html";

/// resultado de handler: erro vira resposta no error handler central
pub type Resultado = Result<Response<Body>, Falha>;

#[derive(Debug)]
pub enum Falha {
    Http(HttpError),
    /// panic dentro do handler (geração em thread bloqueante)
    Panic,
}

impl From<HttpError> for Falha {
    fn from(e: HttpError) -> Self {
        Self::Http(e)
    }
}

/// primeiro valor do parâmetro na query, como `QueryParam` do Echo
fn query_param(req: &Request<Body>, nome: &str) -> Option<String> {
    let query = req.uri().query()?;
    form_urlencoded::parse(query.as_bytes())
        .find(|(k, _)| k == nome)
        .map(|(_, v)| v.into_owned())
}

fn resolver_situacao(req: &Request<Body>) -> Result<SituacaoNotaTipo, HttpError> {
    let Some(stc) = query_param(req, "stc").filter(|s| !s.is_empty()) else {
        return Ok(SituacaoNotaTipo::Valida);
    };
    let valor = danfse_core::domain::utils_strings::parse_atoi(&stc)
        .map_err(|_| HttpError::bad_request(ERRO_STC))?;
    match SituacaoNotaTipo::por_stc(valor) {
        (s, true) => Ok(s),
        _ => Err(HttpError::bad_request(ERRO_STC)),
    }
}

fn resolver_canhoto(req: &Request<Body>) -> Result<bool, HttpError> {
    let Some(valor) = query_param(req, "canhoto").filter(|s| !s.is_empty()) else {
        return Ok(true);
    };
    match valor.to_lowercase().as_str() {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(HttpError::bad_request(ERRO_CANHOTO)),
    }
}

/// lê o corpo até o limite; estourar no meio do streaming é 400 com a
/// mensagem do 413, como o `BodyLimit` do Echo
async fn ler_corpo(body: Body, limite: usize) -> Result<Vec<u8>, HttpError> {
    match http_body_util::Limited::new(body, limite).collect().await {
        Ok(c) => Ok(c.to_bytes().to_vec()),
        Err(e) if e.downcast_ref::<LengthLimitError>().is_some() => Err(HttpError::bad_request(
            status_text(StatusCode::PAYLOAD_TOO_LARGE),
        )),
        Err(e) => Err(HttpError::bad_request(e.to_string())),
    }
}

pub async fn gerar_pdf(
    req: Request<Body>,
    servico: Arc<dyn ServicoGeracaoPdf>,
    limite: usize,
) -> Resultado {
    let situacao = resolver_situacao(&req)?;
    let canhoto = resolver_canhoto(&req)?;
    let xml = ler_corpo(req.into_body(), limite).await?;
    if String::from_utf8_lossy(&xml).trim().is_empty() {
        return Err(HttpError::bad_request(ERRO_XML_VAZIO).into());
    }
    // geração é CPU-bound: fora das threads do runtime
    let gerado =
        tokio::task::spawn_blocking(move || servico.executar(&xml, situacao, canhoto)).await;
    let pdf = match gerado {
        Ok(Ok(pdf)) => pdf,
        Ok(Err(e)) => return Err(HttpError::bad_request(e.to_string()).into()),
        Err(_) => return Err(Falha::Panic),
    };
    let mut resp = blob(StatusCode::OK, "application/pdf", pdf);
    resp.headers_mut().insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_static(CONTENT_DISPOSITION_PDF),
    );
    Ok(resp)
}

fn health_ok() -> Response<Body> {
    json(
        StatusCode::OK,
        &HealthStatus {
            status: "ok".into(),
        },
    )
}

pub fn live() -> Response<Body> {
    health_ok()
}

pub fn ready(readiness: &Readiness) -> Response<Body> {
    if readiness.is_ready() {
        return health_ok();
    }
    json(
        StatusCode::SERVICE_UNAVAILABLE,
        &HealthStatus {
            status: "unavailable".into(),
        },
    )
}

pub fn redirect_swagger() -> Response<Body> {
    redirect(SWAGGER_INDEX)
}

macro_rules! asset {
    ($nome:literal) => {
        (
            $nome,
            include_bytes!(concat!("../../assets/swagger-ui/", $nome)).as_slice(),
        )
    };
}

const ARQUIVOS_SWAGGER: &[(&str, &[u8])] = &[
    asset!("favicon-16x16.png"),
    asset!("favicon-32x32.png"),
    asset!("index.css"),
    asset!("oauth2-redirect.html"),
    asset!("swagger-initializer.js"),
    asset!("swagger-ui-bundle.js"),
    asset!("swagger-ui-es-bundle-core.js"),
    asset!("swagger-ui-es-bundle.js"),
    asset!("swagger-ui-standalone-preset.js"),
    asset!("swagger-ui.css"),
    asset!("swagger-ui.js"),
];
const INDEX_HTML: &[u8] = include_bytes!("../../assets/swagger-ui/index.html");
pub(crate) const DOC_JSON: &[u8] = include_bytes!("../../assets/swagger-ui/doc.json");
const DOC_YAML: &[u8] = include_bytes!("../../assets/swagger-ui/doc.yaml");

fn content_type_swagger(nome: &str) -> &'static str {
    match nome.rsplit_once('.').map(|(_, ext)| ext) {
        Some("html") => "text/html; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("js") => "application/javascript",
        Some("json") => "application/json; charset=utf-8",
        Some("yaml") => "text/plain; charset=utf-8",
        Some("png") => "image/png",
        _ => MIME_TEXT,
    }
}

/// arquivos da UI. O nome vem da URI após a última barra, incluindo a query,
/// como o regex do echo-swagger
pub fn swagger(req: &Request<Body>) -> Response<Body> {
    let uri = req.uri().path_and_query().map_or("", |p| p.as_str());
    let nome = uri.rsplit_once('/').map_or(uri, |(_, n)| n);
    let tipo = content_type_swagger(nome);
    let corpo: Option<&'static [u8]> = match nome {
        "index.html" => Some(INDEX_HTML),
        "doc.json" => Some(DOC_JSON),
        "doc.yaml" => Some(DOC_YAML),
        _ => ARQUIVOS_SWAGGER
            .iter()
            .find(|(n, _)| *n == nome)
            .map(|(_, b)| *b),
    };
    match corpo {
        Some(b) => blob(StatusCode::OK, tipo, b),
        None => blob(
            StatusCode::NOT_FOUND,
            tipo,
            status_text(StatusCode::NOT_FOUND),
        ),
    }
}
