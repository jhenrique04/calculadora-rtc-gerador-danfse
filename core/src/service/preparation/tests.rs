use std::sync::Arc;

use super::*;
use crate::domain::localidade::{ErroCatalogo, Localidades};

fn stub_resolver() -> Option<ResolverMunicipio> {
    Some(Arc::new(|_| {
        Ok(Some(("São Paulo".to_string(), "SP".to_string())))
    }))
}

struct Fixture<'a> {
    id: &'a str,
    serv_extra: &'a str,
    desc: &'a str,
    trib: &'a str,
    valores_dps_extra: &'a str,
    c_mun_emit: &'a str,
    valores_extra: &'a str,
}

impl Default for Fixture<'_> {
    fn default() -> Self {
        Self {
            id: r#"Id="NFS123""#,
            serv_extra: "",
            desc: "Servico",
            trib: "<trib></trib>",
            valores_dps_extra: "",
            c_mun_emit: "3550308",
            valores_extra: "",
        }
    }
}

impl Fixture<'_> {
    fn xml(&self) -> Vec<u8> {
        format!(
            r#"<NFSe versao="1.00">
		<infNFSe {id} dhProc="2026-07-14T10:00:00-03:00">
			<nNFSe>1</nNFSe>
			<ambGer>1</ambGer>
			<DPS>
				<infDPS>
					<nDPS>1</nDPS>
					<serie>1</serie>
					<dhEmi>2026-07-14T10:00:00-03:00</dhEmi>
					<dCompet>2026-07-14</dCompet>
					<tpEmit>1</tpEmit>
					<tpAmb>1</tpAmb>
					<prest><regTrib><opSimpNac>1</opSimpNac></regTrib></prest>
					<serv>
						<cServ><cTribNac>010101</cTribNac><xDescServ>{desc}</xDescServ></cServ>
						{serv_extra}
					</serv>
					<valores>
						<vServPrest><vServ>100</vServ></vServPrest>
						{valores_dps_extra}
						{trib}
					</valores>
				</infDPS>
			</DPS>
			<emit>
				<CNPJ>12345678000195</CNPJ>
				<xNome>Prestador</xNome>
				<enderNac><cMun>{c_mun}</cMun><CEP>01001000</CEP></enderNac>
			</emit>
			<valores><vLiq>100</vLiq>{valores_extra}</valores>
		</infNFSe>
	</NFSe>"#,
            id = self.id,
            desc = self.desc,
            serv_extra = self.serv_extra,
            valores_dps_extra = self.valores_dps_extra,
            trib = self.trib,
            c_mun = self.c_mun_emit,
            valores_extra = self.valores_extra,
        )
        .into_bytes()
    }
}

fn preparar_com(
    resolver: Option<ResolverMunicipio>,
    xml: &[u8],
    situacao: SituacaoNotaTipo,
) -> NFSe {
    match ServicoPreparacaoDanfse::new(resolver).executar(xml, situacao) {
        Ok(n) => n,
        Err(e) => panic!("executar erro inesperado: {e}"),
    }
}

fn preparar(f: &Fixture<'_>) -> NFSe {
    preparar_com(stub_resolver(), &f.xml(), SituacaoNotaTipo::Valida)
}

fn erro(p: &ServicoPreparacaoDanfse, xml: &[u8]) -> String {
    match p.executar(xml, SituacaoNotaTipo::Valida) {
        Ok(_) => panic!("esperava erro"),
        Err(e) => e.to_string(),
    }
}

#[test]
fn preserva_texto_compl_original() {
    let nota = preparar(&Fixture {
        desc: "Servico de Teste Completo",
        serv_extra: "<infoCompl><xInfComp>Informacao complementar importante</xInfComp></infoCompl>",
        ..Fixture::default()
    });
    assert!(
        nota.texto_compl_original
            .contains("Inf. Cont.: Informacao complementar importante")
    );
    assert_eq!(nota.texto_desc_serv_original, "Servico de Teste Completo");
    assert_eq!(
        nota.inf_nfse.qrcode,
        "https://www.nfse.gov.br/ConsultaPublica/?tpc=1&chave=123"
    );
}

#[test]
fn v_exclusoes_reducoes_bc() {
    let nota = preparar(&Fixture {
        valores_dps_extra: "<vDescCondIncond><vDescIncond>10</vDescIncond></vDescCondIncond>",
        trib: "<trib><tribMun><tribISSQN>1</tribISSQN></tribMun></trib>",
        valores_extra: "<vISSQN>5</vISSQN><vCalcDR>3</vCalcDR>",
        ..Fixture::default()
    });
    assert_eq!(nota.v_exclusoes_reducoes_bc, 18.0);
}

#[test]
fn marca_dagua_por_situacao() {
    let xml = Fixture::default().xml();
    let casos = [
        (SituacaoNotaTipo::Valida, ""),
        (SituacaoNotaTipo::Substituida, "SUBSTITUÍDA"),
        (SituacaoNotaTipo::Cancelada, "CANCELADA"),
    ];
    for (situacao, esperado) in casos {
        assert_eq!(
            preparar_com(stub_resolver(), &xml, situacao).marca_dagua,
            esperado
        );
    }
}

#[test]
fn preserva_c_pais_prestacao_e_enriquece_uf() {
    let nota = preparar(&Fixture {
        serv_extra: "<locPrest><cLocPrestacao>3550308</cLocPrestacao><cPaisPrestacao>BR</cPaisPrestacao></locPrest>",
        ..Fixture::default()
    });
    let loc = &nota.inf_nfse.dps.inf_dps.serv.loc_prest;
    assert_eq!(loc.c_pais_prestacao, "BR");
    assert_eq!(loc.uf, "SP");
}

#[test]
fn catalogo_real() {
    let localidades = match Localidades::carregar() {
        Ok(l) => Arc::new(l),
        Err(e) => panic!("falha ao carregar catálogo: {e}"),
    };
    let resolver: ResolverMunicipio = Arc::new(move |codigo| localidades.buscar_municipio(codigo));
    let nota = preparar_com(
        Some(resolver),
        &Fixture::default().xml(),
        SituacaoNotaTipo::Valida,
    );
    assert_eq!(nota.inf_nfse.emit.ender_nac.x_mun, "São Paulo");
    assert_eq!(nota.inf_nfse.emit.ender_nac.uf, "SP");
}

#[test]
fn resolver_sem_sucesso_mantem_campos_vazios() {
    let nao_encontrado: ResolverMunicipio = Arc::new(|_| Ok(None));
    let com_erro: ResolverMunicipio = Arc::new(|_| Err(ErroCatalogo::new("erro de rede")));
    let casos: [(Option<ResolverMunicipio>, &str); 4] = [
        (Some(nao_encontrado), "3550308"),
        (Some(com_erro), "3550308"),
        (stub_resolver(), "NAO_NUMERICO"),
        (None, "3550308"),
    ];
    for (resolver, c_mun) in casos {
        let xml = Fixture {
            c_mun_emit: c_mun,
            ..Fixture::default()
        }
        .xml();
        let nota = preparar_com(resolver, &xml, SituacaoNotaTipo::Valida);
        assert_eq!(nota.inf_nfse.emit.ender_nac.x_mun, "");
        assert_eq!(nota.inf_nfse.emit.ender_nac.uf, "");
    }
}

#[test]
fn resolver_encontrado_com_nome_vazio_aplica_campos() {
    let resolver: ResolverMunicipio = Arc::new(|_| Ok(Some((String::new(), "SP".to_string()))));
    let nota = preparar_com(
        Some(resolver),
        &Fixture::default().xml(),
        SituacaoNotaTipo::Valida,
    );
    assert_eq!(nota.inf_nfse.emit.ender_nac.uf, "SP");
}

#[test]
fn loc_prest_ausente_no_catalogo() {
    let resolver: ResolverMunicipio = Arc::new(|codigo| {
        Ok((codigo == 3550308).then(|| ("São Paulo".to_string(), "SP".to_string())))
    });
    let xml = Fixture {
        serv_extra: "<locPrest><cLocPrestacao>9999999</cLocPrestacao></locPrest>",
        ..Fixture::default()
    }
    .xml();
    let nota = preparar_com(Some(resolver), &xml, SituacaoNotaTipo::Valida);
    assert_eq!(nota.inf_nfse.dps.inf_dps.serv.loc_prest.uf, "");
}

#[test]
fn texto_original_preservado_para_renderizacao_direta() {
    let longo =
        "Texto complementar completo que sera truncado para HTML mas preservado para Folio. "
            .repeat(80);
    let nota = preparar(&Fixture {
        serv_extra: &format!("<infoCompl><xInfComp>{longo}</xInfComp></infoCompl>"),
        ..Fixture::default()
    });
    assert!(nota.texto_compl_original.contains(
        "Texto complementar completo que sera truncado para HTML mas preservado para Folio"
    ));
    assert!(!nota.exibicao_compl.is_empty());
    assert!(nota.exibicao_compl.len() < nota.texto_compl_original.len());
}

#[test]
fn limites_de_tamanho() {
    let padrao = ServicoPreparacaoDanfse::new(stub_resolver());
    let grande = vec![0u8; usize::try_from(MAX_XML_INPUT_SIZE + 1).unwrap_or(usize::MAX)];
    assert_eq!(
        erro(&padrao, &grande),
        format!("input exceeds maximum size of {MAX_XML_INPUT_SIZE} bytes")
    );

    let p64 = ServicoPreparacaoDanfse::with_max_input_size(stub_resolver(), 64);
    assert!(erro(&p64, &[0u8; 65]).contains("input exceeds maximum size"));

    let p16 = ServicoPreparacaoDanfse::with_max_input_size(stub_resolver(), 16);
    assert!(erro(&p16, b"<a>&&&&</a>").contains("sanitized input exceeds maximum size"));

    for valor in [0, -1, MAX_XML_INPUT_SIZE_LIMIT + 1] {
        let p = ServicoPreparacaoDanfse::with_max_input_size(stub_resolver(), valor);
        assert_eq!(
            erro(&p, b"<a/>"),
            format!("invalid XML max input size {valor} bytes")
        );
    }
}

#[test]
fn erros_de_entrada() {
    let p = ServicoPreparacaoDanfse::new(stub_resolver());
    assert_eq!(erro(&p, b"  \n\t "), "input XML is empty");
    assert_eq!(erro(&p, b"<NFSe/>"), "infNFSe Id is empty");
    assert_eq!(
        erro(
            &p,
            &Fixture {
                id: "",
                ..Fixture::default()
            }
            .xml()
        ),
        "infNFSe Id is empty"
    );
    assert_eq!(
        erro(&p, b"<Outro/>"),
        "expected element type <NFSe> but have <Outro>"
    );
    assert_eq!(
        erro(&p, b"<!DOCTYPE x><NFSe/>"),
        "XML contains DTD or entity declaration"
    );
    assert_eq!(
        erro(&p, b"<NFSe>"),
        "XML syntax error on line 1: unexpected EOF"
    );
}

#[test]
fn markup_escapado_preservado() {
    let nota = preparar(&Fixture {
        desc: "&lt;svg&gt;alert&lt;/svg&gt;",
        ..Fixture::default()
    });
    assert_eq!(
        nota.inf_nfse.dps.inf_dps.serv.c_serv.x_desc_serv,
        "<svg>alert</svg>"
    );
}

#[test]
fn id_com_caracteres_especiais_escapado_no_qr() {
    let nota = preparar(&Fixture {
        id: r#"Id="NFS+100&amp;test""#,
        ..Fixture::default()
    });
    assert!(
        nota.inf_nfse.qrcode.ends_with("chave=%2B100%26test"),
        "{}",
        nota.inf_nfse.qrcode
    );
}

#[test]
fn descricao_limites() {
    let d1297 = "A".repeat(1297);
    let nota = preparar(&Fixture {
        desc: &d1297,
        ..Fixture::default()
    });
    assert!(!nota.exibicao_desc_serv.contains("..."));
    assert_eq!(nota.exibicao_desc_serv.len(), 1297);

    let d1301 = "A".repeat(1301);
    let nota = preparar(&Fixture {
        desc: &d1301,
        ..Fixture::default()
    });
    assert_eq!(nota.exibicao_desc_serv.len(), 1300);
    assert!(nota.exibicao_desc_serv.ends_with("..."));
}

#[test]
fn descricao_multilinha_abaixo_do_limite() {
    let desc = [
        "Servico de consultoria tecnica especializada em engenharia civil",
        "com foco em estruturas de concreto armado e protendido",
        "para edificacoes de medio e grande porte",
        "além de suporte tecnico durante a execucao",
        "O valor do servico sera pago em parcelas mensais",
    ]
    .join("\n");
    let nota = preparar(&Fixture {
        desc: &desc,
        ..Fixture::default()
    });
    assert_eq!(nota.exibicao_desc_serv, desc);
}

fn com_complemento(compl: &str) -> NFSe {
    preparar(&Fixture {
        serv_extra: &format!("<infoCompl><xInfComp>{compl}</xInfComp></infoCompl>"),
        trib: "<trib><tribMun><tribISSQN>1</tribISSQN></tribMun></trib>",
        ..Fixture::default()
    })
}

const LINHA_ZERADA: &str = "Totais aproximados dos Tributos cfe. Lei nº 12.741/2012: Federais: R$ 0,00; Estaduais: R$ 0,00; Municipais: R$ 0,00;";

#[test]
fn complemento_limites() {
    let nota = com_complemento(&"X".repeat(2000));
    assert!(
        nota.exibicao_compl
            .contains(" | Totais aproximados dos Tributos")
    );

    let nota = com_complemento(&"X".repeat(2001));
    assert!(
        nota.exibicao_compl
            .contains("... | Totais aproximados dos Tributos")
    );

    let nota = com_complemento(&"X ".repeat(1100));
    assert!(nota.exibicao_compl.contains("..."));
    assert!(nota.exibicao_compl.contains(LINHA_ZERADA));
}

#[test]
fn totais_sem_complemento() {
    let nota = preparar(&Fixture::default());
    assert_eq!(nota.texto_compl_original, LINHA_ZERADA);
    assert_eq!(nota.exibicao_compl, LINHA_ZERADA);
    assert_eq!(nota.exibicao_linha_tributos, LINHA_ZERADA);
}

#[test]
fn totais_com_complemento() {
    let nota = com_complemento("Informacao complementar importante");
    assert_eq!(
        nota.texto_compl_original,
        format!("Inf. Cont.: Informacao complementar importante | {LINHA_ZERADA}")
    );
    assert_eq!(
        nota.texto_compl_original
            .matches("Totais aproximados")
            .count(),
        1
    );
}

#[test]
fn montar_texto_compl_com_todos_os_rotulos() {
    let nota = preparar(&Fixture {
        serv_extra: "<obra><cObra>OB1</cObra></obra><atvEvento><idAtvEvt>EV1</idAtvEvt></atvEvento>\
            <infoCompl><idDocTec>DT1</idDocTec><docRef>REF</docRef><xPed>P1</xPed>\
            <gItemPed><xItemPed>I1</xItemPed></gItemPed><gItemPed><xItemPed> </xItemPed></gItemPed><gItemPed><xItemPed>I2</xItemPed></gItemPed>\
            <xInfComp>INF</xInfComp><xOutInf>OUT</xOutInf></infoCompl>",
        ..Fixture::default()
    });
    assert_eq!(
        montar_texto_compl(&nota),
        "Inf. Cont.: INF | Doc. Ref.: REF | Cod. Obra: OB1 | Cod. Evt.: EV1 | Doc. Tec.: DT1 | Núm. Ped.: P1 | Item Ped.: I1, I2 | Inf. A. T. Mun.: OUT"
    );
}

#[test]
fn truncar_preservando_totais() {
    assert_eq!(truncar_texto_compl_preservando_totais("", "L", 10), "L");
    assert_eq!(truncar_texto_compl_preservando_totais("abc", "", 10), "abc");
    assert_eq!(
        truncar_texto_compl_preservando_totais("abc", "L", 10),
        "abc | L"
    );
    assert_eq!(
        truncar_texto_compl_preservando_totais("abcd  efgh", "L", 9),
        "abcd... | L"
    );
    assert_eq!(
        truncar_texto_compl_preservando_totais("ááááááá", "", 5),
        "áá..."
    );
}

#[test]
fn atoi_como_go() {
    assert_eq!(parse_atoi("+12"), Ok(12));
    assert_eq!(parse_atoi("-7"), Ok(-7));
    assert_eq!(
        parse_atoi(" 1"),
        Err("strconv.Atoi: parsing \" 1\": invalid syntax".into())
    );
    assert_eq!(
        parse_atoi("99999999999999999999"),
        Err("strconv.Atoi: parsing \"99999999999999999999\": value out of range".into())
    );
}

#[test]
fn query_escape_como_go() {
    assert_eq!(query_escape("a b+c&d/é~"), "a+b%2Bc%26d%2F%C3%A9~");
}
