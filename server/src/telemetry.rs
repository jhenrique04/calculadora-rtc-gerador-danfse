//! instrumentação OpenTelemetry opcional, espelhando o runtime Go.
//!
//! Sem OTEL_SDK_DISABLED (ou com "true") nada é exportado: só o contexto W3C
//! recebido é repassado aos logs, como o tracer no-op do Go. "false" liga o
//! SDK com exporters OTLP/HTTP (ou "none"). Erros crus do SDK nunca são
//! expostos, pois podem conter URL/credenciais do endpoint

mod sanitizing;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use axum::http;

use opentelemetry::metrics::{Histogram, Meter, MeterProvider as _};
use opentelemetry::propagation::{Extractor, TextMapPropagator};
use opentelemetry::trace::{
    Span as _, SpanKind, Status, TraceContextExt, Tracer as _, TracerProvider as _,
};
use opentelemetry::{Context, InstrumentationScope, KeyValue};
use opentelemetry_sdk::Resource;
use opentelemetry_sdk::metrics::SdkMeterProvider;
use opentelemetry_sdk::propagation::TraceContextPropagator;
use opentelemetry_sdk::trace::{SdkTracer, SdkTracerProvider};
use thiserror::Error;

pub const DEFAULT_SERVICE_NAME: &str = "danfse-service";
const SCOPE_NAME: &str = "github.com/labstack/echo-otel/v5";
const SCOPE_VERSION: &str = "5.0.0";
const EXPORTER_OTLP: &str = "otlp";
const EXPORTER_NONE: &str = "none";
const DURATION_BUCKETS: [f64; 14] = [
    0.005, 0.01, 0.025, 0.05, 0.075, 0.1, 0.25, 0.5, 0.75, 1.0, 2.5, 5.0, 7.5, 10.0,
];
const KNOWN_METHODS: [&str; 9] = [
    "CONNECT", "DELETE", "GET", "HEAD", "OPTIONS", "PATCH", "POST", "PUT", "TRACE",
];

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ErroTelemetria {
    #[error("telemetry: invalid OTEL_SDK_DISABLED value; expected \"true\" or \"false\"")]
    SdkDisabledInvalido,
    #[error("telemetry: unsupported {0} value; use \"otlp\" or \"none\"")]
    ExporterInvalido(&'static str),
    #[error("telemetry: create OTLP trace exporter failed")]
    ExporterTrace,
    #[error("telemetry: create OTLP metric exporter failed")]
    ExporterMetric,
    #[error("telemetry: shutdown {0} provider failed")]
    Shutdown(&'static str),
}

/// ids de correlação W3C usados nos logs
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpanIds {
    pub trace_id: String,
    pub span_id: String,
}

/// dados da requisição necessários para span e métricas
#[derive(Debug, Clone)]
pub struct RequestInfo {
    pub method: String,
    pub route: Option<&'static str>,
    pub protocol_version: &'static str,
    pub body_size: Option<u64>,
}

/// resultado da requisição já tratada pelo error handler
#[derive(Debug, Clone, Copy)]
pub struct ResponseInfo {
    pub status: u16,
    pub body_size: u64,
}

struct Instrumentos {
    tracer: SdkTracer,
    duration: Histogram<f64>,
    request_size: Histogram<u64>,
    response_size: Histogram<u64>,
    tracer_provider: SdkTracerProvider,
    meter_provider: SdkMeterProvider,
    encerrando: Arc<AtomicBool>,
}

pub struct Telemetry {
    instrumentos: Option<Instrumentos>,
    propagator: TraceContextPropagator,
}

/// span em andamento; `None` quando desabilitado ou probe
pub struct SpanAtivo {
    span: Option<opentelemetry_sdk::trace::Span>,
    pub ids: Option<SpanIds>,
}

impl Telemetry {
    pub fn from_env() -> Result<Self, ErroTelemetria> {
        Self::new(|k| std::env::var(k).ok())
    }

    pub fn new(lookup: impl Fn(&str) -> Option<String>) -> Result<Self, ErroTelemetria> {
        if sdk_disabled(lookup("OTEL_SDK_DISABLED"))? {
            return Ok(Self::disabled());
        }
        let traces = signal_exporter(lookup("OTEL_TRACES_EXPORTER"), "OTEL_TRACES_EXPORTER")?;
        let metrics = signal_exporter(lookup("OTEL_METRICS_EXPORTER"), "OTEL_METRICS_EXPORTER")?;
        let service_name = lookup("OTEL_SERVICE_NAME")
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| DEFAULT_SERVICE_NAME.into());
        let resource = Resource::builder().with_service_name(service_name).build();
        let tracer_provider = new_tracer_provider(resource.clone(), traces)?;
        let encerrando = Arc::new(AtomicBool::new(false));
        let meter_provider = match new_meter_provider(resource, metrics, Arc::clone(&encerrando)) {
            Ok(m) => m,
            Err(e) => {
                let _ = tracer_provider.shutdown();
                return Err(e);
            }
        };
        Ok(Self {
            instrumentos: Some(Instrumentos::new(
                tracer_provider,
                meter_provider,
                encerrando,
            )),
            propagator: TraceContextPropagator::new(),
        })
    }

    pub fn disabled() -> Self {
        Self {
            instrumentos: None,
            propagator: TraceContextPropagator::new(),
        }
    }

    /// abre o span do servidor (probes de health nunca são rastreados)
    pub fn start(&self, headers: &http::HeaderMap, info: &RequestInfo) -> SpanAtivo {
        if info.route.is_some_and(crate::http::is_probe_route) {
            return SpanAtivo {
                span: None,
                ids: None,
            };
        }
        let parent = self.propagator.extract(&HeaderExtractor(headers));
        let Some(inst) = &self.instrumentos else {
            // no-op do Go devolve o span remoto recebido
            return SpanAtivo {
                span: None,
                ids: span_ids(&parent),
            };
        };
        let span = inst
            .tracer
            .span_builder(span_name(info))
            .with_kind(SpanKind::Server)
            .with_attributes(atributos_comuns(info))
            .start_with_context(&inst.tracer, &parent);
        let ids = Some(ids_de(span.span_context()));
        SpanAtivo {
            span: Some(span),
            ids,
        }
    }

    pub fn finish(
        &self,
        ativo: SpanAtivo,
        info: &RequestInfo,
        resp: ResponseInfo,
        duracao: Duration,
    ) {
        let Some(inst) = &self.instrumentos else {
            return;
        };
        let Some(mut span) = ativo.span else {
            return;
        };
        let error_type = error_type(resp.status);
        let mut fim = vec![KeyValue::new(
            "http.response.status_code",
            i64::from(resp.status),
        )];
        if let Some(e) = &error_type {
            fim.push(KeyValue::new("error.type", e.clone()));
        }
        // descrição vazia: o exporter Go também apaga a mensagem do status
        if resp.status >= 500 {
            span.set_status(Status::error(""));
        }
        span.set_attributes(fim.clone());
        span.end();

        let mut attrs = atributos_comuns(info);
        attrs.extend(fim);
        inst.duration.record(duracao.as_secs_f64(), &attrs);
        if let Some(tamanho) = info.body_size {
            inst.request_size.record(tamanho, &attrs);
        }
        inst.response_size.record(resp.body_size, &attrs);
    }

    /// encerra providers; só o rótulo do provider aparece no erro
    pub fn shutdown(&self, timeout: Duration) -> Result<(), Vec<ErroTelemetria>> {
        let Some(inst) = &self.instrumentos else {
            return Ok(());
        };
        inst.encerrando.store(true, Ordering::Release);
        let mut erros = Vec::new();
        if inst.tracer_provider.shutdown_with_timeout(timeout).is_err() {
            erros.push(ErroTelemetria::Shutdown("trace"));
        }
        if inst.meter_provider.shutdown_with_timeout(timeout).is_err() {
            erros.push(ErroTelemetria::Shutdown("metric"));
        }
        if erros.is_empty() { Ok(()) } else { Err(erros) }
    }
}

impl Instrumentos {
    fn new(
        tracer_provider: SdkTracerProvider,
        meter_provider: SdkMeterProvider,
        encerrando: Arc<AtomicBool>,
    ) -> Self {
        let scope = InstrumentationScope::builder(SCOPE_NAME)
            .with_version(SCOPE_VERSION)
            .build();
        let tracer = tracer_provider.tracer_with_scope(scope.clone());
        let meter = meter_provider.meter_with_scope(scope);
        let (duration, request_size, response_size) = histogramas(&meter);
        Self {
            tracer,
            duration,
            request_size,
            response_size,
            tracer_provider,
            meter_provider,
            encerrando,
        }
    }
}

fn histogramas(meter: &Meter) -> (Histogram<f64>, Histogram<u64>, Histogram<u64>) {
    let duration = meter
        .f64_histogram("http.server.request.duration")
        .with_unit("s")
        .with_description("Duration of HTTP server requests.")
        .with_boundaries(DURATION_BUCKETS.to_vec())
        .build();
    let request_size = meter
        .u64_histogram("http.server.request.body.size")
        .with_unit("By")
        .with_description("Size of HTTP server request bodies.")
        .build();
    let response_size = meter
        .u64_histogram("http.server.response.body.size")
        .with_unit("By")
        .with_description("Size of HTTP server response bodies.")
        .build();
    (duration, request_size, response_size)
}

/// `strconv.ParseBool` do Go; ausente equivale a desabilitado
fn sdk_disabled(raw: Option<String>) -> Result<bool, ErroTelemetria> {
    match raw.as_deref() {
        None => Ok(true),
        Some("1" | "t" | "T" | "TRUE" | "true" | "True") => Ok(true),
        Some("0" | "f" | "F" | "FALSE" | "false" | "False") => Ok(false),
        Some(_) => Err(ErroTelemetria::SdkDisabledInvalido),
    }
}

fn signal_exporter(
    raw: Option<String>,
    chave: &'static str,
) -> Result<&'static str, ErroTelemetria> {
    match raw.as_deref() {
        None | Some(EXPORTER_OTLP) => Ok(EXPORTER_OTLP),
        Some(EXPORTER_NONE) => Ok(EXPORTER_NONE),
        Some(_) => Err(ErroTelemetria::ExporterInvalido(chave)),
    }
}

fn new_tracer_provider(
    resource: Resource,
    exporter: &str,
) -> Result<SdkTracerProvider, ErroTelemetria> {
    let builder = SdkTracerProvider::builder().with_resource(resource);
    if exporter == EXPORTER_NONE {
        return Ok(builder.build());
    }
    let exp = opentelemetry_otlp::SpanExporter::builder()
        .with_http()
        .build()
        .map_err(|_| ErroTelemetria::ExporterTrace)?;
    Ok(builder
        .with_batch_exporter(sanitizing::TraceExporter(exp))
        .build())
}

fn new_meter_provider(
    resource: Resource,
    exporter: &str,
    encerrando: Arc<AtomicBool>,
) -> Result<SdkMeterProvider, ErroTelemetria> {
    let builder = SdkMeterProvider::builder().with_resource(resource);
    if exporter == EXPORTER_NONE {
        return Ok(builder.build());
    }
    let exp = opentelemetry_otlp::MetricExporter::builder()
        .with_http()
        .build()
        .map_err(|_| ErroTelemetria::ExporterMetric)?;
    Ok(builder
        .with_periodic_exporter(sanitizing::MetricExporter {
            inner: exp,
            encerrando,
        })
        .build())
}

fn metodo_conhecido(method: &str) -> Option<&str> {
    KNOWN_METHODS.contains(&method).then_some(method)
}

fn span_name(info: &RequestInfo) -> String {
    let method = metodo_conhecido(&info.method).unwrap_or("HTTP");
    match info.route {
        Some(r) => format!("{method} {r}"),
        None => method.to_string(),
    }
}

/// só atributos de baixa cardinalidade e sem dados do cliente (allowlist do Go)
fn atributos_comuns(info: &RequestInfo) -> Vec<KeyValue> {
    let method = metodo_conhecido(&info.method)
        .unwrap_or("_OTHER")
        .to_string();
    let mut attrs = vec![
        KeyValue::new("http.request.method", method),
        KeyValue::new("url.scheme", "http"),
    ];
    if let Some(r) = info.route {
        attrs.push(KeyValue::new("http.route", r));
    }
    attrs.push(KeyValue::new(
        "network.protocol.version",
        info.protocol_version,
    ));
    attrs
}

/// `error.type` só para 5xx: 4xx não é erro em spans de servidor
fn error_type(status: u16) -> Option<String> {
    (status >= 500).then(|| status.to_string())
}

fn ids_de(sc: &opentelemetry::trace::SpanContext) -> SpanIds {
    SpanIds {
        trace_id: sc.trace_id().to_string(),
        span_id: sc.span_id().to_string(),
    }
}

fn span_ids(cx: &Context) -> Option<SpanIds> {
    let span = cx.span();
    let sc = span.span_context();
    sc.is_valid().then(|| ids_de(sc))
}

struct HeaderExtractor<'a>(&'a http::HeaderMap);

impl Extractor for HeaderExtractor<'_> {
    fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).and_then(|v| v.to_str().ok())
    }

    fn keys(&self) -> Vec<&str> {
        self.0.keys().map(http::HeaderName::as_str).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(pares: &'static [(&'static str, &'static str)]) -> impl Fn(&str) -> Option<String> {
        move |k| {
            pares
                .iter()
                .find(|(c, _)| *c == k)
                .map(|(_, v)| (*v).to_string())
        }
    }

    fn info(route: Option<&'static str>) -> RequestInfo {
        RequestInfo {
            method: "POST".into(),
            route,
            protocol_version: "1.1",
            body_size: Some(10),
        }
    }

    fn headers_com_traceparent() -> http::HeaderMap {
        let mut h = http::HeaderMap::new();
        h.insert(
            "traceparent",
            http::HeaderValue::from_static(
                "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01",
            ),
        );
        h
    }

    #[test]
    fn desabilitado_por_padrao() {
        assert!(Telemetry::new(env(&[])).is_ok_and(|t| !t.instrumentos.is_some()));
        assert!(
            Telemetry::new(env(&[("OTEL_SDK_DISABLED", "TRUE")]))
                .is_ok_and(|t| !t.instrumentos.is_some())
        );
    }

    #[test]
    fn valores_invalidos_falham_sem_ecoar() {
        let e = Telemetry::new(env(&[("OTEL_SDK_DISABLED", "talvez")])).err();
        assert_eq!(e, Some(ErroTelemetria::SdkDisabledInvalido));
        assert!(
            !e.map(|e| e.to_string())
                .unwrap_or_default()
                .contains("talvez")
        );
        let e = Telemetry::new(env(&[
            ("OTEL_SDK_DISABLED", ""),
            ("OTEL_TRACES_EXPORTER", "zipkin"),
        ]))
        .err();
        assert_eq!(e, Some(ErroTelemetria::SdkDisabledInvalido));
        let e = Telemetry::new(env(&[
            ("OTEL_SDK_DISABLED", "false"),
            ("OTEL_TRACES_EXPORTER", "zipkin"),
        ]))
        .err();
        assert_eq!(
            e.map(|e| e.to_string()),
            Some(
                "telemetry: unsupported OTEL_TRACES_EXPORTER value; use \"otlp\" or \"none\""
                    .into()
            )
        );
    }

    #[test]
    fn desabilitado_repassa_contexto_recebido() {
        let t = Telemetry::disabled();
        let ativo = t.start(&headers_com_traceparent(), &info(Some("/api/danfse")));
        assert_eq!(
            ativo.ids,
            Some(SpanIds {
                trace_id: "0af7651916cd43dd8448eb211c80319c".into(),
                span_id: "b7ad6b7169203331".into()
            })
        );
        let probe = t.start(&headers_com_traceparent(), &info(Some("/health/live")));
        assert!(probe.ids.is_none());
    }

    #[test]
    fn habilitado_cria_span_filho() {
        let t = Telemetry::new(env(&[
            ("OTEL_SDK_DISABLED", "false"),
            ("OTEL_TRACES_EXPORTER", "none"),
            ("OTEL_METRICS_EXPORTER", "none"),
        ]));
        let Ok(t) = t else { panic!("telemetria") };
        assert!(t.instrumentos.is_some());
        let i = info(Some("/api/danfse"));
        let ativo = t.start(&headers_com_traceparent(), &i);
        let ids = ativo.ids.clone();
        assert_eq!(
            ids.as_ref().map(|i| i.trace_id.as_str()),
            Some("0af7651916cd43dd8448eb211c80319c")
        );
        assert_ne!(ids.map(|i| i.span_id), Some("b7ad6b7169203331".into()));
        t.finish(
            ativo,
            &i,
            ResponseInfo {
                status: 200,
                body_size: 5,
            },
            Duration::from_millis(3),
        );
        assert!(t.shutdown(Duration::from_secs(1)).is_ok());
    }

    #[test]
    fn nomes_e_atributos() {
        assert_eq!(span_name(&info(Some("/api/danfse"))), "POST /api/danfse");
        let mut outro = info(None);
        outro.method = "BREW".into();
        assert_eq!(span_name(&outro), "HTTP");
        let attrs = atributos_comuns(&outro);
        assert!(attrs
            .iter()
            .any(|kv| kv.key.as_str() == "http.request.method" && kv.value.as_str() == "_OTHER"));
        assert!(!attrs.iter().any(|kv| kv.key.as_str() == "http.route"));
        assert_eq!(error_type(503).as_deref(), Some("503"));
        assert_eq!(error_type(404), None);
    }
}
