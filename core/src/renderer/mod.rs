//! renderização direta do `DocumentoDanfse` em PDF, sem HTML intermediário.
//! Coordenadas e regras de texto espelham o renderer Go para gerar o mesmo
//! fluxo de conteúdo

mod labels;
mod layout;
mod style;
mod text;

#[cfg(test)]
mod tests;

use std::borrow::Cow;
use std::sync::Arc;

use crate::domain::presentation::{DocumentoDanfse, SecaoParte};
use crate::pdf::{Image, Page, QrCode, StandardFont, Watermark, write_pdf};
use crate::service::{ErroDanfse, RenderizadorPdfDireto};

use labels as l;
use layout::*;
use style::*;
use text::*;

const REGULAR: StandardFont = StandardFont::Helvetica;
const BOLD: StandardFont = StandardFont::HelveticaBold;
const LOGO_PNG: &[u8] = include_bytes!("../../assets/logo-nfse.png");

const LINE_SPACING: f64 = 1.2;
const LINE_WIDTH_PT: f64 = 0.5;
const FRAME_WIDTH_PT: f64 = 1.0;
const COMPLEMENTARY_SIZE_PT: f64 = 6.5;
const SUPPRESSED_SIZE_PT: f64 = 7.5;
// folga horizontal (pt) entre o texto multilinha e as bordas da página
const MULTILINE_MARGIN_PT: f64 = 3.0;
const MULTILINE_VALUE_OFFSET_CM: f64 = 0.30;
const CANHOTO_TEXT_INSET_PT: f64 = 2.0;
const CANHOTO_LABEL_TOP_CM: f64 = 28.17;
const CANHOTO_VALUE_TOP_CM: f64 = 28.47;
const CANHOTO_VALUE_MAX_LINES: usize = 2;
const HEADER_TITLE_LEFT_CM: f64 = 5.41;
const HEADER_TITLE_WIDTH_CM: f64 = 10.19;
const AMBIENTE_HOMOLOGACAO: &str = "Homologação";

/// renderizador sem estado mutável compartilhado; o logo é decodificado uma vez
#[derive(Debug, Clone)]
pub struct Renderer {
    logo: Arc<Image>,
}

impl Renderer {
    pub fn new() -> Result<Self, ErroDanfse> {
        let logo = Image::from_png(LOGO_PNG)
            .map_err(|e| ErroDanfse::Renderizacao(format!("parse logo: {e}")))?;
        Ok(Self {
            logo: Arc::new(logo),
        })
    }
}

impl RenderizadorPdfDireto for Renderer {
    fn renderizar(&self, doc: &DocumentoDanfse) -> Result<Vec<u8>, ErroDanfse> {
        let mut canvas = Canvas {
            page: Page::new(),
            logo: &self.logo,
        };
        canvas.desenhar_pagina(doc);
        let marca = &doc.marca_dagua;
        let watermark = (!marca.texto.is_empty()).then(|| {
            Watermark::with_defaults(
                &marca.texto,
                marca.tamanho_fonte,
                [marca.color_r, marca.color_g, marca.color_b],
                marca.angulo,
                marca.opacidade,
            )
        });
        write_pdf(canvas.page, watermark.as_ref())
            .map_err(|e| ErroDanfse::Renderizacao(format!("serialização: {e}")))
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum VarianteParte {
    Prestador,
    Tomador,
    Destinatario,
    Intermediario,
}

struct Canvas<'a> {
    page: Page,
    logo: &'a Arc<Image>,
}

fn descricao_servico(doc: &DocumentoDanfse) -> &str {
    if doc.servico.desc_serv.is_empty() {
        &doc.servico.desc_serv_original
    } else {
        &doc.servico.desc_serv
    }
}

/// prioriza `conteudo` (já limitado na preparação); `conteudo_original` só
/// como fallback para documentos montados à mão
fn selecionar_conteudo_complementar(doc: &DocumentoDanfse) -> String {
    let info = &doc.informacoes_complementares;
    let src = if info.conteudo.is_empty() {
        &info.conteudo_original
    } else {
        &info.conteudo
    };
    combinar_conteudo_complementar(src, &info.linha_tributos)
}

fn largura_texto_corrido() -> f64 {
    body_right() - body_left() - MULTILINE_MARGIN_PT
}

impl Canvas<'_> {
    fn desenhar_pagina(&mut self, doc: &DocumentoDanfse) {
        let layout = montar_layout_pagina(doc);
        let desc_lines = medir_linhas(
            descricao_servico(doc),
            largura_texto_corrido(),
            VALUE_SIZE_PT,
        );
        let content = selecionar_conteudo_complementar(doc);
        let content_lines = medir_linhas(&content, largura_texto_corrido(), COMPLEMENTARY_SIZE_PT);
        let alocacao = calcular_alocacao_texto(&layout, doc, desc_lines, content_lines);

        self.desenhar_cabecalho(doc);
        self.desenhar_info_nfse(doc);
        self.desenhar_partes(doc, &layout);
        self.desenhar_servico(doc, alocacao.description_lines, &layout);
        self.desenhar_tributacao_municipal(doc, &layout, alocacao.shift_cm);
        self.desenhar_tributacao_federal(doc, &layout, alocacao.shift_cm);
        self.desenhar_tributacao_ibs_cbs(doc, &layout, alocacao.shift_cm);
        self.desenhar_totais(doc, &layout, alocacao.shift_cm);
        self.desenhar_informacoes_complementares(doc, &layout, alocacao);
        self.desenhar_canhoto(doc);
        self.draw_frame();
    }

    fn line(&mut self, x1: f64, y1: f64, x2: f64, y2: f64) {
        self.page
            .add_line((x1, y1), (x2, y2), LINE_WIDTH_PT, BLACK_RGB);
    }

    fn line_h(&mut self, top_cm: f64) {
        let y = top_to_pdf_y(top_cm);
        self.line(body_left(), y, body_right(), y);
    }

    fn line_v(&mut self, x_cm: f64, top_cm: f64, bottom_cm: f64) {
        self.line(
            x_cm * CM_PT,
            top_to_pdf_y(top_cm),
            x_cm * CM_PT,
            top_to_pdf_y(bottom_cm),
        );
    }

    fn draw_frame(&mut self) {
        let rect = [
            page_left(),
            PAGE_HEIGHT_PT - page_bottom(),
            page_right() - page_left(),
            page_bottom() - page_top(),
        ];
        self.page.add_rect(rect, FRAME_WIDTH_PT, BLACK_RGB);
    }

    fn colored_text(
        &mut self,
        value: &str,
        x: f64,
        y: f64,
        size: f64,
        font: StandardFont,
        cor: [f64; 3],
    ) {
        self.page.stream().set_fill_rgb(cor);
        self.page.add_text(value, font, size, x, y);
        self.page.stream().set_fill_rgb(BLACK_RGB);
    }

    fn text_top_color(
        &mut self,
        value: &str,
        x: f64,
        top_cm: f64,
        size: f64,
        font: StandardFont,
        cor: [f64; 3],
    ) {
        if value.trim().is_empty() {
            return;
        }
        let y = baseline_from_top(top_cm, size, font);
        self.colored_text(value, x, y, size, font, cor);
    }

    fn text_top(&mut self, value: &str, x: f64, top_cm: f64, size: f64, font: StandardFont) {
        self.text_top_color(value, x, top_cm, size, font, BLACK_RGB);
    }

    #[allow(clippy::too_many_arguments)]
    fn center_text(
        &mut self,
        value: &str,
        x: f64,
        top_cm: f64,
        width: f64,
        size: f64,
        font: StandardFont,
        cor: [f64; 3],
    ) {
        let w = font.measure_string(value, size);
        let x_center = x + (width - w) / 2.0;
        let y = baseline_from_top(top_cm, size, font);
        self.colored_text(value, x_center, y, size, font, cor);
    }

    #[allow(clippy::too_many_arguments)]
    fn wrap_text(
        &mut self,
        value: &str,
        x: f64,
        top_cm: f64,
        width: f64,
        size: f64,
        font: StandardFont,
        max_lines: usize,
    ) {
        let line_height = size * LINE_SPACING;
        let mut remaining = value.trim();
        for line in 0..max_lines {
            if remaining.is_empty() {
                break;
            }
            let (part, consumed) = next_text_line(remaining, font, size, width);
            if part.is_empty() || consumed == 0 {
                break;
            }
            let part = if line == max_lines - 1 && consumed < remaining.len() {
                Cow::Owned(truncate_text(
                    &format!("{part}{}", layout::ELLIPSIS),
                    font,
                    size,
                    width,
                ))
            } else {
                Cow::Borrowed(part)
            };
            self.text_top(
                &part,
                x,
                top_cm + line_height * line as f64 / CM_PT,
                size,
                font,
            );
            remaining = remaining[consumed..].trim();
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn desenhar_celula(
        &mut self,
        x: f64,
        top_cm: f64,
        width_cm: f64,
        rotulo: &str,
        valor: &str,
        estilo: EstiloCelula,
    ) {
        let recuo_esquerdo = cm(estilo.recuo_esquerdo_cm);
        let recuo_direito = cm(estilo.recuo_direito_cm);
        let largura = width_cm * CM_PT - recuo_esquerdo - recuo_direito;
        if estilo.sombreado {
            let rect = [
                x,
                top_to_pdf_y(top_cm + estilo.sombreado_top_cm + estilo.sombreado_altura_cm),
                width_cm * CM_PT,
                estilo.sombreado_altura_cm * CM_PT,
            ];
            self.page.add_rect_filled(rect, GRAY_HEADER_RGB);
        }
        if !rotulo.is_empty() {
            let r = fit(rotulo, BOLD, estilo.tamanho_rotulo_pt, largura);
            self.text_top(
                &r,
                x + recuo_esquerdo,
                top_cm + estilo.offset_rotulo_cm,
                estilo.tamanho_rotulo_pt,
                BOLD,
            );
        }
        let v = fit(
            fallback_traco(valor),
            REGULAR,
            estilo.tamanho_valor_pt,
            largura,
        );
        self.text_top(
            &v,
            x + recuo_esquerdo,
            top_cm + estilo.offset_valor_cm,
            estilo.tamanho_valor_pt,
            REGULAR,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn desenhar_campo(
        &mut self,
        coluna: usize,
        colunas: usize,
        top_cm: f64,
        rotulo: &str,
        valor: &str,
        sombreado: bool,
    ) {
        let estilo = if sombreado {
            ESTILO_CAMPO_SOMBREADO
        } else {
            ESTILO_CAMPO
        };
        let largura_cm = largura_colunas(coluna, colunas) / CM_PT;
        self.desenhar_celula(coluna_x(coluna), top_cm, largura_cm, rotulo, valor, estilo);
    }

    #[allow(clippy::too_many_arguments)]
    fn desenhar_campo_identificacao(
        &mut self,
        x: f64,
        top_cm: f64,
        width_cm: f64,
        rotulo: &str,
        valor: &str,
        sombreado: bool,
    ) {
        let estilo = if sombreado {
            ESTILO_IDENTIFICACAO_SOMBREADA
        } else {
            ESTILO_IDENTIFICACAO
        };
        self.desenhar_celula(x, top_cm, width_cm, rotulo, valor, estilo);
    }

    #[allow(clippy::too_many_arguments)]
    fn desenhar_multiplas_linhas(
        &mut self,
        top_cm: f64,
        height: f64,
        rotulo: &str,
        valor: &str,
        size: f64,
    ) {
        let x = body_left();
        let width = body_right() - body_left();
        let recuo = cm(FIELD_TEXT_INSET_CM);
        if !rotulo.is_empty() {
            self.text_top(rotulo, x + recuo, top_cm, LABEL_SIZE_PT, BOLD);
        }
        // cast seguro: altura de bloco em pontos, poucas dezenas de linhas
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let max_lines = (height / (size * LINE_SPACING)).floor().max(1.0) as usize;
        self.wrap_text(
            valor,
            x + recuo,
            top_cm + MULTILINE_VALUE_OFFSET_CM,
            width - 2.0 * recuo,
            size,
            REGULAR,
            max_lines,
        );
    }

    fn desenhar_cabecalho(&mut self, doc: &DocumentoDanfse) {
        let cab = &doc.cabecalho;
        self.page.add_rect_filled(
            [body_left(), top_to_pdf_y(0.30 + 1.16), cm(20.40), cm(1.16)],
            GRAY_HEADER_RGB,
        );
        self.page.add_image(
            self.logo,
            [cm(0.49), top_to_pdf_y(0.44 + 0.85), cm(4.00), cm(0.85)],
        );
        let (titulo_x, titulo_w) = (HEADER_TITLE_LEFT_CM * CM_PT, HEADER_TITLE_WIDTH_CM * CM_PT);
        self.center_text(
            l::DANFSE_VERSAO,
            titulo_x,
            0.35,
            titulo_w,
            TITLE_SIZE_PT + 2.0,
            BOLD,
            BLACK_RGB,
        );
        self.center_text(
            l::DOCUMENTO_AUXILIAR,
            titulo_x,
            0.65,
            titulo_w,
            TITLE_SIZE_PT + 2.0,
            BOLD,
            BLACK_RGB,
        );
        if cab.tipo_ambiente == AMBIENTE_HOMOLOGACAO {
            self.center_text(
                l::NFSE_SEM_VALIDADE_JURIDICA,
                titulo_x,
                0.95,
                titulo_w,
                9.0,
                BOLD,
                RED_RGB,
            );
        }
        self.text_top(
            &format!("Município: {} - {}", cab.municipio, cab.uf),
            grid_x4(),
            0.45,
            8.0,
            REGULAR,
        );
        self.text_top(
            &format!("Ambiente Gerador: {}", cab.ambiente_gerador),
            grid_x4(),
            0.97,
            6.0,
            REGULAR,
        );
        self.text_top(
            &format!("Tipo de Ambiente: {}", cab.tipo_ambiente),
            grid_x4(),
            1.22,
            6.0,
            REGULAR,
        );
        self.line_h(1.48);
    }

    fn desenhar_info_nfse(&mut self, doc: &DocumentoDanfse) {
        let a = &doc.acesso;
        self.desenhar_celula(
            body_left(),
            ACCESS_KEY_TOP_CM,
            ACCESS_KEY_WIDTH_CM,
            l::CHAVE_ACESSO_NFSE,
            &a.chave_acesso,
            ESTILO_ACESSO,
        );
        self.desenhar_qr_code(&a.codigo_qr);
        for (i, linha) in [l::QR_LINE1, l::QR_LINE2, l::QR_LINE3]
            .into_iter()
            .enumerate()
        {
            self.text_top(
                linha,
                cm(QR_TEXT_LEFT_CM),
                QR_TEXT_TOP_CM + i as f64 * QR_TEXT_STEP_CM,
                6.0,
                REGULAR,
            );
        }
        let linhas = [
            (
                NFSE_FIELDS_TOP_CM,
                [
                    (l::NUMERO_NFSE, &a.numero_nfse),
                    (l::COMPETENCIA_NFSE, &a.competencia),
                    (l::DATA_HORA_EMISSAO_NFSE, &a.data_hora_emissao_nfse),
                ],
            ),
            (
                DPS_FIELDS_TOP_CM,
                [
                    (l::NUMERO_DPS, &a.numero_dps),
                    (l::SERIE_DPS, &a.serie_dps),
                    (l::DATA_HORA_EMISSAO_DPS, &a.data_hora_emissao_dps),
                ],
            ),
        ];
        for (top, celulas) in linhas {
            for (coluna, (rotulo, valor)) in celulas.into_iter().enumerate() {
                let largura_cm = largura_colunas(coluna, 1) / CM_PT;
                self.desenhar_celula(
                    coluna_x(coluna),
                    top,
                    largura_cm,
                    rotulo,
                    valor,
                    ESTILO_ACESSO,
                );
            }
        }
    }

    fn desenhar_qr_code(&mut self, codigo: &str) {
        // QR inválido (vazio/longo demais) apenas não é desenhado, como no Go
        if let Ok(qr) = QrCode::new(codigo) {
            let lado = cm(QR_SIZE_CM);
            qr.draw(
                self.page.stream(),
                cm(QR_LEFT_CM),
                top_to_pdf_y(QR_TOP_CM + QR_SIZE_CM),
                lado,
                lado,
            );
        }
    }

    fn desenhar_partes(&mut self, doc: &DocumentoDanfse, layout: &PageLayout) {
        let a = &doc.acesso;
        self.desenhar_campo_identificacao(
            body_left(),
            3.65,
            COL1_CM,
            l::EMITENTE_NFSE,
            &a.tipo_emitente,
            true,
        );
        self.desenhar_campo_identificacao(
            grid_x2(),
            3.65,
            COL2_CM,
            l::SITUACAO_NFSE,
            &a.situacao,
            false,
        );
        self.desenhar_campo_identificacao(
            grid_x3(),
            3.65,
            COL3_CM,
            l::FINALIDADE,
            &a.finalidade,
            false,
        );

        let p = &doc.partes;
        self.desenhar_secao_parte(
            4.34,
            l::PRESTADOR_FORNECEDOR,
            &p.emitente,
            VarianteParte::Prestador,
        );
        let opcionais = [
            (
                layout.altura_tomador,
                l::TOMADOR_ADQUIRENTE,
                &p.tomador,
                VarianteParte::Tomador,
            ),
            (
                layout.altura_destinatario,
                l::DESTINATARIO_OPERACAO,
                &p.destinatario,
                VarianteParte::Destinatario,
            ),
            (
                layout.altura_intermediario,
                l::INTERMEDIARIO_OPERACAO,
                &p.intermediario,
                VarianteParte::Intermediario,
            ),
        ];
        for (top, titulo, parte, variante) in opcionais {
            if parte.visivel {
                self.desenhar_secao_parte(top, titulo, parte, variante);
            } else {
                self.desenhar_bloco_suprimido(
                    top,
                    &parte.mensagem_supressao,
                    SUPPRESSED_PARTY_BLOCK_HEIGHT_CM,
                );
            }
        }
    }

    fn desenhar_servico(&mut self, doc: &DocumentoDanfse, desc_lines: usize, layout: &PageLayout) {
        let y = layout.service_top;
        let s = &doc.servico;
        self.desenhar_secao(y, l::SERVICO_PRESTADO, NORMAL_SECTION_HEIGHT_CM);
        self.desenhar_campo(
            1,
            1,
            y,
            l::CODIGO_TRIB_NAC_MUN,
            &s.codigo_trib_nac_mun,
            false,
        );
        self.desenhar_campo(2, 1, y, l::CODIGO_NBS, &s.codigo_nbs, false);
        self.desenhar_campo(3, 1, y, l::LOCAL_PRESTACAO, &s.local_prestacao_uf, false);
        let largura = body_right() - body_left() - cm(COL_GUTTER_CM);
        let x_trib = truncate_text(&s.x_trib_nac, REGULAR, VALUE_SIZE_PT, largura);
        self.text_top(
            &x_trib,
            body_left() + cm(FIELD_TEXT_INSET_CM),
            y + SERVICE_TAX_DESCRIPTION_OFFSET_CM,
            VALUE_SIZE_PT,
            REGULAR,
        );
        let altura = desc_lines as f64 * LINE_STEP_CM * CM_PT;
        self.desenhar_multiplas_linhas(
            y + SERVICE_DESCRIPTION_OFFSET_CM,
            altura,
            l::DESCRICAO_SERVICO,
            descricao_servico(doc),
            VALUE_SIZE_PT,
        );
    }

    fn desenhar_tributacao_municipal(
        &mut self,
        doc: &DocumentoDanfse,
        layout: &PageLayout,
        shift: f64,
    ) {
        let m = &doc.tributacoes.municipal;
        let top = layout.municipal_top + shift;
        if !m.visivel {
            self.desenhar_bloco_suprimido(
                top,
                &m.mensagem_supressao,
                SUPPRESSED_MUNICIPAL_BLOCK_HEIGHT_CM,
            );
            return;
        }
        self.desenhar_secao(top, l::TRIB_MUNICIPAL, NORMAL_SECTION_HEIGHT_CM);
        self.desenhar_campo(1, 1, top, l::TIPO_TRIB_ISSQN, &m.trib_issqn, false);
        self.desenhar_campo(
            2,
            2,
            top,
            l::MUNICIPIO_UF_PAIS_ISSQN,
            &m.municipio_uf_pais_incid,
            false,
        );
        let linhas = [
            [
                (l::REG_ESP_TRIB_ISSQN, &m.reg_esp_trib),
                (l::TP_IMUNIDADE_ISSQN, &m.tp_imunidade),
                (l::SUSPENSAO_EXIG_ISSQN, &m.suspensao_exig),
                (l::NUMERO_PROCESSO_SUSP, &m.n_processo_susp),
            ],
            [
                (l::BENEFICIO_MUNICIPAL, &m.beneficio_municipal),
                (l::CALCULO_BM, &m.calculo_bm),
                (l::TOTAL_DEDUCOES, &m.total_deducoes),
                (l::DESCONTO_INCONDICIONADO, &m.desconto_incond),
            ],
            [
                (l::BC_ISSQN, &m.bc_issqn),
                (l::ALIQ_APLICADA, &m.aliq_aplic),
                (l::RETENCAO_ISSQN, &m.ret_issqn),
                (l::ISSQN_APURADO, &m.issqn_apurado),
            ],
        ];
        for (i, linha) in linhas.into_iter().enumerate() {
            // mesma ordem de soma do Go: topo + passo*n + shift
            let y = layout.municipal_top + (i + 1) as f64 * MUNICIPAL_ROW_STEP_CM + shift;
            self.desenhar_linha_campos(y, linha);
        }
    }

    fn desenhar_linha_campos(&mut self, top: f64, campos: [(&str, &String); 4]) {
        for (coluna, (rotulo, valor)) in campos.into_iter().enumerate() {
            self.desenhar_campo(coluna, 1, top, rotulo, valor, false);
        }
    }

    fn desenhar_tributacao_federal(
        &mut self,
        doc: &DocumentoDanfse,
        layout: &PageLayout,
        shift: f64,
    ) {
        let f = &doc.tributacoes.federal;
        let top = layout.federal_top + shift;
        self.desenhar_secao(top, l::TRIB_FEDERAL, NORMAL_SECTION_HEIGHT_CM);
        self.desenhar_campo(1, 1, top, l::IRRF, &f.irrf, false);
        self.desenhar_campo(
            2,
            1,
            top,
            l::CONTRIB_PREVID_RET,
            &f.contrib_previd_ret,
            false,
        );
        self.desenhar_campo(3, 1, top, l::CONTRIB_SOC_RET, &f.contrib_soc_ret, false);
        if f.exibir_pis_cofins {
            let y = layout.federal_top + FEDERAL_ROW_STEP_CM + shift;
            self.desenhar_campo(0, 1, y, l::PIS_DEBITO, &f.pis_debito, false);
            self.desenhar_campo(1, 1, y, l::COFINS_DEBITO, &f.cofins_debito, false);
            self.desenhar_campo(
                2,
                2,
                y,
                l::DESCRICAO_CONTRIB_RET,
                &f.desc_contrib_ret,
                false,
            );
        }
    }

    fn desenhar_tributacao_ibs_cbs(
        &mut self,
        doc: &DocumentoDanfse,
        layout: &PageLayout,
        shift: f64,
    ) {
        let top = layout.ibs_top + shift;
        self.desenhar_secao(top, l::TRIB_IBS_CBS, NORMAL_SECTION_HEIGHT_CM);
        let t = &doc.tributacoes.ibs_cbs;
        self.desenhar_campo(1, 1, top, l::CST_CCLASS_TRIB, &t.cst_c_class_trib, false);
        self.desenhar_campo(
            2,
            2,
            top,
            l::IND_OP_COD_IBGE_LOCALIDADE_UF,
            &t.ind_op_cod_ibge_localidade_uf,
            false,
        );
        let linhas = [
            (
                IBS_ROW_STEP_CM,
                [
                    (l::EXCLUSOES_REDUCOES_BC, &t.exclusoes_reducoes_bc),
                    (l::BC_POS_EXCLUSOES, &t.bc_pos_exclusoes),
                    (l::RED_ALIQ_IBS_RED_ALIQ_CBS, &t.red_aliq_ibs_red_aliq_cbs),
                    (l::ALIQ_IBS_UF_IBS_MUN, &t.aliq_ibs_uf_ibs_mun),
                ],
            ),
            (
                IBS_SECOND_ROW_OFFSET_CM,
                [
                    (l::ALIQ_EFET_MUN_IBS, &t.aliq_efet_mun_ibs),
                    (l::VALOR_MUN_IBS, &t.valor_mun_ibs),
                    (l::ALIQ_EFET_EST_IBS, &t.aliq_efet_est_ibs),
                    (l::VALOR_EST_IBS, &t.valor_est_ibs),
                ],
            ),
            (
                IBS_THIRD_ROW_OFFSET_CM,
                [
                    (l::VALOR_TOTAL_IBS, &t.valor_total_ibs),
                    (l::ALIQ_CBS, &t.aliq_cbs),
                    (l::ALIQ_EFET_CBS, &t.aliq_efet_cbs),
                    (l::VALOR_TOTAL_CBS, &t.valor_total_cbs),
                ],
            ),
        ];
        for (offset, campos) in linhas {
            self.desenhar_linha_campos(layout.ibs_top + offset + shift, campos);
        }
    }

    fn desenhar_totais(&mut self, doc: &DocumentoDanfse, layout: &PageLayout, shift: f64) {
        let top = layout.totals_top + shift;
        let t = &doc.totais;
        self.desenhar_secao(top, l::VALOR_TOTAL_NFSE, TOTAL_SECTION_HEIGHT_CM);
        self.desenhar_campo(
            1,
            1,
            top,
            l::VALOR_OPERACAO_SERVICO,
            &t.valor_operacao_servico,
            false,
        );
        self.desenhar_campo(
            2,
            1,
            top,
            l::DESCONTO_INCONDICIONADO,
            &t.desconto_incond,
            false,
        );
        self.desenhar_campo(3, 1, top, l::DESCONTO_CONDICIONADO, &t.desconto_cond, false);
        let y = top + TOTALS_ROW_STEP_CM;
        self.desenhar_campo(0, 1, y, l::TOTAL_RETENCOES, &t.total_retencoes, false);
        self.desenhar_campo(1, 1, y, l::VALOR_LIQUIDO_NFSE, &t.valor_liquido, false);
        self.desenhar_campo(2, 1, y, l::TOTAL_IBS_CBS, &t.total_ibs_cbs, false);
        self.desenhar_campo(
            3,
            1,
            y,
            l::VALOR_LIQUIDO_IBS_CBS,
            &t.valor_liquido_ibs_cbs,
            true,
        );
    }

    fn desenhar_informacoes_complementares(
        &mut self,
        doc: &DocumentoDanfse,
        layout: &PageLayout,
        alocacao: TextAllocation,
    ) {
        let shift = alocacao.shift_cm;
        let lines = alocacao.complementary_lines;
        self.desenhar_titulo_secao(
            layout.complementary_top + shift,
            l::INFORMACOES_COMPLEMENTARES,
            COMPLEMENTARY_SECTION_HEIGHT_CM,
            false,
        );
        let content_top = layout.complementary_top + COMPLEMENTARY_CONTENT_OFFSET_CM + shift;
        let content = selecionar_conteudo_complementar(doc);
        let linha_tributos = &doc.informacoes_complementares.linha_tributos;
        let content = ajustar_informacoes_complementares(
            &content,
            linha_tributos,
            largura_texto_corrido(),
            COMPLEMENTARY_SIZE_PT,
            lines,
        );
        let altura = lines as f64 * COMPLEMENTARY_LINE_STEP_CM * CM_PT;
        self.desenhar_multiplas_linhas(content_top, altura, "", &content, COMPLEMENTARY_SIZE_PT);
    }

    fn desenhar_canhoto(&mut self, doc: &DocumentoDanfse) {
        if !doc.canhoto.visivel {
            return;
        }
        self.line_h(Y_CANHOTO_CM);
        self.line_h(CANHOTO_BOTTOM_CM);
        for x in [GRID_LEFT_CM, COL_X2_CM, COL_X3_CM, COL_X_END_CM] {
            self.line_v(x, Y_CANHOTO_CM, CANHOTO_BOTTOM_CM);
        }
        let inset = CANHOTO_TEXT_INSET_PT;
        self.text_top(
            l::DATA_CIENTIFICACAO,
            body_left() + inset,
            CANHOTO_LABEL_TOP_CM,
            6.0,
            BOLD,
        );
        self.text_top(
            l::IDENTIFICACAO_ASSINATURA,
            grid_x2() + inset,
            CANHOTO_LABEL_TOP_CM,
            6.0,
            BOLD,
        );
        self.text_top(
            l::NUMERO_NFSE_CHAVE,
            grid_x3() + inset,
            CANHOTO_LABEL_TOP_CM,
            6.0,
            BOLD,
        );
        let valor = format!("{} / {}", doc.acesso.numero_nfse, doc.acesso.chave_acesso);
        self.wrap_text(
            &valor,
            grid_x3() + inset,
            CANHOTO_VALUE_TOP_CM,
            body_right() - grid_x3(),
            COMPLEMENTARY_SIZE_PT,
            REGULAR,
            CANHOTO_VALUE_MAX_LINES,
        );
    }

    fn desenhar_secao(&mut self, top_cm: f64, titulo: &str, altura_cm: f64) {
        self.desenhar_titulo_secao(top_cm, titulo, altura_cm, true);
    }

    fn desenhar_titulo_secao(
        &mut self,
        top_cm: f64,
        titulo: &str,
        altura_cm: f64,
        sombreado: bool,
    ) {
        let y = top_to_pdf_y(top_cm);
        let h = altura_cm * CM_PT;
        if sombreado {
            self.page
                .add_rect_filled([body_left(), y - h, COL1_CM * CM_PT, h], GRAY_HEADER_RGB);
        }
        self.text_top(
            titulo,
            body_left() + cm(FIELD_TEXT_INSET_CM),
            top_cm + SECTION_TITLE_OFFSET_CM,
            TITLE_SIZE_PT,
            BOLD,
        );
        self.line(body_left(), y, body_right(), y);
    }

    fn desenhar_bloco_suprimido(&mut self, top_cm: f64, valor: &str, altura_cm: f64) {
        self.line_h(top_cm);
        self.line_h(top_cm + altura_cm);
        let width = body_right() - body_left() - 2.0;
        let texto = fit(valor, REGULAR, SUPPRESSED_SIZE_PT, width);
        let text_top = top_cm + (altura_cm - (SUPPRESSED_SIZE_PT / CM_PT)) / 2.0;
        self.center_text(
            &texto,
            body_left() + 1.0,
            text_top,
            width,
            SUPPRESSED_SIZE_PT,
            REGULAR,
            BLACK_RGB,
        );
    }

    fn desenhar_secao_parte(
        &mut self,
        top_cm: f64,
        titulo: &str,
        p: &SecaoParte,
        variante: VarianteParte,
    ) {
        self.desenhar_secao(top_cm, titulo, NORMAL_SECTION_HEIGHT_CM);
        let destinatario = variante == VarianteParte::Destinatario;
        self.desenhar_campo(
            1,
            if destinatario { 2 } else { 1 },
            top_cm,
            l::CNPJ_CPF_NIF,
            &p.cnpj_cpf_nif,
            false,
        );
        if !destinatario {
            self.desenhar_campo(
                2,
                1,
                top_cm,
                l::INDICADOR_MUNICIPAL,
                &p.inscricao_municipal,
                false,
            );
        }
        self.desenhar_campo(3, 1, top_cm, l::TELEFONE, &p.telefone, false);
        let y1 = top_cm + PARTY_ROW_STEP_CM;
        self.desenhar_campo(0, 2, y1, l::NOME_NOME_EMPRESARIAL, &p.nome, false);
        self.desenhar_campo(2, 1, y1, l::MUNICIPIO_UF, &p.municipio_uf, false);
        self.desenhar_campo(3, 1, y1, l::CODIGO_IBGE_CEP, &p.codigo_ibge_cep, false);
        let y2 = top_cm + 2.0 * PARTY_ROW_STEP_CM;
        self.desenhar_campo(0, 2, y2, l::ENDERECO, &p.endereco, false);
        self.desenhar_campo(2, 2, y2, l::EMAIL, &p.email, false);
        if variante == VarianteParte::Prestador {
            let y3 = top_cm + 3.0 * PARTY_ROW_STEP_CM;
            self.desenhar_campo(0, 2, y3, l::SIMPLES_NACIONAL, &p.simples_nacional, false);
            self.desenhar_campo(
                2,
                2,
                y3,
                l::REGIME_APURACAO_SN,
                &p.regime_apuracao_sn,
                false,
            );
        }
    }
}
