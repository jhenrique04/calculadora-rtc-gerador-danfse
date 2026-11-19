/// situação da nota informada pelo chamador (parâmetro `stc`)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SituacaoNotaTipo {
    #[default]
    Valida,
    Substituida,
    Cancelada,
}

impl SituacaoNotaTipo {
    /// situação correspondente ao código `stc`; o booleano indica se o código é conhecido
    pub fn por_stc(stc: i64) -> (Self, bool) {
        match stc {
            0 => (Self::Valida, true),
            1 => (Self::Substituida, true),
            2 => (Self::Cancelada, true),
            _ => (Self::Valida, false),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Valida => "valida",
            Self::Substituida => "substituida",
            Self::Cancelada => "cancelada",
        }
    }

    pub fn marca_dagua(self) -> &'static str {
        match self {
            Self::Cancelada => "CANCELADA",
            Self::Substituida => "SUBSTITUÍDA",
            Self::Valida => "",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn situacao_por_stc() {
        let casos = [
            (0, SituacaoNotaTipo::Valida, true),
            (1, SituacaoNotaTipo::Substituida, true),
            (2, SituacaoNotaTipo::Cancelada, true),
            (3, SituacaoNotaTipo::Valida, false),
        ];
        for (stc, esperado, encontrado) in casos {
            assert_eq!(
                SituacaoNotaTipo::por_stc(stc),
                (esperado, encontrado),
                "stc={stc}"
            );
        }
    }

    #[test]
    fn marca_dagua_por_situacao() {
        assert_eq!(SituacaoNotaTipo::Valida.marca_dagua(), "");
        assert_eq!(SituacaoNotaTipo::Substituida.marca_dagua(), "SUBSTITUÍDA");
        assert_eq!(SituacaoNotaTipo::Cancelada.marca_dagua(), "CANCELADA");
    }
}
