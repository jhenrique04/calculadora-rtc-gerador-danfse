//! logger JSON estruturado no formato do `slog.JSONHandler` do Go: uma linha
//! por registro com `time`, `level`, `msg` e os atributos na ordem dada.
//!
//! Privacidade: quem chama nunca passa corpo (XML), PDF, query string nem
//! identificadores fiscais

use std::fmt::Write as _;
use std::io::Write;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use chrono::{DateTime, Local, SecondsFormat, TimeZone, Utc};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    Debug,
    Info,
    Warn,
    Error,
}

impl Level {
    fn as_str(self) -> &'static str {
        match self {
            Self::Debug => "DEBUG",
            Self::Info => "INFO",
            Self::Warn => "WARN",
            Self::Error => "ERROR",
        }
    }
}

/// converte LOG_LEVEL; vazio ou desconhecido vira INFO
pub fn parse_level(level: &str) -> Level {
    match level.trim().to_lowercase().as_str() {
        "debug" => Level::Debug,
        "warn" => Level::Warn,
        "error" => Level::Error,
        _ => Level::Info,
    }
}

pub fn is_valid_level(level: &str) -> bool {
    matches!(
        level.trim().to_lowercase().as_str(),
        "debug" | "info" | "warn" | "error"
    )
}

#[derive(Debug, Clone, PartialEq)]
pub enum Valor {
    Str(String),
    Int(i64),
    Time(DateTime<Utc>),
    Duration(Duration),
}

impl From<&str> for Valor {
    fn from(s: &str) -> Self {
        Self::Str(s.into())
    }
}

impl From<String> for Valor {
    fn from(s: String) -> Self {
        Self::Str(s)
    }
}

impl From<i64> for Valor {
    fn from(v: i64) -> Self {
        Self::Int(v)
    }
}

const TARGET_PROPRIO: &str = "danfse";

type Sink = Arc<Mutex<Box<dyn Write + Send>>>;

#[derive(Clone)]
pub struct Logger {
    level: Level,
    sink: Option<Sink>,
}

impl std::fmt::Debug for Logger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Logger")
            .field("level", &self.level)
            .finish()
    }
}

impl Logger {
    pub fn stderr(level: Level) -> Self {
        Self::with_writer(level, Box::new(std::io::stderr()))
    }

    pub fn with_writer(level: Level, writer: Box<dyn Write + Send>) -> Self {
        Self {
            level,
            sink: Some(Arc::new(Mutex::new(writer))),
        }
    }

    /// descarta tudo; padrão do router quando nenhum logger é injetado
    pub fn discard() -> Self {
        Self {
            level: Level::Error,
            sink: None,
        }
    }

    pub fn enabled(&self, level: Level) -> bool {
        self.sink.is_some() && level >= self.level
    }

    pub fn log(&self, level: Level, msg: &str, attrs: &[(&str, Valor)]) {
        let Some(sink) = self.sink.as_ref().filter(|_| level >= self.level) else {
            return;
        };
        let linha = formatar_linha(Local::now(), level, msg, attrs);
        // falha de escrita no log não pode derrubar a requisição
        if let Ok(mut w) = sink.lock() {
            let _ = w.write_all(linha.as_bytes());
        }
    }

    pub fn info(&self, msg: &str, attrs: &[(&str, Valor)]) {
        self.log(Level::Info, msg, attrs);
    }

    pub fn warn(&self, msg: &str, attrs: &[(&str, Valor)]) {
        self.log(Level::Warn, msg, attrs);
    }

    pub fn error(&self, msg: &str, attrs: &[(&str, Valor)]) {
        self.log(Level::Error, msg, attrs);
    }
}

/// encaminha o crate `log` (usado pelo core) ao logger JSON, como o
/// `log.Printf` do Go depois de `slog.SetDefault`
impl log::Log for Logger {
    // só registros do próprio código: dependências (hyper, reqwest) não logam no Go
    fn enabled(&self, metadata: &log::Metadata<'_>) -> bool {
        metadata.target().starts_with(TARGET_PROPRIO) && self.enabled(nivel_log(metadata.level()))
    }

    fn log(&self, record: &log::Record<'_>) {
        if log::Log::enabled(self, record.metadata()) {
            self.log(nivel_log(record.level()), &record.args().to_string(), &[]);
        }
    }

    fn flush(&self) {
        if let Some(Ok(mut w)) = self.sink.as_ref().map(|s| s.lock()) {
            let _ = w.flush();
        }
    }
}

fn nivel_log(level: log::Level) -> Level {
    match level {
        log::Level::Error => Level::Error,
        log::Level::Warn => Level::Warn,
        log::Level::Info => Level::Info,
        log::Level::Debug | log::Level::Trace => Level::Debug,
    }
}

/// RFC3339 com nanossegundos sem zeros à direita (`time.RFC3339Nano`)
pub fn rfc3339_nano<Tz: TimeZone>(t: &DateTime<Tz>) -> String
where
    Tz::Offset: std::fmt::Display,
{
    let base = t.to_rfc3339_opts(SecondsFormat::Nanos, true);
    // separa "AAAA-MM-DDTHH:MM:SS" + ".fffffffff" + offset para aparar a fração
    let Some(ponto) = base.find('.') else {
        return base;
    };
    let fim_fracao = base[ponto + 1..]
        .find(|c: char| !c.is_ascii_digit())
        .map_or(base.len(), |i| ponto + 1 + i);
    let fracao = base[ponto + 1..fim_fracao].trim_end_matches('0');
    let offset = &base[fim_fracao..];
    if fracao.is_empty() {
        format!("{}{offset}", &base[..ponto])
    } else {
        format!("{}.{fracao}{offset}", &base[..ponto])
    }
}

fn formatar_linha(
    agora: DateTime<Local>,
    level: Level,
    msg: &str,
    attrs: &[(&str, Valor)],
) -> String {
    let mut s = String::with_capacity(160);
    s.push_str("{\"time\":");
    push_json_str(&mut s, &rfc3339_nano(&agora));
    s.push_str(",\"level\":");
    push_json_str(&mut s, level.as_str());
    s.push_str(",\"msg\":");
    push_json_str(&mut s, msg);
    for (chave, valor) in attrs {
        s.push(',');
        push_json_str(&mut s, chave);
        s.push(':');
        match valor {
            Valor::Str(v) => push_json_str(&mut s, v),
            Valor::Int(v) => {
                let _ = write!(s, "{v}");
            }
            Valor::Time(t) => push_json_str(&mut s, &rfc3339_nano(t)),
            // slog serializa time.Duration como inteiro em nanossegundos
            Valor::Duration(d) => {
                let _ = write!(s, "{}", d.as_nanos());
            }
        }
    }
    s.push_str("}\n");
    s
}

fn push_json_str(s: &mut String, v: &str) {
    match serde_json::to_string(v) {
        Ok(json) => s.push_str(&json),
        Err(_) => s.push_str("\"\""),
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// writer em memória compartilhado para inspecionar os logs nos testes
    #[derive(Clone, Default)]
    pub struct Captura(pub Arc<Mutex<Vec<u8>>>);

    impl Write for Captura {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            if let Ok(mut v) = self.0.lock() {
                v.extend_from_slice(buf);
            }
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl Captura {
        pub fn linhas(&self) -> Vec<serde_json::Value> {
            let texto = self
                .0
                .lock()
                .map(|v| String::from_utf8_lossy(&v).into_owned())
                .unwrap_or_default();
            texto
                .lines()
                .filter_map(|l| serde_json::from_str(l).ok())
                .collect()
        }
    }

    pub fn logger_capturado(level: Level) -> (Logger, Captura) {
        let cap = Captura::default();
        (Logger::with_writer(level, Box::new(cap.clone())), cap)
    }

    #[test]
    fn niveis() {
        assert_eq!(parse_level("DEBUG"), Level::Debug);
        assert_eq!(parse_level(" warn "), Level::Warn);
        assert_eq!(parse_level("error"), Level::Error);
        assert_eq!(parse_level(""), Level::Info);
        assert_eq!(parse_level("xpto"), Level::Info);
        assert!(is_valid_level("Info") && !is_valid_level("trace"));
    }

    #[test]
    fn formata_como_slog() {
        let (logger, cap) = logger_capturado(Level::Info);
        logger.info(
            "request_completed",
            &[
                ("status", Valor::Int(200)),
                ("route", "/api/danfse".into()),
                ("duration", Valor::Duration(Duration::from_micros(1500))),
            ],
        );
        logger.log(Level::Debug, "oculto", &[]);
        let linhas = cap.linhas();
        assert_eq!(linhas.len(), 1);
        let l = &linhas[0];
        assert_eq!(l["level"], "INFO");
        assert_eq!(l["msg"], "request_completed");
        assert_eq!(l["status"], 200);
        assert_eq!(l["duration"], 1_500_000);
        let ordem: Vec<&str> = l
            .as_object()
            .map(|o| o.keys().map(String::as_str).collect())
            .unwrap_or_default();
        assert!(ordem.contains(&"time"));
    }

    #[test]
    fn rfc3339_nano_apara_zeros() {
        let t = Utc.with_ymd_and_hms(2026, 11, 18, 17, 20, 43).single();
        let Some(t) = t else { panic!("data inválida") };
        assert_eq!(rfc3339_nano(&t), "2026-11-18T17:20:43Z");
        let t = t + chrono::Duration::microseconds(308_400);
        assert_eq!(rfc3339_nano(&t), "2026-11-18T17:20:43.3084Z");
    }

    #[test]
    fn ponte_do_crate_log() {
        let (logger, cap) = logger_capturado(Level::Info);
        log::Log::log(
            &logger,
            &log::Record::builder()
                .level(log::Level::Info)
                .target("danfse_core::service")
                .args(format_args!(
                    "Município não encontrado para o código IBGE [1]"
                ))
                .build(),
        );
        assert_eq!(
            cap.linhas()[0]["msg"],
            "Município não encontrado para o código IBGE [1]"
        );
    }

    #[test]
    fn ponte_ignora_dependencias() {
        let (logger, cap) = logger_capturado(Level::Debug);
        log::Log::log(
            &logger,
            &log::Record::builder()
                .level(log::Level::Debug)
                .target("reqwest::connect")
                .args(format_args!("starting new connection"))
                .build(),
        );
        assert!(cap.linhas().is_empty());
    }

    #[test]
    fn discard_nao_escreve() {
        assert!(!Logger::discard().enabled(Level::Error));
    }
}
