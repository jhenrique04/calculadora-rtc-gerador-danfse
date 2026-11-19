use crate::domain::presentation::DocumentoDanfse;
use crate::pdf::{A4_HEIGHT, StandardFont};

use super::style::STANDARD_FONT_BASELINE_ADJUSTMENT;

pub(super) const PAGE_HEIGHT_PT: f64 = A4_HEIGHT;

/// 1 cm = 72/2,54 pontos
pub(super) const CM_PT: f64 = 72.0 / 2.54;

const PAGE_LEFT_CM: f64 = 0.15;
const PAGE_TOP_CM: f64 = 0.15;
const PAGE_RIGHT_CM: f64 = 20.85;
const PAGE_BOTTOM_CM: f64 = 29.55;

pub(super) const GRID_LEFT_CM: f64 = 0.30;
pub(super) const COL_X2_CM: f64 = 5.41;
pub(super) const COL_X3_CM: f64 = 10.51;
pub(super) const COL_X4_CM: f64 = 15.62;
pub(super) const COL_X_END_CM: f64 = 20.70;
pub(super) const QR_LEFT_CM: f64 = 17.39;
pub(super) const QR_TOP_CM: f64 = 1.58;
pub(super) const QR_SIZE_CM: f64 = 1.90;
pub(super) const QR_TEXT_TOP_CM: f64 = 3.53;
pub(super) const QR_TEXT_STEP_CM: f64 = 0.21;
const QR_TEXT_WIDTH_CM: f64 = 4.72;
pub(super) const ACCESS_KEY_TOP_CM: f64 = 1.48;
pub(super) const ACCESS_KEY_WIDTH_CM: f64 = 15.30;
pub(super) const NFSE_FIELDS_TOP_CM: f64 = 2.27;
pub(super) const DPS_FIELDS_TOP_CM: f64 = 2.96;

pub(super) const QR_TEXT_LEFT_CM: f64 = QR_LEFT_CM + (QR_SIZE_CM - QR_TEXT_WIDTH_CM) / 2.0;

pub(super) const FIELD_TEXT_INSET_CM: f64 = 0.08;
pub(super) const IDENTIFICATION_TEXT_INSET_CM: f64 = FIELD_TEXT_INSET_CM;
pub(super) const IDENTIFICATION_ROW_HEIGHT_CM: f64 = 0.69;
pub(super) const IDENTIFICATION_LABEL_OFFSET_CM: f64 = 0.06;
pub(super) const IDENTIFICATION_VALUE_OFFSET_CM: f64 = 0.36;
pub(super) const IDENTIFICATION_SHADE_TOP_CM: f64 = 0.015;
pub(super) const IDENTIFICATION_SHADE_HEIGHT_CM: f64 = 0.675;
pub(super) const SECTION_TITLE_OFFSET_CM: f64 = 0.07;
pub(super) const PARTY_ROW_STEP_CM: f64 = 0.64;
pub(super) const MUNICIPAL_ROW_STEP_CM: f64 = 0.65;
pub(super) const FEDERAL_ROW_STEP_CM: f64 = 0.65;
pub(super) const IBS_ROW_STEP_CM: f64 = 0.64;
pub(super) const IBS_SECOND_ROW_OFFSET_CM: f64 = 1.29;
pub(super) const IBS_THIRD_ROW_OFFSET_CM: f64 = 1.94;
pub(super) const TOTALS_ROW_STEP_CM: f64 = 0.69;
pub(super) const SERVICE_TAX_DESCRIPTION_OFFSET_CM: f64 = 0.65;
pub(super) const SERVICE_DESCRIPTION_OFFSET_CM: f64 = 1.05;
pub(super) const COMPLEMENTARY_CONTENT_OFFSET_CM: f64 = 0.41;
pub(super) const FIELD_SHADE_TOP_CM: f64 = 0.015;
pub(super) const FIELD_SHADE_HEIGHT_CM: f64 = PARTY_ROW_STEP_CM - FIELD_SHADE_TOP_CM;

pub(super) const COL1_CM: f64 = COL_X2_CM - GRID_LEFT_CM;
pub(super) const COL2_CM: f64 = COL_X3_CM - COL_X2_CM;
pub(super) const COL3_CM: f64 = COL_X4_CM - COL_X3_CM;

// valueSizePt × 1,2 de entrelinha
pub(super) const LINE_STEP_CM: f64 = (7.0 * 1.2) / CM_PT;

pub(super) const COL_GUTTER_CM: f64 = 0.15;
pub(super) const ELLIPSIS: &str = "...";

// linhas mínimas de informações complementares preservadas mesmo com descrição longa
const MIN_COMPL_LINES: usize = 12;

pub(super) const Y_CANHOTO_CM: f64 = 28.10;
const FOLGA_INFO_CM: f64 = 0.10;

pub(super) const NORMAL_SECTION_HEIGHT_CM: f64 = 0.63;
pub(super) const COMPLEMENTARY_SECTION_HEIGHT_CM: f64 = 0.39;
pub(super) const TOTAL_SECTION_HEIGHT_CM: f64 = 0.67;
pub(super) const CANHOTO_BOTTOM_CM: f64 = 28.77;

pub(super) const SUPPRESSED_PARTY_BLOCK_HEIGHT_CM: f64 = 0.40;

pub(super) const COMPLEMENTARY_LINE_STEP_CM: f64 = (6.5 * 1.2) / CM_PT;

pub(super) const SUPPRESSED_MUNICIPAL_BLOCK_HEIGHT_CM: f64 = 0.32;
const PARTY_BLOCK_HEIGHT_CM: f64 = 1.94;
const MUNICIPAL_BLOCK_HEIGHT_CM: f64 = 2.59;

const TOMADOR_TOP_CM: f64 = 6.92;
const SERVICE_TOP_CM: f64 = 12.74;
const MUNICIPAL_TOP_CM: f64 = 14.43;
const FEDERAL_TOP_CM: f64 = 17.02;
const IBS_TOP_CM: f64 = 18.32;
const TOTALS_TOP_CM: f64 = 20.90;
const COMPLEMENTARY_TOP_CM: f64 = 22.27;

// tolerância para floor não perder uma linha por erro de ponto flutuante
const EPSILON_LINHAS: f64 = 1e-9;

/// limite do bloco de informações complementares: o canhoto quando visível,
/// senão o fim dele
pub(super) fn limite_inferior_bloco_cm(visivel_canhoto: bool) -> f64 {
    if visivel_canhoto {
        Y_CANHOTO_CM
    } else {
        CANHOTO_BOTTOM_CM
    }
}

pub(super) fn top_to_pdf_y(top_cm: f64) -> f64 {
    PAGE_HEIGHT_PT - top_cm * CM_PT
}

pub(super) fn cm(v: f64) -> f64 {
    v * CM_PT
}

/// linha de base pelo ascent da fonte, não `topo - tamanho`
pub(super) fn baseline_from_top(top_cm: f64, size: f64, font: StandardFont) -> f64 {
    top_to_pdf_y(top_cm) - font.ascent(size) - size * STANDARD_FONT_BASELINE_ADJUSTMENT
}

pub(super) fn grid_x2() -> f64 {
    COL_X2_CM * CM_PT
}
pub(super) fn grid_x3() -> f64 {
    COL_X3_CM * CM_PT
}
pub(super) fn grid_x4() -> f64 {
    COL_X4_CM * CM_PT
}
pub(super) fn body_left() -> f64 {
    GRID_LEFT_CM * CM_PT
}
pub(super) fn body_right() -> f64 {
    COL_X_END_CM * CM_PT
}

const LIMITES_COLUNAS: [f64; 5] = [GRID_LEFT_CM, COL_X2_CM, COL_X3_CM, COL_X4_CM, COL_X_END_CM];

pub(super) fn coluna_x(indice: usize) -> f64 {
    if indice >= 4 {
        return 0.0;
    }
    LIMITES_COLUNAS[indice] * CM_PT
}

pub(super) fn largura_colunas(inicio: usize, quantidade: usize) -> f64 {
    if quantidade == 0 || inicio + quantidade > 4 {
        return 0.0;
    }
    (LIMITES_COLUNAS[inicio + quantidade] - LIMITES_COLUNAS[inicio]) * CM_PT
}

pub(super) fn page_left() -> f64 {
    PAGE_LEFT_CM * CM_PT
}
pub(super) fn page_top() -> f64 {
    PAGE_TOP_CM * CM_PT
}
pub(super) fn page_right() -> f64 {
    PAGE_RIGHT_CM * CM_PT
}
pub(super) fn page_bottom() -> f64 {
    PAGE_BOTTOM_CM * CM_PT
}

/// topo (cm) de cada seção da página única
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct PageLayout {
    pub altura_tomador: f64,
    pub altura_destinatario: f64,
    pub altura_intermediario: f64,
    pub service_top: f64,
    pub municipal_top: f64,
    pub federal_top: f64,
    pub ibs_top: f64,
    pub totals_top: f64,
    pub complementary_top: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct TextAllocation {
    pub description_lines: usize,
    pub complementary_lines: usize,
    pub shift_cm: f64,
}

fn altura_parte(visivel: bool) -> f64 {
    if visivel {
        PARTY_BLOCK_HEIGHT_CM
    } else {
        SUPPRESSED_PARTY_BLOCK_HEIGHT_CM
    }
}

pub(super) fn montar_layout_pagina(doc: &DocumentoDanfse) -> PageLayout {
    let partes = &doc.partes;
    let altura_tomador = TOMADOR_TOP_CM;
    let altura_destinatario = altura_tomador + altura_parte(partes.tomador.visivel);
    let altura_intermediario = altura_destinatario + altura_parte(partes.destinatario.visivel);
    let service_top = altura_intermediario + altura_parte(partes.intermediario.visivel);
    let municipal_top = service_top + (MUNICIPAL_TOP_CM - SERVICE_TOP_CM);
    let altura_municipal = if doc.tributacoes.municipal.visivel {
        MUNICIPAL_BLOCK_HEIGHT_CM
    } else {
        SUPPRESSED_MUNICIPAL_BLOCK_HEIGHT_CM
    };
    let federal_top = municipal_top + altura_municipal;
    let ibs_top = federal_top + (IBS_TOP_CM - FEDERAL_TOP_CM);
    let totals_top = ibs_top + (TOTALS_TOP_CM - IBS_TOP_CM);
    let complementary_top = totals_top + (COMPLEMENTARY_TOP_CM - TOTALS_TOP_CM);
    PageLayout {
        altura_tomador,
        altura_destinatario,
        altura_intermediario,
        service_top,
        municipal_top,
        federal_top,
        ibs_top,
        totals_top,
        complementary_top,
    }
}

fn floor_linhas(cm: f64, passo: f64) -> i64 {
    // cast seguro: alturas da página são poucas dezenas de linhas
    #[allow(clippy::cast_possible_truncation)]
    let linhas = (cm / passo + EPSILON_LINHAS).floor() as i64;
    linhas
}

/// linhas que cabem entre o topo do conteúdo e a linha de totais, no mínimo 1
pub(super) fn contar_linhas_informacoes_complementares(
    content_top_cm: f64,
    tax_line_top_cm: f64,
) -> usize {
    let linhas = floor_linhas(
        tax_line_top_cm - content_top_cm - FOLGA_INFO_CM,
        COMPLEMENTARY_LINE_STEP_CM,
    );
    usize::try_from(linhas).unwrap_or(0).max(1)
}

pub(super) fn calcular_alocacao_texto(
    layout: &PageLayout,
    doc: &DocumentoDanfse,
    descricao_linhas: usize,
    complementar_linhas: usize,
) -> TextAllocation {
    let mut descricao_linhas = descricao_linhas.max(1);
    let reserva_complementar = complementar_linhas.clamp(1, MIN_COMPL_LINES);
    let tax_line_top = limite_inferior_bloco_cm(doc.canhoto.visivel);
    let base_content_top = layout.complementary_top + COMPLEMENTARY_CONTENT_OFFSET_CM;
    let disponivel_cm = (tax_line_top - base_content_top - FOLGA_INFO_CM).max(0.0);

    // a descrição ganha o espaço primeiro; o complementar mantém ao menos a reserva
    let extra_descricao_cm = (descricao_linhas - 1) as f64 * LINE_STEP_CM;
    let pedido_complementar_cm = reserva_complementar as f64 * COMPLEMENTARY_LINE_STEP_CM;
    if extra_descricao_cm + pedido_complementar_cm > disponivel_cm {
        let max_extra_cm = (disponivel_cm - pedido_complementar_cm).max(0.0);
        let max_linhas = 1 + usize::try_from(floor_linhas(max_extra_cm, LINE_STEP_CM)).unwrap_or(0);
        descricao_linhas = descricao_linhas.min(max_linhas);
    }
    let descricao_linhas = descricao_linhas.max(1);

    let shift = (descricao_linhas - 1) as f64 * LINE_STEP_CM;
    let content_top = base_content_top + shift;
    TextAllocation {
        description_lines: descricao_linhas,
        complementary_lines: contar_linhas_informacoes_complementares(content_top, tax_line_top),
        shift_cm: shift,
    }
}
