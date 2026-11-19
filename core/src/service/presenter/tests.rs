use super::*;
use crate::domain::models::{
    AliqCbs, AliqIbsMun, AliqIbsUf, Bm, EndNac, Endereco, EnderecoDps, EnderecoTomador, GCbsTot,
    GIbsCbs, GIbsMunTot, GIbsTot, GIbsUfTot, IbsCbsDps, PisCofins, Prestador, RegTrib, TotCibs,
    TribIbsCbs, ValoresIbsCbs, ValoresIbsCbsResp,
};

fn apresentar(nota: &NFSe) -> DocumentoDanfse {
    ServicoApresentacaoDanfse::new().apresentar(nota)
}

fn com(f: impl FnOnce(&mut NFSe)) -> NFSe {
    let mut n = NFSe::default();
    f(&mut n);
    n
}

fn trib_mun_basico() -> TribMun {
    TribMun {
        trib_issqn: 1,
        tp_ret_issqn: 1,
        ..TribMun::default()
    }
}

fn endereco_emitente() -> Endereco {
    Endereco {
        x_mun: "São Paulo".into(),
        uf: "SP".into(),
        c_mun: "3550308".into(),
        cep: "01001000".into(),
        x_lgr: "Rua Teste".into(),
        nro: "123".into(),
        x_bairro: "Centro".into(),
        ..Endereco::default()
    }
}

#[test]
fn marca_dagua() {
    let doc = apresentar(&com(|n| n.marca_dagua = "CANCELADA".into()));
    assert_eq!(
        doc.marca_dagua,
        MarcaDagua {
            texto: "CANCELADA".into(),
            tamanho_fonte: 60.0,
            color_r: 0.35,
            color_g: 0.35,
            color_b: 0.35,
            angulo: 45.0,
            opacidade: 0.35,
        }
    );
    assert_eq!(apresentar(&NFSe::default()).marca_dagua.texto, "");
}

#[test]
fn partes_ausentes_suprimidas() {
    let doc = apresentar(&NFSe::default());
    let p = &doc.partes;
    assert!(!p.tomador.visivel && !p.destinatario.visivel && !p.intermediario.visivel);
    assert_eq!(
        p.tomador.mensagem_supressao,
        "TOMADOR/ADQUIRENTE DA OPERAÇÃO NÃO IDENTIFICADO NA NFS-e"
    );
    assert_eq!(
        p.destinatario.mensagem_supressao,
        "DESTINATÁRIO DA OPERAÇÃO NÃO IDENTIFICADO NA NFS-e"
    );
    assert_eq!(
        p.intermediario.mensagem_supressao,
        "INTERMEDIÁRIO DA OPERAÇÃO NÃO IDENTIFICADO NA NFS-e"
    );
    assert!(doc.canhoto.visivel);
}

fn nota_com_destinatario(cnpj_toma: &str, cnpj_dest: &str) -> NFSe {
    com(|n| {
        let dps = &mut n.inf_nfse.dps.inf_dps;
        dps.toma = Some(Tomador {
            cnpj: cnpj_toma.into(),
            x_nome: "Tomador".into(),
            ..Tomador::default()
        });
        dps.ibscbs = Some(IbsCbsDps {
            dest: Some(Destinatario {
                cnpj: cnpj_dest.into(),
                x_nome: "Dest".into(),
                ..Destinatario::default()
            }),
            ..IbsCbsDps::default()
        });
    })
}

#[test]
fn destinatario_igual_ao_tomador_suprimido() {
    let doc = apresentar(&nota_com_destinatario("12345678000195", "12345678000195"));
    assert!(!doc.partes.destinatario.visivel);
    assert!(
        doc.partes
            .destinatario
            .mensagem_supressao
            .contains("PRÓPRIO TOMADOR")
    );
}

#[test]
fn destinatario_diferente_visivel() {
    let doc = apresentar(&nota_com_destinatario("11111111000111", "22222222000222"));
    assert!(doc.partes.destinatario.visivel);
    assert_eq!(doc.partes.destinatario.titulo, "DESTINATÁRIO DA OPERAÇÃO");
    assert_eq!(doc.partes.destinatario.cnpj_cpf_nif, "22.222.222/0002-22");
    assert_eq!(doc.partes.destinatario.endereco, "-");
}

#[test]
fn trib_mun_ausente() {
    let doc = apresentar(&NFSe::default());
    let t = &doc.tributacoes;
    assert!(!t.municipal.visivel);
    assert_eq!(
        t.municipal.mensagem_supressao,
        "TRIBUTAÇÃO MUNICIPAL (ISSQN) - OPERAÇÃO NÃO SUJEITA AO ISSQN"
    );
    assert!(t.federal.visivel);
    assert!(t.ibs_cbs.visivel);
    assert_eq!(t.municipal.trib_issqn, "-");
    assert_eq!(t.federal.irrf, "-");
    assert_eq!(t.ibs_cbs.cst_c_class_trib, "-");
}

#[test]
fn trib_mun_visivel_e_rotulos() {
    let doc = apresentar(&com(|n| {
        n.inf_nfse.dps.inf_dps.valores.trib.trib_mun = Some(TribMun {
            bm: Bm {
                v_red_bc_bm: 100.0,
                ..Bm::default()
            },
            ..trib_mun_basico()
        });
    }));
    let m = &doc.tributacoes.municipal;
    assert!(m.visivel);
    assert_eq!(m.trib_issqn, "Operação Tributável");
    assert_eq!(m.ret_issqn, "Não Retido");
    assert_eq!(m.beneficio_municipal, "-");
    assert_eq!(m.calculo_bm, "-");
    assert_eq!(m.n_processo_susp, "-");
    assert_eq!(m.municipio_uf_pais_incid, "- / - / BR");
}

#[test]
fn cabecalho() {
    let doc = apresentar(&com(|n| {
        n.marca_dagua = "TESTE".into();
        n.inf_nfse.amb_ger = "1".into();
        n.inf_nfse.dps.inf_dps.tp_amb = 1;
        n.inf_nfse.emit.ender_nac = Endereco {
            x_mun: "São Paulo".into(),
            uf: "SP".into(),
            ..Endereco::default()
        };
    }));
    let c = &doc.cabecalho;
    assert_eq!((c.municipio.as_str(), c.uf.as_str()), ("São Paulo", "SP"));
    assert_eq!(c.ambiente_gerador, "Ambiente de Produção");
    assert_eq!(c.tipo_ambiente, "Produção");
    assert_eq!(c.marca_dagua, "TESTE");
}

#[test]
fn acesso_preenchido() {
    let doc = apresentar(&com(|n| {
        n.inf_nfse.id = "NFS123456789".into();
        n.inf_nfse.qrcode = "https://qrcode.test".into();
        n.inf_nfse.n_nfse = "42".into();
        n.inf_nfse.dh_proc = "2026-07-14T10:00:00-03:00".into();
        n.inf_nfse.c_stat = "100".into();
        let dps = &mut n.inf_nfse.dps.inf_dps;
        dps.n_dps = "10".into();
        dps.serie = "S1".into();
        dps.dh_emi = "2026-07-14T09:00:00-03:00".into();
        dps.d_compet = "2026-07-14".into();
        dps.tp_emit = 1;
    }));
    let a = &doc.acesso;
    assert_eq!(a.chave_acesso, "123456789");
    assert_eq!(a.numero_nfse, "42");
    assert_eq!(a.numero_dps, "10");
    assert_eq!(a.serie_dps, "S1");
    assert_eq!(a.competencia, "14/07/2026");
    assert_eq!(a.data_hora_emissao_nfse, "14/07/2026 10:00:00");
    assert_eq!(a.data_hora_emissao_dps, "14/07/2026 09:00:00");
    assert_eq!(a.tipo_emitente, "Prestador");
    assert_eq!(a.situacao, "NFS-e Regular");
    assert_eq!(a.finalidade, "-");
}

#[test]
fn acesso_vazio() {
    let doc = apresentar(&com(|n| n.inf_nfse.qrcode = "https://qrcode.test".into()));
    let a = &doc.acesso;
    for v in [
        &a.chave_acesso,
        &a.numero_nfse,
        &a.competencia,
        &a.data_hora_emissao_nfse,
        &a.numero_dps,
        &a.serie_dps,
        &a.data_hora_emissao_dps,
    ] {
        assert_eq!(v, "-");
    }
    assert_eq!(a.codigo_qr, "https://qrcode.test");
}

#[test]
fn acesso_finalidade() {
    let com_fin = |fin: Option<i64>| {
        apresentar(&com(|n| {
            n.inf_nfse.dps.inf_dps.ibscbs = Some(IbsCbsDps {
                fin_nfse: fin,
                ..IbsCbsDps::default()
            });
        }))
        .acesso
        .finalidade
    };
    assert_eq!(com_fin(Some(0)), "NFS-e regular");
    assert_eq!(com_fin(None), "-");
}

#[test]
fn servico() {
    let doc = apresentar(&com(|n| {
        n.exibicao_desc_serv = "Serviço de Teste".into();
        n.inf_nfse.x_trib_nac = "Transporte de cargas".into();
        let serv = &mut n.inf_nfse.dps.inf_dps.serv;
        serv.c_serv.c_trib_nac = "010101".into();
        serv.c_serv.c_trib_mun = "001".into();
        serv.c_serv.c_nbs = "123456789".into();
        serv.loc_prest.uf = "SP".into();
        serv.loc_prest.c_pais_prestacao = "BR".into();
    }));
    let s = &doc.servico;
    assert_eq!(s.desc_serv, "Serviço de Teste");
    assert_eq!(s.codigo_trib_nac_mun, "01.01.01 / 001");
    assert_eq!(s.codigo_nbs, "1.2345.67.89");
    assert_eq!(s.local_prestacao_uf, "- / SP / BR");
    assert_eq!(s.x_trib_nac, "Transporte de cargas");
}

#[test]
fn servico_vazio_e_paises() {
    let s = apresentar(&NFSe::default()).servico;
    assert_eq!(s.codigo_trib_nac_mun, "");
    assert_eq!(s.codigo_nbs, "-");
    assert_eq!(s.local_prestacao_uf, "- / - / BR");
    assert_eq!(s.x_trib_nac, "-");

    let us = apresentar(&com(|n| {
        n.inf_nfse.dps.inf_dps.serv.loc_prest.c_pais_prestacao = "us".into()
    }));
    assert_eq!(us.servico.local_prestacao_uf, "- / - / US");
}

#[test]
fn servico_x_trib_mun_tem_prioridade() {
    let doc = apresentar(&com(|n| {
        n.inf_nfse.x_trib_mun = "Tributação municipal".into();
        n.inf_nfse.x_trib_nac = "Tributação nacional".into();
    }));
    assert_eq!(doc.servico.x_trib_nac, "Tributação municipal");
}

#[test]
fn codigos_tributacao() {
    let casos = [
        ("010101", "001", "01.01.01 / 001"),
        ("010101", "", "01.01.01"),
        ("", "001", "001"),
        ("", "", ""),
    ];
    for (nac, mun, esperado) in casos {
        assert_eq!(codigo_tributacao_nacional_ou_municipal(nac, mun), esperado);
    }
}

#[test]
fn informacoes_complementares() {
    let doc = apresentar(&com(|n| {
        n.exibicao_compl = "Info complementar".into();
        n.exibicao_linha_tributos = "Linha tributária".into();
    }));
    assert_eq!(doc.informacoes_complementares.conteudo, "Info complementar");
    assert_eq!(
        doc.informacoes_complementares.linha_tributos,
        "Linha tributária"
    );
}

#[test]
fn emitente() {
    let doc = apresentar(&com(|n| {
        n.inf_nfse.emit = Emitente {
            cnpj: "12345678000195".into(),
            im: "123456".into(),
            fone: "11999998888".into(),
            x_nome: "Empresa Teste".into(),
            ender_nac: endereco_emitente(),
            email: "teste@empresa.com".into(),
            ..Emitente::default()
        };
        n.inf_nfse.dps.inf_dps.prest = Some(Prestador {
            reg_trib: RegTrib {
                op_simp_nac: 2,
                reg_ap_trib_sn: 1,
                reg_esp_trib: 0,
            },
        });
        n.inf_nfse.dps.inf_dps.toma = Some(Tomador {
            cnpj: "98765432000199".into(),
            ..Tomador::default()
        });
    }));
    let e = &doc.partes.emitente;
    assert!(e.visivel);
    assert_eq!(e.cnpj_cpf_nif, "12.345.678/0001-95");
    assert_eq!(e.nome, "Empresa Teste");
    assert_eq!(e.telefone, "(11) 99999-8888");
    assert_eq!(e.municipio_uf, "São Paulo / SP");
    assert_eq!(e.codigo_ibge_cep, "3550308 / 01.001-000");
    assert_eq!(e.endereco, "Rua Teste, 123, Centro");
    assert_eq!(e.simples_nacional, "2 - Optante - MEI");
    assert_eq!(
        e.regime_apuracao_sn,
        "1 - Regime de apuração dos tributos federais e municipal pelo SN"
    );
    assert_eq!(doc.partes.tomador.simples_nacional, "");
    assert_eq!(doc.partes.tomador.regime_apuracao_sn, "");
}

#[test]
fn emitente_campos_vazios_e_sem_prestador() {
    let doc = apresentar(&com(|n| {
        n.inf_nfse.emit = Emitente {
            cnpj: "12345678000195".into(),
            ender_nac: endereco_emitente(),
            ..Emitente::default()
        };
    }));
    let e = &doc.partes.emitente;
    for v in [
        &e.inscricao_municipal,
        &e.telefone,
        &e.nome,
        &e.email,
        &e.simples_nacional,
        &e.regime_apuracao_sn,
    ] {
        assert_eq!(v, "-");
    }
}

#[test]
fn tomador() {
    let doc = apresentar(&com(|n| {
        n.inf_nfse.dps.inf_dps.toma = Some(Tomador {
            cpf: "12345678901".into(),
            x_nome: "Tomador Teste".into(),
            end: Some(EnderecoTomador {
                x_lgr: "Av. Tomador".into(),
                nro: "456".into(),
                x_bairro: "Jardim".into(),
                end_nac: Some(EndNac {
                    x_mun: "Rio de Janeiro".into(),
                    uf: "RJ".into(),
                    c_mun: "3304557".into(),
                    cep: "20000000".into(),
                    ..EndNac::default()
                }),
                ..EnderecoTomador::default()
            }),
            ..Tomador::default()
        });
    }));
    let t = &doc.partes.tomador;
    assert!(t.visivel);
    assert_eq!(t.cnpj_cpf_nif, "123.456.789-01");
    assert_eq!(t.municipio_uf, "Rio de Janeiro / RJ");
    assert_eq!(t.endereco, "Av. Tomador, 456, Jardim");
}

#[test]
fn tomador_sem_endereco() {
    let doc = apresentar(&com(|n| {
        n.inf_nfse.dps.inf_dps.toma = Some(Tomador {
            cnpj: "12345678000195".into(),
            ..Tomador::default()
        });
    }));
    let t = &doc.partes.tomador;
    assert!(t.visivel);
    assert_eq!(
        (
            t.municipio_uf.as_str(),
            t.codigo_ibge_cep.as_str(),
            t.endereco.as_str()
        ),
        ("-", "-", "-")
    );
}

#[test]
fn intermediario() {
    let doc = apresentar(&com(|n| {
        n.inf_nfse.dps.inf_dps.interm = Some(Interm {
            cnpj: "98765432000199".into(),
            x_nome: "Intermediário Ltda".into(),
            end: EnderecoDps {
                x_lgr: "Rua Inter".into(),
                end_nac: Some(Endereco {
                    x_mun: "Belo Horizonte".into(),
                    uf: "MG".into(),
                    ..Endereco::default()
                }),
                ..EnderecoDps::default()
            },
            ..Interm::default()
        });
    }));
    let i = &doc.partes.intermediario;
    assert!(i.visivel);
    assert_eq!(i.cnpj_cpf_nif, "98.765.432/0001-99");
    assert_eq!(i.municipio_uf, "Belo Horizonte / MG");
}

#[test]
fn totais() {
    let doc = apresentar(&com(|n| {
        n.inf_nfse.valores.v_liq = 900.0;
        n.inf_nfse.valores.v_total_ret = 100.0;
        let v = &mut n.inf_nfse.dps.inf_dps.valores;
        v.v_serv_prest.v_serv = 1000.0;
        v.v_desc_cond_incond.v_desc_incond = 50.0;
        v.v_desc_cond_incond.v_desc_cond = 25.0;
    }));
    let t = &doc.totais;
    assert_eq!(t.valor_operacao_servico, "R$ 1.000,00");
    assert_eq!(t.valor_liquido, "R$ 900,00");
    assert_eq!(t.total_retencoes, "R$ 100,00");
    assert_eq!(t.desconto_incond, "R$ 50,00");
    assert_eq!(t.desconto_cond, "R$ 25,00");
    assert_eq!(t.total_ibs_cbs, "-");
    assert_eq!(t.valor_liquido_ibs_cbs, "-");
}

#[test]
fn totais_zerados() {
    let t = apresentar(&NFSe::default()).totais;
    assert_eq!(t.valor_operacao_servico, "R$ 0,00");
    assert_eq!(t.valor_liquido, "R$ 0,00");
    assert_eq!(t.desconto_incond, "-");
    assert_eq!(t.desconto_cond, "-");
    assert_eq!(t.total_retencoes, "-");
}

fn nota_com_totais_ibs_cbs(ibs: f64, cbs: f64, tot_nf: f64) -> NFSe {
    com(|n| {
        n.inf_nfse.ibscbs = Some(IbsCbs {
            tot_cibs: TotCibs {
                g_ibs: GIbsTot {
                    v_ibs_tot: ibs,
                    ..GIbsTot::default()
                },
                g_cbs: GCbsTot { v_cbs: cbs },
                v_tot_nf: tot_nf,
            },
            ..IbsCbs::default()
        });
    })
}

#[test]
fn totais_ibs_cbs() {
    let casos = [
        ((150.0, 250.0, 1200.0), ("R$ 400,00", "R$ 1.200,00")),
        ((300.0, 0.0, 500.0), ("R$ 300,00", "R$ 500,00")),
        ((0.0, 175.0, 825.0), ("R$ 175,00", "R$ 825,00")),
        ((100.0, 200.0, 0.0), ("R$ 300,00", "-")),
        ((0.0, 0.0, 0.0), ("-", "-")),
    ];
    for ((ibs, cbs, nf), (total, liquido)) in casos {
        let t = apresentar(&nota_com_totais_ibs_cbs(ibs, cbs, nf)).totais;
        assert_eq!(
            (t.total_ibs_cbs.as_str(), t.valor_liquido_ibs_cbs.as_str()),
            (total, liquido)
        );
    }
}

#[test]
fn ibs_cbs_ausente_ou_vazio() {
    for nota in [
        NFSe::default(),
        com(|n| n.inf_nfse.ibscbs = Some(IbsCbs::default())),
    ] {
        let i = apresentar(&nota).tributacoes.ibs_cbs;
        assert!(i.visivel);
        assert_eq!(i.cst_c_class_trib, "-");
        assert_eq!(i.ind_op_cod_ibge_localidade_uf, "-");
        assert_eq!(i.red_aliq_ibs_red_aliq_cbs, "-");
        assert_eq!(i.aliq_ibs_uf_ibs_mun, "-");
    }
}

fn dps_ibs_cbs(c_ind_op: &str) -> IbsCbsDps {
    IbsCbsDps {
        c_ind_op: c_ind_op.into(),
        valores: ValoresIbsCbs {
            trib: TribIbsCbs {
                g_ibs_cbs: GIbsCbs {
                    cst: "01".into(),
                    c_class_trib: "001".into(),
                },
            },
        },
        ..IbsCbsDps::default()
    }
}

#[test]
fn ibs_cbs_dps_nao_vaza_sem_nfse() {
    let i = apresentar(&com(|n| {
        n.inf_nfse.dps.inf_dps.ibscbs = Some(dps_ibs_cbs("1"))
    }))
    .tributacoes
    .ibs_cbs;
    assert_eq!(i.cst_c_class_trib, "-");
    assert_eq!(i.ind_op_cod_ibge_localidade_uf, "-");
}

#[test]
fn ibs_cbs_parcial() {
    let i = apresentar(&com(|n| {
        n.inf_nfse.ibscbs = Some(IbsCbs {
            c_localidade_incid: "12345".into(),
            valores: ValoresIbsCbsResp {
                v_bc: 1000.0,
                uf: AliqIbsUf {
                    p_ibs_uf: Some(2.5),
                    p_aliq_efet_uf: Some(2.5),
                    ..AliqIbsUf::default()
                },
                mun: AliqIbsMun {
                    p_ibs_mun: Some(2.5),
                    p_aliq_efet_mun: Some(2.5),
                    ..AliqIbsMun::default()
                },
                ..ValoresIbsCbsResp::default()
            },
            ..IbsCbs::default()
        });
    }))
    .tributacoes
    .ibs_cbs;
    assert_eq!(i.cst_c_class_trib, "-");
    assert_eq!(i.ind_op_cod_ibge_localidade_uf, "- / 12345 / - / -");
    assert_eq!(i.red_aliq_ibs_red_aliq_cbs, "-");
    assert_eq!(i.aliq_ibs_uf_ibs_mun, "2,50% / 2,50%");
    assert_eq!(i.bc_pos_exclusoes, "R$ 1.000,00");
}

#[test]
fn ibs_cbs_preenchido() {
    let i = apresentar(&com(|n| {
        n.inf_nfse.ibscbs = Some(IbsCbs {
            c_localidade_incid: "3550308".into(),
            x_localidade_incid: "São Paulo".into(),
            uf: "SP".into(),
            valores: ValoresIbsCbsResp {
                v_bc: 5000.0,
                v_calc_ree_rep_res: 0.0,
                uf: AliqIbsUf {
                    p_ibs_uf: Some(1.5),
                    p_aliq_efet_uf: Some(1.5),
                    p_red_aliq_uf: Some(0.5),
                },
                mun: AliqIbsMun {
                    p_ibs_mun: Some(2.5),
                    p_aliq_efet_mun: Some(2.5),
                    p_red_aliq_mun: Some(1.0),
                },
                fed: AliqCbs {
                    p_cbs: Some(2.5),
                    p_aliq_efet_cbs: Some(2.5),
                    p_red_aliq_cbs: Some(0.25),
                },
            },
            tot_cibs: TotCibs {
                g_ibs: GIbsTot {
                    v_ibs_tot: 200.0,
                    g_ibs_mun_tot: GIbsMunTot {
                        v_ibs_mun: Some(100.0),
                    },
                    g_ibs_uf_tot: GIbsUfTot {
                        v_ibs_uf: Some(100.0),
                    },
                },
                g_cbs: GCbsTot { v_cbs: 300.0 },
                v_tot_nf: 0.0,
            },
        });
        n.inf_nfse.dps.inf_dps.ibscbs = Some(dps_ibs_cbs("0"));
    }))
    .tributacoes
    .ibs_cbs;
    assert_eq!(i.cst_c_class_trib, "01 / 001");
    assert_eq!(
        i.ind_op_cod_ibge_localidade_uf,
        "0 / 3550308 / São Paulo / SP"
    );
    assert_eq!(i.red_aliq_ibs_red_aliq_cbs, "0,50% / 1,00% / 0,25%");
    assert_eq!(i.aliq_ibs_uf_ibs_mun, "1,50% / 2,50%");
    assert_eq!(i.valor_mun_ibs, "R$ 100,00");
    assert_eq!(i.valor_est_ibs, "R$ 100,00");
    assert_eq!(i.valor_total_ibs, "R$ 200,00");
    assert_eq!(i.valor_total_cbs, "R$ 300,00");
    assert_eq!(i.aliq_cbs, "2,50%");
}

#[test]
fn municipal_campos_monetarios() {
    let doc = apresentar(&com(|n| {
        n.inf_nfse.valores.tp_bm = 1;
        n.inf_nfse.valores.v_bc = 2000.0;
        n.inf_nfse.valores.v_issqn = 100.0;
        let v = &mut n.inf_nfse.dps.inf_dps.valores;
        v.trib.trib_mun = Some(TribMun {
            bm: Bm {
                v_red_bc_bm: 1000.0,
                ..Bm::default()
            },
            ..trib_mun_basico()
        });
        v.v_ded_red.v_dr = 200.0;
        v.v_desc_cond_incond.v_desc_incond = 100.0;
    }));
    let m = &doc.tributacoes.municipal;
    assert_eq!(m.calculo_bm, "R$ 1.000,00");
    assert_eq!(m.total_deducoes, "R$ 200,00");
    assert_eq!(m.desconto_incond, "R$ 100,00");
    assert_eq!(m.bc_issqn, "R$ 2.000,00");
    assert_eq!(m.issqn_apurado, "R$ 100,00");
    assert_eq!(m.beneficio_municipal, "Isenção");
}

#[test]
fn municipal_zeros_e_aliquota_zero() {
    let doc = apresentar(&com(|n| {
        n.inf_nfse.valores.tp_bm = 1;
        n.inf_nfse.valores.p_aliq_aplic = Some(0.0);
        n.inf_nfse.dps.inf_dps.valores.trib.trib_mun = Some(trib_mun_basico());
    }));
    let m = &doc.tributacoes.municipal;
    for v in [
        &m.calculo_bm,
        &m.total_deducoes,
        &m.desconto_incond,
        &m.bc_issqn,
        &m.issqn_apurado,
        &m.aliq_aplic,
    ] {
        assert_eq!(v, "-");
    }
}

#[test]
fn municipal_pais_resultado() {
    let pais = |c: &str| {
        apresentar(&com(|n| {
            n.inf_nfse.x_loc_incid = "São Paulo".into();
            n.inf_nfse.x_uf_incid = "SP".into();
            n.inf_nfse.dps.inf_dps.valores.trib.trib_mun = Some(TribMun {
                c_pais_result: c.into(),
                ..trib_mun_basico()
            });
        }))
        .tributacoes
        .municipal
        .municipio_uf_pais_incid
    };
    assert_eq!(pais("BR"), "São Paulo / SP / BR");
    assert_eq!(pais(""), "São Paulo / SP / BR");
}

fn nota_federal(d_compet: &str, pc: PisCofins, fed: TribFed) -> NFSe {
    com(|n| {
        n.inf_nfse.dps.inf_dps.d_compet = d_compet.into();
        n.inf_nfse.dps.inf_dps.valores.trib.trib_fed = Some(TribFed {
            pis_cofins: Some(pc),
            ..fed
        });
    })
}

#[test]
fn federal_sem_pis_cofins() {
    let doc = apresentar(&com(|n| {
        n.inf_nfse.dps.inf_dps.valores.trib.trib_fed = Some(TribFed::default())
    }));
    let f = &doc.tributacoes.federal;
    assert!(f.visivel);
    for v in [
        &f.irrf,
        &f.contrib_previd_ret,
        &f.contrib_soc_ret,
        &f.pis_debito,
        &f.cofins_debito,
        &f.desc_contrib_ret,
    ] {
        assert_eq!(v, "-");
    }
}

#[test]
fn federal_tp_ret_1_soma_e_zera_proprios() {
    let pc = PisCofins {
        v_pis: 10.0,
        v_cofins: 20.0,
        tp_ret_pis_cofins: 1,
        ..PisCofins::default()
    };
    let fed = TribFed {
        v_ret_irrf: 50.0,
        v_ret_cp: 100.0,
        v_ret_csll: 30.0,
        pis_cofins: None,
    };
    let f = apresentar(&nota_federal("2025-06-15", pc, fed))
        .tributacoes
        .federal;
    assert_eq!(f.contrib_soc_ret, "R$ 60,00");
    assert_eq!(f.pis_debito, "R$ 0,00");
    assert_eq!(f.cofins_debito, "R$ 0,00");
    assert_eq!(f.irrf, "R$ 50,00");
    assert_eq!(f.desc_contrib_ret, "PIS/COFINS Retidos");
}

#[test]
fn federal_tp_ret_diferente_de_1_preserva() {
    let pc = PisCofins {
        v_pis: 10.0,
        v_cofins: 20.0,
        tp_ret_pis_cofins: 0,
        ..PisCofins::default()
    };
    let f = apresentar(&nota_federal("2025-06-15", pc, TribFed::default()))
        .tributacoes
        .federal;
    assert_eq!(f.pis_debito, "R$ 10,00");
    assert_eq!(f.cofins_debito, "R$ 20,00");
    assert_eq!(f.contrib_soc_ret, "-");
}

#[test]
fn exibir_pis_cofins_limites() {
    let casos = [
        ("2026-12-31", true),
        ("2027-01-01", false),
        ("2025-01-01", true),
        ("", true),
        ("data-invalida", true),
        ("2027-02-30", true),
        ("2028-02-29", false),
        ("2027-1-01", true),
        ("2027-01-01T00:00:00", true),
    ];
    for (d, esperado) in casos {
        assert_eq!(exibir_pis_cofins(d), esperado, "{d}");
    }
}

#[test]
fn combinadores() {
    assert_eq!(combinar_com_barra(&["", " ", ""]), "-");
    assert_eq!(combinar_com_barra(&["a", "", "b"]), "a / - / b");
    assert_eq!(combinar_valores_com_barra(&["-", " "]), "-");
    assert_eq!(combinar_valores_com_barra(&["-", "1,00%"]), "- / 1,00%");
    assert_eq!(combinar_valores_com_barra(&[" x ", ""]), " x  / -");
}
