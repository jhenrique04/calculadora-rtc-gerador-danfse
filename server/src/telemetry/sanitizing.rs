//! exporters que trocam erros crus (que podem conter URL do collector) por
//! mensagens estáveis, como os wrappers do runtime Go

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use opentelemetry::trace::Status;
use opentelemetry_sdk::Resource;
use opentelemetry_sdk::error::{OTelSdkError, OTelSdkResult};
use opentelemetry_sdk::metrics::Temporality;
use opentelemetry_sdk::metrics::data::ResourceMetrics;
use opentelemetry_sdk::metrics::exporter::PushMetricExporter;
use opentelemetry_sdk::trace::{SpanData, SpanExporter};

const TRACE_EXPORT_FAILED: &str = "telemetry: trace export failed";
const METRICS_EXPORT_FAILED: &str = "telemetry: metrics export failed";

#[derive(Debug)]
pub struct TraceExporter<E>(pub E);

impl<E: SpanExporter> SpanExporter for TraceExporter<E> {
    async fn export(&self, mut batch: Vec<SpanData>) -> OTelSdkResult {
        // a descrição do status pode carregar texto cru de erro
        for span in &mut batch {
            if matches!(span.status, Status::Error { .. }) {
                span.status = Status::error("");
            }
        }
        self.0.export(batch).await.map_err(|_| {
            // o handler global do Go imprime o erro sanitizado via log padrão
            log::info!("{TRACE_EXPORT_FAILED}");
            OTelSdkError::InternalFailure(TRACE_EXPORT_FAILED.into())
        })
    }

    fn shutdown_with_timeout(&self, timeout: Duration) -> OTelSdkResult {
        self.0.shutdown_with_timeout(timeout)
    }

    fn force_flush(&self) -> OTelSdkResult {
        self.0.force_flush()
    }

    fn set_resource(&mut self, resource: &Resource) {
        self.0.set_resource(resource);
    }
}

pub struct MetricExporter<E> {
    pub inner: E,
    /// no shutdown o erro volta pelo retorno; só falhas periódicas são logadas
    pub encerrando: Arc<AtomicBool>,
}

impl<E: PushMetricExporter> PushMetricExporter for MetricExporter<E> {
    async fn export(&self, metrics: &ResourceMetrics) -> OTelSdkResult {
        self.inner.export(metrics).await.map_err(|_| {
            if !self.encerrando.load(Ordering::Acquire) {
                log::info!("{METRICS_EXPORT_FAILED}");
            }
            OTelSdkError::InternalFailure(METRICS_EXPORT_FAILED.into())
        })
    }

    fn force_flush(&self) -> OTelSdkResult {
        self.inner.force_flush()
    }

    fn shutdown_with_timeout(&self, timeout: Duration) -> OTelSdkResult {
        self.inner.shutdown_with_timeout(timeout)
    }

    fn temporality(&self) -> Temporality {
        self.inner.temporality()
    }
}
