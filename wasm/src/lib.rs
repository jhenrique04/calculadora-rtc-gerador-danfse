//! build WASM do gerador de DANFSe. Expõe `globalThis.danfseWasm` com
//! `{ ready, error, gerarPdf(json) }`, mesmo contrato do binário Go

mod go_json;
#[cfg(target_arch = "wasm32")]
mod js;

use std::sync::Arc;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use danfse_core::app::new_pdf_service_with_dependencies;
use danfse_core::domain::enums::SituacaoNotaTipo;
use danfse_core::domain::localidade::buscar_municipio_gerado;
use danfse_core::renderer::Renderer;
use danfse_core::service::{
    MAX_XML_INPUT_SIZE, ServicoApresentacaoDanfse, ServicoGeracaoPdf, ServicoGeracaoPdfDanfseDireto,
};

use go_json::Valor;

const STRUCT_GO: &str = "danfseRequest";

/// corpo JSON recebido do JavaScript
#[derive(Debug, Default, PartialEq)]
pub struct DanfseRequest {
    pub xml: String,
    pub stc: i64,
    pub canhoto: Option<bool>,
}

/// resposta de sucesso: PDF em base64 e tamanho em bytes
#[derive(Debug, PartialEq)]
pub struct DanfseResponse {
    pub pdf: String,
    pub size: usize,
}

/// dependências reaproveitadas entre chamadas
pub struct App {
    servico: ServicoGeracaoPdfDanfseDireto,
}

impl App {
    pub fn new() -> Result<Self, String> {
        let renderizador = Renderer::new().map_err(|e| format!("renderer: {e}"))?;
        let servico = new_pdf_service_with_dependencies(
            Arc::new(buscar_municipio_gerado),
            Arc::new(renderizador),
            Arc::new(ServicoApresentacaoDanfse::new()),
            None,
        );
        Ok(Self { servico })
    }

    /// fluxo de `gerarPdf`; `None` quando o JS não passou argumento
    pub fn gerar_pdf(&self, entrada: Option<&str>) -> Result<DanfseResponse, String> {
        let Some(entrada) = entrada else {
            return Err("missing request JSON argument".into());
        };
        let req = decodificar(entrada).map_err(|e| format!("invalid request JSON: {e}"))?;
        let xml = req.xml.trim();
        if xml.is_empty() {
            return Err("xml is required".into());
        }
        if i64::try_from(xml.len()).map_or(true, |n| n > MAX_XML_INPUT_SIZE) {
            return Err(format!(
                "xml exceeds maximum size of {MAX_XML_INPUT_SIZE} bytes"
            ));
        }
        let situacao = match SituacaoNotaTipo::por_stc(req.stc) {
            (s, true) => s,
            _ => {
                return Err(format!(
                    "invalid stc value: {} (expected 0, 1, or 2)",
                    req.stc
                ));
            }
        };
        let pdf = self
            .servico
            .executar(xml.as_bytes(), situacao, req.canhoto.unwrap_or(true))
            .map_err(|e| format!("PDF generation failed: {e}"))?;
        Ok(DanfseResponse {
            size: pdf.len(),
            pdf: STANDARD.encode(pdf),
        })
    }
}

/// `json.Unmarshal` em `danfseRequest`: chaves sem distinção de caixa, a
/// última ocorrência vence e o primeiro erro de tipo é o reportado
pub fn decodificar(entrada: &str) -> Result<DanfseRequest, String> {
    let campos = match go_json::parse(entrada)? {
        Valor::Null => return Ok(DanfseRequest::default()),
        Valor::Objeto(campos) => campos,
        outro => {
            return Err(format!(
                "json: cannot unmarshal {} into Go value of type main.{STRUCT_GO}",
                outro.tipo()
            ));
        }
    };
    let mut req = DanfseRequest::default();
    let mut primeiro_erro = None;
    for (chave, valor) in campos {
        let resultado = if mesma_chave(&chave, "xml") {
            atribuir_xml(&mut req, valor)
        } else if mesma_chave(&chave, "stc") {
            atribuir_stc(&mut req, valor)
        } else if mesma_chave(&chave, "canhoto") {
            atribuir_canhoto(&mut req, valor)
        } else {
            Ok(())
        };
        if let Err(e) = resultado {
            primeiro_erro.get_or_insert(e);
        }
    }
    match primeiro_erro {
        Some(e) => Err(e),
        None => Ok(req),
    }
}

/// comparação com case folding simples, como o `foldName` do Go
fn mesma_chave(chave: &str, campo: &str) -> bool {
    let dobrar = |c: char| match c {
        'ſ' => 's',
        '\u{212A}' => 'k',
        c => c.to_lowercase().next().unwrap_or(c),
    };
    chave.chars().map(dobrar).eq(campo.chars())
}

fn erro_tipo(valor: &str, campo: &str, tipo: &str) -> String {
    format!(
        "json: cannot unmarshal {valor} into Go struct field {STRUCT_GO}.{campo} of type {tipo}"
    )
}

fn atribuir_xml(req: &mut DanfseRequest, valor: Valor) -> Result<(), String> {
    match valor {
        Valor::Texto(s) => req.xml = s,
        Valor::Null => {}
        outro => return Err(erro_tipo(outro.tipo(), "xml", "string")),
    }
    Ok(())
}

fn atribuir_stc(req: &mut DanfseRequest, valor: Valor) -> Result<(), String> {
    match valor {
        Valor::Numero(n) => {
            req.stc = n
                .parse()
                .map_err(|_| erro_tipo(&format!("number {n}"), "stc", "int"))?;
        }
        Valor::Null => {}
        outro => return Err(erro_tipo(outro.tipo(), "stc", "int")),
    }
    Ok(())
}

fn atribuir_canhoto(req: &mut DanfseRequest, valor: Valor) -> Result<(), String> {
    match valor {
        Valor::Bool(b) => req.canhoto = Some(b),
        Valor::Null => req.canhoto = None,
        outro => return Err(erro_tipo(outro.tipo(), "canhoto", "bool")),
    }
    Ok(())
}

#[cfg(test)]
mod tests;
