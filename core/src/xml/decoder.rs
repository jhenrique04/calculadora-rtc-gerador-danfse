//! tokenizador XML estrito com a mesma semântica (e as mesmas mensagens de
//! erro) do `encoding/xml` do Go usado pelo serviço original: linhas contadas
//! por `\n`, `\r\n` normalizado, apenas as cinco entidades predefinidas e
//! referências numéricas, e nenhuma resolução de DTD

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Name {
    pub space: String,
    pub local: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attr {
    pub name: Name,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartElement {
    pub name: Name,
    pub attrs: Vec<Attr>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    Start(StartElement),
    End(Name),
    CharData(Vec<u8>),
    Comment(Vec<u8>),
    ProcInst { target: String, inst: Vec<u8> },
    Directive(Vec<u8>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XmlError {
    Syntax { msg: String, line: usize },
    Other(String),
    Eof,
}

impl fmt::Display for XmlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            XmlError::Syntax { msg, line } => write!(f, "XML syntax error on line {line}: {msg}"),
            XmlError::Other(msg) => f.write_str(msg),
            XmlError::Eof => f.write_str("EOF"),
        }
    }
}

impl std::error::Error for XmlError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StackKind {
    Start,
    Ns,
}

#[derive(Debug)]
struct StackEntry {
    kind: StackKind,
    name: Name,
    ok: bool,
}

pub struct Decoder<'a> {
    input: &'a [u8],
    pos: usize,
    next_byte: Option<u8>,
    buf: Vec<u8>,
    stack: Vec<StackEntry>,
    ns: std::collections::HashMap<String, String>,
    need_close: bool,
    to_close: Name,
    err: Option<XmlError>,
    line: usize,
}

const XML_URL: &str = "http://www.w3.org/XML/1998/namespace";
const XMLNS_PREFIX: &str = "xmlns";
const XML_PREFIX: &str = "xml";

impl<'a> Decoder<'a> {
    pub fn new(input: &'a [u8]) -> Self {
        Self {
            input,
            pos: 0,
            next_byte: None,
            buf: Vec::new(),
            stack: Vec::new(),
            ns: std::collections::HashMap::new(),
            need_close: false,
            to_close: Name::default(),
            err: None,
            line: 1,
        }
    }

    /// próximo token; `Ok(None)` no fim da entrada
    pub fn token(&mut self) -> Result<Option<Token>, XmlError> {
        let tok = match self.raw_token() {
            Ok(t) => t,
            Err(XmlError::Eof) => {
                if !self.stack.is_empty() {
                    return Err(self.syntax_error("unexpected EOF"));
                }
                return Ok(None);
            }
            Err(e) => return Err(e),
        };
        match tok {
            Token::Start(mut start) => {
                for a in &start.attrs {
                    if a.name.space == XMLNS_PREFIX {
                        let old = self.ns.get(&a.name.local).cloned();
                        self.push_ns(&a.name.local, old);
                        self.ns.insert(a.name.local.clone(), a.value.clone());
                    }
                    if a.name.space.is_empty() && a.name.local == XMLNS_PREFIX {
                        let old = self.ns.get("").cloned();
                        self.push_ns("", old);
                        self.ns.insert(String::new(), a.value.clone());
                    }
                }
                self.stack.push(StackEntry {
                    kind: StackKind::Start,
                    name: start.name.clone(),
                    ok: false,
                });
                self.translate(&mut start.name, true);
                for a in &mut start.attrs {
                    let mut n = std::mem::take(&mut a.name);
                    self.translate(&mut n, false);
                    a.name = n;
                }
                Ok(Some(Token::Start(start)))
            }
            Token::End(name) => self.pop_element(name).map(|n| Some(Token::End(n))),
            other => Ok(Some(other)),
        }
    }

    /// consome tokens até o fim do elemento corrente (equivalente a `Decoder.Skip`)
    pub fn skip(&mut self) -> Result<(), XmlError> {
        let mut depth = 0usize;
        loop {
            match self.token()? {
                Some(Token::Start(_)) => depth += 1,
                Some(Token::End(_)) => {
                    if depth == 0 {
                        return Ok(());
                    }
                    depth -= 1;
                }
                Some(_) => {}
                None => return Err(XmlError::Eof),
            }
        }
    }

    fn push_ns(&mut self, local: &str, old: Option<String>) {
        let ok = old.is_some();
        self.stack.push(StackEntry {
            kind: StackKind::Ns,
            name: Name {
                space: old.unwrap_or_default(),
                local: local.to_string(),
            },
            ok,
        });
    }

    fn translate(&self, n: &mut Name, is_element_name: bool) {
        if n.space == XMLNS_PREFIX {
            return;
        }
        if n.space.is_empty() && !is_element_name {
            return;
        }
        if n.space == XML_PREFIX {
            n.space = XML_URL.to_string();
        } else if n.space.is_empty() && n.local == XMLNS_PREFIX {
            return;
        }
        if let Some(v) = self.ns.get(&n.space) {
            n.space = v.clone();
        }
    }

    fn pop_element(&mut self, mut name: Name) -> Result<Name, XmlError> {
        let top = self.stack.pop();
        match top {
            Some(s) if s.kind == StackKind::Start => {
                if s.name.local != name.local {
                    let e = self.syntax_error(&format!(
                        "element <{}> closed by </{}>",
                        s.name.local, name.local
                    ));
                    self.err = Some(e.clone());
                    return Err(e);
                }
                if s.name.space != name.space {
                    let ns = if name.space.is_empty() {
                        "\"\"".to_string()
                    } else {
                        name.space.clone()
                    };
                    let e = self.syntax_error(&format!(
                        "element <{}> in space {} closed by </{}> in space {}",
                        s.name.local, s.name.space, name.local, ns
                    ));
                    self.err = Some(e.clone());
                    return Err(e);
                }
            }
            _ => {
                let e = self.syntax_error(&format!("unexpected end element </{}>", name.local));
                self.err = Some(e.clone());
                return Err(e);
            }
        }
        self.translate(&mut name, true);
        while let Some(top) = self.stack.last() {
            if top.kind == StackKind::Start {
                break;
            }
            if let Some(s) = self.stack.pop() {
                if s.ok {
                    self.ns.insert(s.name.local, s.name.space);
                } else {
                    self.ns.remove(&s.name.local);
                }
            }
        }
        Ok(name)
    }

    fn syntax_error(&self, msg: &str) -> XmlError {
        XmlError::Syntax {
            msg: msg.to_string(),
            line: self.line,
        }
    }

    fn fail<T>(&mut self, msg: &str) -> Result<T, XmlError> {
        let e = self.syntax_error(msg);
        self.err = Some(e.clone());
        Err(e)
    }

    fn getc(&mut self) -> Result<u8, XmlError> {
        if let Some(e) = &self.err {
            return Err(e.clone());
        }
        let b = if let Some(b) = self.next_byte.take() {
            b
        } else {
            let Some(&b) = self.input.get(self.pos) else {
                self.err = Some(XmlError::Eof);
                return Err(XmlError::Eof);
            };
            self.pos += 1;
            b
        };
        if b == b'\n' {
            self.line += 1;
        }
        Ok(b)
    }

    fn mustgetc(&mut self) -> Result<u8, XmlError> {
        match self.getc() {
            Err(XmlError::Eof) => {
                let e = self.syntax_error("unexpected EOF");
                self.err = Some(e.clone());
                Err(e)
            }
            other => other,
        }
    }

    fn ungetc(&mut self, b: u8) {
        if b == b'\n' {
            self.line -= 1;
        }
        self.next_byte = Some(b);
    }

    // no EOF o erro fica registrado e o próximo mustgetc o converte em "unexpected EOF"
    fn space(&mut self) {
        while let Ok(b) = self.getc() {
            if !matches!(b, b' ' | b'\r' | b'\n' | b'\t') {
                self.ungetc(b);
                return;
            }
        }
    }

    fn raw_token(&mut self) -> Result<Token, XmlError> {
        if let Some(e) = &self.err {
            return Err(e.clone());
        }
        if self.need_close {
            self.need_close = false;
            return Ok(Token::End(std::mem::take(&mut self.to_close)));
        }

        let b = self.getc()?;
        if b != b'<' {
            self.ungetc(b);
            let data = self.text(None, false)?;
            return Ok(Token::CharData(data));
        }

        let b = self.mustgetc()?;
        match b {
            b'/' => {
                let Some(name) = self.nsname()? else {
                    return self.fail("expected element name after </");
                };
                self.space();
                let b = self.mustgetc()?;
                if b != b'>' {
                    return self.fail(&format!(
                        "invalid characters between </{} and >",
                        name.local
                    ));
                }
                Ok(Token::End(name))
            }
            b'?' => {
                let Some(target) = self.name()? else {
                    return self.fail("expected target name after <?");
                };
                self.space();
                self.buf.clear();
                let mut b0 = 0u8;
                loop {
                    let b = self.mustgetc()?;
                    self.buf.push(b);
                    if b0 == b'?' && b == b'>' {
                        break;
                    }
                    b0 = b;
                }
                let data = self.buf[..self.buf.len() - 2].to_vec();
                if target == "xml" {
                    let content = String::from_utf8_lossy(&data).into_owned();
                    let ver = proc_inst("version", &content);
                    if !ver.is_empty() && ver != "1.0" {
                        let e = XmlError::Other(format!(
                            "xml: unsupported version {}; only version 1.0 is supported",
                            go_quote(&ver)
                        ));
                        self.err = Some(e.clone());
                        return Err(e);
                    }
                    let enc = proc_inst("encoding", &content);
                    if !enc.is_empty() && !enc.eq_ignore_ascii_case("utf-8") {
                        let e = XmlError::Other(format!(
                            "xml: encoding {} declared but Decoder.CharsetReader is nil",
                            go_quote(&enc)
                        ));
                        self.err = Some(e.clone());
                        return Err(e);
                    }
                }
                Ok(Token::ProcInst { target, inst: data })
            }
            b'!' => self.bang(),
            _ => {
                self.ungetc(b);
                self.start_element()
            }
        }
    }

    fn bang(&mut self) -> Result<Token, XmlError> {
        let b = self.mustgetc()?;
        match b {
            b'-' => {
                let b = self.mustgetc()?;
                if b != b'-' {
                    return self.fail("invalid sequence <!- not part of <!--");
                }
                self.buf.clear();
                let (mut b0, mut b1) = (0u8, 0u8);
                loop {
                    let b = self.mustgetc()?;
                    self.buf.push(b);
                    if b0 == b'-' && b1 == b'-' {
                        if b != b'>' {
                            return self.fail("invalid sequence \"--\" not allowed in comments");
                        }
                        break;
                    }
                    b0 = b1;
                    b1 = b;
                }
                Ok(Token::Comment(self.buf[..self.buf.len() - 3].to_vec()))
            }
            b'[' => {
                for &expected in b"CDATA[" {
                    let b = self.mustgetc()?;
                    if b != expected {
                        return self.fail("invalid <![ sequence");
                    }
                }
                let data = self.text(None, true)?;
                Ok(Token::CharData(data))
            }
            first => self.directive(first),
        }
    }

    fn directive(&mut self, first: u8) -> Result<Token, XmlError> {
        self.buf.clear();
        self.buf.push(first);
        let mut inquote = 0u8;
        let mut depth = 0i32;
        let mut pending: Option<u8> = None;
        loop {
            let b = match pending.take() {
                Some(b) => b,
                None => {
                    let b = self.mustgetc()?;
                    if inquote == 0 && b == b'>' && depth == 0 {
                        break;
                    }
                    b
                }
            };
            self.buf.push(b);
            if b == inquote {
                inquote = 0;
            } else if inquote != 0 {
            } else if b == b'\'' || b == b'"' {
                inquote = b;
            } else if b == b'>' {
                depth -= 1;
            } else if b == b'<' {
                let marker = b"!--";
                let mut matched = true;
                for i in 0..marker.len() {
                    let c = self.mustgetc()?;
                    if c != marker[i] {
                        self.buf.extend_from_slice(&marker[..i]);
                        depth += 1;
                        pending = Some(c);
                        matched = false;
                        break;
                    }
                }
                if !matched {
                    continue;
                }
                self.buf.pop();
                let (mut b0, mut b1) = (0u8, 0u8);
                loop {
                    let c = self.mustgetc()?;
                    if b0 == b'-' && b1 == b'-' && c == b'>' {
                        break;
                    }
                    b0 = b1;
                    b1 = c;
                }
                self.buf.push(b' ');
            }
        }
        Ok(Token::Directive(self.buf.clone()))
    }

    fn start_element(&mut self) -> Result<Token, XmlError> {
        let Some(name) = self.nsname()? else {
            return self.fail("expected element name after <");
        };
        let mut attrs = Vec::new();
        let mut empty = false;
        loop {
            self.space();
            let b = self.mustgetc()?;
            if b == b'/' {
                empty = true;
                let b = self.mustgetc()?;
                if b != b'>' {
                    return self.fail("expected /> in element");
                }
                break;
            }
            if b == b'>' {
                break;
            }
            self.ungetc(b);
            let Some(attr_name) = self.nsname()? else {
                return self.fail("expected attribute name in element");
            };
            self.space();
            let b = self.mustgetc()?;
            if b != b'=' {
                return self.fail("attribute name without = in element");
            }
            self.space();
            let b = self.mustgetc()?;
            if b != b'"' && b != b'\'' {
                return self.fail("unquoted or missing attribute value in element");
            }
            let data = self.text(Some(b), false)?;
            attrs.push(Attr {
                name: attr_name,
                value: String::from_utf8_lossy(&data).into_owned(),
            });
        }
        if empty {
            self.need_close = true;
            self.to_close = name.clone();
        }
        Ok(Token::Start(StartElement { name, attrs }))
    }

    fn text(&mut self, quote: Option<u8>, cdata: bool) -> Result<Vec<u8>, XmlError> {
        let (mut b0, mut b1) = (0u8, 0u8);
        let mut trunc = 0usize;
        self.buf.clear();
        loop {
            let b = match self.getc() {
                Ok(b) => b,
                Err(XmlError::Eof) => {
                    if cdata {
                        return self.fail("unexpected EOF in CDATA section");
                    }
                    // EOF encerra o texto; o erro fica registrado para o próximo token
                    break;
                }
                Err(e) => return Err(e),
            };

            if quote.is_none() && b0 == b']' && b1 == b']' && b == b'>' {
                if cdata {
                    trunc = 2;
                    break;
                }
                return self.fail("unescaped ]]> not in CDATA section");
            }
            if b == b'<' && !cdata {
                if quote.is_some() {
                    return self.fail("unescaped < inside quoted string");
                }
                self.ungetc(b'<');
                break;
            }
            if quote == Some(b) {
                break;
            }
            if b == b'&' && !cdata {
                self.entity_ref()?;
                b0 = 0;
                b1 = 0;
                continue;
            }
            if b == b'\r' {
                self.buf.push(b'\n');
            } else if b1 == b'\r' && b == b'\n' {
            } else {
                self.buf.push(b);
            }
            b0 = b1;
            b1 = b;
        }
        let data = self.buf[..self.buf.len() - trunc].to_vec();
        validate_chars(&data).map_err(|msg| {
            let e = self.syntax_error(&msg);
            self.err = Some(e.clone());
            e
        })?;
        Ok(data)
    }

    fn entity_ref(&mut self) -> Result<(), XmlError> {
        let before = self.buf.len();
        self.buf.push(b'&');
        let mut replacement: Option<String> = None;
        let mut b = self.mustgetc()?;
        if b == b'#' {
            self.buf.push(b);
            b = self.mustgetc()?;
            let mut base = 10;
            if b == b'x' {
                base = 16;
                self.buf.push(b);
                b = self.mustgetc()?;
            }
            let start = self.buf.len();
            while b.is_ascii_digit() || (base == 16 && b.is_ascii_hexdigit()) {
                self.buf.push(b);
                b = self.mustgetc()?;
            }
            if b != b';' {
                self.ungetc(b);
            } else {
                let digits = String::from_utf8_lossy(&self.buf[start..]).into_owned();
                self.buf.push(b';');
                if let Ok(n) = u64::from_str_radix(&digits, base)
                    && n <= 0x10FFFF
                {
                    replacement = Some(char_from_u32_lossy(n as u32));
                }
            }
        } else {
            self.ungetc(b);
            let _ok = self.read_name()?;
            let b = self.mustgetc()?;
            if b != b';' {
                self.ungetc(b);
            } else {
                let name = self.buf[before + 1..].to_vec();
                self.buf.push(b';');
                if is_name(&name) {
                    replacement = match name.as_slice() {
                        b"lt" => Some("<".into()),
                        b"gt" => Some(">".into()),
                        b"amp" => Some("&".into()),
                        b"apos" => Some("'".into()),
                        b"quot" => Some("\"".into()),
                        _ => None,
                    };
                }
            }
        }
        if let Some(text) = replacement {
            self.buf.truncate(before);
            self.buf.extend_from_slice(text.as_bytes());
            return Ok(());
        }
        let mut ent = String::from_utf8_lossy(&self.buf[before..]).into_owned();
        if !ent.ends_with(';') {
            ent.push_str(" (no semicolon)");
        }
        self.fail(&format!("invalid character entity {ent}"))
    }

    fn nsname(&mut self) -> Result<Option<Name>, XmlError> {
        let Some(s) = self.name()? else {
            return Ok(None);
        };
        if s.matches(':').count() > 1 {
            return Ok(None);
        }
        match s.split_once(':') {
            Some((space, local)) if !space.is_empty() && !local.is_empty() => Ok(Some(Name {
                space: space.to_string(),
                local: local.to_string(),
            })),
            _ => Ok(Some(Name {
                space: String::new(),
                local: s,
            })),
        }
    }

    fn name(&mut self) -> Result<Option<String>, XmlError> {
        self.buf.clear();
        if !self.read_name()? {
            return Ok(None);
        }
        if !is_name(&self.buf) {
            let msg = format!("invalid XML name: {}", String::from_utf8_lossy(&self.buf));
            return self.fail(&msg);
        }
        Ok(Some(String::from_utf8_lossy(&self.buf).into_owned()))
    }

    fn read_name(&mut self) -> Result<bool, XmlError> {
        let b = self.mustgetc()?;
        if b < 0x80 && !is_name_byte(b) {
            self.ungetc(b);
            return Ok(false);
        }
        self.buf.push(b);
        loop {
            let b = self.mustgetc()?;
            if b < 0x80 && !is_name_byte(b) {
                self.ungetc(b);
                return Ok(true);
            }
            self.buf.push(b);
        }
    }
}

fn char_from_u32_lossy(n: u32) -> String {
    char::from_u32(n)
        .map(String::from)
        .unwrap_or_else(|| "\u{FFFD}".to_string())
}

fn is_name_byte(c: u8) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, b'_' | b':' | b'.' | b'-')
}

fn is_name_first(c: char) -> bool {
    c == '_' || c == ':' || c.is_alphabetic()
}

fn is_name_second(c: char) -> bool {
    is_name_first(c) || c.is_alphanumeric() || matches!(c, '-' | '.' | '\u{B7}') || is_combining(c)
}

fn is_combining(c: char) -> bool {
    matches!(c as u32, 0x0300..=0x036F | 0x0483..=0x0489 | 0x0591..=0x05BD | 0x20D0..=0x20FF)
}

fn is_name(s: &[u8]) -> bool {
    let Ok(text) = std::str::from_utf8(s) else {
        return false;
    };
    let mut chars = text.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    is_name_first(first) && chars.all(is_name_second)
}

fn validate_chars(data: &[u8]) -> Result<(), String> {
    let text = match std::str::from_utf8(data) {
        Ok(t) => t,
        Err(_) => return Err("invalid UTF-8".to_string()),
    };
    for c in text.chars() {
        let r = c as u32;
        let in_range = r == 0x09
            || r == 0x0A
            || r == 0x0D
            || (0x20..=0xD7FF).contains(&r)
            || (0xE000..=0xFFFD).contains(&r)
            || (0x10000..=0x10FFFF).contains(&r);
        if !in_range {
            return Err(format!("illegal character code U+{r:04X}"));
        }
    }
    Ok(())
}

/// valor de um parâmetro em uma instrução de processamento (`procInst` do Go)
fn proc_inst(param: &str, s: &str) -> String {
    let needle = format!("{param}=");
    let mut rest = s;
    let idx = loop {
        let Some(i) = rest.find(&needle) else {
            return String::new();
        };
        let absolute = s.len() - rest.len() + i;
        let preceded_ok = absolute == 0 || {
            let prev = s.as_bytes()[absolute - 1];
            matches!(prev, b' ' | b'\r' | b'\n' | b'\t')
        };
        if preceded_ok {
            break absolute + needle.len();
        }
        rest = &rest[i + needle.len()..];
    };
    let v = &s[idx..];
    let Some(q) = v.chars().next() else {
        return String::new();
    };
    if q != '\'' && q != '"' {
        return String::new();
    }
    let inner = &v[1..];
    match inner.find(q) {
        Some(end) => inner[..end].to_string(),
        None => String::new(),
    }
}

/// aspas no estilo `strconv.Quote` do Go (suficiente para mensagens de erro)
pub fn go_quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{7}' => out.push_str("\\a"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            '\u{b}' => out.push_str("\\v"),
            c if (c as u32) < 0x20 || c as u32 == 0x7F => {
                out.push_str(&format!("\\x{:02x}", c as u32))
            }
            '\u{FFFD}' => out.push_str("\\ufffd"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
