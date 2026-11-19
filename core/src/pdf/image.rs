use thiserror::Error;

use super::document::deflate;

#[derive(Debug, Error)]
pub enum ErroImagem {
    #[error("image: png: {0}")]
    Png(#[from] png::DecodingError),
    #[error("image: png: unsupported color type {0:?}")]
    TipoCor(png::ColorType),
    #[error("image: png: {0}")]
    Compressao(#[from] std::io::Error),
}

/// imagem RGB já comprimida para embutir, com soft mask separada apenas
/// quando algum pixel não é opaco. Comprimir uma vez evita refazer o
/// deflate do logo a cada PDF
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image {
    width: u32,
    height: u32,
    rgb_flate: Vec<u8>,
    alpha_flate: Option<Vec<u8>>,
}

impl Image {
    pub fn from_png(data: &[u8]) -> Result<Self, ErroImagem> {
        let mut decoder = png::Decoder::new(data);
        decoder.set_transformations(png::Transformations::normalize_to_color8());
        let mut reader = decoder.read_info()?;
        let mut buf = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buf)?;
        buf.truncate(info.buffer_size());
        let (rgb, alpha) = match info.color_type {
            png::ColorType::Rgba => separar_alfa(&buf),
            png::ColorType::Rgb => (buf, None),
            outro => return Err(ErroImagem::TipoCor(outro)),
        };
        Ok(Self {
            width: info.width,
            height: info.height,
            rgb_flate: deflate(&rgb)?,
            alpha_flate: alpha.map(|a| deflate(&a)).transpose()?,
        })
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub(super) fn rgb_flate(&self) -> &[u8] {
        &self.rgb_flate
    }

    pub(super) fn alpha_flate(&self) -> Option<&[u8]> {
        self.alpha_flate.as_deref()
    }
}

fn separar_alfa(rgba: &[u8]) -> (Vec<u8>, Option<Vec<u8>>) {
    let pixels = rgba.len() / 4;
    let mut rgb = Vec::with_capacity(pixels * 3);
    let mut alpha = Vec::with_capacity(pixels);
    for px in rgba.as_chunks::<4>().0 {
        rgb.extend_from_slice(&px[..3]);
        alpha.push(px[3]);
    }
    let tem_alfa = alpha.iter().any(|&a| a != u8::MAX);
    (rgb, tem_alfa.then_some(alpha))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodifica_logo_com_alfa() {
        let logo = include_bytes!("../../assets/logo-nfse.png");
        let img = match Image::from_png(logo) {
            Ok(i) => i,
            Err(e) => panic!("{e}"),
        };
        assert_eq!((img.width(), img.height()), (1920, 389));
        assert!(!img.rgb_flate().is_empty());
        assert!(img.alpha_flate().is_some());
    }

    #[test]
    fn separa_alfa_opaco() {
        assert_eq!(separar_alfa(&[1, 2, 3, 255]), (vec![1, 2, 3], None));
        assert_eq!(separar_alfa(&[1, 2, 3, 7]), (vec![1, 2, 3], Some(vec![7])));
    }

    #[test]
    fn rejeita_png_invalido() {
        assert!(Image::from_png(b"nao e png").is_err());
    }
}
