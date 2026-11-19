//! escritor PDF mínimo que reproduz a saída do folio usada pelo DANFSe

mod content;
mod document;
mod font;
mod image;
mod metrics_data;
mod qr;

pub use content::{ContentStream, format_num};
pub use document::{A4_HEIGHT, A4_WIDTH, Page, Watermark, write_pdf};
pub use font::{StandardFont, win_ansi_encode};
pub use image::{ErroImagem, Image};
pub use qr::{ErroQr, QrCode};
