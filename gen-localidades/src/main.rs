//! regenera `danfse/core/src/domain/localidade/data.rs` a partir do catálogo JSON.
//!
//! Uso: `gen-localidades -input <json> -output <rs>`

use std::collections::HashSet;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::Deserialize;

#[derive(Deserialize)]
struct Estado {
    codigo: i64,
    nome: String,
    sigla: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Municipio {
    codigo: i64,
    nome: String,
    uf_codigo: i64,
}

#[derive(Deserialize)]
struct Payload {
    #[serde(default)]
    estados: Vec<Estado>,
    #[serde(default)]
    municipios: Vec<Municipio>,
}

struct Args {
    input: PathBuf,
    output: PathBuf,
}

fn parse_args() -> Result<Args> {
    let mut input = None;
    let mut output = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.trim_start_matches('-') {
            "input" => input = args.next(),
            "output" => output = args.next(),
            _ => {}
        }
    }
    match (input, output) {
        (Some(i), Some(o)) if !i.is_empty() && !o.is_empty() => Ok(Args {
            input: i.into(),
            output: o.into(),
        }),
        _ => bail!("uso: gen-localidades -input <json> -output <rs>"),
    }
}

fn validar(p: &Payload) -> Result<()> {
    if p.estados.is_empty() {
        bail!("validação: estados vazio");
    }
    if p.municipios.is_empty() {
        bail!("validação: municípios vazio");
    }
    let mut codigos = HashSet::new();
    for e in &p.estados {
        if e.codigo == 0 {
            bail!("validação: estado com código zero");
        }
        if e.nome.is_empty() {
            bail!("validação: estado {} com nome vazio", e.codigo);
        }
        if e.sigla.is_empty() {
            bail!("validação: estado {} com sigla vazia", e.codigo);
        }
        if !codigos.insert(e.codigo) {
            bail!("validação: código de estado duplicado {}", e.codigo);
        }
    }
    for m in &p.municipios {
        if m.codigo == 0 {
            bail!("validação: município com código zero");
        }
        if m.nome.is_empty() {
            bail!("validação: município {} com nome vazio", m.codigo);
        }
        if m.uf_codigo == 0 {
            bail!("validação: município {} com ufCodigo zero", m.codigo);
        }
        if !codigos.contains(&m.uf_codigo) {
            bail!(
                "validação: município {} referencia UF inexistente {}",
                m.codigo,
                m.uf_codigo
            );
        }
    }
    Ok(())
}

fn gerar(p: &Payload) -> Result<String> {
    let mut out = String::new();
    out.push_str("// Código gerado por gen-localidades; NÃO EDITAR.\n\n");
    out.push_str("/// Estados (UFs) ordenados por código: (código, nome, sigla).\n");
    out.push_str("pub(super) static ESTADOS: &[(i64, &str, &str)] = &[\n");
    for e in &p.estados {
        writeln!(out, "    ({}, {:?}, {:?}),", e.codigo, e.nome, e.sigla)?;
    }
    out.push_str("];\n\n");
    out.push_str("/// Municípios ordenados por código: (código, nome, código da UF).\n");
    out.push_str("pub(super) static MUNICIPIOS: &[(i64, &str, i64)] = &[\n");
    for m in &p.municipios {
        writeln!(out, "    ({}, {:?}, {}),", m.codigo, m.nome, m.uf_codigo)?;
    }
    out.push_str("];\n");
    Ok(out)
}

fn escrever_atomico(destino: &Path, conteudo: &str) -> Result<()> {
    let dir = destino
        .parent()
        .filter(|d| !d.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(dir).context("criação do diretório de saída")?;
    let tmp = dir.join(format!(".gen-localidades-{}.rs.tmp", std::process::id()));
    std::fs::write(&tmp, conteudo).context("escrita do arquivo temporário")?;
    if let Err(e) = std::fs::rename(&tmp, destino) {
        let _ = std::fs::remove_file(&tmp);
        return Err(e).context("renomeação do arquivo temporário para a saída");
    }
    Ok(())
}

fn run() -> Result<()> {
    let args = parse_args()?;
    let data = std::fs::read(&args.input).context("leitura da entrada")?;
    let mut payload: Payload = serde_json::from_slice(&data).context("análise do JSON")?;
    validar(&payload)?;
    payload.estados.sort_by_key(|e| e.codigo);
    payload.municipios.sort_by_key(|m| m.codigo);
    escrever_atomico(&args.output, &gerar(&payload)?)?;
    println!(
        "gerado {} ({} estados, {} municípios)",
        args.output.display(),
        payload.estados.len(),
        payload.municipios.len()
    );
    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("{e:#}");
        std::process::exit(1);
    }
}
