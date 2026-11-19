use std::fmt::Write as _;

use super::font::win_ansi_encode;

const LIMITE_INTEIRO: f64 = 1e15;

/// formata números como o `formatNum` do folio: inteiros sem casas, senão
/// seis casas sem zeros à direita
pub fn format_num(v: f64) -> String {
    if !v.is_finite() {
        return "0".into();
    }
    if v == v.trunc() && v.abs() < LIMITE_INTEIRO {
        // cast seguro: |v| < 1e15 cabe em i64
        #[allow(clippy::cast_possible_truncation)]
        return format!("{}", v as i64);
    }
    let s = format!("{v:.6}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// escapa uma string literal PDF preservando bytes WinAnsi (128-255)
pub fn escape_literal(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len());
    for &c in bytes {
        match c {
            b'\\' => out.extend_from_slice(b"\\\\"),
            b'(' => out.extend_from_slice(b"\\("),
            b')' => out.extend_from_slice(b"\\)"),
            b'\n' => out.extend_from_slice(b"\\n"),
            b'\r' => out.extend_from_slice(b"\\r"),
            b'\t' => out.extend_from_slice(b"\\t"),
            0..=0x1F => out.extend_from_slice(format!("\\{c:03o}").as_bytes()),
            _ => out.push(c),
        }
    }
    out
}

/// fluxo de conteúdo com os operadores usados pelo DANFSe, uma instrução por linha
#[derive(Debug, Default, Clone)]
pub struct ContentStream {
    buf: Vec<u8>,
}

impl ContentStream {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn bytes(&self) -> &[u8] {
        &self.buf
    }

    // folio separa operadores com '\n' antes de cada linha, sem quebra final
    fn separar(&mut self) {
        if !self.buf.is_empty() {
            self.buf.push(b'\n');
        }
    }

    fn line(&mut self, s: &str) {
        self.separar();
        self.buf.extend_from_slice(s.as_bytes());
    }

    fn nums(valores: &[f64]) -> String {
        let mut s = String::new();
        for (i, v) in valores.iter().enumerate() {
            if i > 0 {
                s.push(' ');
            }
            let _ = write!(s, "{}", format_num(*v));
        }
        s
    }

    pub fn save_state(&mut self) {
        self.line("q");
    }

    pub fn restore_state(&mut self) {
        self.line("Q");
    }

    pub fn set_fill_rgb(&mut self, c: [f64; 3]) {
        self.line(&format!("{} rg", Self::nums(&c)));
    }

    pub fn set_stroke_rgb(&mut self, c: [f64; 3]) {
        self.line(&format!("{} RG", Self::nums(&c)));
    }

    pub fn set_line_width(&mut self, w: f64) {
        self.line(&format!("{} w", format_num(w)));
    }

    pub fn rectangle(&mut self, x: f64, y: f64, w: f64, h: f64) {
        self.line(&format!("{} re", Self::nums(&[x, y, w, h])));
    }

    pub fn move_to(&mut self, x: f64, y: f64) {
        self.line(&format!("{} m", Self::nums(&[x, y])));
    }

    pub fn line_to(&mut self, x: f64, y: f64) {
        self.line(&format!("{} l", Self::nums(&[x, y])));
    }

    pub fn fill(&mut self) {
        self.line("f");
    }

    pub fn stroke(&mut self) {
        self.line("S");
    }

    pub fn concat_matrix(&mut self, m: [f64; 6]) {
        self.line(&format!("{} cm", Self::nums(&m)));
    }

    pub fn set_text_matrix(&mut self, m: [f64; 6]) {
        self.line(&format!("{} Tm", Self::nums(&m)));
    }

    pub fn do_xobject(&mut self, nome: &str) {
        self.line(&format!("/{nome} Do"));
    }

    pub fn set_ext_gstate(&mut self, nome: &str) {
        self.line(&format!("/{nome} gs"));
    }

    pub fn begin_text(&mut self) {
        self.line("BT");
    }

    pub fn end_text(&mut self) {
        self.line("ET");
    }

    pub fn set_font(&mut self, nome: &str, size: f64) {
        self.line(&format!("/{nome} {} Tf", format_num(size)));
    }

    pub fn move_text(&mut self, x: f64, y: f64) {
        self.line(&format!("{} Td", Self::nums(&[x, y])));
    }

    pub fn show_text(&mut self, texto: &str) {
        self.separar();
        self.buf.push(b'(');
        self.buf
            .extend_from_slice(&escape_literal(&win_ansi_encode(texto)));
        self.buf.extend_from_slice(b") Tj");
    }

    pub fn append(&mut self, outro: &ContentStream) {
        if outro.buf.is_empty() {
            return;
        }
        self.separar();
        self.buf.extend_from_slice(&outro.buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formata_numeros_como_folio() {
        assert_eq!(format_num(0.0), "0");
        assert_eq!(format_num(-0.0), "0");
        assert_eq!(format_num(9.0), "9");
        assert_eq!(format_num(0.5), "0.5");
        assert_eq!(format_num(0.9490196078), "0.94902");
        assert_eq!(format_num(8.503937007874), "8.503937");
        assert_eq!(format_num(1e-7), "0");
        assert_eq!(format_num(-1e-7), "-0");
        assert_eq!(format_num(f64::NAN), "0");
    }

    #[test]
    fn escapa_literais() {
        assert_eq!(
            escape_literal(b"a(b)\\\n\x01\xC7"),
            b"a\\(b\\)\\\\\\n\\001\xC7".to_vec()
        );
    }

    #[test]
    fn operadores() {
        let mut s = ContentStream::new();
        s.save_state();
        s.set_fill_rgb([0.0, 0.0, 0.0]);
        s.rectangle(1.0, 2.5, 3.0, 4.0);
        s.fill();
        s.restore_state();
        s.show_text("São (x)");
        assert_eq!(
            s.bytes(),
            b"q\n0 0 0 rg\n1 2.5 3 4 re\nf\nQ\n(S\xE3o \\(x\\)) Tj".as_slice()
        );
    }
}
