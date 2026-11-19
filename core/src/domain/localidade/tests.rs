use super::*;

fn catalogo() -> Localidades {
    match Localidades::carregar() {
        Ok(c) => c,
        Err(e) => panic!("carregar() erro inesperado: {e}"),
    }
}

#[test]
fn contagens() {
    let c = catalogo();
    assert_eq!(c.contagem_estados(), 28);
    assert_eq!(c.contagem_municipios(), 5525);
    assert_eq!(c.todos_estados().len(), 28);
}

#[test]
fn municipios_conhecidos() {
    let c = catalogo();
    let casos = [
        (3550308, "São Paulo", 35),
        (5101837, "Boa Esperança do Norte", 51),
        (9999999, "EXTERIOR", 99),
    ];
    for (codigo, nome, uf) in casos {
        let m = c
            .municipio_por_codigo(codigo)
            .unwrap_or_else(|| panic!("município {codigo} ausente"));
        assert_eq!(m.nome, nome);
        assert_eq!(m.uf_codigo, uf);
    }
}

#[test]
fn relacionamentos() {
    let c = catalogo();
    assert_eq!(
        c.estado_por_municipio(3550308).map(|e| e.sigla.as_str()),
        Some("SP")
    );
    assert_eq!(
        c.estado_por_municipio(9999999).map(|e| e.sigla.as_str()),
        Some("EX")
    );
    assert_eq!(c.estado_por_municipio_sigla(3550308), Some("SP"));
    assert_eq!(c.estado_por_municipio_nome(3550308), Some("SÃO PAULO"));
}

#[test]
fn existencia() {
    let c = catalogo();
    assert!(c.estado_existe(35));
    assert!(c.estado_existe(99));
    assert!(!c.estado_existe(999));
    assert!(c.municipio_existe(3550308));
    assert!(c.municipio_existe(9999999));
    assert!(!c.municipio_existe(0));
}

#[test]
fn buscas_por_nome_e_sigla() {
    let c = catalogo();
    let sp = c
        .estado_por_sigla("SP")
        .map(|e| (e.codigo, e.nome.as_str()));
    assert_eq!(sp, Some((35, "SÃO PAULO")));
    assert!(c.estado_por_sigla("XX").is_none());
    assert_eq!(
        c.estado_por_nome("SÃO PAULO").map(|e| e.sigla.as_str()),
        Some("SP")
    );
    assert_eq!(
        c.municipio_por_nome_e_uf("São Paulo", 35).map(|m| m.codigo),
        Some(3550308)
    );
}

#[test]
fn municipios_por_uf_inclui_capital() {
    let c = catalogo();
    let munis = c.municipios_por_uf(35);
    assert!(!munis.is_empty());
    assert!(munis.iter().any(|m| m.codigo == 3550308));
}

#[test]
fn buscar_municipio() {
    let c = catalogo();
    assert_eq!(
        c.buscar_municipio(3550308),
        Ok(Some(("São Paulo".into(), "SP".into())))
    );
    assert_eq!(c.buscar_municipio(1), Ok(None));
    assert_eq!(
        c.buscar_municipio(9999999),
        Ok(Some(("EXTERIOR".into(), "EX".into())))
    );
}

#[test]
fn buscar_municipio_com_uf_invalida() {
    let c = Localidades {
        estados: HashMap::from([(
            1,
            Estado {
                codigo: 1,
                nome: "A".into(),
                sigla: "AA".into(),
            },
        )]),
        municipios: HashMap::from([(
            1,
            Municipio {
                codigo: 1,
                nome: "M".into(),
                uf_codigo: 999,
            },
        )]),
        municipios_por_uf: HashMap::new(),
    };
    assert!(c.buscar_municipio(1).is_err());
}

#[test]
fn busca_gerada_equivale_ao_catalogo() {
    let c = catalogo();
    for codigo in [3550308, 5101837, 9999999, 1, 0, 5300108] {
        assert_eq!(
            buscar_municipio_gerado(codigo),
            c.buscar_municipio(codigo),
            "código {codigo}"
        );
    }
}

#[test]
fn decodificar_json_invalidos() {
    let casos = [
        "not json",
        r#"{"estados":[],"municipios":[]}"#,
        r#"{"estados":[{"codigo":1,"nome":"A","sigla":"AA"},{"codigo":1,"nome":"B","sigla":"BB"}],"municipios":[{"codigo":1,"nome":"M","ufCodigo":1}]}"#,
        r#"{"estados":[{"codigo":1,"nome":"A","sigla":"AA"}],"municipios":[{"codigo":1,"nome":"M","ufCodigo":1},{"codigo":1,"nome":"M2","ufCodigo":1}]}"#,
        r#"{"estados":[{"codigo":1,"nome":"A","sigla":"AA"}],"municipios":[{"codigo":1,"nome":"M","ufCodigo":999}]}"#,
        r#"{"estados":[{"codigo":0,"nome":"A","sigla":"AA"}],"municipios":[{"codigo":1,"nome":"M","ufCodigo":1}]}"#,
        r#"{"estados":[{"codigo":1,"nome":"","sigla":"AA"}],"municipios":[{"codigo":1,"nome":"M","ufCodigo":1}]}"#,
        r#"{"estados":[{"codigo":1,"nome":"A","sigla":""}],"municipios":[{"codigo":1,"nome":"M","ufCodigo":1}]}"#,
        r#"{"estados":[{"codigo":1,"nome":"A","sigla":"AA"}],"municipios":[{"codigo":1,"nome":"","ufCodigo":1}]}"#,
        r#"{"estados":[{"codigo":1,"nome":"A","sigla":"AA"}],"municipios":[{"codigo":1,"nome":"M","ufCodigo":0}]}"#,
    ];
    for json in casos {
        assert!(
            Localidades::decodificar_json(json.as_bytes()).is_err(),
            "{json}"
        );
    }
}

#[test]
fn decodificar_json_valido() {
    let json = r#"{"estados":[{"codigo":35,"nome":"SÃO PAULO","sigla":"SP"}],"municipios":[{"codigo":3550308,"nome":"São Paulo","ufCodigo":35}]}"#;
    let c = match Localidades::decodificar_json(json.as_bytes()) {
        Ok(c) => c,
        Err(e) => panic!("erro inesperado: {e}"),
    };
    assert_eq!(c.contagem_estados(), 1);
    assert_eq!(c.contagem_municipios(), 1);
}
