use super::ErroDanfse;
use crate::xml::{Decoder, Token};

/// limite padrão da entrada XML (5 MiB)
pub const MAX_XML_INPUT_SIZE: i64 = 5 * 1024 * 1024;

/// teto (32 MiB) para um limite configurável, evitando valores que desliguem a proteção
pub const MAX_XML_INPUT_SIZE_LIMIT: i64 = 32 * 1024 * 1024;

/// percorre todo o XML e rejeita diretivas DOCTYPE ou ENTITY (sem diferenciar
/// maiúsculas). Erros de sintaxe encontrados no caminho são devolvidos
pub fn rejeitar_dtd_e_entidades(xml: &[u8]) -> Result<(), ErroDanfse> {
    let mut d = Decoder::new(xml);
    while let Some(tok) = d.token()? {
        if let Token::Directive(diretiva) = tok {
            let texto = String::from_utf8_lossy(&diretiva).to_uppercase();
            if texto.contains("DOCTYPE") || texto.contains("ENTITY") {
                return Err(ErroDanfse::XmlContemDtd);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aceita_xml_simples() {
        assert_eq!(
            rejeitar_dtd_e_entidades(
                br#"<?xml version="1.0"?><NFSe xmlns="http://x"><a>1</a></NFSe>"#
            ),
            Ok(())
        );
    }

    #[test]
    fn rejeita_doctype() {
        let xml = br#"<?xml version="1.0"?><!DOCTYPE foo [<!ENTITY xxe SYSTEM "file:///etc/passwd">]><NFSe>&xxe;</NFSe>"#;
        assert_eq!(rejeitar_dtd_e_entidades(xml), Err(ErroDanfse::XmlContemDtd));
    }

    #[test]
    fn rejeita_entity_minusculo() {
        let xml = b"<!entity foo \"bar\"><NFSe/>";
        assert_eq!(rejeitar_dtd_e_entidades(xml), Err(ErroDanfse::XmlContemDtd));
    }

    #[test]
    fn rejeita_doctype_simples() {
        assert_eq!(
            rejeitar_dtd_e_entidades(b"<!DOCTYPE NFSe><NFSe/>"),
            Err(ErroDanfse::XmlContemDtd)
        );
    }

    #[test]
    fn mensagem_do_erro() {
        assert_eq!(
            ErroDanfse::XmlContemDtd.to_string(),
            "XML contains DTD or entity declaration"
        );
    }

    #[test]
    fn xml_mal_formado_devolve_erro_de_sintaxe() {
        let err = rejeitar_dtd_e_entidades(b"<NFSe><a></NFSe>")
            .err()
            .map(|e| e.to_string());
        assert_eq!(
            err.as_deref(),
            Some("XML syntax error on line 1: element <a> closed by </NFSe>")
        );
    }
}
