//! gera um DANFSe a partir de um XML: `render <xml> <stc> <canhoto> <saida.pdf>`

use std::process::ExitCode;

use danfse_core::app::new_pdf_service;
use danfse_core::domain::enums::SituacaoNotaTipo;
use danfse_core::service::ServicoGeracaoPdf;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("erro: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let [_, xml, stc, canhoto, saida] = args.as_slice() else {
        return Err("uso: render <xml> <stc> <canhoto> <saida.pdf>".into());
    };
    let (situacao, ok) = SituacaoNotaTipo::por_stc(stc.parse()?);
    if !ok {
        return Err("stc inválido".into());
    }
    let servico = new_pdf_service(None)?;
    let pdf = servico.executar(&std::fs::read(xml)?, situacao, canhoto.parse()?)?;
    std::fs::write(saida, pdf)?;
    Ok(())
}
