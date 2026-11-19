use std::sync::Arc;

use crate::domain::enums::SituacaoNotaTipo;

use super::{
    ApresentadorDanfse, ErroDanfse, PreparadorDanfse, RenderizadorPdfDireto, ServicoGeracaoPdf,
};

/// gera o PDF a partir do modelo NFSe enriquecido, sem HTML intermediário
#[derive(Clone)]
pub struct ServicoGeracaoPdfDanfseDireto {
    preparador: Arc<dyn PreparadorDanfse>,
    apresentador: Arc<dyn ApresentadorDanfse>,
    renderizador: Arc<dyn RenderizadorPdfDireto>,
}

impl ServicoGeracaoPdfDanfseDireto {
    pub fn new(
        preparador: Arc<dyn PreparadorDanfse>,
        apresentador: Arc<dyn ApresentadorDanfse>,
        renderizador: Arc<dyn RenderizadorPdfDireto>,
    ) -> Self {
        Self {
            preparador,
            apresentador,
            renderizador,
        }
    }
}

impl ServicoGeracaoPdf for ServicoGeracaoPdfDanfseDireto {
    fn executar(
        &self,
        xml: &[u8],
        situacao: SituacaoNotaTipo,
        canhoto: bool,
    ) -> Result<Vec<u8>, ErroDanfse> {
        let nota = self.preparador.executar(xml, situacao)?;
        let mut documento = self.apresentador.apresentar(&nota);
        documento.canhoto.visivel = canhoto;
        self.renderizador.renderizar(&documento)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;
    use crate::domain::models::NFSe;
    use crate::domain::presentation::DocumentoDanfse;

    struct MockPreparador(Option<ErroDanfse>);

    impl PreparadorDanfse for MockPreparador {
        fn executar(&self, _: &[u8], _: SituacaoNotaTipo) -> Result<NFSe, ErroDanfse> {
            match &self.0 {
                Some(e) => Err(e.clone()),
                None => Ok(NFSe::default()),
            }
        }
    }

    struct MockApresentador;

    impl ApresentadorDanfse for MockApresentador {
        fn apresentar(&self, _: &NFSe) -> DocumentoDanfse {
            DocumentoDanfse::default()
        }
    }

    #[derive(Default)]
    struct MockRenderizador {
        erro: Option<ErroDanfse>,
        ultimo: Mutex<Option<DocumentoDanfse>>,
    }

    impl RenderizadorPdfDireto for MockRenderizador {
        fn renderizar(&self, doc: &DocumentoDanfse) -> Result<Vec<u8>, ErroDanfse> {
            if let Ok(mut u) = self.ultimo.lock() {
                *u = Some(doc.clone());
            }
            match &self.erro {
                Some(e) => Err(e.clone()),
                None => Ok(b"%PDF-fake".to_vec()),
            }
        }
    }

    fn servico(
        prep: Option<ErroDanfse>,
        render: Arc<MockRenderizador>,
    ) -> ServicoGeracaoPdfDanfseDireto {
        ServicoGeracaoPdfDanfseDireto::new(
            Arc::new(MockPreparador(prep)),
            Arc::new(MockApresentador),
            render,
        )
    }

    fn canhoto_renderizado(render: &MockRenderizador) -> Option<bool> {
        render
            .ultimo
            .lock()
            .ok()
            .and_then(|u| u.as_ref().map(|d| d.canhoto.visivel))
    }

    #[test]
    fn sucesso() {
        let svc = servico(None, Arc::default());
        assert_eq!(
            svc.executar(b"<xml/>", SituacaoNotaTipo::Valida, true),
            Ok(b"%PDF-fake".to_vec())
        );
    }

    #[test]
    fn propaga_erro_do_preparador() {
        let svc = servico(Some(ErroDanfse::XmlVazio), Arc::default());
        assert_eq!(
            svc.executar(b"<xml/>", SituacaoNotaTipo::Valida, true),
            Err(ErroDanfse::XmlVazio)
        );
    }

    #[test]
    fn propaga_erro_do_renderizador() {
        let render = Arc::new(MockRenderizador {
            erro: Some(ErroDanfse::Renderizacao("render falhou".into())),
            ..Default::default()
        });
        let r = servico(None, render).executar(b"<xml/>", SituacaoNotaTipo::Valida, true);
        assert_eq!(
            r.map_err(|e| e.to_string()),
            Err("folio direto: render falhou".into())
        );
    }

    #[test]
    fn repassa_flag_do_canhoto() {
        for canhoto in [false, true] {
            let render = Arc::new(MockRenderizador::default());
            let svc = servico(None, Arc::clone(&render));
            assert!(
                svc.executar(b"<xml/>", SituacaoNotaTipo::Valida, canhoto)
                    .is_ok()
            );
            assert_eq!(canhoto_renderizado(&render), Some(canhoto));
        }
    }
}
