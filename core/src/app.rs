//! montagem da cadeia de geração: catálogo → preparador → apresentador →
//! renderizador

use std::sync::Arc;

use thiserror::Error;

use crate::domain::localidade::{ErroCatalogo, Localidades};
use crate::renderer::Renderer;
use crate::service::{
    ApresentadorDanfse, ErroDanfse, RenderizadorPdfDireto, ResolverMunicipio,
    ServicoApresentacaoDanfse, ServicoGeracaoPdfDanfseDireto, ServicoPreparacaoDanfse,
};

#[derive(Debug, Error)]
pub enum ErroInicializacao {
    #[error(transparent)]
    Catalogo(#[from] ErroCatalogo),
    #[error(transparent)]
    Renderizador(#[from] ErroDanfse),
}

/// serviço a partir de dependências já construídas (reaproveitadas entre
/// chamadas no WASM). Sem `max_input_size` vale o padrão de 5 MiB
pub fn new_pdf_service_with_dependencies(
    resolver: ResolverMunicipio,
    renderizador: Arc<dyn RenderizadorPdfDireto>,
    apresentador: Arc<dyn ApresentadorDanfse>,
    max_input_size: Option<i64>,
) -> ServicoGeracaoPdfDanfseDireto {
    let preparador = match max_input_size {
        Some(max) => ServicoPreparacaoDanfse::with_max_input_size(Some(resolver), max),
        None => ServicoPreparacaoDanfse::new(Some(resolver)),
    };
    ServicoGeracaoPdfDanfseDireto::new(Arc::new(preparador), apresentador, renderizador)
}

pub fn new_pdf_service(
    max_input_size: Option<i64>,
) -> Result<ServicoGeracaoPdfDanfseDireto, ErroInicializacao> {
    let catalogo = Arc::new(Localidades::carregar()?);
    let resolver: ResolverMunicipio = Arc::new(move |codigo| catalogo.buscar_municipio(codigo));
    let renderizador = Arc::new(Renderer::new()?);
    Ok(new_pdf_service_with_dependencies(
        resolver,
        renderizador,
        Arc::new(ServicoApresentacaoDanfse::new()),
        max_input_size,
    ))
}
