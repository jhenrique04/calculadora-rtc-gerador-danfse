//! catálogo em memória de UFs e municípios (códigos IBGE)

mod data;

use std::collections::HashMap;

use serde::Deserialize;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Estado {
    #[serde(default)]
    pub codigo: i64,
    #[serde(default)]
    pub nome: String,
    #[serde(default)]
    pub sigla: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Municipio {
    #[serde(default)]
    pub codigo: i64,
    #[serde(default)]
    pub nome: String,
    #[serde(default, rename = "ufCodigo")]
    pub uf_codigo: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{0}")]
pub struct ErroCatalogo(String);

/// coleção imutável de estados e municípios
#[derive(Debug, Clone)]
pub struct Localidades {
    estados: HashMap<i64, Estado>,
    municipios: HashMap<i64, Municipio>,
    municipios_por_uf: HashMap<i64, Vec<Municipio>>,
}

impl ErroCatalogo {
    pub fn new(msg: impl Into<String>) -> Self {
        Self(msg.into())
    }
}

fn erro(msg: impl Into<String>) -> ErroCatalogo {
    ErroCatalogo::new(msg)
}

fn construir(
    estados: Vec<Estado>,
    municipios: Vec<Municipio>,
) -> Result<Localidades, ErroCatalogo> {
    if estados.is_empty() {
        return Err(erro("catalogo: catálogo vazio — nenhuma UF encontrada"));
    }
    if municipios.is_empty() {
        return Err(erro(
            "catalogo: catálogo vazio — nenhum município encontrado",
        ));
    }
    let mut c = Localidades {
        estados: HashMap::with_capacity(estados.len()),
        municipios: HashMap::with_capacity(municipios.len()),
        municipios_por_uf: HashMap::new(),
    };
    for e in estados {
        if e.codigo == 0 {
            return Err(erro("catalogo: UF com código vazio"));
        }
        if e.nome.is_empty() {
            return Err(erro(format!("catalogo: UF {} com nome vazio", e.codigo)));
        }
        if e.sigla.is_empty() {
            return Err(erro(format!("catalogo: UF {} com sigla vazia", e.codigo)));
        }
        if c.estados.contains_key(&e.codigo) {
            return Err(erro(format!("catalogo: UF duplicada código {}", e.codigo)));
        }
        c.estados.insert(e.codigo, e);
    }
    for m in municipios {
        if m.codigo == 0 {
            return Err(erro("catalogo: município com código vazio"));
        }
        if m.nome.is_empty() {
            return Err(erro(format!(
                "catalogo: município {} com nome vazio",
                m.codigo
            )));
        }
        if m.uf_codigo == 0 {
            return Err(erro(format!(
                "catalogo: município {} com UF código vazio",
                m.codigo
            )));
        }
        if c.municipios.contains_key(&m.codigo) {
            return Err(erro(format!(
                "catalogo: município duplicado código {}",
                m.codigo
            )));
        }
        if !c.estados.contains_key(&m.uf_codigo) {
            return Err(erro(format!(
                "catalogo: município {} referencia UF inexistente {}",
                m.codigo, m.uf_codigo
            )));
        }
        c.municipios_por_uf
            .entry(m.uf_codigo)
            .or_default()
            .push(m.clone());
        c.municipios.insert(m.codigo, m);
    }
    Ok(c)
}

impl Localidades {
    /// catálogo a partir dos dados embutidos, validado
    pub fn carregar() -> Result<Self, ErroCatalogo> {
        let estados = data::ESTADOS
            .iter()
            .map(|&(codigo, nome, sigla)| Estado {
                codigo,
                nome: nome.into(),
                sigla: sigla.into(),
            })
            .collect();
        let municipios = data::MUNICIPIOS
            .iter()
            .map(|&(codigo, nome, uf_codigo)| Municipio {
                codigo,
                nome: nome.into(),
                uf_codigo,
            })
            .collect();
        construir(estados, municipios)
    }

    /// catálogo a partir de um JSON alternativo, com as mesmas validações de `carregar`
    pub fn decodificar_json(data: &[u8]) -> Result<Self, ErroCatalogo> {
        #[derive(Deserialize)]
        struct Payload {
            #[serde(default)]
            estados: Option<Vec<Estado>>,
            #[serde(default)]
            municipios: Option<Vec<Municipio>>,
        }
        let payload: Payload = serde_json::from_slice(data)
            .map_err(|e| erro(format!("catalogo: JSON inválido: {e}")))?;
        construir(
            payload.estados.unwrap_or_default(),
            payload.municipios.unwrap_or_default(),
        )
    }

    pub fn estado_por_codigo(&self, codigo: i64) -> Option<&Estado> {
        self.estados.get(&codigo)
    }

    pub fn municipio_por_codigo(&self, codigo: i64) -> Option<&Municipio> {
        self.municipios.get(&codigo)
    }

    pub fn municipios_por_uf(&self, uf_codigo: i64) -> &[Municipio] {
        self.municipios_por_uf
            .get(&uf_codigo)
            .map(Vec::as_slice)
            .unwrap_or_default()
    }

    pub fn todos_estados(&self) -> Vec<Estado> {
        self.estados.values().cloned().collect()
    }

    pub fn contagem_estados(&self) -> usize {
        self.estados.len()
    }

    pub fn contagem_municipios(&self) -> usize {
        self.municipios.len()
    }

    pub fn estado_existe(&self, codigo: i64) -> bool {
        self.estados.contains_key(&codigo)
    }

    pub fn municipio_existe(&self, codigo: i64) -> bool {
        self.municipios.contains_key(&codigo)
    }

    pub fn estado_por_sigla(&self, sigla: &str) -> Option<&Estado> {
        self.estados.values().find(|e| e.sigla == sigla)
    }

    pub fn estado_por_nome(&self, nome: &str) -> Option<&Estado> {
        self.estados.values().find(|e| e.nome == nome)
    }

    pub fn municipio_por_nome_e_uf(&self, nome: &str, uf_codigo: i64) -> Option<&Municipio> {
        self.municipios_por_uf(uf_codigo)
            .iter()
            .find(|m| m.nome == nome)
    }

    pub fn estado_por_municipio(&self, municipio_codigo: i64) -> Option<&Estado> {
        let m = self.municipios.get(&municipio_codigo)?;
        self.estados.get(&m.uf_codigo)
    }

    pub fn estado_por_municipio_sigla(&self, municipio_codigo: i64) -> Option<&str> {
        self.estado_por_municipio(municipio_codigo)
            .map(|e| e.sigla.as_str())
    }

    pub fn estado_por_municipio_nome(&self, municipio_codigo: i64) -> Option<&str> {
        self.estado_por_municipio(municipio_codigo)
            .map(|e| e.nome.as_str())
    }

    /// nome e sigla da UF do município; `Ok(None)` quando o código não existe
    pub fn buscar_municipio(&self, codigo: i64) -> Result<Option<(String, String)>, ErroCatalogo> {
        let Some(m) = self.municipios.get(&codigo) else {
            return Ok(None);
        };
        let Some(e) = self.estados.get(&m.uf_codigo) else {
            return Err(uf_inexistente(m.codigo, &m.nome, m.uf_codigo));
        };
        Ok(Some((m.nome.clone(), e.sigla.clone())))
    }
}

fn uf_inexistente(codigo: i64, nome: &str, uf: i64) -> ErroCatalogo {
    erro(format!(
        "município {codigo} ({nome}) referencia UF {uf} inexistente no catálogo"
    ))
}

/// busca binária direta nos dados embutidos (ordenados por código), sem montar o catálogo
pub fn buscar_municipio_gerado(codigo: i64) -> Result<Option<(String, String)>, ErroCatalogo> {
    let Ok(idx) = data::MUNICIPIOS.binary_search_by_key(&codigo, |m| m.0) else {
        return Ok(None);
    };
    let (m_codigo, m_nome, uf) = data::MUNICIPIOS[idx];
    match data::ESTADOS.binary_search_by_key(&uf, |e| e.0) {
        Ok(e) => Ok(Some((m_nome.to_string(), data::ESTADOS[e].2.to_string()))),
        Err(_) => Err(uf_inexistente(m_codigo, m_nome, uf)),
    }
}

#[cfg(test)]
mod tests;
