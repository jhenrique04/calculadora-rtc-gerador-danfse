use std::io::Read;

use flate2::read::ZlibDecoder;

use super::layout::*;
use super::style::*;
use super::text::*;
use super::*;
use crate::domain::models::NFSe;
use crate::domain::presentation::{
    DadosAcesso, DadosCabecalho, DadosCanhoto, DadosInformacoesComplementares, DadosPartes,
    DadosServico, DadosTotais, DadosTributacaoFederal, DadosTributacaoIbsCbs,
    DadosTributacaoMunicipal, Tributacoes,
};
use crate::pdf::win_ansi_encode;
use crate::service::{ApresentadorDanfse, ServicoApresentacaoDanfse};

const EPS: f64 = 1e-9;

fn renderer() -> Renderer {
    match Renderer::new() {
        Ok(r) => r,
        Err(e) => panic!("{e}"),
    }
}

fn renderizar(doc: &DocumentoDanfse) -> Vec<u8> {
    match renderer().renderizar(doc) {
        Ok(p) => p,
        Err(e) => panic!("{e}"),
    }
}

/// fluxo de conteúdo descomprimido da página (último stream antes do /Page)
fn conteudo(pdf: &[u8]) -> Vec<u8> {
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

fn contem(pdf: &[u8], texto: &str) -> bool {
    let alvo = win_ansi_encode(texto);
    conteudo(pdf)
        .windows(alvo.len())
        .any(|w| w == alvo.as_slice())
}

fn doc_base() -> DocumentoDanfse {
    DocumentoDanfse {
        cabecalho: DadosCabecalho {
            municipio: "São Paulo".into(),
            uf: "SP".into(),
            ambiente_gerador: "Emissor de teste".into(),
            tipo_ambiente: "Homologação".into(),
            exibicao_desc_serv: "Serviço de teste".into(),
            ..Default::default()
        },
        acesso: DadosAcesso {
            chave_acesso: "35260800000000000000000000000000001234567890".into(),
            codigo_qr: "https://exemplo/qr?token=abc".into(),
            numero_nfse: "1".into(),
            competencia: "08/2025".into(),
            data_hora_emissao_nfse: "2025-08-01T10:00:00".into(),
            numero_dps: "1".into(),
            serie_dps: "1".into(),
            data_hora_emissao_dps: "2025-08-01T09:00:00".into(),
            ..Default::default()
        },
        partes: DadosPartes {
            emitente: SecaoParte {
                titulo: "EMITENTE".into(),
                cnpj_cpf_nif: "12345678000199".into(),
                inscricao_municipal: "1234567".into(),
                nome: "PRESTADOR DE TESTE LTDA".into(),
                municipio_uf: "São Paulo/SP".into(),
                ..Default::default()
            },
            tomador: SecaoParte {
                titulo: "TOMADOR".into(),
                cnpj_cpf_nif: "99887766000155".into(),
                nome: "TOMADOR DE TESTE LTDA".into(),
                municipio_uf: "São Paulo/SP".into(),
                visivel: true,
                ..Default::default()
            },
            ..Default::default()
        },
        servico: DadosServico {
            codigo_nbs: "999999".into(),
            local_prestacao_uf: "BR".into(),
            desc_serv: "Serviço de teste para validação de placeholders".into(),
            ..Default::default()
        },
        tributacoes: Tributacoes {
            municipal: DadosTributacaoMunicipal {
                visivel: true,
                ..Default::default()
            },
            federal: DadosTributacaoFederal {
                visivel: true,
                ..Default::default()
            },
            ibs_cbs: DadosTributacaoIbsCbs {
                visivel: true,
                ..Default::default()
            },
        },
        totais: DadosTotais {
            valor_operacao_servico: "100,00".into(),
            valor_liquido: "100,00".into(),
            ..Default::default()
        },
        informacoes_complementares: DadosInformacoesComplementares {
            linha_tributos: "Totais aproximados dos Tributos: -".into(),
            ..Default::default()
        },
        canhoto: DadosCanhoto { visivel: false },
        ..Default::default()
    }
}

#[test]
fn conversao_cm_pt() {
    assert!((CM_PT - 28.34645669).abs() < 1e-6);
    assert!((cm(1.0) - CM_PT).abs() < 1e-6);
    assert_eq!(cm(0.0), 0.0);
    assert!((cm(2.54) - 72.0).abs() < 1e-6);
}

#[test]
fn top_to_pdf_y_inverte_eixo() {
    assert_eq!(top_to_pdf_y(0.0), PAGE_HEIGHT_PT);
    assert!((top_to_pdf_y(1.0) - (PAGE_HEIGHT_PT - CM_PT)).abs() < 1e-6);
    assert_eq!(PAGE_HEIGHT_PT, 841.89);
}

#[test]
fn grade_e_colunas() {
    assert!(body_left() < grid_x2() && grid_x2() < grid_x3() && grid_x3() < grid_x4());
    assert!(grid_x4() < body_right());
    assert!((COL1_CM - 5.11).abs() < 1e-6);
    assert!((COL2_CM - 5.10).abs() < 1e-6);
    assert!((COL3_CM - 5.11).abs() < 1e-6);
    assert!((coluna_x(0) - body_left()).abs() < EPS);
    assert!((coluna_x(3) - grid_x4()).abs() < EPS);
    assert!((largura_colunas(0, 1) - cm(COL1_CM)).abs() < EPS);
    assert!((largura_colunas(2, 2) - cm(COL_X_END_CM - COL_X3_CM)).abs() < EPS);
    assert_eq!(coluna_x(4), 0.0);
    assert_eq!(largura_colunas(3, 2), 0.0);
    assert_eq!(largura_colunas(0, 0), 0.0);
}

#[test]
fn geometria_qr_e_identificacao() {
    const { assert!(QR_TEXT_TOP_CM >= QR_TOP_CM + QR_SIZE_CM) };
    assert!((QR_TEXT_LEFT_CM - 15.98).abs() < EPS);
    assert_eq!(IDENTIFICATION_TEXT_INSET_CM, FIELD_TEXT_INSET_CM);
    assert_eq!(IDENTIFICATION_SHADE_HEIGHT_CM, 0.675);
}

#[test]
fn baseline_inclui_ajuste_da_helvetica() {
    let y = top_to_pdf_y(10.0);
    let esperado = y - REGULAR.ascent(7.0) - 7.0 * 0.19;
    assert!((baseline_from_top(10.0, 7.0, REGULAR) - esperado).abs() < EPS);
    assert!(baseline_from_top(10.0, 7.0, REGULAR) < y);
}

#[test]
fn passo_de_linha() {
    assert!((LINE_STEP_CM - (VALUE_SIZE_PT * 1.2) / CM_PT).abs() < 1e-6);
    const { assert!(COMPLEMENTARY_LINE_STEP_CM > 0.0) };
}

#[test]
fn estilos_de_celula() {
    assert_eq!(ESTILO_CAMPO.recuo_esquerdo_cm, FIELD_TEXT_INSET_CM);
    assert_eq!(ESTILO_CAMPO.recuo_direito_cm, FIELD_TEXT_INSET_CM);
    const { assert!(!ESTILO_IDENTIFICACAO.sombreado && ESTILO_IDENTIFICACAO_SOMBREADA.sombreado) };
    assert_eq!(ESTILO_IDENTIFICACAO.tamanho_rotulo_pt, TITLE_SIZE_PT);
    assert_eq!(ESTILO_IDENTIFICACAO.tamanho_valor_pt, VALUE_SIZE_PT);
    assert_eq!(ESTILO_ACESSO.tamanho_rotulo_pt, VALUE_SIZE_PT);
}

#[test]
fn alocacao_usa_espaco_disponivel() {
    let mut doc = DocumentoDanfse::default();
    doc.partes.tomador.visivel = true;
    doc.canhoto.visivel = true;
    let layout = montar_layout_pagina(&doc);
    let a = calcular_alocacao_texto(&layout, &doc, 16, 20);
    assert!(a.description_lines > 8, "{a:?}");
    assert!(a.complementary_lines >= 1);
}

#[test]
fn canhoto_oculto_libera_linhas_complementares() {
    let visivel = contar_linhas_informacoes_complementares(23.09, limite_inferior_bloco_cm(true));
    let oculto = contar_linhas_informacoes_complementares(23.09, limite_inferior_bloco_cm(false));
    assert!(oculto > visivel && visivel >= 1);
    assert_eq!(contar_linhas_informacoes_complementares(30.0, 28.0), 1);
}

#[test]
fn layout_suprime_blocos_opcionais() {
    let mut doc = DocumentoDanfse::default();
    doc.tributacoes.municipal.visivel = true;
    let l = montar_layout_pagina(&doc);
    for diff in [
        l.altura_destinatario - l.altura_tomador,
        l.altura_intermediario - l.altura_destinatario,
        l.service_top - l.altura_intermediario,
    ] {
        assert!((diff - SUPPRESSED_PARTY_BLOCK_HEIGHT_CM).abs() < EPS);
    }
    assert!((l.federal_top - l.municipal_top - 2.59).abs() < EPS);

    doc.tributacoes.municipal.visivel = false;
    let l = montar_layout_pagina(&doc);
    assert!((l.federal_top - l.municipal_top - SUPPRESSED_MUNICIPAL_BLOCK_HEIGHT_CM).abs() < EPS);
}

#[test]
fn truncate_text_semantica() {
    assert_eq!(truncate_text("Hello", REGULAR, 7.0, 200.0), "Hello");
    assert_eq!(truncate_text("", REGULAR, 7.0, 100.0), "");
    let longo = "This is a very long text that should be truncated with ellipsis";
    let t = truncate_text(longo, REGULAR, 7.0, 50.0);
    assert!(t.ends_with("...") && t.len() < longo.len());
    assert!(REGULAR.measure_string(&t, 7.0) <= 50.0);
    assert_eq!(truncate_text(longo, REGULAR, 7.0, 5.0), "");
}

#[test]
fn fit_so_trunca_quando_necessario() {
    assert!(matches!(
        fit("abc", REGULAR, 7.0, 100.0),
        Cow::Borrowed("abc")
    ));
    assert!(fit(&"x".repeat(200), REGULAR, 7.0, 100.0).ends_with("..."));
}

#[test]
fn quebra_de_linhas_por_palavra() {
    let texto = "This is a test of word boundary splitting";
    let linhas = medir_linhas(texto, 80.0, 7.0);
    assert!(linhas > 1);
    let (primeira, consumido) = next_text_line(texto, REGULAR, 7.0, 80.0);
    assert!(texto.starts_with(primeira) && !primeira.ends_with(' '));
    assert!(texto[consumido..].starts_with(' '));
    assert_eq!(medir_linhas("", 80.0, 7.0), 0);
}

#[test]
fn truncate_lines_limita_e_marca() {
    let texto = "Line 1\nLine 2\nLine 3\nLine 4\nLine 5\nLine 6";
    let t = truncate_lines(texto, 30.0, 7.0, 3);
    assert!(t.ends_with("...") && t.matches('\n').count() <= 2, "{t:?}");
    assert_eq!(truncate_lines("curto", 300.0, 7.0, 3), "curto");
}

#[test]
fn fallback_traco_casos() {
    for (entrada, esperado) in [
        ("", "-"),
        ("   ", "-"),
        ("\t\n", "-"),
        ("-", "-"),
        ("- / -", "- / -"),
        ("SP", "SP"),
        ("  SP  ", "  SP  "),
        ("100,00", "100,00"),
    ] {
        assert_eq!(fallback_traco(entrada), esperado, "{entrada:?}");
    }
}

const LINHA_TRIBUTOS: &str = "Totais aproximados dos Tributos cfe. Lei nº 12.741/2012: Federais: R$ 100,00; Estaduais: R$ 50,00; Municipais: R$ 25,00;";

#[test]
fn combinar_conteudo_complementar_casos() {
    assert_eq!(
        combinar_conteudo_complementar("", LINHA_TRIBUTOS),
        LINHA_TRIBUTOS
    );
    let ja = format!("Informacao complementar | {LINHA_TRIBUTOS}");
    assert_eq!(combinar_conteudo_complementar(&ja, LINHA_TRIBUTOS), ja);
    assert_eq!(
        combinar_conteudo_complementar("Informacao complementar", LINHA_TRIBUTOS),
        format!("Informacao complementar | {LINHA_TRIBUTOS}")
    );
    assert_eq!(combinar_conteudo_complementar("x", ""), "x");
}

#[test]
fn ajuste_complementar_preserva_linha_tributos() {
    let texto = format!("{} | {LINHA_TRIBUTOS}", "palavra ".repeat(400));
    let largura = largura_texto_corrido();
    let ajustado = ajustar_informacoes_complementares(&texto, LINHA_TRIBUTOS, largura, 6.5, 3);
    assert!(
        ajustado.ends_with(&format!("... | {LINHA_TRIBUTOS}")),
        "{ajustado}"
    );
    assert!(medir_linhas(&ajustado, largura, 6.5) <= 3);

    let curto = ajustar_informacoes_complementares("curto", LINHA_TRIBUTOS, largura, 6.5, 3);
    assert_eq!(curto, "curto");

    let sem_sufixo = ajustar_informacoes_complementares(
        &"palavra ".repeat(400),
        LINHA_TRIBUTOS,
        largura,
        6.5,
        2,
    );
    assert!(sem_sufixo.ends_with("..."));

    let so_tributos = ajustar_informacoes_complementares(&texto, LINHA_TRIBUTOS, 40.0, 6.5, 1);
    assert_eq!(so_tributos, LINHA_TRIBUTOS);
}

#[test]
fn produz_pdf_a4_de_uma_pagina_com_marca_dagua() {
    let mut nota = NFSe {
        marca_dagua: "CANCELADA".into(),
        exibicao_desc_serv: "Serviço de teste".into(),
        exibicao_linha_tributos: "Totais aproximados dos Tributos: -".into(),
        ..Default::default()
    };
    nota.inf_nfse.id = "NFS123456".into();
    nota.inf_nfse.n_nfse = "1".into();
    let doc = ServicoApresentacaoDanfse::new().apresentar(&nota);
    let pdf = renderizar(&doc);
    let texto = String::from_utf8_lossy(&pdf);
    assert!(texto.starts_with("%PDF-1.7"));
    assert!(texto.contains("/Count 1 "));
    assert!(texto.contains("/MediaBox [0 0 595.28 841.89]"));
    assert!(texto.contains("/ExtGState"));
    assert!(contem(&pdf, "(CANCELADA) Tj"));
}

#[test]
fn ibs_cbs_vazio_usa_traco() {
    let pdf = renderizar(&doc_base());
    assert!(contem(&pdf, "(TRIBUTAÇÃO IBS / CBS) Tj"));
    assert!(conteudo(&pdf).windows(6).filter(|w| w == b"(-) Tj").count() >= 2);
}

#[test]
fn pis_cofins_ocultos_quando_desabilitados() {
    let mut doc = doc_base();
    doc.tributacoes.federal = DadosTributacaoFederal {
        visivel: true,
        exibir_pis_cofins: false,
        irrf: "R$ 50,00".into(),
        pis_debito: "R$ 0,00".into(),
        cofins_debito: "R$ 0,00".into(),
        desc_contrib_ret: "Retido na Fonte".into(),
        ..Default::default()
    };
    let pdf = renderizar(&doc);
    assert!(contem(&pdf, "(R$ 50,00) Tj"));
    assert!(!contem(&pdf, "PIS - Débito"));
    assert!(!contem(&pdf, "COFINS - Débito"));

    doc.tributacoes.federal.exibir_pis_cofins = true;
    let pdf = renderizar(&doc);
    assert!(contem(&pdf, "PIS - Débito"));
    assert!(contem(&pdf, "(Retido na Fonte) Tj"));
}

#[test]
fn homologacao_exibe_aviso_e_producao_nao() {
    let mut doc = doc_base();
    assert!(contem(&renderizar(&doc), "NFS-e SEM VALIDADE JURÍDICA"));
    doc.cabecalho.tipo_ambiente = "Produção".into();
    assert!(!contem(&renderizar(&doc), "NFS-e SEM VALIDADE JURÍDICA"));
}

#[test]
fn canhoto_so_quando_visivel() {
    let mut doc = doc_base();
    assert!(!contem(&renderizar(&doc), "DATA CIENTIFICAÇÃO:"));
    doc.canhoto.visivel = true;
    let pdf = renderizar(&doc);
    assert!(contem(&pdf, "DATA CIENTIFICAÇÃO:"));
    assert!(contem(
        &pdf,
        "(1 / 35260800000000000000000000000000001234567890) Tj"
    ));
}

#[test]
fn partes_suprimidas_mostram_mensagem() {
    let mut doc = doc_base();
    doc.partes.destinatario.mensagem_supressao = "O DESTINATÁRIO É O PRÓPRIO TOMADOR".into();
    let pdf = renderizar(&doc);
    assert!(contem(&pdf, "O DESTINATÁRIO É O PRÓPRIO TOMADOR"));
    assert!(!contem(&pdf, "DESTINATÁRIO DA OPERAÇÃO"));
}

#[test]
fn renderizacao_deterministica() {
    let doc = doc_base();
    assert_eq!(renderizar(&doc), renderizar(&doc));
}
