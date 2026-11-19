use serde::Deserialize;

use super::*;

/// saída de `json.Unmarshal` do Go 1.27 para cada entrada (gerada com o
/// mesmo struct `danfseRequest` do binário WASM Go)
#[derive(Deserialize)]
struct Referencia {
    #[serde(rename = "in")]
    entrada: String,
    err: String,
    xml: String,
    stc: i64,
    canhoto: Option<bool>,
}

#[test]
fn decodificacao_identica_ao_go() {
    let refs: Vec<Referencia> =
        match serde_json::from_str(include_str!("../tests/fixtures/go_json_ref.json")) {
            Ok(r) => r,
            Err(e) => panic!("{e}"),
        };
    for r in refs {
        let obtido = decodificar(&r.entrada).map_err(|e| format!("invalid request JSON: {e}"));
        if r.err.is_empty() {
            let req = match obtido {
                Ok(req) => req,
                Err(e) => panic!("{:?}: {e}", r.entrada),
            };
            assert_eq!(
                (req.xml.trim(), req.stc, req.canhoto),
                (r.xml.as_str(), r.stc, r.canhoto),
                "{:?}",
                r.entrada
            );
        } else {
            assert_eq!(
                obtido.err().as_deref(),
                Some(r.err.as_str()),
                "{:?}",
                r.entrada
            );
        }
    }
}

fn app() -> App {
    match App::new() {
        Ok(a) => a,
        Err(e) => panic!("{e}"),
    }
}

#[test]
fn validacoes_do_fluxo() {
    let a = app();
    let erro = |entrada: Option<&str>| a.gerar_pdf(entrada).err();
    assert_eq!(erro(None).as_deref(), Some("missing request JSON argument"));
    assert_eq!(
        erro(Some("<undefined>")).as_deref(),
        Some("invalid request JSON: invalid character '<' looking for beginning of value")
    );
    assert_eq!(
        erro(Some(r#"{"xml":"  "}"#)).as_deref(),
        Some("xml is required")
    );
    assert_eq!(
        erro(Some(r#"{"xml":"<a/>","stc":3}"#)).as_deref(),
        Some("invalid stc value: 3 (expected 0, 1, or 2)")
    );
    assert_eq!(
        erro(Some(r#"{"xml":"<x/>"}"#)).as_deref(),
        Some("PDF generation failed: expected element type <NFSe> but have <x>")
    );
    let grande = format!(r#"{{"xml":"{}"}}"#, "a".repeat(5 * 1024 * 1024 + 1));
    assert_eq!(
        erro(Some(&grande)).as_deref(),
        Some("xml exceeds maximum size of 5242880 bytes")
    );
}

#[test]
fn gera_pdf_em_base64() {
    let xml = include_str!("../../core/tests/fixtures/paridade/min.xml");
    let entrada =
        match serde_json::to_string(&serde_json::json!({"xml": xml, "stc": 1, "canhoto": false})) {
            Ok(s) => s,
            Err(e) => panic!("{e}"),
        };
    let resp = match app().gerar_pdf(Some(&entrada)) {
        Ok(r) => r,
        Err(e) => panic!("{e}"),
    };
    let pdf = match STANDARD.decode(&resp.pdf) {
        Ok(p) => p,
        Err(e) => panic!("{e}"),
    };
    assert_eq!(pdf.len(), resp.size);
    assert!(pdf.starts_with(b"%PDF-1.7"));
}
