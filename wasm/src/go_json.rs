//! parser JSON mínimo com as mesmas mensagens de erro do `encoding/json` do
//! Go (toolchain 1.27), para o contrato do WASM ser idêntico ao do runtime Go

use danfse_core::xml::go_quote;

pub const EOF: &str = "unexpected end of JSON input";

#[derive(Debug, Clone, PartialEq)]
pub enum Valor {
    Null,
    Bool(bool),
    /// literal numérico cru, como aparece no texto
    Numero(String),
    Texto(String),
    Lista,
    Objeto(Vec<(String, Valor)>),
}

impl Valor {
    /// nome do tipo JSON usado nas mensagens `cannot unmarshal`
    pub fn tipo(&self) -> &'static str {
        match self {
            Self::Null => "null",
            Self::Bool(_) => "bool",
            Self::Numero(_) => "number",
            Self::Texto(_) => "string",
            Self::Lista => "array",
            Self::Objeto(_) => "object",
        }
    }
}

#[derive(Clone, Copy)]
enum Contexto {
    InicioValor,
    InicioChave,
    AposChave,
    AposPar,
    AposElemento,
    AposTopo,
    Numero,
}

impl Contexto {
    fn descricao(self) -> &'static str {
        match self {
            Self::InicioValor => "looking for beginning of value",
            Self::InicioChave => "looking for beginning of object key string",
            Self::AposChave => "after object key",
            Self::AposPar => "after object key:value pair",
            Self::AposElemento => "after array element",
            Self::AposTopo => "after top-level value",
            Self::Numero => "in numeric literal",
        }
    }
}

/// valida o documento inteiro e devolve o valor de topo
pub fn parse(texto: &str) -> Result<Valor, String> {
    let mut p = Parser {
        chars: texto.char_indices().peekable(),
        texto,
    };
    let valor = p.valor()?;
    p.espacos();
    match p.chars.peek() {
        None => Ok(valor),
        Some(&(_, c)) => Err(caractere_invalido(c, Contexto::AposTopo.descricao())),
    }
}

/// `quoteChar` do scanner Go: aspas simples em volta do caractere escapado
fn citar(c: char) -> String {
    match c {
        '\'' => r"'\''".into(),
        '"' => "'\"'".into(),
        _ => {
            let q = go_quote(&c.to_string());
            format!("'{}'", &q[1..q.len() - 1])
        }
    }
}

fn caractere_invalido(c: char, contexto: &str) -> String {
    format!("invalid character {} {contexto}", citar(c))
}

struct Parser<'a> {
    chars: std::iter::Peekable<std::str::CharIndices<'a>>,
    texto: &'a str,
}

impl Parser<'_> {
    fn espacos(&mut self) {
        while self
            .chars
            .next_if(|&(_, c)| matches!(c, ' ' | '\t' | '\n' | '\r'))
            .is_some()
        {}
    }

    fn proximo(&mut self) -> Result<(usize, char), String> {
        self.chars.next().ok_or_else(|| EOF.to_string())
    }

    fn espiar(&mut self) -> Result<char, String> {
        self.chars
            .peek()
            .map(|&(_, c)| c)
            .ok_or_else(|| EOF.to_string())
    }

    fn valor(&mut self) -> Result<Valor, String> {
        self.espacos();
        match self.espiar()? {
            '{' => self.objeto(),
            '[' => self.lista(),
            '"' => self.texto().map(Valor::Texto),
            't' => self.literal("true").map(|()| Valor::Bool(true)),
            'f' => self.literal("false").map(|()| Valor::Bool(false)),
            'n' => self.literal("null").map(|()| Valor::Null),
            '-' | '0'..='9' => self.numero().map(Valor::Numero),
            c => Err(caractere_invalido(c, Contexto::InicioValor.descricao())),
        }
    }

    fn objeto(&mut self) -> Result<Valor, String> {
        self.proximo()?;
        let mut campos = Vec::new();
        self.espacos();
        if self.chars.next_if(|&(_, c)| c == '}').is_some() {
            return Ok(Valor::Objeto(campos));
        }
        loop {
            self.espacos();
            match self.espiar()? {
                '"' => {}
                c => return Err(caractere_invalido(c, Contexto::InicioChave.descricao())),
            }
            let chave = self.texto()?;
            self.espacos();
            self.esperar(':', Contexto::AposChave)?;
            let valor = self.valor()?;
            campos.push((chave, valor));
            self.espacos();
            match self.proximo()?.1 {
                ',' => {}
                '}' => return Ok(Valor::Objeto(campos)),
                c => return Err(caractere_invalido(c, Contexto::AposPar.descricao())),
            }
        }
    }

    fn lista(&mut self) -> Result<Valor, String> {
        self.proximo()?;
        self.espacos();
        if self.chars.next_if(|&(_, c)| c == ']').is_some() {
            return Ok(Valor::Lista);
        }
        loop {
            self.valor()?;
            self.espacos();
            match self.proximo()?.1 {
                ',' => {}
                ']' => return Ok(Valor::Lista),
                c => return Err(caractere_invalido(c, Contexto::AposElemento.descricao())),
            }
        }
    }

    fn esperar(&mut self, esperado: char, contexto: Contexto) -> Result<(), String> {
        match self.proximo()?.1 {
            c if c == esperado => Ok(()),
            c => Err(caractere_invalido(c, contexto.descricao())),
        }
    }

    fn literal(&mut self, palavra: &'static str) -> Result<(), String> {
        for esperado in palavra.chars() {
            let c = self.proximo()?.1;
            if c != esperado {
                let contexto = format!("in literal {palavra} (expecting {})", citar(esperado));
                return Err(caractere_invalido(c, &contexto));
            }
        }
        Ok(())
    }

    fn digitos(&mut self, obrigatorio: bool) -> Result<(), String> {
        if obrigatorio && !self.espiar()?.is_ascii_digit() {
            return Err(caractere_invalido(
                self.espiar()?,
                Contexto::Numero.descricao(),
            ));
        }
        while self.chars.next_if(|&(_, c)| c.is_ascii_digit()).is_some() {}
        Ok(())
    }

    fn numero(&mut self) -> Result<String, String> {
        let inicio = self.chars.peek().map_or(self.texto.len(), |&(i, _)| i);
        self.chars.next_if(|&(_, c)| c == '-');
        // zero à esquerda encerra a parte inteira, como na gramática JSON
        if self.chars.next_if(|&(_, c)| c == '0').is_none() {
            self.digitos(true)?;
        }
        if self.chars.next_if(|&(_, c)| c == '.').is_some() {
            self.digitos(true)?;
        }
        if self.chars.next_if(|&(_, c)| c == 'e' || c == 'E').is_some() {
            self.chars.next_if(|&(_, c)| c == '+' || c == '-');
            self.digitos(true)?;
        }
        let fim = self.chars.peek().map_or(self.texto.len(), |&(i, _)| i);
        Ok(self.texto[inicio..fim].to_string())
    }

    fn texto(&mut self) -> Result<String, String> {
        self.proximo()?;
        let mut out = String::new();
        loop {
            let (i, c) = self.proximo()?;
            match c {
                '"' => return Ok(out),
                '\\' => self.escape(i, &mut out)?,
                c if c < ' ' => return Err(format!("invalid character {} in string", citar(c))),
                c => out.push(c),
            }
        }
    }

    fn escape(&mut self, inicio: usize, out: &mut String) -> Result<(), String> {
        let c = self.proximo()?.1;
        let simples = match c {
            '"' => Some('"'),
            '\\' => Some('\\'),
            '/' => Some('/'),
            'b' => Some('\u{8}'),
            'f' => Some('\u{c}'),
            'n' => Some('\n'),
            'r' => Some('\r'),
            't' => Some('\t'),
            'u' => None,
            _ => return Err(self.escape_invalido(inicio, 2)),
        };
        match simples {
            Some(s) => out.push(s),
            None => {
                let alto = self.hex4(inicio)?;
                out.push(self.completar_surrogate(alto)?);
            }
        }
        Ok(())
    }

    fn completar_surrogate(&mut self, alto: u32) -> Result<char, String> {
        if !(0xD800..0xDC00).contains(&alto) {
            return Ok(char::from_u32(alto).unwrap_or(char::REPLACEMENT_CHARACTER));
        }
        // par só se o próximo escape for \u de surrogate baixo
        let resto = self.chars.peek().map_or("", |&(i, _)| &self.texto[i..]);
        let baixo = resto
            .strip_prefix("\\u")
            .and_then(|r| r.get(..4))
            .and_then(|h| u32::from_str_radix(h, 16).ok())
            .filter(|b| (0xDC00..0xE000).contains(b));
        let Some(baixo) = baixo else {
            return Ok(char::REPLACEMENT_CHARACTER);
        };
        for _ in 0..6 {
            self.chars.next();
        }
        let cp = 0x10000 + ((alto - 0xD800) << 10) + (baixo - 0xDC00);
        Ok(char::from_u32(cp).unwrap_or(char::REPLACEMENT_CHARACTER))
    }

    fn hex4(&mut self, inicio: usize) -> Result<u32, String> {
        let mut v = 0;
        for _ in 0..4 {
            let c = self.proximo()?.1;
            match c.to_digit(16) {
                Some(d) => v = v * 16 + d,
                None => return Err(self.escape_invalido(inicio, 6)),
            }
        }
        Ok(v)
    }

    fn escape_invalido(&self, inicio: usize, tamanho: usize) -> String {
        let trecho: String = self.texto[inicio..].chars().take(tamanho).collect();
        format!("invalid escape sequence `{trecho}` in string")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mensagens_de_sintaxe() {
        let casos = [
            ("", EOF),
            ("{", EOF),
            ("x", "invalid character 'x' looking for beginning of value"),
            (
                "'x'",
                r"invalid character '\'' looking for beginning of value",
            ),
            (
                "\u{1}",
                r"invalid character '\x01' looking for beginning of value",
            ),
            ("é", "invalid character 'é' looking for beginning of value"),
            (
                r#"{"a":1} x"#,
                "invalid character 'x' after top-level value",
            ),
            (
                r#"{"a":1,}"#,
                "invalid character '}' looking for beginning of object key string",
            ),
            (r#"{"a" 1}"#, "invalid character '1' after object key"),
            (
                r#"{"a":"x" "b"}"#,
                "invalid character '\"' after object key:value pair",
            ),
            ("[1 2]", "invalid character '2' after array element"),
            (r#"{"a":"\q"}"#, r"invalid escape sequence `\q` in string"),
            (
                r#"{"a":"\u12"}"#,
                r#"invalid escape sequence `\u12"}` in string"#,
            ),
            ("{\"a\":\"x\ny\"}", r"invalid character '\n' in string"),
            (
                r#"{"a":01}"#,
                "invalid character '1' after object key:value pair",
            ),
            (r#"{"a":-}"#, "invalid character '}' in numeric literal"),
            (r#"{"a":1.}"#, "invalid character '}' in numeric literal"),
            (r#"{"a":1e}"#, "invalid character '}' in numeric literal"),
            (
                r#"{"a":tru}"#,
                "invalid character '}' in literal true (expecting 'e')",
            ),
            (
                r#"{"a":tx}"#,
                "invalid character 'x' in literal true (expecting 'r')",
            ),
            (
                r#"{"a":nul}"#,
                "invalid character '}' in literal null (expecting 'l')",
            ),
        ];
        for (entrada, esperado) in casos {
            assert_eq!(parse(entrada), Err(esperado.to_string()), "{entrada:?}");
        }
    }

    #[test]
    fn valores_e_escapes() {
        let v = parse(r#" {"x":"é😀\ud800","n":-0.5e+3,"l":[1,{"c":null}],"b":false} "#);
        assert_eq!(
            v,
            Ok(Valor::Objeto(vec![
                ("x".into(), Valor::Texto("é😀\u{FFFD}".into())),
                ("n".into(), Valor::Numero("-0.5e+3".into())),
                ("l".into(), Valor::Lista),
                ("b".into(), Valor::Bool(false)),
            ]))
        );
        assert_eq!(parse("null"), Ok(Valor::Null));
    }
}
