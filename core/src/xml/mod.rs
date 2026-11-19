//! leitura do XML da NFS-e com a semântica de `xml.Unmarshal` do Go: campos
//! casados pelo nome local do elemento, elementos repetidos mesclados no mesmo
//! valor (o último vence), filhos desconhecidos ignorados e números vazios
//! tratados como zero

mod decoder;

pub use decoder::{Attr, Decoder, Name, StartElement, Token, XmlError, go_quote};

/// estrutura que pode ser preenchida a partir de um elemento XML
pub trait FromXml {
    fn attr(&mut self, _attr: &Attr) -> Result<(), XmlError> {
        Ok(())
    }

    /// trata um filho; devolve `false` quando o elemento não pertence à estrutura
    fn child(&mut self, d: &mut Decoder<'_>, start: &StartElement) -> Result<bool, XmlError>;
}

pub fn decode_struct<T: FromXml>(
    target: &mut T,
    d: &mut Decoder<'_>,
    start: &StartElement,
) -> Result<(), XmlError> {
    for attr in &start.attrs {
        target.attr(attr)?;
    }
    loop {
        match d.token()? {
            Some(Token::Start(child)) => {
                if !target.child(d, &child)? {
                    d.skip()?;
                }
            }
            Some(Token::End(_)) => return Ok(()),
            Some(_) => {}
            None => return Err(XmlError::Eof),
        }
    }
}

/// conteúdo textual direto do elemento corrente (filhos são descartados)
pub fn decode_chardata(d: &mut Decoder<'_>) -> Result<Vec<u8>, XmlError> {
    let mut data = Vec::new();
    loop {
        match d.token()? {
            Some(Token::Start(_)) => d.skip()?,
            Some(Token::End(_)) => return Ok(data),
            Some(Token::CharData(text)) => data.extend_from_slice(&text),
            Some(_) => {}
            None => return Err(XmlError::Eof),
        }
    }
}

pub fn decode_string(d: &mut Decoder<'_>) -> Result<String, XmlError> {
    decode_chardata(d).map(bytes_to_string)
}

pub fn decode_i64(d: &mut Decoder<'_>) -> Result<i64, XmlError> {
    parse_i64(&decode_chardata(d)?)
}

pub fn decode_f64(d: &mut Decoder<'_>) -> Result<f64, XmlError> {
    parse_f64(&decode_chardata(d)?)
}

pub fn bytes_to_string(data: Vec<u8>) -> String {
    String::from_utf8(data).unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned())
}

fn trim_go_space(s: &str) -> &str {
    s.trim_matches(|c: char| c.is_whitespace())
}

pub fn parse_i64(data: &[u8]) -> Result<i64, XmlError> {
    if data.is_empty() {
        return Ok(0);
    }
    let raw = String::from_utf8_lossy(data);
    let s = trim_go_space(&raw);
    let digits = s.strip_prefix(['+', '-']).unwrap_or(s);
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return Err(num_error("ParseInt", s, "invalid syntax"));
    }
    s.parse::<i64>()
        .map_err(|_| num_error("ParseInt", s, "value out of range"))
}

pub fn parse_f64(data: &[u8]) -> Result<f64, XmlError> {
    if data.is_empty() {
        return Ok(0.0);
    }
    let raw = String::from_utf8_lossy(data);
    let s = trim_go_space(&raw);
    let lower = s.to_ascii_lowercase();
    let unsigned = lower.strip_prefix(['+', '-']).unwrap_or(&lower);
    let is_special = matches!(unsigned, "inf" | "infinity" | "nan");
    let well_formed = is_special || is_decimal_float(unsigned);
    if !well_formed {
        return Err(num_error("ParseFloat", s, "invalid syntax"));
    }
    let value: f64 = lower
        .parse()
        .map_err(|_| num_error("ParseFloat", s, "invalid syntax"))?;
    if value.is_infinite() && !is_special {
        return Err(num_error("ParseFloat", s, "value out of range"));
    }
    Ok(value)
}

fn is_decimal_float(s: &str) -> bool {
    let (mantissa, exponent) = match s.find('e') {
        Some(i) => (&s[..i], Some(&s[i + 1..])),
        None => (s, None),
    };
    let mut digits = 0;
    let mut dots = 0;
    for b in mantissa.bytes() {
        match b {
            b'0'..=b'9' => digits += 1,
            b'.' => dots += 1,
            _ => return false,
        }
    }
    if digits == 0 || dots > 1 {
        return false;
    }
    match exponent {
        None => true,
        Some(e) => {
            let e = e.strip_prefix(['+', '-']).unwrap_or(e);
            !e.is_empty() && e.bytes().all(|b| b.is_ascii_digit())
        }
    }
}

fn num_error(func: &str, input: &str, reason: &str) -> XmlError {
    XmlError::Other(format!(
        "strconv.{func}: parsing {}: {reason}",
        go_quote(input)
    ))
}

/// primeiro elemento do documento (equivalente ao laço inicial de `Decoder.unmarshal`)
pub fn first_start_element(d: &mut Decoder<'_>) -> Result<StartElement, XmlError> {
    loop {
        match d.token()? {
            Some(Token::Start(start)) => return Ok(start),
            Some(_) => {}
            None => return Err(XmlError::Eof),
        }
    }
}
