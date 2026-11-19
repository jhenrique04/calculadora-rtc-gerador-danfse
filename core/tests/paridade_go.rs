//! compara o fluxo de conteúdo descomprimido com o gerado pelo serviço Go
//! (folio) para os mesmos XMLs. Os bytes zlib diferem entre implementações,
//! então a paridade é verificada no conteúdo da página

use std::io::Read;
use std::path::PathBuf;

use danfse_core::app::new_pdf_service;
use danfse_core::domain::enums::SituacaoNotaTipo;
use danfse_core::service::ServicoGeracaoPdf;
use flate2::read::ZlibDecoder;

fn fixture(nome: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/paridade")
        .join(nome)
}

fn ler(nome: &str) -> Vec<u8> {
    match std::fs::read(fixture(nome)) {
        Ok(b) => b,
        Err(e) => panic!("{nome}: {e}"),
    }
}

fn conteudo_da_pagina(pdf: &[u8]) -> Vec<u8> {
    let marcador = b"<< /Filter /FlateDecode /Length ";
    let Some(ini) = pdf.windows(marcador.len()).rposition(|w| w == marcador) else {
        panic!("content stream ausente");
    };
    let resto = &pdf[ini..];
    let Some(s) = resto.windows(8).position(|w| w == b"\nstream\n") else {
        panic!("stream sem corpo");
    };
    let mut out = Vec::new();
    if let Err(e) = ZlibDecoder::new(&resto[s + 8..]).read_to_end(&mut out) {
        panic!("{e}");
    }
    out
}

fn primeira_diferenca(a: &[u8], b: &[u8]) -> String {
    let (la, lb): (Vec<_>, Vec<_>) = (
        a.split(|&c| c == b'\n').collect(),
        b.split(|&c| c == b'\n').collect(),
    );
    for (i, (x, y)) in la.iter().zip(&lb).enumerate() {
        if x != y {
            return format!(
                "linha {}: go={:?} rust={:?}",
                i + 1,
                String::from_utf8_lossy(x),
                String::from_utf8_lossy(y)
            );
        }
    }
    format!(
        "tamanhos diferentes: go={} rust={} linhas",
        la.len(),
        lb.len()
    )
}

fn verificar(xml: &str, stc: i64, canhoto: bool) {
    let servico = match new_pdf_service(None) {
        Ok(s) => s,
        Err(e) => panic!("{e}"),
    };
    let (situacao, ok) = SituacaoNotaTipo::por_stc(stc);
    assert!(ok);
    let pdf = match servico.executar(&ler(&format!("{xml}.xml")), situacao, canhoto) {
        Ok(p) => p,
        Err(e) => panic!("{xml}: {e}"),
    };
    let esperado = ler(&format!("{xml}_{stc}_{canhoto}.content"));
    let obtido = conteudo_da_pagina(&pdf);
    assert!(
        obtido == esperado,
        "{xml} stc={stc} canhoto={canhoto}: {}",
        primeira_diferenca(&esperado, &obtido)
    );
}

#[test]
fn nota_completa_valida_com_canhoto() {
    verificar("full", 0, true);
}

#[test]
fn nota_completa_cancelada_sem_canhoto() {
    verificar("full", 2, false);
}

#[test]
fn nota_minima() {
    verificar("min", 0, true);
}

#[test]
fn partes_suprimidas_substituida() {
    verificar("sem_partes", 1, false);
}

#[test]
fn textos_longos_truncados() {
    verificar("ambos_longos", 2, true);
}

#[test]
fn caracteres_especiais_e_fora_do_win_ansi() {
    verificar("especiais", 0, true);
}

#[test]
fn issqn_imune() {
    verificar("imune", 0, true);
}
