//! serviços de geração do DANFSe: preparação do modelo, apresentação e
//! renderização direta em PDF

mod generation;
mod preparation;
mod presenter;
mod tax_totals;
mod xml_security;

use std::sync::Arc;

use thiserror::Error;

use crate::domain::enums::SituacaoNotaTipo;
use crate::domain::localidade::ErroCatalogo;
use crate::domain::models::NFSe;
use crate::domain::presentation::DocumentoDanfse;
use crate::xml::XmlError;

pub use generation::ServicoGeracaoPdfDanfseDireto;
pub use preparation::ServicoPreparacaoDanfse;
pub use presenter::ServicoApresentacaoDanfse;
pub use xml_security::{MAX_XML_INPUT_SIZE, MAX_XML_INPUT_SIZE_LIMIT, rejeitar_dtd_e_entidades};

#[derive(Debug, Clone, PartialEq, Error)]
pub enum ErroDanfse {
    #[error("invalid XML max input size {0} bytes")]
    TamanhoMaximoInvalido(i64),
    #[error("input XML is empty")]
    XmlVazio,
    #[error("input exceeds maximum size of {0} bytes")]
    EntradaExcedeTamanho(i64),
    #[error("sanitized input exceeds maximum size of {0} bytes")]
    EntradaSanitizadaExcedeTamanho(i64),
    #[error("XML contains DTD or entity declaration")]
    XmlContemDtd,
    #[error(transparent)]
    Xml(#[from] XmlError),
    #[error("infNFSe Id is empty")]
    IdVazio,
    #[error("folio direto: {0}")]
    Renderizacao(String),
}

/// resolve nome e UF a partir do código IBGE do município
pub type ResolverMunicipio =
    Arc<dyn Fn(i64) -> Result<Option<(String, String)>, ErroCatalogo> + Send + Sync>;

/// gera o DANFSe em PDF
pub trait ServicoGeracaoPdf: Send + Sync {
    fn executar(
        &self,
        xml: &[u8],
        situacao: SituacaoNotaTipo,
        canhoto: bool,
    ) -> Result<Vec<u8>, ErroDanfse>;
}

/// analisa e enriquece o modelo NFSe independentemente da saída
pub trait PreparadorDanfse: Send + Sync {
    fn executar(&self, xml: &[u8], situacao: SituacaoNotaTipo) -> Result<NFSe, ErroDanfse>;
}

/// mapeia um modelo NFSe enriquecido para um documento de apresentação puro
pub trait ApresentadorDanfse: Send + Sync {
    fn apresentar(&self, nota: &NFSe) -> DocumentoDanfse;
}

/// renderiza um documento de apresentação diretamente em PDF
pub trait RenderizadorPdfDireto: Send + Sync {
    fn renderizar(&self, doc: &DocumentoDanfse) -> Result<Vec<u8>, ErroDanfse>;
}
