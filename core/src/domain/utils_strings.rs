/// escapa `&` soltos (que não iniciam uma entidade XML válida) como `&amp;`,
/// recuperando XMLs legados que trazem o caractere sem escape
pub fn sanitizar_ampersand(xml: &[u8]) -> Vec<u8> {
    if !xml.contains(&b'&') {
        return xml.to_vec();
    }
    let mut out = Vec::with_capacity(xml.len() + 16);
    for (i, &b) in xml.iter().enumerate() {
        if b == b'&' && !inicia_entidade_xml(&xml[i..]) {
            out.extend_from_slice(b"&amp;");
            continue;
        }
        out.push(b);
    }
    out
}

fn inicia_entidade_xml(s: &[u8]) -> bool {
    let Some(rest) = s.strip_prefix(b"&") else {
        return false;
    };
    for nome in [&b"amp;"[..], b"lt;", b"gt;", b"quot;", b"apos;"] {
        if rest.starts_with(nome) {
            return true;
        }
    }
    let Some(numero) = rest.strip_prefix(b"#") else {
        return false;
    };
    let (digitos, eh_digito): (&[u8], fn(&u8) -> bool) = match numero.strip_prefix(b"x") {
        Some(hex) => (hex, u8::is_ascii_hexdigit),
        None => (numero, u8::is_ascii_digit),
    };
    let tamanho = digitos.iter().take_while(|b| eh_digito(b)).count();
    tamanho > 0 && digitos.get(tamanho) == Some(&b';')
}

/// limita o texto a `max_linhas` linhas de `max_cars` caracteres, marcando o corte com "..."
pub fn truncar_texto(texto: &str, max_linhas: usize, max_cars: usize) -> String {
    if texto.is_empty() || max_linhas == 0 || max_cars == 0 {
        return String::new();
    }
    let mut out: Vec<String> = Vec::new();
    let mut usadas = 0usize;
    let mut truncou = false;
    for seg in texto.split('\n') {
        if usadas >= max_linhas {
            truncou = true;
            break;
        }
        let runes: Vec<char> = seg.chars().collect();
        let necess = if runes.is_empty() {
            1
        } else {
            runes.len().div_ceil(max_cars)
        };
        if usadas + necess <= max_linhas {
            out.push(seg.to_string());
            usadas += necess;
            continue;
        }
        let limite = (max_linhas - usadas) * max_cars;
        if runes.len() > limite {
            out.push(runes[..limite].iter().collect());
        } else {
            out.push(seg.to_string());
        }
        truncou = true;
        break;
    }
    let res = out.join("\n");
    if truncou {
        return format!("{}...", res.trim_end_matches([' ', '\n']));
    }
    res
}

/// corta o texto em no máximo `max_cars` caracteres, reservando 3 deles para as reticências
pub fn truncar_por_tamanho(texto: &str, max_cars: usize) -> String {
    let runes: Vec<char> = texto.chars().collect();
    if runes.len() <= max_cars {
        return texto.to_string();
    }
    let limite = max_cars.saturating_sub(3);
    let prefixo: String = runes[..limite].iter().collect();
    format!("{}...", prefixo.trim_end_matches([' ', '\n']))
}

/// corta o conteúdo em `max_conteudo` caracteres e acrescenta reticências quando há corte
pub fn truncar_com_reticencias(texto: &str, max_conteudo: usize) -> String {
    let runes: Vec<char> = texto.chars().collect();
    if runes.len() <= max_conteudo {
        return texto.to_string();
    }
    let prefixo: String = runes[..max_conteudo].iter().collect();
    format!("{}...", prefixo.trim_end_matches([' ', '\n']))
}

/// `strconv.Atoi` do Go, com as mesmas mensagens de erro
pub fn parse_atoi(s: &str) -> Result<i64, String> {
    let digitos = s.strip_prefix(['+', '-']).unwrap_or(s);
    let quoted = crate::xml::go_quote(s);
    if digitos.is_empty() || !digitos.bytes().all(|b| b.is_ascii_digit()) {
        return Err(format!("strconv.Atoi: parsing {quoted}: invalid syntax"));
    }
    s.parse::<i64>()
        .map_err(|_| format!("strconv.Atoi: parsing {quoted}: value out of range"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sanitizar(s: &str) -> String {
        String::from_utf8(sanitizar_ampersand(s.as_bytes())).unwrap_or_default()
    }

    #[test]
    fn entidades_validas_preservadas() {
        let entrada = "&amp; &lt; &gt; &quot; &apos; &#65; &#x41;";
        assert_eq!(sanitizar(entrada), entrada);
    }

    #[test]
    fn ampersand_solto_escapado() {
        assert_eq!(sanitizar("A & B"), "A &amp; B");
    }

    #[test]
    fn sem_ampersand_inalterado() {
        assert_eq!(sanitizar("no ampersand here"), "no ampersand here");
    }

    #[test]
    fn misto() {
        assert_eq!(
            sanitizar("safe &amp; unsafe & evil"),
            "safe &amp; unsafe &amp; evil"
        );
    }

    #[test]
    fn referencia_numerica_sem_ponto_e_virgula_e_escapada() {
        assert_eq!(
            sanitizar("&#65 &#x; &nbsp;"),
            "&amp;#65 &amp;#x; &amp;nbsp;"
        );
    }

    #[test]
    fn truncar_texto_respeita_linhas() {
        assert_eq!(truncar_texto("abcdef", 1, 3), "abc...");
        assert_eq!(truncar_texto("ab\ncd", 2, 3), "ab\ncd");
        assert_eq!(truncar_texto("ab\ncd\nef", 2, 3), "ab\ncd...");
        assert_eq!(truncar_texto("", 2, 3), "");
    }

    #[test]
    fn truncar_por_tamanho_reserva_reticencias() {
        assert_eq!(truncar_por_tamanho("abcdefgh", 6), "abc...");
        assert_eq!(truncar_por_tamanho("abc", 6), "abc");
        assert_eq!(truncar_por_tamanho("abcdef", 2), "...");
    }

    #[test]
    fn truncar_com_reticencias_acrescenta_sufixo() {
        assert_eq!(truncar_com_reticencias("abc def", 4), "abc...");
        assert_eq!(truncar_com_reticencias("abc", 4), "abc");
    }
}
