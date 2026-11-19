//! servidor HTTP do DANFSe: POST /api/danfse (XML da NFS-e → PDF), probes de
//! health e Swagger UI

mod config;
mod http;
mod logging;
mod telemetry;

use std::net::{Ipv6Addr, SocketAddr};
use std::process::ExitCode;
use std::sync::Arc;
use std::time::Duration;

use danfse_core::app::new_pdf_service;
use hyper_util::rt::{TokioExecutor, TokioIo, TokioTimer};
use hyper_util::server::conn::auto;
use hyper_util::server::graceful::GracefulShutdown;
use hyper_util::service::TowerToHyperService;
use tokio::net::TcpListener;

use config::Env;
use http::{App, Readiness};
use logging::{Logger, Valor, parse_level};
use telemetry::Telemetry;

// sem timeout curto de escrita: gerar PDF pode demorar
const READ_HEADER_TIMEOUT: Duration = Duration::from_secs(10);
const HTTP_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(10);
const TELEMETRY_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);
const DEFAULT_LOG_LEVEL: &str = "info";
const ERRO_CONFIG_TELEMETRIA: &str =
    "configuração de telemetria inválida (OTEL_*); corrija as variáveis e reinicie";

fn main() -> ExitCode {
    // logs de boot também saem em JSON
    let mut logger = Logger::stderr(parse_level(DEFAULT_LOG_LEVEL));
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => return falhar(&logger, &e.to_string()),
    };
    match runtime.block_on(run(&mut logger)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => falhar(&logger, &e),
    }
}

fn falhar(logger: &Logger, erro: &str) -> ExitCode {
    logger.error(
        "servidor danfse finalizado com erro",
        &[("error", erro.into())],
    );
    ExitCode::FAILURE
}

fn instalar_ponte_log(logger: &Logger, nivel: &str) {
    let filtro = match parse_level(nivel) {
        logging::Level::Debug => log::LevelFilter::Debug,
        logging::Level::Info => log::LevelFilter::Info,
        logging::Level::Warn => log::LevelFilter::Warn,
        logging::Level::Error => log::LevelFilter::Error,
    };
    if log::set_boxed_logger(Box::new(logger.clone())).is_ok() {
        log::set_max_level(filtro);
    }
}

async fn run(logger: &mut Logger) -> Result<(), String> {
    let env = Env::carregar().map_err(|e| e.to_string())?;
    *logger = Logger::stderr(parse_level(&env.log_level));
    instalar_ponte_log(logger, &env.log_level);
    let readiness = Readiness::new(true);

    // config OTel inválida falha o boot; o erro do SDK nunca é exposto
    let Ok(telemetry) = Telemetry::from_env() else {
        logger.error(ERRO_CONFIG_TELEMETRIA, &[]);
        return Err(ERRO_CONFIG_TELEMETRIA.into());
    };
    let telemetry = Arc::new(telemetry);
    let resultado = servir(logger, &env, &readiness, Arc::clone(&telemetry)).await;
    shutdown_telemetry(&telemetry, logger).await;
    resultado
}

async fn servir(
    logger: &Logger,
    env: &Env,
    readiness: &Readiness,
    telemetry: Arc<Telemetry>,
) -> Result<(), String> {
    let servico = new_pdf_service(Some(env.xml_max_size_bytes)).map_err(|e| e.to_string())?;
    let router = App::new(Arc::new(servico))
        .with_logger(logger.clone())
        .with_readiness(readiness.clone())
        .with_body_limit(env.xml_max_size_bytes)
        .with_telemetry(telemetry)
        .router();

    let porta = u16::try_from(env.port)
        .map_err(|_| format!("listen tcp: address {}: invalid port", env.port))?;
    // bind antes de anunciar "online"
    let listener = TcpListener::bind(SocketAddr::from((Ipv6Addr::UNSPECIFIED, porta)))
        .await
        .map_err(|e| format!("listen tcp :{porta}: {e}"))?;

    logger.info(
        "servidor danfse online",
        &[
            ("port", porta.to_string().into()),
            ("log_level", env.log_level.clone().into()),
        ],
    );
    logger.info(
        "swagger disponível",
        &[(
            "url",
            format!("http://localhost:{porta}/api/swagger-ui/index.html").into(),
        )],
    );

    let graceful = aceitar_ate_sinal(listener, router, logger).await;
    logger.info(
        "sinal de encerramento recebido; iniciando graceful shutdown",
        &[],
    );
    // só /health/ready passa a 503; requisições em voo terminam normalmente
    readiness.set_ready(false);
    match tokio::time::timeout(HTTP_SHUTDOWN_TIMEOUT, graceful.shutdown()).await {
        Ok(()) => logger.info("servidor HTTP encerrado", &[]),
        Err(_) => logger.error(
            "erro ao encerrar servidor HTTP",
            &[("error", "context deadline exceeded".into())],
        ),
    }
    Ok(())
}

async fn aceitar_ate_sinal(
    listener: TcpListener,
    router: axum::Router,
    logger: &Logger,
) -> GracefulShutdown {
    let graceful = GracefulShutdown::new();
    let mut builder = auto::Builder::new(TokioExecutor::new());
    builder
        .http1()
        // nomes de header no formato canônico do Go (X-Request-Id)
        .title_case_headers(true)
        .timer(TokioTimer::new())
        .header_read_timeout(READ_HEADER_TIMEOUT);
    let sinal = sinal_encerramento();
    tokio::pin!(sinal);
    loop {
        tokio::select! {
            aceito = listener.accept() => {
                let stream = match aceito {
                    Ok((s, _)) => s,
                    Err(e) => {
                        logger.error("http: Accept error", &[("error", Valor::Str(e.to_string()))]);
                        continue;
                    }
                };
                let servico = TowerToHyperService::new(router.clone());
                let conexao = builder.serve_connection(TokioIo::new(stream), servico).into_owned();
                let conexao = graceful.watch(conexao);
                tokio::spawn(async move {
                    // erro de conexão (cliente caiu) não afeta o servidor
                    let _ = conexao.await;
                });
            }
            () = &mut sinal => break,
        }
    }
    graceful
}

async fn sinal_encerramento() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    #[cfg(unix)]
    let term = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut s) => {
                s.recv().await;
            }
            Err(_) => std::future::pending::<()>().await,
        }
    };
    #[cfg(not(unix))]
    let term = std::future::pending::<()>();
    tokio::select! {
        () = ctrl_c => {},
        () = term => {},
    }
}

/// encerra a telemetria com prazo; nunca ecoa o erro do SDK
async fn shutdown_telemetry(telemetry: &Arc<Telemetry>, logger: &Logger) {
    let t = Arc::clone(telemetry);
    let resultado =
        tokio::task::spawn_blocking(move || t.shutdown(TELEMETRY_SHUTDOWN_TIMEOUT)).await;
    if !matches!(resultado, Ok(Ok(()))) {
        logger.warn(
            "falha ao encerrar telemetria; reportar sem detalhes de configuração",
            &[],
        );
    }
}
