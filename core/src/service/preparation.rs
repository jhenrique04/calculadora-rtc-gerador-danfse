use super::tax_totals::montar_linha_totais_tributos;
use super::xml_security::{MAX_XML_INPUT_SIZE, MAX_XML_INPUT_SIZE_LIMIT, rejeitar_dtd_e_entidades};
use super::{ErroDanfse, PreparadorDanfse, ResolverMunicipio};
use crate::domain::enums::SituacaoNotaTipo;
use crate::domain::models::NFSe;
use crate::domain::utils_strings::{parse_atoi, sanitizar_ampersand, truncar_com_reticencias};
use crate::xml::{Decoder, XmlError, decode_struct, first_start_element};

const MAX_CARACTERES_DESCRICAO: usize = 1300;
const MAX_CARACTERES_INFO_COMPL: usize = 2000;

/// base oficial fixa da consulta pública da NFS-e, usada em todos os ambientes
const CONSULTA_PUBLICA_BASE_URL: &str = "https://www.nfse.gov.br";

/// analisa e enriquece o modelo NFSe compartilhado por todos os renderizadores
pub struct ServicoPreparacaoDanfse {
    resolver: Option<ResolverMunicipio>,
    // valor explícito inválido é rejeitado em executar, nunca tratado como ilimitado
    max_input_size: Option<i64>,
}

impl ServicoPreparacaoDanfse {
    pub fn new(resolver: Option<ResolverMunicipio>) -> Self {
        Self {
            resolver,
            max_input_size: None,
        }
    }

    pub fn with_max_input_size(resolver: Option<ResolverMunicipio>, max_input_size: i64) -> Self {
        Self {
            resolver,
            max_input_size: Some(max_input_size),
        }
    }

    fn limite_entrada(&self) -> Result<i64, ErroDanfse> {
        match self.max_input_size {
            None => Ok(MAX_XML_INPUT_SIZE),
            Some(v) if v <= 0 || v > MAX_XML_INPUT_SIZE_LIMIT => {
                Err(ErroDanfse::TamanhoMaximoInvalido(v))
            }
            Some(v) => Ok(v),
        }
    }

    fn enriquecer_municipios(&self, nota: &mut NFSe) {
        let inf = &mut nota.inf_nfse;
        if let Some((nome, uf)) = self.resolver_municipio(&inf.c_loc_incid) {
            inf.x_loc_incid = nome;
            inf.x_uf_incid = uf;
        }
        if let Some((nome, uf)) = self.resolver_municipio(&inf.emit.ender_nac.c_mun) {
            inf.emit.ender_nac.x_mun = nome;
            inf.emit.ender_nac.uf = uf;
        }
        let dps = &mut inf.dps.inf_dps;
        if let Some(end_nac) = dps
            .toma
            .as_mut()
            .and_then(|t| t.end.as_mut())
            .and_then(|e| e.end_nac.as_mut())
            && let Some((nome, uf)) = self.resolver_municipio(&end_nac.c_mun)
        {
            end_nac.x_mun = nome;
            end_nac.uf = uf;
        }
        let dest_end_nac = dps
            .ibscbs
            .as_mut()
            .and_then(|i| i.dest.as_mut())
            .and_then(|d| d.end.as_mut())
            .and_then(|e| e.end_nac.as_mut());
        if let Some(end_nac) = dest_end_nac
            && let Some((nome, uf)) = self.resolver_municipio(&end_nac.c_mun)
        {
            end_nac.x_mun = nome;
            end_nac.uf = uf;
        }
        if let Some(end_nac) = dps.interm.as_mut().and_then(|i| i.end.end_nac.as_mut())
            && let Some((nome, uf)) = self.resolver_municipio(&end_nac.c_mun)
        {
            end_nac.x_mun = nome;
            end_nac.uf = uf;
        }
        if let Some(ibscbs) = inf.ibscbs.as_mut()
            && let Some((nome, uf)) = self.resolver_municipio(&ibscbs.c_localidade_incid)
        {
            ibscbs.x_localidade_incid = nome;
            ibscbs.uf = uf;
        }
        let loc_prest = &mut inf.dps.inf_dps.serv.loc_prest;
        if let Some((_, uf)) = self.resolver_municipio(&loc_prest.c_loc_prestacao) {
            loc_prest.uf = uf;
        }
    }

    fn resolver_municipio(&self, codigo: &str) -> Option<(String, String)> {
        if codigo.is_empty() {
            return None;
        }
        let cod = match parse_atoi(codigo) {
            Ok(c) => c,
            Err(e) => {
                log::info!("Código IBGE inválido [{codigo}]: {e}");
                return None;
            }
        };
        let Some(resolver) = &self.resolver else {
            log::info!("Município não encontrado para o código IBGE [{codigo}]");
            return None;
        };
        match resolver(cod) {
            Ok(Some(encontrado)) => Some(encontrado),
            Ok(None) => {
                log::info!("Município não encontrado para o código IBGE [{codigo}]");
                None
            }
            Err(e) => {
                log::info!("Erro ao buscar município pelo código IBGE [{codigo}]: {e}");
                None
            }
        }
    }
}

/// `strconv.Atoi` do Go: inteiro decimal com sinal opcional, sem espaços
fn esta_em_branco(dados: &[u8]) -> bool {
    String::from_utf8_lossy(dados)
        .chars()
        .all(char::is_whitespace)
}

/// `url.QueryEscape` do Go
fn query_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(char::from(b))
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

fn decodificar_nfse(xml: &[u8], nota: &mut NFSe) -> Result<(), XmlError> {
    let mut d = Decoder::new(xml);
    let raiz = first_start_element(&mut d)?;
    if raiz.name.local != "NFSe" {
        return Err(XmlError::Other(format!(
            "expected element type <NFSe> but have <{}>",
            raiz.name.local
        )));
    }
    decode_struct(nota, &mut d, &raiz)
}

impl PreparadorDanfse for ServicoPreparacaoDanfse {
    fn executar(&self, xml: &[u8], situacao: SituacaoNotaTipo) -> Result<NFSe, ErroDanfse> {
        let mut nota = NFSe {
            marca_dagua: situacao.marca_dagua().to_string(),
            ..NFSe::default()
        };

        let limite = self.limite_entrada()?;
        if esta_em_branco(xml) {
            return Err(ErroDanfse::XmlVazio);
        }
        let tamanho = |dados: &[u8]| i64::try_from(dados.len()).unwrap_or(i64::MAX);
        if tamanho(xml) > limite {
            return Err(ErroDanfse::EntradaExcedeTamanho(limite));
        }

        let xml = sanitizar_ampersand(xml);
        if tamanho(&xml) > limite {
            return Err(ErroDanfse::EntradaSanitizadaExcedeTamanho(limite));
        }

        // roda depois do reparo de '&' legados, mantendo as declarações intactas para rejeição
        rejeitar_dtd_e_entidades(&xml)?;
        decodificar_nfse(&xml, &mut nota)?;

        if nota.inf_nfse.id.trim().is_empty() {
            return Err(ErroDanfse::IdVazio);
        }

        nota.inf_nfse.qrcode = format!(
            "{CONSULTA_PUBLICA_BASE_URL}/ConsultaPublica/?tpc=1&chave={}",
            query_escape(&nota.inf_nfse.id.replace("NFS", ""))
        );

        self.enriquecer_municipios(&mut nota);
        calcular_exclusoes_reducoes_bc(&mut nota);
        calcular_linhas(&mut nota);
        Ok(nota)
    }
}

fn calcular_exclusoes_reducoes_bc(nota: &mut NFSe) {
    let valores_dps = &nota.inf_nfse.dps.inf_dps.valores;
    let mut total = valores_dps.v_desc_cond_incond.v_desc_incond
        + nota.inf_nfse.valores.v_issqn
        + nota.inf_nfse.valores.v_calc_dr;
    if let Some(ibscbs) = &nota.inf_nfse.ibscbs {
        total += ibscbs.valores.v_calc_ree_rep_res;
    }
    if let Some(pc) = valores_dps
        .trib
        .trib_fed
        .as_ref()
        .and_then(|t| t.pis_cofins.as_ref())
    {
        total += pc.v_pis + pc.v_cofins;
    }
    nota.v_exclusoes_reducoes_bc = total;
}

fn calcular_linhas(nota: &mut NFSe) {
    let descricao = nota
        .inf_nfse
        .dps
        .inf_dps
        .serv
        .c_serv
        .x_desc_serv
        .trim()
        .to_string();
    let texto_base = montar_texto_compl(nota);
    let linha_tributos = montar_linha_totais_tributos(nota);

    let texto_informacoes = if texto_base.is_empty() {
        linha_tributos.clone()
    } else {
        format!("{texto_base} | {linha_tributos}")
    };

    nota.exibicao_desc_serv = truncar_com_reticencias(&descricao, MAX_CARACTERES_DESCRICAO - 3);
    nota.texto_desc_serv_original = descricao;
    // textos originais preservados antes de qualquer truncamento para a renderização direta
    nota.texto_compl_original = texto_informacoes;
    nota.exibicao_compl = truncar_texto_compl_preservando_totais(
        &texto_base,
        &linha_tributos,
        MAX_CARACTERES_INFO_COMPL,
    );
    nota.exibicao_linha_tributos = linha_tributos;
}

/// regra da NT-008: quando a descrição complementar excede o limite, ela
/// recebe reticências e a linha fixa de totais é anexada integralmente
pub(super) fn truncar_texto_compl_preservando_totais(
    texto_base: &str,
    linha_tributos: &str,
    limite: usize,
) -> String {
    if texto_base.is_empty() {
        return linha_tributos.to_string();
    }
    let limite_descricao = limite.saturating_sub(3);
    let runes: Vec<char> = texto_base.chars().collect();
    let descricao = if runes.len() <= limite_descricao {
        texto_base.to_string()
    } else {
        let prefixo: String = runes[..limite_descricao].iter().collect();
        format!("{}...", prefixo.trim_end_matches([' ', '\n']))
    };
    if linha_tributos.is_empty() {
        return descricao;
    }
    format!("{descricao} | {linha_tributos}")
}

fn montar_texto_compl(nota: &NFSe) -> String {
    let dps = &nota.inf_nfse.dps.inf_dps;
    let serv = &dps.serv;
    let mut insc_imob = serv.obra.insc_imob_fisc.trim().to_string();
    if insc_imob.is_empty()
        && let Some(imovel) = dps.ibscbs.as_ref().and_then(|i| i.imovel.as_ref())
    {
        insc_imob = imovel.insc_imob_fisc.trim().to_string();
    }
    let itens_pedido: Vec<&str> = serv
        .info_compl
        .g_item_ped
        .iter()
        .map(|i| i.x_item_ped.trim())
        .filter(|v| !v.is_empty())
        .collect();
    let itens_pedido = itens_pedido.join(", ");

    let campos: [(&str, &str); 10] = [
        ("Inf. Cont.:", serv.info_compl.x_inf_comp.trim()),
        ("NFS-e Subst.:", dps.subst.ch_substda.trim()),
        ("Doc. Ref.:", serv.info_compl.doc_ref.trim()),
        ("Cod. Obra:", serv.obra.c_obra.trim()),
        ("Insc. Imob.:", &insc_imob),
        ("Cod. Evt.:", serv.atv_evento.id_atv_evt.trim()),
        ("Doc. Tec.:", serv.info_compl.id_doc_tec.trim()),
        ("Núm. Ped.:", serv.info_compl.x_ped.trim()),
        ("Item Ped.:", &itens_pedido),
        ("Inf. A. T. Mun.:", serv.info_compl.x_out_inf.trim()),
    ];
    campos
        .iter()
        .filter(|(_, valor)| !valor.is_empty())
        .map(|(rotulo, valor)| format!("{rotulo} {valor}"))
        .collect::<Vec<_>>()
        .join(" | ")
}

#[cfg(test)]
mod tests;
