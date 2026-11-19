//! contrato puro de apresentação do PDF do DANFSe; não referencia o modelo
//! da NFS-e nem o gerador de PDF

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Campo {
    pub rotulo: String,
    pub valor: String,
    pub sombreado: bool,
}

/// bloco de pessoa (Prestador, Tomador, Destinatário, Intermediário)
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SecaoParte {
    pub titulo: String,
    pub cnpj_cpf_nif: String,
    pub inscricao_municipal: String,
    pub telefone: String,
    pub nome: String,
    pub municipio_uf: String,
    pub codigo_ibge_cep: String,
    pub endereco: String,
    pub email: String,
    pub visivel: bool,
    pub mensagem_supressao: String,
    pub simples_nacional: String,
    pub regime_apuracao_sn: String,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Secao {
    pub titulo: String,
    pub altura_cm: f64,
    pub campos: Vec<Campo>,
    pub visivel: bool,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct DadosCabecalho {
    pub marca_dagua: String,
    pub exibicao_desc_serv: String,
    pub exibicao_compl: String,
    pub exibicao_linha_tributos: String,
    pub municipio: String,
    pub uf: String,
    pub ambiente_gerador: String,
    pub tipo_ambiente: String,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct DadosAcesso {
    pub chave_acesso: String,
    pub codigo_qr: String,
    pub numero_nfse: String,
    pub competencia: String,
    pub data_hora_emissao_nfse: String,
    pub numero_dps: String,
    pub serie_dps: String,
    pub data_hora_emissao_dps: String,
    pub tipo_emitente: String,
    pub situacao: String,
    pub finalidade: String,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct DadosPartes {
    pub emitente: SecaoParte,
    pub tomador: SecaoParte,
    pub destinatario: SecaoParte,
    pub intermediario: SecaoParte,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct DadosServico {
    pub codigo_trib_nac_mun: String,
    pub codigo_nbs: String,
    pub local_prestacao_uf: String,
    pub x_trib_nac: String,
    pub desc_serv: String,
    pub desc_serv_original: String,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct DadosTributacaoMunicipal {
    pub trib_issqn: String,
    pub municipio_uf_pais_incid: String,
    pub reg_esp_trib: String,
    pub tp_imunidade: String,
    pub suspensao_exig: String,
    pub n_processo_susp: String,
    pub beneficio_municipal: String,
    pub calculo_bm: String,
    pub total_deducoes: String,
    pub desconto_incond: String,
    pub bc_issqn: String,
    pub aliq_aplic: String,
    pub ret_issqn: String,
    pub issqn_apurado: String,
    pub visivel: bool,
    pub mensagem_supressao: String,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct DadosTributacaoFederal {
    pub irrf: String,
    pub contrib_previd_ret: String,
    pub contrib_soc_ret: String,
    pub pis_debito: String,
    pub cofins_debito: String,
    pub desc_contrib_ret: String,
    pub exibir_pis_cofins: bool,
    pub visivel: bool,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct DadosTributacaoIbsCbs {
    pub cst_c_class_trib: String,
    pub ind_op_cod_ibge_localidade_uf: String,
    pub exclusoes_reducoes_bc: String,
    pub bc_pos_exclusoes: String,
    pub red_aliq_ibs_red_aliq_cbs: String,
    pub aliq_ibs_uf_ibs_mun: String,
    pub aliq_efet_mun_ibs: String,
    pub valor_mun_ibs: String,
    pub aliq_efet_est_ibs: String,
    pub valor_est_ibs: String,
    pub valor_total_ibs: String,
    pub aliq_cbs: String,
    pub aliq_efet_cbs: String,
    pub valor_total_cbs: String,
    pub visivel: bool,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct DadosTotais {
    pub valor_operacao_servico: String,
    pub desconto_incond: String,
    pub desconto_cond: String,
    pub total_retencoes: String,
    pub valor_liquido: String,
    pub total_ibs_cbs: String,
    pub valor_liquido_ibs_cbs: String,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct DadosInformacoesComplementares {
    pub conteudo: String,
    pub conteudo_original: String,
    pub linha_tributos: String,
    pub visivel: bool,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct DadosCanhoto {
    pub visivel: bool,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct MarcaDagua {
    pub texto: String,
    pub tamanho_fonte: f64,
    pub color_r: f64,
    pub color_g: f64,
    pub color_b: f64,
    pub angulo: f64,
    pub opacidade: f64,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Tributacoes {
    pub municipal: DadosTributacaoMunicipal,
    pub federal: DadosTributacaoFederal,
    pub ibs_cbs: DadosTributacaoIbsCbs,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct DocumentoDanfse {
    pub marca_dagua: MarcaDagua,
    pub cabecalho: DadosCabecalho,
    pub acesso: DadosAcesso,
    pub partes: DadosPartes,
    pub servico: DadosServico,
    pub tributacoes: Tributacoes,
    pub totais: DadosTotais,
    pub informacoes_complementares: DadosInformacoesComplementares,
    pub canhoto: DadosCanhoto,
}
