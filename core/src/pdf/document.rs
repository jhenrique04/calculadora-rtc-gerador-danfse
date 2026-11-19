use std::fmt::Write as _;
use std::io::Write as _;
use std::sync::Arc;

use flate2::Compression;
use flate2::write::ZlibEncoder;
use md5::{Digest, Md5};

use super::content::ContentStream;
use super::font::StandardFont;
use super::image::Image;

pub const A4_WIDTH: f64 = 595.28;
pub const A4_HEIGHT: f64 = 841.89;
const VERSAO_PDF: &str = "1.7";

/// marca d'água aplicada por cima do conteúdo, centrada e rotacionada
#[derive(Debug, Clone, PartialEq)]
pub struct Watermark {
    pub text: String,
    pub font_size: f64,
    pub color: [f64; 3],
    pub angle: f64,
    pub opacity: f64,
}

const WATERMARK_FONT_SIZE: f64 = 60.0;
const WATERMARK_GRAY: f64 = 0.85;
const WATERMARK_ANGLE: f64 = 45.0;
const WATERMARK_OPACITY: f64 = 0.3;

impl Watermark {
    /// campos zerados viram os padrões do folio (60pt, cinza 0.85, 45°, 30%)
    pub fn with_defaults(
        text: &str,
        font_size: f64,
        color: [f64; 3],
        angle: f64,
        opacity: f64,
    ) -> Self {
        let ou = |v: f64, padrao: f64| if v == 0.0 { padrao } else { v };
        Self {
            text: text.to_string(),
            font_size: ou(font_size, WATERMARK_FONT_SIZE),
            color: if color == [0.0; 3] {
                [WATERMARK_GRAY; 3]
            } else {
                color
            },
            angle: ou(angle, WATERMARK_ANGLE),
            opacity: ou(opacity, WATERMARK_OPACITY),
        }
    }
}

/// página única A4 com recursos registrados na ordem de uso, como no folio
#[derive(Debug, Default)]
pub struct Page {
    stream: ContentStream,
    fonts: Vec<StandardFont>,
    images: Vec<Arc<Image>>,
    ext_gstates: Vec<f64>,
}

impl Page {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn stream(&mut self) -> &mut ContentStream {
        &mut self.stream
    }

    fn font_resource(&mut self, font: StandardFont) -> String {
        let idx = match self.fonts.iter().position(|f| *f == font) {
            Some(i) => i,
            None => {
                self.fonts.push(font);
                self.fonts.len() - 1
            }
        };
        format!("F{}", idx + 1)
    }

    pub fn add_text(&mut self, text: &str, font: StandardFont, size: f64, x: f64, y: f64) {
        let nome = self.font_resource(font);
        self.stream.begin_text();
        self.stream.set_font(&nome, size);
        self.stream.move_text(x, y);
        self.stream.show_text(text);
        self.stream.end_text();
    }

    pub fn add_line(&mut self, from: (f64, f64), to: (f64, f64), width: f64, color: [f64; 3]) {
        self.stream.save_state();
        self.stream.set_line_width(width);
        self.stream.set_stroke_rgb(color);
        self.stream.move_to(from.0, from.1);
        self.stream.line_to(to.0, to.1);
        self.stream.stroke();
        self.stream.restore_state();
    }

    pub fn add_rect(&mut self, rect: [f64; 4], stroke_width: f64, color: [f64; 3]) {
        self.stream.save_state();
        self.stream.set_line_width(stroke_width);
        self.stream.set_stroke_rgb(color);
        self.stream.rectangle(rect[0], rect[1], rect[2], rect[3]);
        self.stream.stroke();
        self.stream.restore_state();
    }

    pub fn add_rect_filled(&mut self, rect: [f64; 4], color: [f64; 3]) {
        self.stream.save_state();
        self.stream.set_fill_rgb(color);
        self.stream.rectangle(rect[0], rect[1], rect[2], rect[3]);
        self.stream.fill();
        self.stream.restore_state();
    }

    pub fn add_image(&mut self, image: &Arc<Image>, rect: [f64; 4]) {
        let idx = match self.images.iter().position(|i| Arc::ptr_eq(i, image)) {
            Some(i) => i,
            None => {
                self.images.push(Arc::clone(image));
                self.images.len() - 1
            }
        };
        self.stream.save_state();
        self.stream
            .concat_matrix([rect[2], 0.0, 0.0, rect[3], rect[0], rect[1]]);
        self.stream.do_xobject(&format!("Im{}", idx + 1));
        self.stream.restore_state();
    }

    fn apply_watermark(&mut self, wm: &Watermark) {
        let fonte = self.font_resource(StandardFont::Helvetica);
        self.ext_gstates.push(wm.opacity);
        let gs = format!("GS{}", self.ext_gstates.len());

        let rad = wm.angle * std::f64::consts::PI / 180.0;
        let (cos, sin) = (rad.cos(), rad.sin());
        let largura = StandardFont::Helvetica.measure_string(&wm.text, wm.font_size);
        let cx = A4_WIDTH / 2.0 - (largura / 2.0) * cos;
        let cy = A4_HEIGHT / 2.0 - (largura / 2.0) * sin;

        let mut s = ContentStream::new();
        s.save_state();
        s.set_ext_gstate(&gs);
        s.begin_text();
        s.set_font(&fonte, wm.font_size);
        s.set_fill_rgb(wm.color);
        s.set_text_matrix([cos, sin, -sin, cos, cx, cy]);
        s.show_text(&wm.text);
        s.end_text();
        s.restore_state();
        self.stream.append(&s);
    }
}

/// serializa a página em um PDF determinístico com a mesma estrutura de
/// objetos do folio (catálogo, páginas, fontes, imagens, ExtGState,
/// conteúdo, página) e /ID derivado do MD5 dos objetos
pub fn write_pdf(mut page: Page, watermark: Option<&Watermark>) -> std::io::Result<Vec<u8>> {
    if let Some(wm) = watermark.filter(|w| !w.text.is_empty()) {
        page.apply_watermark(wm);
    }
    let mut objetos: Vec<Vec<u8>> = vec![Vec::new(), Vec::new()];
    let fonte_ref: Vec<usize> = page
        .fonts
        .iter()
        .map(|f| push(&mut objetos, font_dict(*f)))
        .collect();
    let mut imagem_ref = Vec::with_capacity(page.images.len());
    for img in &page.images {
        let smask = img
            .alpha_flate()
            .map(|a| push(&mut objetos, image_stream(img, a, "DeviceGray", None)));
        imagem_ref.push(push(
            &mut objetos,
            image_stream(img, img.rgb_flate(), "DeviceRGB", smask),
        ));
    }
    let gs_ref: Vec<usize> = page
        .ext_gstates
        .iter()
        .map(|o| {
            let real = format_real(*o);
            push(
                &mut objetos,
                format!("<< /Type /ExtGState /ca {real} /CA {real} >>").into_bytes(),
            )
        })
        .collect();
    let conteudo = stream_object(
        "<< /Filter /FlateDecode /Length {len} >>",
        &deflate(page.stream.bytes())?,
    );
    let conteudo_ref = push(&mut objetos, conteudo);

    let mut recursos = String::new();
    recursos_dict(&mut recursos, "Font", "F", &fonte_ref);
    recursos_dict(&mut recursos, "XObject", "Im", &imagem_ref);
    recursos_dict(&mut recursos, "ExtGState", "GS", &gs_ref);
    let pagina_ref = objetos.len() + 1;
    objetos.push(
        format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {} {}] /Resources <<{recursos} >> /Contents {conteudo_ref} 0 R >>",
            format_real(A4_WIDTH),
            format_real(A4_HEIGHT)
        )
        .into_bytes(),
    );
    objetos[0] = b"<< /Type /Catalog /Pages 2 0 R >>".to_vec();
    objetos[1] = format!("<< /Type /Pages /Count 1 /Kids [{pagina_ref} 0 R] >>").into_bytes();

    Ok(serializar(&objetos))
}

fn push(objetos: &mut Vec<Vec<u8>>, obj: Vec<u8>) -> usize {
    objetos.push(obj);
    objetos.len()
}

fn font_dict(f: StandardFont) -> Vec<u8> {
    format!(
        "<< /Type /Font /Subtype /Type1 /BaseFont /{} /Encoding /WinAnsiEncoding >>",
        f.base_font()
    )
    .into_bytes()
}

fn image_stream(img: &Image, comprimido: &[u8], cs: &str, smask: Option<usize>) -> Vec<u8> {
    let smask = smask
        .map(|r| format!(" /SMask {r} 0 R"))
        .unwrap_or_default();
    let dict = format!(
        "<< /Type /XObject /Subtype /Image /Width {} /Height {} /ColorSpace /{cs} /BitsPerComponent 8{smask} /Filter /FlateDecode /Length {{len}} >>",
        img.width(),
        img.height()
    );
    stream_object(&dict, comprimido)
}

/// zlib no nível máximo, como o FlateDecode do folio
pub(super) fn deflate(data: &[u8]) -> std::io::Result<Vec<u8>> {
    let mut enc = ZlibEncoder::new(Vec::new(), Compression::best());
    enc.write_all(data)?;
    enc.finish()
}

fn stream_object(dict_template: &str, comprimido: &[u8]) -> Vec<u8> {
    let mut out = dict_template
        .replace("{len}", &comprimido.len().to_string())
        .into_bytes();
    out.extend_from_slice(b"\nstream\n");
    out.extend_from_slice(comprimido);
    out.extend_from_slice(b"\nendstream");
    out
}

fn recursos_dict(out: &mut String, chave: &str, prefixo: &str, refs: &[usize]) {
    if refs.is_empty() {
        return;
    }
    let _ = write!(out, " /{chave} <<");
    for (i, r) in refs.iter().enumerate() {
        let _ = write!(out, " /{prefixo}{} {r} 0 R", i + 1);
    }
    out.push_str(" >>");
}

/// real PDF como o `formatReal` do folio: sempre com ao menos uma casa decimal
fn format_real(v: f64) -> String {
    if !v.is_finite() {
        return "0.0".into();
    }
    if v == v.trunc() && v.abs() < 1e15 {
        return format!("{v:.1}");
    }
    let s = format!("{v:.6}");
    let s = s.trim_end_matches('0');
    if s.ends_with('.') {
        return format!("{s}0");
    }
    s.to_string()
}

fn corpo_objeto(n: usize, obj: &[u8]) -> Vec<u8> {
    let mut out = format!("{n} 0 obj\n").into_bytes();
    out.extend_from_slice(obj);
    out.extend_from_slice(b"\nendobj\n");
    out
}

fn serializar(objetos: &[Vec<u8>]) -> Vec<u8> {
    let corpos: Vec<Vec<u8>> = objetos
        .iter()
        .enumerate()
        .map(|(i, o)| corpo_objeto(i + 1, o))
        .collect();
    let mut hasher = Md5::new();
    for c in &corpos {
        hasher.update(c);
    }
    let id: String = hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02X}"))
        .collect();

    let mut out = format!("%PDF-{VERSAO_PDF}\n%").into_bytes();
    out.extend_from_slice(b"\xe2\xe3\xcf\xd3\n");
    let mut offsets = Vec::with_capacity(corpos.len());
    for c in &corpos {
        offsets.push(out.len());
        out.extend_from_slice(c);
    }
    let xref = out.len();
    let total = objetos.len() + 1;
    let mut trailer = format!("xref\n0 {total}\n0000000000 65535 f \n");
    for off in offsets {
        let _ = writeln!(trailer, "{off:010} 00000 n ");
    }
    let _ = write!(
        trailer,
        "trailer\n<< /Root 1 0 R /ID [<{id}> <{id}>] /Size {total} >>\nstartxref\n{xref}\n%%EOF\n"
    );
    out.extend_from_slice(trailer.as_bytes());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reais_como_folio() {
        assert_eq!(format_real(0.35), "0.35");
        assert_eq!(format_real(595.28), "595.28");
        assert_eq!(format_real(2.0), "2.0");
    }

    #[test]
    fn estrutura_minima() {
        let mut page = Page::new();
        page.add_text("oi", StandardFont::HelveticaBold, 9.0, 1.0, 2.0);
        page.add_text("x", StandardFont::Helvetica, 9.0, 1.0, 2.0);
        let wm = Watermark {
            text: "CANCELADA".into(),
            font_size: 60.0,
            color: [0.35; 3],
            angle: 45.0,
            opacity: 0.35,
        };
        let pdf = match write_pdf(page, Some(&wm)) {
            Ok(p) => String::from_utf8_lossy(&p).into_owned(),
            Err(e) => panic!("{e}"),
        };
        assert!(pdf.starts_with("%PDF-1.7\n%"));
        assert!(pdf.contains("3 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold"));
        assert!(pdf.contains("5 0 obj\n<< /Type /ExtGState /ca 0.35 /CA 0.35 >>"));
        assert!(pdf.contains(
            "/Resources << /Font << /F1 3 0 R /F2 4 0 R >> /ExtGState << /GS1 5 0 R >> >> /Contents 6 0 R >>"
        ));
        assert!(pdf.contains("/Size 8 >>"));
        assert!(pdf.ends_with("%%EOF\n"));
    }

    #[test]
    fn deterministico() {
        let gerar = || {
            let mut p = Page::new();
            p.add_line((0.0, 0.0), (1.0, 1.0), 0.5, [0.0; 3]);
            write_pdf(p, None).ok()
        };
        assert_eq!(gerar(), gerar());
    }
}
