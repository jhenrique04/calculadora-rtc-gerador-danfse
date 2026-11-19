use std::borrow::Cow;

use crate::pdf::StandardFont;

use super::layout::ELLIPSIS;

const QUEBRAS: [char; 2] = [' ', '\n'];

/// próxima linha pelo primeiro estouro, recuando até o último espaço quando
/// sobra texto. Devolve a linha e quantos bytes de `text` ela consumiu
pub(super) fn next_text_line(
    text: &str,
    font: StandardFont,
    size: f64,
    width: f64,
) -> (&str, usize) {
    let (fit_bytes, _, _) = font.prefix_fit(text, size, width);
    if fit_bytes == 0 {
        return ("", 0);
    }
    let mut part = text[..fit_bytes].trim();
    let mut consumed = fit_bytes;
    if fit_bytes < text.len()
        && let Some(espaco) = ultimo_espaco(part)
    {
        part = part[..espaco].trim();
        consumed = text
            .get(..espaco)
            .map_or(0, |t| t.trim_end_matches(QUEBRAS).len());
    }
    (part, consumed)
}

fn ultimo_espaco(s: &str) -> Option<usize> {
    s.rfind(QUEBRAS).filter(|&n| n > 0)
}

/// quebra `text` em linhas com a fonte regular, como `countLines` do Go
fn quebrar_linhas(
    text: &str,
    width: f64,
    size: f64,
    max_lines: Option<usize>,
) -> (Vec<&str>, &str) {
    let mut linhas = Vec::new();
    let mut remaining = text.trim();
    while !remaining.is_empty() && max_lines.is_none_or(|m| linhas.len() < m) {
        let (fit_bytes, _, _) = StandardFont::Helvetica.prefix_fit(remaining, size, width);
        if fit_bytes == 0 {
            break;
        }
        let mut part = remaining[..fit_bytes].trim();
        if fit_bytes < remaining.len()
            && let Some(n) = ultimo_espaco(part)
        {
            part = part[..n].trim();
        }
        if part.is_empty() {
            break;
        }
        linhas.push(part);
        remaining = remaining[part.len()..].trim();
    }
    (linhas, remaining)
}

pub(super) fn medir_linhas(text: &str, width: f64, size: f64) -> usize {
    quebrar_linhas(text, width, size, None).0.len()
}

pub(super) fn truncate_lines(text: &str, width: f64, size: f64, max_lines: usize) -> String {
    let (linhas, remaining) = quebrar_linhas(text, width, size, Some(max_lines));
    let result = linhas.join("\n");
    if remaining.is_empty() {
        return result;
    }
    format!("{}{ELLIPSIS}", result.trim_end_matches(QUEBRAS))
}

pub(super) fn fit<'a>(value: &'a str, font: StandardFont, size: f64, width: f64) -> Cow<'a, str> {
    if font.measure_string(value, size) <= width {
        return Cow::Borrowed(value);
    }
    Cow::Owned(truncate_text(value, font, size, width))
}

/// maior prefixo (busca binária em caracteres) que cabe com "..." anexado;
/// vazio quando só a reticência caberia
pub(super) fn truncate_text(text: &str, font: StandardFont, size: f64, max_width: f64) -> String {
    if text.is_empty() || font.measure_string(text, size) <= max_width {
        return text.to_string();
    }
    let chars: Vec<char> = text.chars().collect();
    let candidato = |meio: usize| -> String {
        let prefixo: String = chars[..meio].iter().collect();
        format!("{}{ELLIPSIS}", prefixo.trim_end_matches(' '))
    };
    let melhor = maior_prefixo(chars.len(), |meio| {
        let cand = candidato(meio);
        (font.measure_string(&cand, size) <= max_width).then_some(cand)
    });
    match melhor {
        Some(m) if m != ELLIPSIS => m,
        _ => String::new(),
    }
}

/// busca binária em [0, n-1] pelo maior índice aceito, como os laços lo/hi do Go
fn maior_prefixo(n: usize, mut aceita: impl FnMut(usize) -> Option<String>) -> Option<String> {
    let (mut lo, mut hi) = (0isize, n as isize - 1);
    let mut melhor = None;
    while lo <= hi {
        let meio = (lo + hi) / 2;
        match aceita(meio as usize) {
            Some(c) => {
                melhor = Some(c);
                lo = meio + 1;
            }
            None => hi = meio - 1,
        }
    }
    melhor
}

pub(super) fn fallback_traco(s: &str) -> &str {
    if s.trim().is_empty() { "-" } else { s }
}

/// texto complementar com a linha tributária anexada uma única vez
pub(super) fn combinar_conteudo_complementar(conteudo: &str, linha_tributos: &str) -> String {
    if conteudo.is_empty() {
        return linha_tributos.to_string();
    }
    if conteudo.contains(linha_tributos) || linha_tributos.is_empty() {
        return conteudo.to_string();
    }
    format!("{conteudo} | {linha_tributos}")
}

/// ajusta as informações complementares a `max_lines`, preservando a linha
/// tributária no fim quando ela é o sufixo do texto
pub(super) fn ajustar_informacoes_complementares(
    texto: &str,
    linha_tributos: &str,
    width: f64,
    size: f64,
    max_lines: usize,
) -> String {
    if medir_linhas(texto, width, size) <= max_lines {
        return texto.to_string();
    }
    let Some(prefixo) = texto
        .strip_suffix(linha_tributos)
        .filter(|_| !linha_tributos.is_empty())
    else {
        return truncate_lines(texto, width, size, max_lines);
    };
    let chars: Vec<char> = prefixo.chars().collect();
    let melhor = maior_prefixo(chars.len(), |meio| {
        let inicio: String = chars[..meio].iter().collect();
        let cand = format!(
            "{}{ELLIPSIS} | {linha_tributos}",
            inicio.trim_end_matches(' ')
        );
        (medir_linhas(&cand, width, size) <= max_lines).then_some(cand)
    });
    melhor.unwrap_or_else(|| linha_tributos.to_string())
}
