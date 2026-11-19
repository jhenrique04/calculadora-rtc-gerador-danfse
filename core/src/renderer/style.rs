use super::layout::{
    FIELD_SHADE_HEIGHT_CM, FIELD_SHADE_TOP_CM, FIELD_TEXT_INSET_CM, IDENTIFICATION_LABEL_OFFSET_CM,
    IDENTIFICATION_ROW_HEIGHT_CM, IDENTIFICATION_SHADE_HEIGHT_CM, IDENTIFICATION_SHADE_TOP_CM,
    IDENTIFICATION_TEXT_INSET_CM, IDENTIFICATION_VALUE_OFFSET_CM,
};

pub(super) const LABEL_SIZE_PT: f64 = 6.0;
pub(super) const VALUE_SIZE_PT: f64 = 7.0;
pub(super) const TITLE_SIZE_PT: f64 = 7.0;

// compensa o deslocamento para cima da Helvetica em relação à fonte original do leiaute
pub(super) const STANDARD_FONT_BASELINE_ADJUSTMENT: f64 = 0.19;

pub(super) const BLACK_RGB: [f64; 3] = [0.0, 0.0, 0.0];
pub(super) const GRAY_HEADER_RGB: [f64; 3] = [0.9490196078, 0.9490196078, 0.9490196078];
pub(super) const RED_RGB: [f64; 3] = [1.0, 0.0, 0.0];

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct EstiloCelula {
    pub tamanho_rotulo_pt: f64,
    pub tamanho_valor_pt: f64,
    pub recuo_esquerdo_cm: f64,
    pub recuo_direito_cm: f64,
    pub offset_rotulo_cm: f64,
    pub offset_valor_cm: f64,
    pub altura_cm: f64,
    pub sombreado: bool,
    pub sombreado_top_cm: f64,
    pub sombreado_altura_cm: f64,
}

const OFFSET_VALOR_CAMPO_CM: f64 = 0.30;

pub(super) const ESTILO_CAMPO: EstiloCelula = EstiloCelula {
    tamanho_rotulo_pt: LABEL_SIZE_PT,
    tamanho_valor_pt: VALUE_SIZE_PT,
    recuo_esquerdo_cm: FIELD_TEXT_INSET_CM,
    recuo_direito_cm: FIELD_TEXT_INSET_CM,
    offset_rotulo_cm: 0.0,
    offset_valor_cm: OFFSET_VALOR_CAMPO_CM,
    altura_cm: 0.0,
    sombreado: false,
    sombreado_top_cm: 0.0,
    sombreado_altura_cm: 0.0,
};

pub(super) const ESTILO_ACESSO: EstiloCelula = EstiloCelula {
    tamanho_rotulo_pt: VALUE_SIZE_PT,
    ..ESTILO_CAMPO
};

pub(super) const ESTILO_IDENTIFICACAO: EstiloCelula = EstiloCelula {
    tamanho_rotulo_pt: TITLE_SIZE_PT,
    tamanho_valor_pt: VALUE_SIZE_PT,
    recuo_esquerdo_cm: IDENTIFICATION_TEXT_INSET_CM,
    recuo_direito_cm: IDENTIFICATION_TEXT_INSET_CM,
    offset_rotulo_cm: IDENTIFICATION_LABEL_OFFSET_CM,
    offset_valor_cm: IDENTIFICATION_VALUE_OFFSET_CM,
    altura_cm: IDENTIFICATION_ROW_HEIGHT_CM,
    sombreado: false,
    sombreado_top_cm: 0.0,
    sombreado_altura_cm: 0.0,
};

pub(super) const ESTILO_IDENTIFICACAO_SOMBREADA: EstiloCelula = EstiloCelula {
    sombreado: true,
    sombreado_top_cm: IDENTIFICATION_SHADE_TOP_CM,
    sombreado_altura_cm: IDENTIFICATION_SHADE_HEIGHT_CM,
    ..ESTILO_IDENTIFICACAO
};

pub(super) const ESTILO_CAMPO_SOMBREADO: EstiloCelula = EstiloCelula {
    sombreado: true,
    sombreado_top_cm: FIELD_SHADE_TOP_CM,
    sombreado_altura_cm: FIELD_SHADE_HEIGHT_CM,
    ..ESTILO_CAMPO
};
