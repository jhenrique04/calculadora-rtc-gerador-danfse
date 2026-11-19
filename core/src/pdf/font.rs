use unicode_segmentation::UnicodeSegmentation;

use super::metrics_data::{
    HELVETICA_BOLD_KERNS, HELVETICA_BOLD_WIDTHS, HELVETICA_KERNS, HELVETICA_WIDTHS,
};

// ascent do Apêndice D da especificação PDF, igual para Helvetica e Helvetica-Bold
const HELVETICA_ASCENT: f64 = 718.0;
const UNIDADES_POR_EM: f64 = 1000.0;
const SUBSTITUTO_WIN_ANSI: u8 = b'?';

/// fonte padrão (core-14) usada pelo DANFSe
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StandardFont {
    Helvetica,
    HelveticaBold,
}

impl StandardFont {
    pub fn base_font(self) -> &'static str {
        match self {
            Self::Helvetica => "Helvetica",
            Self::HelveticaBold => "Helvetica-Bold",
        }
    }

    fn widths(self) -> &'static [(u32, u16)] {
        match self {
            Self::Helvetica => HELVETICA_WIDTHS,
            Self::HelveticaBold => HELVETICA_BOLD_WIDTHS,
        }
    }

    fn kerns(self) -> &'static [(u32, u32, i16)] {
        match self {
            Self::Helvetica => HELVETICA_KERNS,
            Self::HelveticaBold => HELVETICA_BOLD_KERNS,
        }
    }

    pub fn ascent(self, size: f64) -> f64 {
        HELVETICA_ASCENT / UNIDADES_POR_EM * size
    }

    fn width(self, c: char) -> f64 {
        let tabela = self.widths();
        let w = match tabela.binary_search_by_key(&u32::from(c), |e| e.0) {
            Ok(i) => tabela[i].1,
            // entrada 0 é a largura padrão (.notdef)
            Err(_) => tabela.first().map_or(0, |e| e.1),
        };
        f64::from(w)
    }

    fn kern(self, esquerda: char, direita: char) -> f64 {
        let chave = (u32::from(esquerda), u32::from(direita));
        self.kerns()
            .binary_search_by_key(&chave, |e| (e.0, e.1))
            .map_or(0.0, |i| f64::from(self.kerns()[i].2))
    }

    /// largura em pontos somando clusters de grafemas com kerning entre eles,
    /// como `Standard.MeasureString` do folio
    pub fn measure_string(self, text: &str, size: f64) -> f64 {
        let mut total = 0.0;
        let mut anterior: Option<char> = None;
        for cluster in text.graphemes(true) {
            let Some(base) = cluster.chars().next() else {
                continue;
            };
            if let Some(prev) = anterior {
                total += self.kern(prev, base);
            }
            total += self.width(base);
            anterior = Some(base);
        }
        total / UNIDADES_POR_EM * size
    }

    /// maior prefixo que cabe em `max_width` pela regra do primeiro estouro,
    /// mantendo ao menos um caractere. Devolve (bytes, caracteres, largura)
    // marcas combinantes (Extend) não somam largura; SpacingMark não ocorre em texto latino
    pub fn prefix_fit(self, text: &str, size: f64, max_width: f64) -> (usize, usize, f64) {
        let (mut bytes, mut runas, mut largura) = (0usize, 0usize, 0.0f64);
        let mut total = 0.0;
        let mut anterior: Option<char> = None;
        let mut pos = 0usize;
        for cluster in text.graphemes(true) {
            let mut chars = cluster.chars();
            let Some(base) = chars.next() else {
                continue;
            };
            if let Some(prev) = anterior {
                total += self.kern(prev, base);
            }
            total += self.width(base);
            pos += base.len_utf8();
            let w = total / UNIDADES_POR_EM * size;
            if runas >= 1 && w > max_width {
                return (bytes, runas, largura);
            }
            (bytes, runas, largura) = (pos, runas + 1, w);
            for c in chars {
                // primeiro cluster pode ter estourado sozinho; aí suas marcas ficam de fora
                if w > max_width {
                    return (bytes, runas, largura);
                }
                pos += c.len_utf8();
                (bytes, runas) = (pos, runas + 1);
            }
            anterior = Some(base);
        }
        (bytes, runas, largura)
    }
}

/// codifica em WinAnsi (cp1252); fora da tabela vira '?'
pub fn win_ansi_encode(s: &str) -> Vec<u8> {
    s.chars().map(win_ansi_byte).collect()
}

fn win_ansi_byte(c: char) -> u8 {
    let cp = u32::from(c);
    if cp < 0x80 || (0xA0..=0xFF).contains(&cp) {
        return u8::try_from(cp).unwrap_or(SUBSTITUTO_WIN_ANSI);
    }
    const ESPECIAIS: &[(u32, u8)] = &[
        (0x20AC, 128),
        (0x201A, 130),
        (0x0192, 131),
        (0x201E, 132),
        (0x2026, 133),
        (0x2020, 134),
        (0x2021, 135),
        (0x02C6, 136),
        (0x2030, 137),
        (0x0160, 138),
        (0x2039, 139),
        (0x0152, 140),
        (0x017D, 142),
        (0x2018, 145),
        (0x2019, 146),
        (0x201C, 147),
        (0x201D, 148),
        (0x2022, 149),
        (0x2013, 150),
        (0x2014, 151),
        (0x02DC, 152),
        (0x2122, 153),
        (0x0161, 154),
        (0x203A, 155),
        (0x0153, 156),
        (0x017E, 158),
        (0x0178, 159),
    ];
    ESPECIAIS
        .iter()
        .find(|(u, _)| *u == cp)
        .map_or(SUBSTITUTO_WIN_ANSI, |(_, b)| *b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mede_texto_com_kerning() {
        let f = StandardFont::Helvetica;
        // "AV" tem kerning negativo na Helvetica
        assert!(
            f.measure_string("AV", 10.0)
                < f.measure_string("A", 10.0) + f.measure_string("V", 10.0)
        );
        assert_eq!(f.measure_string("", 10.0), 0.0);
        assert_eq!(f.measure_string(" ", 1000.0), 278.0);
    }

    #[test]
    fn caractere_desconhecido_usa_largura_padrao() {
        assert_eq!(
            StandardFont::Helvetica.measure_string("\u{4E2D}", 1000.0),
            278.0
        );
    }

    #[test]
    fn prefix_fit_primeiro_estouro() {
        let f = StandardFont::Helvetica;
        let (b, r, w) = f.prefix_fit("abc", 10.0, 0.0);
        assert_eq!((b, r), (1, 1));
        assert!(w > 0.0);
        let total = f.measure_string("abc", 10.0);
        assert_eq!(f.prefix_fit("abc", 10.0, total), (3, 3, total));
        assert_eq!(f.prefix_fit("", 10.0, 5.0), (0, 0, 0.0));
        assert_eq!(f.prefix_fit("çé", 10.0, 1000.0).0, 4);
    }

    #[test]
    fn win_ansi() {
        assert_eq!(win_ansi_encode("aÇ€中"), vec![b'a', 0xC7, 128, b'?']);
    }
}
