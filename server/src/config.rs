use danfse_core::domain::utils_strings::parse_atoi;
use danfse_core::service::{MAX_XML_INPUT_SIZE, MAX_XML_INPUT_SIZE_LIMIT};
use danfse_core::xml::go_quote;
use thiserror::Error;

use crate::logging::is_valid_level;

const DEFAULT_PORT: i64 = 8080;
const DEFAULT_LOG_LEVEL: &str = "info";

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ErroConfig {
    #[error("{chave} inválido: deve ser um número inteiro: {causa}")]
    NaoInteiro { chave: &'static str, causa: String },
    #[error("XML_MAX_SIZE_BYTES inválido: deve ser positivo, obteve {0}")]
    TamanhoNaoPositivo(i64),
    #[error(
        "XML_MAX_SIZE_BYTES acima do teto aceito de {MAX_XML_INPUT_SIZE_LIMIT} bytes (32 MiB): {0}"
    )]
    TamanhoAcimaDoTeto(i64),
    #[error("LOG_LEVEL inválido: deve ser debug|info|warn|error, obteve {0}")]
    NivelLog(String),
}

/// configuração do processo lida das variáveis de ambiente
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Env {
    pub port: i64,
    pub xml_max_size_bytes: i64,
    pub log_level: String,
}

impl Env {
    pub fn carregar() -> Result<Self, ErroConfig> {
        Self::carregar_de(|k| std::env::var(k).ok())
    }

    /// variável vazia equivale a ausente, como no `envString` do Go
    pub fn carregar_de(lookup: impl Fn(&str) -> Option<String>) -> Result<Self, ErroConfig> {
        let valor = |k: &str| lookup(k).filter(|v| !v.is_empty());
        let port = env_int(valor("SERVER_PORT"), "SERVER_PORT", DEFAULT_PORT)?;
        let max = env_int(
            valor("XML_MAX_SIZE_BYTES"),
            "XML_MAX_SIZE_BYTES",
            MAX_XML_INPUT_SIZE,
        )?;
        if max <= 0 {
            return Err(ErroConfig::TamanhoNaoPositivo(max));
        }
        if max > MAX_XML_INPUT_SIZE_LIMIT {
            return Err(ErroConfig::TamanhoAcimaDoTeto(max));
        }
        let log_level = valor("LOG_LEVEL")
            .unwrap_or_else(|| DEFAULT_LOG_LEVEL.into())
            .to_lowercase();
        if !is_valid_level(&log_level) {
            return Err(ErroConfig::NivelLog(go_quote(&log_level)));
        }
        Ok(Self {
            port,
            xml_max_size_bytes: max,
            log_level,
        })
    }
}

fn env_int(valor: Option<String>, chave: &'static str, padrao: i64) -> Result<i64, ErroConfig> {
    match valor {
        None => Ok(padrao),
        Some(v) => parse_atoi(&v).map_err(|causa| ErroConfig::NaoInteiro { chave, causa }),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn carregar(pares: &[(&str, &str)]) -> Result<Env, ErroConfig> {
        let mapa: HashMap<String, String> = pares
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect();
        Env::carregar_de(|k| mapa.get(k).cloned())
    }

    #[test]
    fn padroes() {
        assert_eq!(
            carregar(&[]),
            Ok(Env {
                port: 8080,
                xml_max_size_bytes: MAX_XML_INPUT_SIZE,
                log_level: "info".into()
            })
        );
    }

    #[test]
    fn valores_vazios_usam_padrao() {
        let env = carregar(&[("SERVER_PORT", ""), ("LOG_LEVEL", "")]);
        assert_eq!(
            env.map(|e| (e.port, e.log_level)),
            Ok((8080, "info".into()))
        );
    }

    #[test]
    fn le_valores_validos() {
        let env = carregar(&[
            ("SERVER_PORT", "9090"),
            ("XML_MAX_SIZE_BYTES", "1048576"),
            ("LOG_LEVEL", "DEBUG"),
        ]);
        assert_eq!(
            env,
            Ok(Env {
                port: 9090,
                xml_max_size_bytes: 1_048_576,
                log_level: "debug".into()
            })
        );
    }

    #[test]
    fn porta_invalida() {
        let erro = carregar(&[("SERVER_PORT", "abc")]).map_err(|e| e.to_string());
        assert_eq!(
            erro,
            Err("SERVER_PORT inválido: deve ser um número inteiro: strconv.Atoi: parsing \"abc\": invalid syntax".into())
        );
    }

    #[test]
    fn tamanho_fora_dos_limites() {
        let e = |v: &str| {
            carregar(&[("XML_MAX_SIZE_BYTES", v)])
                .map_err(|e| e.to_string())
                .err()
        };
        assert_eq!(
            e("0").as_deref(),
            Some("XML_MAX_SIZE_BYTES inválido: deve ser positivo, obteve 0")
        );
        assert_eq!(
            e("-5").as_deref(),
            Some("XML_MAX_SIZE_BYTES inválido: deve ser positivo, obteve -5")
        );
        assert_eq!(
            e("33554433").as_deref(),
            Some("XML_MAX_SIZE_BYTES acima do teto aceito de 33554432 bytes (32 MiB): 33554433")
        );
        assert!(e("33554432").is_none());
        assert!(e("x").is_some_and(|m| {
            m.starts_with("XML_MAX_SIZE_BYTES inválido: deve ser um número inteiro")
        }));
    }

    #[test]
    fn nivel_de_log_invalido() {
        assert_eq!(
            carregar(&[("LOG_LEVEL", "Trace")]).map_err(|e| e.to_string()),
            Err("LOG_LEVEL inválido: deve ser debug|info|warn|error, obteve \"trace\"".into())
        );
    }
}
