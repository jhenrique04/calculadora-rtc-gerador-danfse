//! modelo da NFS-e nacional (subconjunto usado pelo DANFSe)

use crate::xml::{
    Attr, Decoder, FromXml, StartElement, XmlError, decode_f64, decode_i64, decode_string,
    decode_struct,
};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct NFSe {
    pub versao: String,
    pub marca_dagua: String,
    pub exibicao_desc_serv: String,
    pub exibicao_compl: String,
    pub exibicao_linha_tributos: String,
    pub v_exclusoes_reducoes_bc: f64,
    pub texto_desc_serv_original: String,
    pub texto_compl_original: String,
    pub inf_nfse: InfNFSe,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct InfNFSe {
    pub id: String,
    pub situacao: String,
    pub qrcode: String,
    pub dh_proc: String,
    pub n_nfse: String,
    pub amb_ger: String,
    pub c_stat: String,
    pub x_loc_emi: String,
    pub c_loc_incid: String,
    pub x_loc_incid: String,
    pub x_uf_incid: String,
    pub x_pais_incid: String,
    pub x_trib_nac: String,
    pub x_trib_mun: String,
    pub x_loc_prestacao: String,
    pub c_pais_prestacao: String,
    pub dps: Dps,
    pub emit: Emitente,
    pub valores: Valores,
    pub ibscbs: Option<IbsCbs>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Dps {
    pub inf_dps: InfDps,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct InfDps {
    pub id: String,
    pub n_dps: String,
    pub serie: String,
    pub dh_emi: String,
    pub d_compet: String,
    pub c_loc_emi: String,
    pub x_uf_emi: String,
    pub tp_emit: i64,
    pub tp_amb: i64,
    pub prest: Option<Prestador>,
    pub toma: Option<Tomador>,
    pub interm: Option<Interm>,
    pub serv: Servico,
    pub valores: ValoresDps,
    pub subst: Subst,
    pub ibscbs: Option<IbsCbsDps>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Emitente {
    pub cnpj: String,
    pub cpf: String,
    pub nif: String,
    pub c_nao_nif: i64,
    pub im: String,
    pub x_nome: String,
    pub x_fant: String,
    pub fone: String,
    pub email: String,
    pub ender_nac: Endereco,
    pub ender_ext: Option<EndExt>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Tomador {
    pub cnpj: String,
    pub cpf: String,
    pub nif: String,
    pub c_nao_nif: i64,
    pub im: String,
    pub x_nome: String,
    pub fone: String,
    pub email: String,
    pub end: Option<EnderecoTomador>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct EnderecoTomador {
    pub x_lgr: String,
    pub nro: String,
    pub x_cpl: String,
    pub x_bairro: String,
    pub end_nac: Option<EndNac>,
    pub end_ext: Option<EndExt>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct EndNac {
    pub x_lgr: String,
    pub nro: String,
    pub x_cpl: String,
    pub x_bairro: String,
    pub c_mun: String,
    pub x_mun: String,
    pub uf: String,
    pub cep: String,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Interm {
    pub cnpj: String,
    pub cpf: String,
    pub nif: String,
    pub c_nao_nif: i64,
    pub im: String,
    pub x_nome: String,
    pub fone: String,
    pub email: String,
    pub end: EnderecoDps,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Destinatario {
    pub cnpj: String,
    pub cpf: String,
    pub nif: String,
    pub x_nome: String,
    pub fone: String,
    pub email: String,
    pub end: Option<EnderecoTomador>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Endereco {
    pub x_lgr: String,
    pub nro: String,
    pub x_cpl: String,
    pub x_bairro: String,
    pub x_mun: String,
    pub c_mun: String,
    pub uf: String,
    pub cep: String,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct EnderecoDps {
    pub x_lgr: String,
    pub nro: String,
    pub x_cpl: String,
    pub x_bairro: String,
    pub end_nac: Option<Endereco>,
    pub end_ext: Option<EndExt>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct EndExt {
    pub x_cidade: String,
    pub c_pais: String,
    pub c_end_post: String,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Prestador {
    pub reg_trib: RegTrib,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct RegTrib {
    pub op_simp_nac: i64,
    pub reg_ap_trib_sn: i64,
    pub reg_esp_trib: i64,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Servico {
    pub c_serv: CServ,
    pub obra: Obra,
    pub info_compl: InfoCompl,
    pub loc_prest: LocPrest,
    pub atv_evento: AtvEvento,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct LocPrest {
    pub c_loc_prestacao: String,
    pub c_pais_prestacao: String,
    pub uf: String,
    pub pais: String,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct CServ {
    pub c_trib_nac: String,
    pub c_trib_mun: String,
    pub x_desc_serv: String,
    pub c_nbs: String,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Obra {
    pub c_obra: String,
    pub insc_imob_fisc: String,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct InfoCompl {
    pub x_inf_comp: String,
    pub id_doc_tec: String,
    pub doc_ref: String,
    pub x_ped: String,
    pub x_out_inf: String,
    pub g_item_ped: Vec<ItemPed>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ItemPed {
    pub x_item_ped: String,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct AtvEvento {
    pub id_atv_evt: String,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Subst {
    pub ch_substda: String,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Valores {
    pub v_liq: f64,
    pub v_bc: f64,
    pub v_issqn: f64,
    pub p_aliq_aplic: Option<f64>,
    pub v_calc_bm: f64,
    pub tp_bm: i64,
    pub v_calc_dr: f64,
    pub v_total_ret: f64,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ValoresDps {
    pub v_serv_prest: VServPrest,
    pub v_desc_cond_incond: VDescCondIncond,
    pub v_ded_red: VDedRed,
    pub trib: Trib,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct VServPrest {
    pub v_serv: f64,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct VDescCondIncond {
    pub v_desc_cond: f64,
    pub v_desc_incond: f64,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct VDedRed {
    pub p_dr: f64,
    pub v_dr: f64,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Trib {
    pub trib_mun: Option<TribMun>,
    pub trib_fed: Option<TribFed>,
    pub tot_trib: TotTrib,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct TribMun {
    pub trib_issqn: i64,
    pub tp_ret_issqn: i64,
    pub c_pais_result: String,
    pub tp_imunidade: i64,
    pub exig_susp: ExigSusp,
    pub bm: Bm,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ExigSusp {
    pub tp_susp: i64,
    pub n_processo: String,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Bm {
    pub n_bm: String,
    pub v_red_bc_bm: f64,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct TribFed {
    pub v_ret_irrf: f64,
    pub v_ret_csll: f64,
    pub v_ret_cp: f64,
    pub pis_cofins: Option<PisCofins>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct PisCofins {
    pub v_bc_pis_cofins: f64,
    pub v_cofins: f64,
    pub v_pis: f64,
    pub tp_ret_pis_cofins: i64,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct TotTrib {
    pub v_tot_trib: VTotTrib,
    pub p_tot_trib: PTotTrib,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct VTotTrib {
    pub v_tot_trib_fed: f64,
    pub v_tot_trib_est: f64,
    pub v_tot_trib_mun: f64,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct PTotTrib {
    pub p_tot_trib_fed: f64,
    pub p_tot_trib_est: f64,
    pub p_tot_trib_mun: f64,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct IbsCbsDps {
    pub fin_nfse: Option<i64>,
    pub c_ind_op: String,
    pub imovel: Option<Imovel>,
    pub dest: Option<Destinatario>,
    pub valores: ValoresIbsCbs,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct IbsCbs {
    pub x_localidade_incid: String,
    pub uf: String,
    pub c_localidade_incid: String,
    pub valores: ValoresIbsCbsResp,
    pub tot_cibs: TotCibs,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ValoresIbsCbs {
    pub trib: TribIbsCbs,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct TribIbsCbs {
    pub g_ibs_cbs: GIbsCbs,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct GIbsCbs {
    pub cst: String,
    pub c_class_trib: String,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ValoresIbsCbsResp {
    pub v_bc: f64,
    pub v_calc_ree_rep_res: f64,
    pub uf: AliqIbsUf,
    pub mun: AliqIbsMun,
    pub fed: AliqCbs,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct AliqIbsUf {
    pub p_ibs_uf: Option<f64>,
    pub p_aliq_efet_uf: Option<f64>,
    pub p_red_aliq_uf: Option<f64>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct AliqIbsMun {
    pub p_ibs_mun: Option<f64>,
    pub p_aliq_efet_mun: Option<f64>,
    pub p_red_aliq_mun: Option<f64>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct AliqCbs {
    pub p_cbs: Option<f64>,
    pub p_aliq_efet_cbs: Option<f64>,
    pub p_red_aliq_cbs: Option<f64>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct TotCibs {
    pub g_ibs: GIbsTot,
    pub g_cbs: GCbsTot,
    pub v_tot_nf: f64,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct GIbsTot {
    pub v_ibs_tot: f64,
    pub g_ibs_mun_tot: GIbsMunTot,
    pub g_ibs_uf_tot: GIbsUfTot,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct GIbsMunTot {
    pub v_ibs_mun: Option<f64>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct GIbsUfTot {
    pub v_ibs_uf: Option<f64>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct GCbsTot {
    pub v_cbs: f64,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Imovel {
    pub insc_imob_fisc: String,
}

/// mapeia cada nome de elemento filho para o campo correspondente.
///
/// `str` → texto, `int`/`float` → número (vazio = 0), `opt_float`/`opt_int`
/// → número presente, `elem` → estrutura aninhada (mesclada),
/// `opt_elem` → estrutura criada na primeira ocorrência e `vec` → item novo
/// por ocorrência
macro_rules! from_xml {
    ($ty:ty { $($tag:literal => $kind:ident $field:ident),* $(,)? }) => {
        impl FromXml for $ty {
            fn child(&mut self, d: &mut Decoder<'_>, start: &StartElement) -> Result<bool, XmlError> {
                match start.name.local.as_str() {
                    $($tag => from_xml!(@set $kind, self.$field, d, start),)*
                    _ => return Ok(false),
                }
                Ok(true)
            }
        }
    };
    (@set str, $f:expr, $d:ident, $s:ident) => { $f = decode_string($d)? };
    (@set int, $f:expr, $d:ident, $s:ident) => { $f = decode_i64($d)? };
    (@set float, $f:expr, $d:ident, $s:ident) => { $f = decode_f64($d)? };
    (@set opt_int, $f:expr, $d:ident, $s:ident) => { $f = Some(decode_i64($d)?) };
    (@set opt_float, $f:expr, $d:ident, $s:ident) => { $f = Some(decode_f64($d)?) };
    (@set elem, $f:expr, $d:ident, $s:ident) => { decode_struct(&mut $f, $d, $s)? };
    (@set opt_elem, $f:expr, $d:ident, $s:ident) => {
        decode_struct($f.get_or_insert_with(Default::default), $d, $s)?
    };
    (@set vec, $f:expr, $d:ident, $s:ident) => {{
        let mut item = Default::default();
        decode_struct(&mut item, $d, $s)?;
        $f.push(item);
    }};
}

impl FromXml for NFSe {
    fn attr(&mut self, attr: &Attr) -> Result<(), XmlError> {
        if attr.name.local == "versao" {
            self.versao = attr.value.clone();
        }
        Ok(())
    }

    fn child(&mut self, d: &mut Decoder<'_>, start: &StartElement) -> Result<bool, XmlError> {
        // campos sem tag no modelo Go também são casados pelo nome do campo
        match start.name.local.as_str() {
            "infNFSe" => decode_struct(&mut self.inf_nfse, d, start)?,
            "MarcaDagua" => self.marca_dagua = decode_string(d)?,
            "ExibicaoDescServ" => self.exibicao_desc_serv = decode_string(d)?,
            "ExibicaoCompl" => self.exibicao_compl = decode_string(d)?,
            "ExibicaoLinhaTributos" => self.exibicao_linha_tributos = decode_string(d)?,
            "VExclusoesReducoesBC" => self.v_exclusoes_reducoes_bc = decode_f64(d)?,
            "TextoDescServOriginal" => self.texto_desc_serv_original = decode_string(d)?,
            "TextoComplOriginal" => self.texto_compl_original = decode_string(d)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

impl FromXml for InfNFSe {
    fn attr(&mut self, attr: &Attr) -> Result<(), XmlError> {
        match attr.name.local.as_str() {
            "Id" => self.id = attr.value.clone(),
            "situacao" => self.situacao = attr.value.clone(),
            "qrcode" => self.qrcode = attr.value.clone(),
            _ => {}
        }
        Ok(())
    }

    fn child(&mut self, d: &mut Decoder<'_>, start: &StartElement) -> Result<bool, XmlError> {
        match start.name.local.as_str() {
            "dhProc" => self.dh_proc = decode_string(d)?,
            "nNFSe" => self.n_nfse = decode_string(d)?,
            "ambGer" => self.amb_ger = decode_string(d)?,
            "cStat" => self.c_stat = decode_string(d)?,
            "xLocEmi" => self.x_loc_emi = decode_string(d)?,
            "cLocIncid" => self.c_loc_incid = decode_string(d)?,
            "xLocIncid" => self.x_loc_incid = decode_string(d)?,
            "xUFIncid" => self.x_uf_incid = decode_string(d)?,
            "xPaisIncid" => self.x_pais_incid = decode_string(d)?,
            "xTribNac" => self.x_trib_nac = decode_string(d)?,
            "xTribMun" => self.x_trib_mun = decode_string(d)?,
            "xLocPrestacao" => self.x_loc_prestacao = decode_string(d)?,
            "cPaisPrestacao" => self.c_pais_prestacao = decode_string(d)?,
            "DPS" => decode_struct(&mut self.dps, d, start)?,
            "emit" => decode_struct(&mut self.emit, d, start)?,
            "valores" => decode_struct(&mut self.valores, d, start)?,
            "IBSCBS" => decode_struct(self.ibscbs.get_or_insert_with(Default::default), d, start)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

from_xml!(Dps { "infDPS" => elem inf_dps });

impl FromXml for InfDps {
    fn attr(&mut self, attr: &Attr) -> Result<(), XmlError> {
        if attr.name.local == "Id" {
            self.id = attr.value.clone();
        }
        Ok(())
    }

    fn child(&mut self, d: &mut Decoder<'_>, start: &StartElement) -> Result<bool, XmlError> {
        match start.name.local.as_str() {
            "nDPS" => self.n_dps = decode_string(d)?,
            "serie" => self.serie = decode_string(d)?,
            "dhEmi" => self.dh_emi = decode_string(d)?,
            "dCompet" => self.d_compet = decode_string(d)?,
            "cLocEmi" => self.c_loc_emi = decode_string(d)?,
            "xUFEmi" => self.x_uf_emi = decode_string(d)?,
            "tpEmit" => self.tp_emit = decode_i64(d)?,
            "tpAmb" => self.tp_amb = decode_i64(d)?,
            "prest" => decode_struct(self.prest.get_or_insert_with(Default::default), d, start)?,
            "toma" => decode_struct(self.toma.get_or_insert_with(Default::default), d, start)?,
            "interm" => decode_struct(self.interm.get_or_insert_with(Default::default), d, start)?,
            "serv" => decode_struct(&mut self.serv, d, start)?,
            "valores" => decode_struct(&mut self.valores, d, start)?,
            "subst" => decode_struct(&mut self.subst, d, start)?,
            "IBSCBS" => decode_struct(self.ibscbs.get_or_insert_with(Default::default), d, start)?,
            _ => return Ok(false),
        }
        Ok(true)
    }
}

from_xml!(Emitente {
    "CNPJ" => str cnpj,
    "CPF" => str cpf,
    "NIF" => str nif,
    "cNaoNIF" => int c_nao_nif,
    "IM" => str im,
    "xNome" => str x_nome,
    "xFant" => str x_fant,
    "fone" => str fone,
    "email" => str email,
    "enderNac" => elem ender_nac,
    "enderExt" => opt_elem ender_ext,
});

from_xml!(Tomador {
    "CNPJ" => str cnpj,
    "CPF" => str cpf,
    "NIF" => str nif,
    "cNaoNIF" => int c_nao_nif,
    "IM" => str im,
    "xNome" => str x_nome,
    "fone" => str fone,
    "email" => str email,
    "end" => opt_elem end,
});

from_xml!(EnderecoTomador {
    "xLgr" => str x_lgr,
    "nro" => str nro,
    "xCpl" => str x_cpl,
    "xBairro" => str x_bairro,
    "endNac" => opt_elem end_nac,
    "endExt" => opt_elem end_ext,
});

from_xml!(EndNac {
    "xLgr" => str x_lgr,
    "nro" => str nro,
    "xCpl" => str x_cpl,
    "xBairro" => str x_bairro,
    "cMun" => str c_mun,
    "xMun" => str x_mun,
    "UF" => str uf,
    "CEP" => str cep,
});

from_xml!(Interm {
    "CNPJ" => str cnpj,
    "CPF" => str cpf,
    "NIF" => str nif,
    "cNaoNIF" => int c_nao_nif,
    "IM" => str im,
    "xNome" => str x_nome,
    "fone" => str fone,
    "email" => str email,
    "end" => elem end,
});

from_xml!(Destinatario {
    "CNPJ" => str cnpj,
    "CPF" => str cpf,
    "NIF" => str nif,
    "xNome" => str x_nome,
    "fone" => str fone,
    "email" => str email,
    "end" => opt_elem end,
});

from_xml!(Endereco {
    "xLgr" => str x_lgr,
    "nro" => str nro,
    "xCpl" => str x_cpl,
    "xBairro" => str x_bairro,
    "xMun" => str x_mun,
    "cMun" => str c_mun,
    "UF" => str uf,
    "CEP" => str cep,
});

from_xml!(EnderecoDps {
    "xLgr" => str x_lgr,
    "nro" => str nro,
    "xCpl" => str x_cpl,
    "xBairro" => str x_bairro,
    "endNac" => opt_elem end_nac,
    "endExt" => opt_elem end_ext,
});

from_xml!(EndExt {
    "xCidade" => str x_cidade,
    "cPais" => str c_pais,
    "cEndPost" => str c_end_post,
});

from_xml!(Prestador { "regTrib" => elem reg_trib });

from_xml!(RegTrib {
    "opSimpNac" => int op_simp_nac,
    "regApTribSN" => int reg_ap_trib_sn,
    "regEspTrib" => int reg_esp_trib,
});

from_xml!(Servico {
    "cServ" => elem c_serv,
    "obra" => elem obra,
    "infoCompl" => elem info_compl,
    "locPrest" => elem loc_prest,
    "atvEvento" => elem atv_evento,
});

from_xml!(LocPrest {
    "cLocPrestacao" => str c_loc_prestacao,
    "cPaisPrestacao" => str c_pais_prestacao,
    "UF" => str uf,
    "pais" => str pais,
});

from_xml!(CServ {
    "cTribNac" => str c_trib_nac,
    "cTribMun" => str c_trib_mun,
    "xDescServ" => str x_desc_serv,
    "cNBS" => str c_nbs,
});

from_xml!(Obra {
    "cObra" => str c_obra,
    "inscImobFisc" => str insc_imob_fisc,
});

from_xml!(InfoCompl {
    "xInfComp" => str x_inf_comp,
    "idDocTec" => str id_doc_tec,
    "docRef" => str doc_ref,
    "xPed" => str x_ped,
    "xOutInf" => str x_out_inf,
    "gItemPed" => vec g_item_ped,
});

from_xml!(ItemPed { "xItemPed" => str x_item_ped });
from_xml!(AtvEvento { "idAtvEvt" => str id_atv_evt });
from_xml!(Subst { "chSubstda" => str ch_substda });

from_xml!(Valores {
    "vLiq" => float v_liq,
    "vBC" => float v_bc,
    "vISSQN" => float v_issqn,
    "pAliqAplic" => opt_float p_aliq_aplic,
    "vCalcBM" => float v_calc_bm,
    "tpBM" => int tp_bm,
    "vCalcDR" => float v_calc_dr,
    "vTotalRet" => float v_total_ret,
});

from_xml!(ValoresDps {
    "vServPrest" => elem v_serv_prest,
    "vDescCondIncond" => elem v_desc_cond_incond,
    "vDedRed" => elem v_ded_red,
    "trib" => elem trib,
});

from_xml!(VServPrest { "vServ" => float v_serv });

from_xml!(VDescCondIncond {
    "vDescCond" => float v_desc_cond,
    "vDescIncond" => float v_desc_incond,
});

from_xml!(VDedRed {
    "pDR" => float p_dr,
    "vDR" => float v_dr,
});

from_xml!(Trib {
    "tribMun" => opt_elem trib_mun,
    "tribFed" => opt_elem trib_fed,
    "totTrib" => elem tot_trib,
});

from_xml!(TribMun {
    "tribISSQN" => int trib_issqn,
    "tpRetISSQN" => int tp_ret_issqn,
    "cPaisResult" => str c_pais_result,
    "tpImunidade" => int tp_imunidade,
    "exigSusp" => elem exig_susp,
    "BM" => elem bm,
});

from_xml!(ExigSusp {
    "tpSusp" => int tp_susp,
    "nProcesso" => str n_processo,
});

from_xml!(Bm {
    "nBM" => str n_bm,
    "vRedBCBM" => float v_red_bc_bm,
});

from_xml!(TribFed {
    "vRetIRRF" => float v_ret_irrf,
    "vRetCSLL" => float v_ret_csll,
    "vRetCP" => float v_ret_cp,
    "piscofins" => opt_elem pis_cofins,
});

from_xml!(PisCofins {
    "vBCPisCofins" => float v_bc_pis_cofins,
    "vCofins" => float v_cofins,
    "vPis" => float v_pis,
    "tpRetPisCofins" => int tp_ret_pis_cofins,
});

from_xml!(TotTrib {
    "vTotTrib" => elem v_tot_trib,
    "pTotTrib" => elem p_tot_trib,
});

from_xml!(VTotTrib {
    "vTotTribFed" => float v_tot_trib_fed,
    "vTotTribEst" => float v_tot_trib_est,
    "vTotTribMun" => float v_tot_trib_mun,
});

from_xml!(PTotTrib {
    "pTotTribFed" => float p_tot_trib_fed,
    "pTotTribEst" => float p_tot_trib_est,
    "pTotTribMun" => float p_tot_trib_mun,
});

from_xml!(IbsCbsDps {
    "finNFSe" => opt_int fin_nfse,
    "cIndOp" => str c_ind_op,
    "imovel" => opt_elem imovel,
    "dest" => opt_elem dest,
    "valores" => elem valores,
});

from_xml!(IbsCbs {
    "xLocalidadeIncid" => str x_localidade_incid,
    "uf" => str uf,
    "cLocalidadeIncid" => str c_localidade_incid,
    "valores" => elem valores,
    "totCIBS" => elem tot_cibs,
});

from_xml!(ValoresIbsCbs { "trib" => elem trib });
from_xml!(TribIbsCbs { "gIBSCBS" => elem g_ibs_cbs });

from_xml!(GIbsCbs {
    "CST" => str cst,
    "cClassTrib" => str c_class_trib,
});

from_xml!(ValoresIbsCbsResp {
    "vBC" => float v_bc,
    "vCalcReeRepRes" => float v_calc_ree_rep_res,
    "uf" => elem uf,
    "mun" => elem mun,
    "fed" => elem fed,
});

from_xml!(AliqIbsUf {
    "pIBSUF" => opt_float p_ibs_uf,
    "pAliqEfetUF" => opt_float p_aliq_efet_uf,
    "pRedAliqUF" => opt_float p_red_aliq_uf,
});

from_xml!(AliqIbsMun {
    "pIBSMun" => opt_float p_ibs_mun,
    "pAliqEfetMun" => opt_float p_aliq_efet_mun,
    "pRedAliqMun" => opt_float p_red_aliq_mun,
});

from_xml!(AliqCbs {
    "pCBS" => opt_float p_cbs,
    "pAliqEfetCBS" => opt_float p_aliq_efet_cbs,
    "pRedAliqCBS" => opt_float p_red_aliq_cbs,
});

from_xml!(TotCibs {
    "gIBS" => elem g_ibs,
    "gCBS" => elem g_cbs,
    "vTotNF" => float v_tot_nf,
});

from_xml!(GIbsTot {
    "vIBSTot" => float v_ibs_tot,
    "gIBSMunTot" => elem g_ibs_mun_tot,
    "gIBSUFTot" => elem g_ibs_uf_tot,
});

from_xml!(GIbsMunTot { "vIBSMun" => opt_float v_ibs_mun });
from_xml!(GIbsUfTot { "vIBSUF" => opt_float v_ibs_uf });
from_xml!(GCbsTot { "vCBS" => float v_cbs });
from_xml!(Imovel { "inscImobFisc" => str insc_imob_fisc });
