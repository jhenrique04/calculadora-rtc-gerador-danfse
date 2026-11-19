use super::ApresentadorDanfse;
use crate::domain::models::{
    Destinatario, Emitente, IbsCbs, Interm, NFSe, Tomador, TribFed, TribMun,
};
use crate::domain::presentation::format::*;
use crate::domain::presentation::{
    DadosAcesso, DadosCabecalho, DadosCanhoto, DadosInformacoesComplementares, DadosPartes,
    DadosServico, DadosTotais, DadosTributacaoFederal, DadosTributacaoIbsCbs,
    DadosTributacaoMunicipal, DocumentoDanfse, MarcaDagua, SecaoParte, Tributacoes,
};

const TRACO: &str = "-";
const MARCA_DAGUA_TAMANHO_FONTE: f64 = 60.0;
const MARCA_DAGUA_COR: f64 = 0.35;
const MARCA_DAGUA_ANGULO: f64 = 45.0;
const MARCA_DAGUA_OPACIDADE: f64 = 0.35;
// PIS/COFINS deixam de ser exibidos após a vigência plena da reforma
const FIM_EXIBICAO_PIS_COFINS: (u32, u32, u32) = (2026, 12, 31);

/// apresentador puro: sem I/O, XML ou dependências de PDF
#[derive(Debug, Default, Clone, Copy)]
pub struct ServicoApresentacaoDanfse;

impl ServicoApresentacaoDanfse {
    pub fn new() -> Self {
        Self
    }
}

impl ApresentadorDanfse for ServicoApresentacaoDanfse {
    fn apresentar(&self, nota: &NFSe) -> DocumentoDanfse {
        DocumentoDanfse {
            marca_dagua: montar_marca_dagua(nota),
            cabecalho: montar_cabecalho(nota),
            acesso: montar_acesso(nota),
            partes: montar_partes(nota),
            servico: montar_servico(nota),
            tributacoes: Tributacoes {
                municipal: montar_tributacao_municipal(nota),
                federal: montar_tributacao_federal(nota),
                ibs_cbs: montar_tributacao_ibs_cbs(nota),
            },
            totais: montar_totais(nota),
            informacoes_complementares: montar_informacoes_complementares(nota),
            canhoto: DadosCanhoto { visivel: true },
        }
    }
}

fn montar_marca_dagua(nota: &NFSe) -> MarcaDagua {
    MarcaDagua {
        texto: nota.marca_dagua.clone(),
        tamanho_fonte: MARCA_DAGUA_TAMANHO_FONTE,
        color_r: MARCA_DAGUA_COR,
        color_g: MARCA_DAGUA_COR,
        color_b: MARCA_DAGUA_COR,
        angulo: MARCA_DAGUA_ANGULO,
        opacidade: MARCA_DAGUA_OPACIDADE,
    }
}

fn montar_cabecalho(nota: &NFSe) -> DadosCabecalho {
    DadosCabecalho {
        marca_dagua: nota.marca_dagua.clone(),
        exibicao_desc_serv: nota.exibicao_desc_serv.clone(),
        exibicao_compl: nota.exibicao_compl.clone(),
        exibicao_linha_tributos: nota.exibicao_linha_tributos.clone(),
        municipio: nota.inf_nfse.emit.ender_nac.x_mun.clone(),
        uf: nota.inf_nfse.emit.ender_nac.uf.clone(),
        ambiente_gerador: mapear_ambiente_gerador(&nota.inf_nfse.amb_ger),
        tipo_ambiente: mapear_tipo_ambiente(nota.inf_nfse.dps.inf_dps.tp_amb),
    }
}

fn montar_acesso(nota: &NFSe) -> DadosAcesso {
    let inf = &nota.inf_nfse;
    let dps = &inf.dps.inf_dps;
    let finalidade = dps
        .ibscbs
        .as_ref()
        .and_then(|i| i.fin_nfse)
        .map_or_else(|| TRACO.to_string(), mapear_finalidade_nfse);
    DadosAcesso {
        chave_acesso: traco_se_vazio(&inf.id.replace("NFS", "")),
        codigo_qr: inf.qrcode.clone(),
        numero_nfse: traco_se_vazio(&inf.n_nfse),
        competencia: formatar_data(&dps.d_compet),
        data_hora_emissao_nfse: formatar_data_hora(&inf.dh_proc),
        numero_dps: traco_se_vazio(&dps.n_dps),
        serie_dps: traco_se_vazio(&dps.serie),
        data_hora_emissao_dps: formatar_data_hora(&dps.dh_emi),
        tipo_emitente: mapear_tipo_emitente(dps.tp_emit),
        situacao: mapear_c_stat(&inf.c_stat),
        finalidade,
    }
}

fn secao_suprimida(mensagem: &str) -> SecaoParte {
    SecaoParte {
        visivel: false,
        mensagem_supressao: mensagem.to_string(),
        ..SecaoParte::default()
    }
}

fn montar_partes(nota: &NFSe) -> DadosPartes {
    let dps = &nota.inf_nfse.dps.inf_dps;
    let tomador = dps.toma.as_ref().map_or_else(
        || secao_suprimida("TOMADOR/ADQUIRENTE DA OPERAÇÃO NÃO IDENTIFICADO NA NFS-e"),
        montar_tomador,
    );
    let destinatario = montar_destinatario(nota);
    let destinatario = if destinatario.visivel || !destinatario.mensagem_supressao.is_empty() {
        destinatario
    } else {
        secao_suprimida("DESTINATÁRIO DA OPERAÇÃO NÃO IDENTIFICADO NA NFS-e")
    };
    let intermediario = dps.interm.as_ref().map_or_else(
        || secao_suprimida("INTERMEDIÁRIO DA OPERAÇÃO NÃO IDENTIFICADO NA NFS-e"),
        montar_intermediario,
    );
    DadosPartes {
        emitente: montar_emitente(nota, &nota.inf_nfse.emit),
        tomador,
        destinatario,
        intermediario,
    }
}

fn documento(cnpj: &str, cpf: &str, nif: &str) -> String {
    traco_se_vazio(&formatar_documento(primeiro_nao_vazio(&[cnpj, cpf, nif])))
}

fn telefone(fone: &str) -> String {
    traco_se_vazio(&formatar_telefone(fone))
}

fn endereco(x_lgr: &str, nro: &str, x_cpl: &str, x_bairro: &str) -> String {
    traco_se_vazio(&montar_endereco(&[x_lgr, nro, x_cpl, x_bairro]))
}

fn montar_emitente(nota: &NFSe, e: &Emitente) -> SecaoParte {
    let (simples_nacional, regime_apuracao_sn) = match &nota.inf_nfse.dps.inf_dps.prest {
        Some(p) => (
            mapear_opcao_simples_nacional(p.reg_trib.op_simp_nac),
            mapear_regime_apuracao_sn(p.reg_trib.reg_ap_trib_sn),
        ),
        None => (TRACO.to_string(), TRACO.to_string()),
    };
    let end = &e.ender_nac;
    SecaoParte {
        titulo: "PRESTADOR / FORNECEDOR".into(),
        cnpj_cpf_nif: documento(&e.cnpj, &e.cpf, &e.nif),
        inscricao_municipal: traco_se_vazio(&e.im),
        telefone: telefone(&e.fone),
        nome: traco_se_vazio(&e.x_nome),
        municipio_uf: formatar_municipio_uf(&end.x_mun, &end.uf),
        codigo_ibge_cep: formatar_codigo_ibge_cep(&end.c_mun, &end.cep),
        endereco: endereco(&end.x_lgr, &end.nro, &end.x_cpl, &end.x_bairro),
        email: traco_se_vazio(&e.email),
        visivel: true,
        simples_nacional,
        regime_apuracao_sn,
        ..SecaoParte::default()
    }
}

fn montar_tomador(t: &Tomador) -> SecaoParte {
    let end_nac = t.end.as_ref().and_then(|e| e.end_nac.as_ref());
    SecaoParte {
        titulo: "TOMADOR / ADQUIRENTE".into(),
        cnpj_cpf_nif: documento(&t.cnpj, &t.cpf, &t.nif),
        inscricao_municipal: traco_se_vazio(&t.im),
        telefone: telefone(&t.fone),
        nome: traco_se_vazio(&t.x_nome),
        municipio_uf: end_nac
            .map_or_else(|| TRACO.into(), |n| formatar_municipio_uf(&n.x_mun, &n.uf)),
        codigo_ibge_cep: end_nac.map_or_else(
            || TRACO.into(),
            |n| formatar_codigo_ibge_cep(&n.c_mun, &n.cep),
        ),
        endereco: t.end.as_ref().map_or_else(
            || TRACO.into(),
            |e| endereco(&e.x_lgr, &e.nro, &e.x_cpl, &e.x_bairro),
        ),
        email: traco_se_vazio(&t.email),
        visivel: true,
        ..SecaoParte::default()
    }
}

fn montar_destinatario(nota: &NFSe) -> SecaoParte {
    let dps = &nota.inf_nfse.dps.inf_dps;
    let Some(dest) = dps.ibscbs.as_ref().and_then(|i| i.dest.as_ref()) else {
        return SecaoParte::default();
    };
    if destinatario_eh_tomador(dps.toma.as_ref(), dest) {
        return secao_suprimida("O DESTINATÁRIO É O PRÓPRIO TOMADOR/ADQUIRENTE DA OPERAÇÃO");
    }
    let end_nac = dest.end.as_ref().and_then(|e| e.end_nac.as_ref());
    SecaoParte {
        titulo: "DESTINATÁRIO DA OPERAÇÃO".into(),
        cnpj_cpf_nif: documento(&dest.cnpj, &dest.cpf, &dest.nif),
        telefone: telefone(&dest.fone),
        nome: traco_se_vazio(&dest.x_nome),
        municipio_uf: end_nac
            .map_or_else(|| TRACO.into(), |n| formatar_municipio_uf(&n.x_mun, &n.uf)),
        codigo_ibge_cep: end_nac.map_or_else(
            || TRACO.into(),
            |n| formatar_codigo_ibge_cep(&n.c_mun, &n.cep),
        ),
        endereco: dest.end.as_ref().map_or_else(
            || TRACO.into(),
            |e| endereco(&e.x_lgr, &e.nro, &e.x_cpl, &e.x_bairro),
        ),
        email: traco_se_vazio(&dest.email),
        visivel: true,
        ..SecaoParte::default()
    }
}

fn montar_intermediario(i: &Interm) -> SecaoParte {
    let end_nac = i.end.end_nac.as_ref();
    SecaoParte {
        titulo: "INTERMEDIÁRIO DA OPERAÇÃO".into(),
        cnpj_cpf_nif: documento(&i.cnpj, &i.cpf, &i.nif),
        inscricao_municipal: traco_se_vazio(&i.im),
        telefone: telefone(&i.fone),
        nome: traco_se_vazio(&i.x_nome),
        municipio_uf: end_nac
            .map_or_else(|| TRACO.into(), |n| formatar_municipio_uf(&n.x_mun, &n.uf)),
        codigo_ibge_cep: end_nac.map_or_else(
            || TRACO.into(),
            |n| formatar_codigo_ibge_cep(&n.c_mun, &n.cep),
        ),
        endereco: endereco(&i.end.x_lgr, &i.end.nro, &i.end.x_cpl, &i.end.x_bairro),
        email: traco_se_vazio(&i.email),
        visivel: true,
        ..SecaoParte::default()
    }
}

fn montar_servico(nota: &NFSe) -> DadosServico {
    let serv = &nota.inf_nfse.dps.inf_dps.serv;
    let cs = &serv.c_serv;
    DadosServico {
        codigo_trib_nac_mun: codigo_tributacao_nacional_ou_municipal(
            &cs.c_trib_nac,
            &cs.c_trib_mun,
        ),
        codigo_nbs: traco_se_vazio(&formatar_nbs(&cs.c_nbs)),
        local_prestacao_uf: combinar_com_barra(&[
            &nota.inf_nfse.x_loc_prestacao,
            &serv.loc_prest.uf,
            &codigo_pais_ou_br(&serv.loc_prest.c_pais_prestacao),
        ]),
        x_trib_nac: traco_se_vazio(primeiro_nao_vazio(&[
            &nota.inf_nfse.x_trib_mun,
            &nota.inf_nfse.x_trib_nac,
        ])),
        desc_serv: nota.exibicao_desc_serv.clone(),
        desc_serv_original: nota.texto_desc_serv_original.clone(),
    }
}

fn codigo_tributacao_nacional_ou_municipal(nacional: &str, municipal: &str) -> String {
    let (nacional, municipal) = (nacional.trim(), municipal.trim());
    if nacional.is_empty() {
        return municipal.to_string();
    }
    let nacional = formatar_codigo_tributacao_nacional(nacional);
    if municipal.is_empty() {
        return nacional;
    }
    format!("{nacional} / {municipal}")
}

fn montar_tributacao_municipal(nota: &NFSe) -> DadosTributacaoMunicipal {
    let inf = &nota.inf_nfse;
    let dps = &inf.dps.inf_dps;
    let padrao = TribMun::default();
    let trib = dps.valores.trib.trib_mun.as_ref().unwrap_or(&padrao);

    // prefere vCalcBM da NFS-e; senão o vRedBCBM da DPS
    let calculo_bm = if inf.valores.v_calc_bm == 0.0 {
        trib.bm.v_red_bc_bm
    } else {
        inf.valores.v_calc_bm
    };
    let reg_esp = dps.prest.as_ref().map_or(0, |p| p.reg_trib.reg_esp_trib);
    let exibir_opcionais = inf.valores.tp_bm > 0
        || inf.valores.v_calc_bm > 0.0
        || dps.valores.v_ded_red.v_dr > 0.0
        || dps.valores.v_desc_cond_incond.v_desc_incond > 0.0;

    let mut retorno = DadosTributacaoMunicipal {
        trib_issqn: mapear_tributacao_issqn(trib.trib_issqn),
        municipio_uf_pais_incid: combinar_com_barra(&[
            &inf.x_loc_incid,
            &inf.x_uf_incid,
            &codigo_pais_ou_br(&trib.c_pais_result),
        ]),
        reg_esp_trib: mapear_regime_especial_tributacao(reg_esp),
        tp_imunidade: mapear_tipo_imunidade(trib.tp_imunidade),
        suspensao_exig: mapear_suspensao_exigibilidade(trib.exig_susp.tp_susp),
        n_processo_susp: traco_se_vazio(&trib.exig_susp.n_processo),
        beneficio_municipal: mapear_tipo_beneficio_municipal(inf.valores.tp_bm),
        calculo_bm: formatar_valor_monetario_ou_traco(calculo_bm),
        total_deducoes: formatar_valor_monetario_ou_traco(dps.valores.v_ded_red.v_dr),
        desconto_incond: formatar_valor_monetario_ou_traco(
            dps.valores.v_desc_cond_incond.v_desc_incond,
        ),
        bc_issqn: formatar_valor_monetario_ou_traco(inf.valores.v_bc),
        aliq_aplic: formatar_numero_percentual_ou_traco(inf.valores.p_aliq_aplic.unwrap_or(0.0)),
        ret_issqn: mapear_tipo_retencao_issqn(trib.tp_ret_issqn),
        issqn_apurado: formatar_valor_monetario_ou_traco(inf.valores.v_issqn),
        visivel: true,
        mensagem_supressao: String::new(),
    };

    if !exibir_opcionais {
        retorno.beneficio_municipal = TRACO.into();
        retorno.calculo_bm = TRACO.into();
        retorno.total_deducoes = TRACO.into();
        retorno.desconto_incond = TRACO.into();
    }

    let tem_dados = dps
        .valores
        .trib
        .trib_mun
        .as_ref()
        .is_some_and(|t| tem_dados_tributacao_municipal(nota, t, reg_esp, calculo_bm));
    if !tem_dados {
        retorno.visivel = false;
        retorno.mensagem_supressao =
            "TRIBUTAÇÃO MUNICIPAL (ISSQN) - OPERAÇÃO NÃO SUJEITA AO ISSQN".into();
    }
    retorno
}

fn tem_dados_tributacao_municipal(
    nota: &NFSe,
    trib: &TribMun,
    reg_esp: i64,
    calculo_bm: f64,
) -> bool {
    let inf = &nota.inf_nfse;
    let valores_dps = &inf.dps.inf_dps.valores;
    trib.trib_issqn != 0
        || !inf.x_loc_incid.trim().is_empty()
        || !inf.x_uf_incid.trim().is_empty()
        || !trib.c_pais_result.trim().is_empty()
        || reg_esp != 0
        || trib.tp_imunidade != 0
        || trib.exig_susp.tp_susp != 0
        || !trib.exig_susp.n_processo.trim().is_empty()
        || inf.valores.tp_bm != 0
        || calculo_bm != 0.0
        || valores_dps.v_ded_red.v_dr != 0.0
        || valores_dps.v_desc_cond_incond.v_desc_incond != 0.0
        || inf.valores.v_bc != 0.0
        || inf.valores.p_aliq_aplic.is_some()
        || trib.tp_ret_issqn != 0
        || inf.valores.v_issqn != 0.0
}

fn montar_tributacao_federal(nota: &NFSe) -> DadosTributacaoFederal {
    let dps = &nota.inf_nfse.dps.inf_dps;
    let padrao = TribFed::default();
    let trib = dps.valores.trib.trib_fed.as_ref().unwrap_or(&padrao);

    let mut retorno = DadosTributacaoFederal {
        irrf: formatar_valor_monetario_ou_traco(trib.v_ret_irrf),
        contrib_previd_ret: formatar_valor_monetario_ou_traco(trib.v_ret_cp),
        contrib_soc_ret: formatar_valor_monetario_ou_traco(trib.v_ret_csll),
        pis_debito: TRACO.into(),
        cofins_debito: TRACO.into(),
        desc_contrib_ret: TRACO.into(),
        exibir_pis_cofins: exibir_pis_cofins(&dps.d_compet),
        visivel: true,
    };

    if let Some(pc) = &trib.pis_cofins {
        if pc.tp_ret_pis_cofins == 1 {
            // NT-008 v1.02 §2.4.5: com tpRetPisCofins=1 os próprios ficam zerados e a retenção soma CSLL+PIS+COFINS
            retorno.pis_debito = formatar_valor_monetario(0.0);
            retorno.cofins_debito = formatar_valor_monetario(0.0);
            retorno.contrib_soc_ret =
                formatar_valor_monetario(trib.v_ret_csll + pc.v_pis + pc.v_cofins);
        } else {
            retorno.pis_debito = formatar_valor_monetario_ou_traco(pc.v_pis);
            retorno.cofins_debito = formatar_valor_monetario_ou_traco(pc.v_cofins);
        }
        retorno.desc_contrib_ret = mapear_tipo_retencao_pis_cofins(pc.tp_ret_pis_cofins);
    }
    retorno
}

/// competência até 2026-12-31 exibe PIS/COFINS; vazia ou inválida também (compatibilidade)
fn exibir_pis_cofins(d_compet: &str) -> bool {
    parse_data_iso(d_compet).is_none_or(|data| data <= FIM_EXIBICAO_PIS_COFINS)
}

/// equivalente estrito a `time.Parse("2006-01-02", s)` do Go
fn parse_data_iso(s: &str) -> Option<(u32, u32, u32)> {
    let b = s.as_bytes();
    if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
        return None;
    }
    let numero = |r: std::ops::Range<usize>| -> Option<u32> {
        let parte = s.get(r)?;
        if !parte.bytes().all(|c| c.is_ascii_digit()) {
            return None;
        }
        parte.parse().ok()
    };
    let (ano, mes, dia) = (numero(0..4)?, numero(5..7)?, numero(8..10)?);
    if !(1..=12).contains(&mes) || dia < 1 || dia > dias_no_mes(ano, mes) {
        return None;
    }
    Some((ano, mes, dia))
}

fn dias_no_mes(ano: u32, mes: u32) -> u32 {
    match mes {
        2 if ano.is_multiple_of(4) && (!ano.is_multiple_of(100) || ano.is_multiple_of(400)) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

fn montar_tributacao_ibs_cbs(nota: &NFSe) -> DadosTributacaoIbsCbs {
    let dps = &nota.inf_nfse.dps.inf_dps;
    let padrao = IbsCbs::default();
    let ibscbs = nota.inf_nfse.ibscbs.as_ref().unwrap_or(&padrao);

    // componentes da DPS só valem quando a NFS-e também traz IBS/CBS
    let (cst, c_class_trib, c_ind_op) = match (&nota.inf_nfse.ibscbs, &dps.ibscbs) {
        (Some(_), Some(d)) => (
            traco_se_vazio(&d.valores.trib.g_ibs_cbs.cst),
            traco_se_vazio(&d.valores.trib.g_ibs_cbs.c_class_trib),
            traco_se_vazio(&d.c_ind_op),
        ),
        _ => (TRACO.into(), TRACO.into(), TRACO.into()),
    };
    let v = &ibscbs.valores;
    let tot = &ibscbs.tot_cibs;
    let pct = |p: Option<f64>| formatar_percentual_ou_traco(p.unwrap_or(0.0));

    DadosTributacaoIbsCbs {
        cst_c_class_trib: combinar_valores_com_barra(&[&cst, &c_class_trib]),
        ind_op_cod_ibge_localidade_uf: combinar_valores_com_barra(&[
            &c_ind_op,
            &traco_se_vazio(&ibscbs.c_localidade_incid),
            &traco_se_vazio(&ibscbs.x_localidade_incid),
            &traco_se_vazio(&ibscbs.uf),
        ]),
        exclusoes_reducoes_bc: formatar_valor_monetario_ou_traco(nota.v_exclusoes_reducoes_bc),
        bc_pos_exclusoes: formatar_valor_monetario_ou_traco(v.v_bc),
        red_aliq_ibs_red_aliq_cbs: combinar_valores_com_barra(&[
            &pct(v.uf.p_red_aliq_uf),
            &pct(v.mun.p_red_aliq_mun),
            &pct(v.fed.p_red_aliq_cbs),
        ]),
        aliq_ibs_uf_ibs_mun: combinar_valores_com_barra(&[
            &pct(v.uf.p_ibs_uf),
            &pct(v.mun.p_ibs_mun),
        ]),
        aliq_efet_mun_ibs: pct(v.mun.p_aliq_efet_mun),
        valor_mun_ibs: formatar_valor_monetario_ou_traco(
            tot.g_ibs.g_ibs_mun_tot.v_ibs_mun.unwrap_or(0.0),
        ),
        aliq_efet_est_ibs: pct(v.uf.p_aliq_efet_uf),
        valor_est_ibs: formatar_valor_monetario_ou_traco(
            tot.g_ibs.g_ibs_uf_tot.v_ibs_uf.unwrap_or(0.0),
        ),
        valor_total_ibs: formatar_valor_monetario_ou_traco(tot.g_ibs.v_ibs_tot),
        aliq_cbs: pct(v.fed.p_cbs),
        aliq_efet_cbs: pct(v.fed.p_aliq_efet_cbs),
        valor_total_cbs: formatar_valor_monetario_ou_traco(tot.g_cbs.v_cbs),
        visivel: true,
    }
}

fn montar_totais(nota: &NFSe) -> DadosTotais {
    let inf = &nota.inf_nfse;
    let valores_dps = &inf.dps.inf_dps.valores;
    DadosTotais {
        valor_operacao_servico: formatar_valor_monetario(valores_dps.v_serv_prest.v_serv),
        desconto_incond: formatar_valor_monetario_ou_traco(
            valores_dps.v_desc_cond_incond.v_desc_incond,
        ),
        desconto_cond: formatar_valor_monetario_ou_traco(
            valores_dps.v_desc_cond_incond.v_desc_cond,
        ),
        total_retencoes: formatar_valor_monetario_ou_traco(inf.valores.v_total_ret),
        valor_liquido: formatar_valor_monetario(inf.valores.v_liq),
        total_ibs_cbs: inf.ibscbs.as_ref().map_or_else(
            || TRACO.into(),
            |i| {
                formatar_valor_monetario_ou_traco(
                    i.tot_cibs.g_ibs.v_ibs_tot + i.tot_cibs.g_cbs.v_cbs,
                )
            },
        ),
        valor_liquido_ibs_cbs: inf.ibscbs.as_ref().map_or_else(
            || TRACO.into(),
            |i| formatar_valor_monetario_ou_traco(i.tot_cibs.v_tot_nf),
        ),
    }
}

fn montar_informacoes_complementares(nota: &NFSe) -> DadosInformacoesComplementares {
    DadosInformacoesComplementares {
        conteudo: nota.exibicao_compl.clone(),
        conteudo_original: nota.texto_compl_original.clone(),
        linha_tributos: nota.exibicao_linha_tributos.clone(),
        visivel: true,
    }
}

/// aplica `traco_se_vazio` a cada componente e junta com " / "; tudo vazio vira "-"
fn combinar_com_barra(valores: &[&str]) -> String {
    let partes: Vec<String> = valores.iter().map(|v| traco_se_vazio(v)).collect();
    if partes.iter().all(|p| p == TRACO) {
        return TRACO.into();
    }
    partes.join(" / ")
}

/// junta valores já formatados com " / ", trocando vazios por "-" e preservando o original
fn combinar_valores_com_barra(valores: &[&str]) -> String {
    let mut tem_informacao = false;
    let partes: Vec<&str> = valores
        .iter()
        .map(|v| {
            let aparado = v.trim();
            if aparado.is_empty() {
                return TRACO;
            }
            tem_informacao |= aparado != TRACO;
            v
        })
        .collect();
    if !tem_informacao {
        return TRACO.into();
    }
    partes.join(" / ")
}

fn destinatario_eh_tomador(toma: Option<&Tomador>, dest: &Destinatario) -> bool {
    let Some(toma) = toma else {
        return false;
    };
    [
        (&toma.cnpj, &dest.cnpj),
        (&toma.cpf, &dest.cpf),
        (&toma.nif, &dest.nif),
    ]
    .iter()
    .any(|(t, d)| !t.trim().is_empty() && t == d)
}

fn codigo_pais_ou_br(s: &str) -> String {
    let s = s.trim();
    if s.is_empty() {
        return "BR".into();
    }
    s.to_uppercase()
}

#[cfg(test)]
mod tests;
