//! formatadores e mapeamentos puros usados na apresentação do DANFSe

const TRACO: &str = "-";

fn lookup<K: PartialEq + Copy>(tabela: &[(K, &'static str)], chave: K) -> Option<&'static str> {
    tabela.iter().find(|(k, _)| *k == chave).map(|(_, v)| *v)
}

const NAO_NIF: &[(i64, &str)] = &[
    (1, "Não informado na nota de origem"),
    (2, "Dispensado do NIF"),
    (3, "Não exigência do NIF"),
];

const TRIBUTACAO_ISSQN: &[(i64, &str)] = &[
    (1, "Operação Tributável"),
    (2, "Imunidade"),
    (3, "Exportação de Serviço"),
    (4, "Não Incidência"),
];

const OPCAO_SIMPLES_NACIONAL: &[(i64, &str)] = &[
    (1, "1 - Não Optante"),
    (2, "2 - Optante - MEI"),
    (3, "3 - Optante - ME/EPP"),
];

const SUSPENSAO_EXIGIBILIDADE: &[(i64, &str)] = &[
    (1, "Exigibilidade Suspensa por Decisão Judicial"),
    (2, "Exigibilidade Suspensa por Processo Administrativo"),
];

const TIPO_BENEFICIO_MUNICIPAL: &[(i64, &str)] = &[
    (1, "Isenção"),
    (2, "Redução da BC em 'ppBM' %'"),
    (3, "Redução da BC em R$ 'vInfoBM'"),
    (4, "Alíquota Diferenciada de 'aliqDifBM' %"),
];

const TIPO_RETENCAO_ISSQN: &[(i64, &str)] = &[
    (1, "Não Retido"),
    (2, "Retido pelo Tomador"),
    (3, "Retido pelo Intermediário"),
];

const TIPO_RETENCAO_PIS_COFINS: &[(i64, &str)] = &[
    (0, "PIS/COFINS/CSLL Não Retidos"),
    (1, "PIS/COFINS Retidos"),
    (2, "PIS/COFINS Não Retidos"),
    (3, "PIS/COFINS/CSLL Retidos"),
    (4, "PIS/COFINS Retidos, CSLL Não Retido"),
    (5, "PIS Retido, COFINS/CSLL Não Retido"),
    (6, "COFINS Retido, PIS/CSLL Não Retido"),
    (7, "PIS Não Retido, COFINS/CSLL Retidos"),
    (8, "PIS/COFINS Não Retidos, CSLL Retido"),
    (9, "COFINS Não Retido, PIS/CSLL Retidos"),
];

const TIPO_IMUNIDADE: &[(i64, &str)] = &[
    (0, ""),
    (
        1,
        "Patrimônio, renda ou serviços, uns dos outros (CF88, Art 150, VI, a)",
    ),
    (2, "Templos de qualquer culto (CF88, Art 150, VI, b)"),
    (
        3,
        "Patrimônio, renda ou serviços dos partidos políticos, inclusive suas fundações, das entidades sindicais dos trabalhadores, das instituições de educação e de assistência social, sem fins lucrativos, atendidos os requisitos da lei (CF88, Art 150, VI, c)",
    ),
    (
        4,
        "Livros, jornais, periódicos e o papel destinado a sua impressão (CF88, Art 150, VI, d)",
    ),
    (
        5,
        "Fonogramas e videofonogramas musicais produzidos no Brasil contendo obras musicais ou literomusicais de autores brasileiros e/ou obras em geral interpretadas por artistas brasileiros bem como os suportes materiais ou arquivos digitais que os contenham, salvo na etapa de replicação industrial de mídias ópticas de leitura a laser.   (CF88, Art 150, VI, e)",
    ),
];

const REGIME_ESPECIAL_TRIBUTACAO: &[(i64, &str)] = &[
    (0, "Nenhum"),
    (1, "Ato Cooperado (Cooperativa)"),
    (2, "Estimativa"),
    (3, "Microempresa Municipal"),
    (4, "Notário ou Registrador"),
    (5, "Profissional Autônomo"),
    (6, "Sociedade de Profissionais"),
    (9, "Outros"),
];

const REGIME_APURACAO_SN: &[(i64, &str)] = &[
    (
        1,
        "1 - Regime de apuração dos tributos federais e municipal pelo SN",
    ),
    (
        2,
        "2 - Regime de apuração dos tributos federais pelo SN e ISSQN por fora do SN",
    ),
];

const TIPO_EMITENTE: &[(i64, &str)] = &[(1, "Prestador"), (2, "Tomador"), (3, "Intermediário")];

const C_STAT: &[(&str, &str)] = &[
    ("100", "NFS-e Regular"),
    ("101", "NFS-e Cancelada"),
    ("102", "NFS-e Substituída"),
    ("103", "NFS-e de Decisão Judicial ou Administrativa"),
    ("104", "NFS-e Emitida em Contingência"),
    ("105", "NFS-e de Ajuste"),
];

const FINALIDADE_NFSE: &[(i64, &str)] = &[
    (0, "NFS-e regular"),
    (1, "NFS-e de substituição"),
    (2, "NFS-e de ajuste"),
];

const AMBIENTE_GERADOR: &[(&str, &str)] = &[
    ("1", "Ambiente de Produção"),
    ("2", "Ambiente de Homologação"),
];

const TIPO_AMBIENTE: &[(i64, &str)] = &[(1, "Produção"), (2, "Homologação")];

fn mapear_ou_traco(tabela: &[(i64, &'static str)], chave: i64) -> String {
    lookup(tabela, chave).unwrap_or(TRACO).to_string()
}

/// trecho `[inicio, fim)` em bytes, como o fatiamento de strings do Go
fn fatia(s: &[u8], inicio: usize, fim: usize) -> &[u8] {
    &s[inicio..fim]
}

fn juntar_bytes(partes: &[&[u8]]) -> String {
    String::from_utf8_lossy(&partes.concat()).into_owned()
}

/// máscara de CPF (11 dígitos) ou CNPJ (14 dígitos); demais tamanhos voltam como vieram
pub fn formatar_documento(s: &str) -> String {
    let s = s.trim();
    let b = s.as_bytes();
    match b.len() {
        11 => juntar_bytes(&[
            fatia(b, 0, 3),
            b".",
            fatia(b, 3, 6),
            b".",
            fatia(b, 6, 9),
            b"-",
            fatia(b, 9, 11),
        ]),
        14 => juntar_bytes(&[
            fatia(b, 0, 2),
            b".",
            fatia(b, 2, 5),
            b".",
            fatia(b, 5, 8),
            b"/",
            fatia(b, 8, 12),
            b"-",
            fatia(b, 12, 14),
        ]),
        _ => s.to_string(),
    }
}

/// CEP de 8 dígitos no formato "XX.XXX-XXX"
pub fn formatar_cep(s: &str) -> String {
    let b = s.as_bytes();
    if b.len() == 8 {
        return juntar_bytes(&[fatia(b, 0, 2), b".", fatia(b, 2, 5), b"-", fatia(b, 5, 8)]);
    }
    s.to_string()
}

/// telefone com 10 dígitos → (XX) XXXX-XXXX; 11 dígitos → (XX) XXXXX-XXXX
pub fn formatar_telefone(s: &str) -> String {
    let digitos: String = s.chars().filter(char::is_ascii_digit).collect();
    match digitos.len() {
        10 => format!("({}) {}-{}", &digitos[..2], &digitos[2..6], &digitos[6..]),
        11 => format!("({}) {}-{}", &digitos[..2], &digitos[2..7], &digitos[7..]),
        _ => digitos,
    }
}

/// valor monetário "R$ X.XXX,XX" com sinal (prefixo "R$ -" para negativos)
pub fn formatar_moeda(v: f64) -> String {
    let prefixo = if v < 0.0 { "R$ -" } else { "R$ " };
    format!("{prefixo}{}", formatar_moeda_agrupada(v))
}

pub fn formatar_percentual(v: f64) -> String {
    format!("{}%", formatar_numero_percentual(v))
}

pub fn formatar_numero_percentual(v: f64) -> String {
    format!("{v:.2}").replacen('.', ",", 1)
}

pub fn formatar_numero_percentual_ou_traco(v: f64) -> String {
    if v == 0.0 {
        return TRACO.to_string();
    }
    formatar_numero_percentual(v)
}

pub fn formatar_percentual_ou_traco(v: f64) -> String {
    if v == 0.0 {
        return TRACO.to_string();
    }
    formatar_percentual(v)
}

/// "R$ X.XXX,XX" sempre (inclusive zero), usando o valor absoluto
pub fn formatar_valor_monetario(v: f64) -> String {
    format!("R$ {}", formatar_moeda_agrupada(v))
}

/// data ISO "AAAA-MM-DD" → "DD/MM/AAAA"; vazio vira "-", formatos inválidos voltam como vieram
pub fn formatar_data(s: &str) -> String {
    let s = s.trim();
    if s.is_empty() {
        return TRACO.to_string();
    }
    let b = s.as_bytes();
    if b.len() == 10 {
        let (ano, mes, dia) = (fatia(b, 0, 4), fatia(b, 5, 7), fatia(b, 8, 10));
        if sao_digitos(ano) && sao_digitos(mes) && sao_digitos(dia) && b[4] == b'-' && b[7] == b'-'
        {
            return juntar_bytes(&[dia, b"/", mes, b"/", ano]);
        }
    }
    s.to_string()
}

/// data/hora ISO "AAAA-MM-DDTHH:MM:SS..." → "DD/MM/AAAA HH:MM:SS"
pub fn formatar_data_hora(s: &str) -> String {
    let s = s.trim();
    if s.is_empty() {
        return TRACO.to_string();
    }
    let b = s.as_bytes();
    if b.len() == 10 {
        return formatar_data(s);
    }
    if b.len() < 19 || b[10] != b'T' {
        return s.to_string();
    }
    let parte_data = String::from_utf8_lossy(fatia(b, 0, 10)).into_owned();
    let data = formatar_data(&parte_data);
    if data == parte_data {
        return s.to_string();
    }
    let hora_valida = sao_digitos(fatia(b, 11, 13))
        && b[13] == b':'
        && sao_digitos(fatia(b, 14, 16))
        && b[16] == b':'
        && sao_digitos(fatia(b, 17, 19));
    if hora_valida {
        return format!("{data} {}", String::from_utf8_lossy(fatia(b, 11, 19)));
    }
    s.to_string()
}

fn sao_digitos(s: &[u8]) -> bool {
    !s.is_empty() && s.iter().all(u8::is_ascii_digit)
}

/// código de tributação nacional de 6 dígitos no formato "XX.XX.XX"
pub fn formatar_codigo_tributacao_nacional(s: &str) -> String {
    let sem_pontos = s.replace('.', "");
    let s = sem_pontos.trim();
    let b = s.as_bytes();
    if b.len() != 6 {
        return s.to_string();
    }
    juntar_bytes(&[fatia(b, 0, 2), b".", fatia(b, 2, 4), b".", fatia(b, 4, 6)])
}

/// código NBS de 9 dígitos no formato "X.XXXX.XX.XXX"
pub fn formatar_nbs(conteudo: &str) -> String {
    let s: String = conteudo.chars().filter(char::is_ascii_digit).collect();
    if s.len() != 9 {
        return conteudo.to_string();
    }
    format!("{}.{}.{}.{}", &s[..1], &s[1..5], &s[5..7], &s[7..])
}

pub fn formatar_municipio_uf(municipio: &str, uf: &str) -> String {
    let (municipio, uf) = (municipio.trim(), uf.trim());
    match (municipio.is_empty(), uf.is_empty()) {
        (true, true) => TRACO.to_string(),
        (false, false) => format!("{municipio} / {uf}"),
        (false, true) => municipio.to_string(),
        (true, false) => uf.to_string(),
    }
}

pub fn mapear_c_stat(s: &str) -> String {
    let s = s.trim();
    if s.is_empty() {
        return TRACO.to_string();
    }
    lookup(C_STAT, s).map_or_else(|| s.to_string(), str::to_string)
}

pub fn mapear_tipo_ambiente(i: i64) -> String {
    mapear_ou_traco(TIPO_AMBIENTE, i)
}

pub fn mapear_ambiente_gerador(s: &str) -> String {
    lookup(AMBIENTE_GERADOR, s).unwrap_or(TRACO).to_string()
}

pub fn mapear_tipo_emitente(i: i64) -> String {
    mapear_ou_traco(TIPO_EMITENTE, i)
}

pub fn mapear_finalidade_nfse(i: i64) -> String {
    mapear_ou_traco(FINALIDADE_NFSE, i)
}

pub fn mapear_tributacao_issqn(i: i64) -> String {
    mapear_ou_traco(TRIBUTACAO_ISSQN, i)
}

pub fn mapear_opcao_simples_nacional(i: i64) -> String {
    mapear_ou_traco(OPCAO_SIMPLES_NACIONAL, i)
}

pub fn mapear_regime_especial_tributacao(i: i64) -> String {
    mapear_ou_traco(REGIME_ESPECIAL_TRIBUTACAO, i)
}

pub fn mapear_regime_apuracao_sn(i: i64) -> String {
    mapear_ou_traco(REGIME_APURACAO_SN, i)
}

pub fn mapear_suspensao_exigibilidade(i: i64) -> String {
    mapear_ou_traco(SUSPENSAO_EXIGIBILIDADE, i)
}

pub fn mapear_tipo_beneficio_municipal(i: i64) -> String {
    mapear_ou_traco(TIPO_BENEFICIO_MUNICIPAL, i)
}

pub fn mapear_tipo_retencao_issqn(i: i64) -> String {
    mapear_ou_traco(TIPO_RETENCAO_ISSQN, i)
}

pub fn mapear_tipo_retencao_pis_cofins(i: i64) -> String {
    mapear_ou_traco(TIPO_RETENCAO_PIS_COFINS, i)
}

pub fn mapear_tipo_imunidade(i: i64) -> String {
    match lookup(TIPO_IMUNIDADE, i) {
        Some(v) if !v.is_empty() => v.to_string(),
        _ => TRACO.to_string(),
    }
}

pub fn mapear_codigo_nao_nif(i: i64) -> String {
    mapear_ou_traco(NAO_NIF, i)
}

/// combina código IBGE e CEP (formatado) separados por " / "
pub fn formatar_codigo_ibge_cep(ibge: &str, cep: &str) -> String {
    let (ibge, cep) = (ibge.trim(), cep.trim());
    if ibge.is_empty() && cep.is_empty() {
        return TRACO.to_string();
    }
    let cep = if cep.len() == 8 {
        formatar_cep(cep)
    } else {
        cep.to_string()
    };
    match (ibge.is_empty(), cep.is_empty()) {
        (false, false) => format!("{ibge} / {cep}"),
        (false, true) => ibge.to_string(),
        _ => cep,
    }
}

pub fn traco_se_vazio(s: &str) -> String {
    let s = s.trim();
    if s.is_empty() {
        return TRACO.to_string();
    }
    s.to_string()
}

/// "-" para ausente ou zero; caso contrário "R$ X.XXX,XX"
pub fn formatar_valor_monetario_ou_traco(valor: impl Into<Option<f64>>) -> String {
    match valor.into() {
        Some(v) if v != 0.0 => format!("R$ {}", formatar_moeda_agrupada(v)),
        _ => TRACO.to_string(),
    }
}

/// valor absoluto com separador de milhar "." e decimal ","
fn formatar_moeda_agrupada(v: f64) -> String {
    let texto = format!("{:.2}", v.abs());
    let (inteiro, decimal) = texto.split_once('.').unwrap_or((texto.as_str(), "00"));
    let digitos = inteiro.as_bytes();
    let mut agrupado = String::with_capacity(digitos.len() + digitos.len() / 3);
    for (i, d) in digitos.iter().enumerate() {
        let restantes = digitos.len() - i;
        if i > 0 && restantes % 3 == 0 {
            agrupado.push('.');
        }
        agrupado.push(char::from(*d));
    }
    format!("{agrupado},{decimal}")
}

/// primeiro valor não vazio (após trim), ou "-"
pub fn primeiro_nao_vazio<'a>(valores: &[&'a str]) -> &'a str {
    valores
        .iter()
        .copied()
        .find(|v| !v.trim().is_empty())
        .unwrap_or(TRACO)
}

/// junta as partes não vazias do endereço com ", "
pub fn montar_endereco(partes: &[&str]) -> String {
    let valores: Vec<&str> = partes
        .iter()
        .map(|p| p.trim())
        .filter(|p| !p.is_empty())
        .collect();
    if valores.is_empty() {
        return TRACO.to_string();
    }
    valores.join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn documento() {
        assert_eq!(formatar_documento("12345678901"), "123.456.789-01");
        assert_eq!(formatar_documento("12345678000195"), "12.345.678/0001-95");
        assert_eq!(formatar_documento("12345"), "12345");
        assert_eq!(formatar_documento("  12345678901  "), "123.456.789-01");
    }

    #[test]
    fn cep() {
        assert_eq!(formatar_cep("01001000"), "01.001-000");
        assert_eq!(formatar_cep("12345"), "12345");
    }

    #[test]
    fn telefone() {
        assert_eq!(formatar_telefone("(11) 1234-5678"), "(11) 1234-5678");
        assert_eq!(formatar_telefone("11912345678"), "(11) 91234-5678");
        assert_eq!(formatar_telefone("12345"), "12345");
    }

    #[test]
    fn moeda() {
        assert_eq!(formatar_moeda(1234.56), "R$ 1.234,56");
        assert_eq!(formatar_moeda(-1234.56), "R$ -1.234,56");
        assert_eq!(formatar_moeda(0.0), "R$ 0,00");
        assert_eq!(formatar_moeda(1234567.891), "R$ 1.234.567,89");
    }

    #[test]
    fn percentuais() {
        assert_eq!(formatar_percentual(12.345), "12,35%");
        assert_eq!(formatar_numero_percentual(12.345), "12,35");
        assert_eq!(formatar_numero_percentual_ou_traco(0.0), "-");
        assert_eq!(formatar_numero_percentual_ou_traco(12.345), "12,35");
        assert_eq!(formatar_percentual_ou_traco(0.0), "-");
        assert_eq!(formatar_percentual_ou_traco(0.10), "0,10%");
        assert_eq!(formatar_percentual_ou_traco(1.00), "1,00%");
        assert_eq!(formatar_percentual_ou_traco(2.50), "2,50%");
        assert_eq!(formatar_percentual_ou_traco(12.345), "12,35%");
    }

    #[test]
    fn codigo_tributacao_nacional() {
        assert_eq!(formatar_codigo_tributacao_nacional("010101"), "01.01.01");
        assert_eq!(formatar_codigo_tributacao_nacional("12345"), "12345");
        assert_eq!(formatar_codigo_tributacao_nacional("01.01.01"), "01.01.01");
    }

    #[test]
    fn nbs() {
        assert_eq!(formatar_nbs("110010100"), "1.1001.01.00");
        assert_eq!(formatar_nbs("12345"), "12345");
    }

    #[test]
    fn c_stat() {
        let casos = [
            ("100", "NFS-e Regular"),
            ("101", "NFS-e Cancelada"),
            ("102", "NFS-e Substituída"),
            ("103", "NFS-e de Decisão Judicial ou Administrativa"),
            ("104", "NFS-e Emitida em Contingência"),
            ("105", "NFS-e de Ajuste"),
            ("", "-"),
            ("999", "999"),
        ];
        for (codigo, esperado) in casos {
            assert_eq!(mapear_c_stat(codigo), esperado, "{codigo}");
        }
    }

    #[test]
    fn ambientes() {
        assert_eq!(mapear_tipo_ambiente(1), "Produção");
        assert_eq!(mapear_tipo_ambiente(2), "Homologação");
        assert_eq!(mapear_tipo_ambiente(9), "-");
        assert_eq!(mapear_ambiente_gerador("1"), "Ambiente de Produção");
        assert_eq!(mapear_ambiente_gerador("2"), "Ambiente de Homologação");
        assert_eq!(mapear_ambiente_gerador("9"), "-");
    }

    fn verificar(f: fn(i64) -> String, casos: &[(i64, &str)]) {
        for (valor, esperado) in casos {
            assert_eq!(f(*valor), *esperado, "valor {valor}");
        }
    }

    #[test]
    fn mapeamentos_numericos() {
        verificar(
            mapear_tipo_emitente,
            &[
                (1, "Prestador"),
                (2, "Tomador"),
                (3, "Intermediário"),
                (9, "-"),
            ],
        );
        verificar(
            mapear_finalidade_nfse,
            &[
                (0, "NFS-e regular"),
                (1, "NFS-e de substituição"),
                (2, "NFS-e de ajuste"),
                (9, "-"),
            ],
        );
        verificar(
            mapear_tributacao_issqn,
            &[
                (1, "Operação Tributável"),
                (2, "Imunidade"),
                (3, "Exportação de Serviço"),
                (4, "Não Incidência"),
                (0, "-"),
                (99, "-"),
            ],
        );
        verificar(
            mapear_opcao_simples_nacional,
            &[
                (1, "1 - Não Optante"),
                (2, "2 - Optante - MEI"),
                (3, "3 - Optante - ME/EPP"),
                (0, "-"),
                (99, "-"),
            ],
        );
        verificar(
            mapear_regime_especial_tributacao,
            &[
                (0, "Nenhum"),
                (1, "Ato Cooperado (Cooperativa)"),
                (2, "Estimativa"),
                (3, "Microempresa Municipal"),
                (4, "Notário ou Registrador"),
                (5, "Profissional Autônomo"),
                (6, "Sociedade de Profissionais"),
                (9, "Outros"),
                (99, "-"),
            ],
        );
        verificar(
            mapear_regime_apuracao_sn,
            &[
                (
                    1,
                    "1 - Regime de apuração dos tributos federais e municipal pelo SN",
                ),
                (
                    2,
                    "2 - Regime de apuração dos tributos federais pelo SN e ISSQN por fora do SN",
                ),
                (0, "-"),
                (99, "-"),
            ],
        );
        verificar(
            mapear_suspensao_exigibilidade,
            &[
                (1, "Exigibilidade Suspensa por Decisão Judicial"),
                (2, "Exigibilidade Suspensa por Processo Administrativo"),
                (0, "-"),
                (99, "-"),
            ],
        );
        verificar(
            mapear_tipo_beneficio_municipal,
            &[
                (1, "Isenção"),
                (2, "Redução da BC em 'ppBM' %'"),
                (3, "Redução da BC em R$ 'vInfoBM'"),
                (4, "Alíquota Diferenciada de 'aliqDifBM' %"),
                (0, "-"),
                (99, "-"),
            ],
        );
        verificar(
            mapear_tipo_retencao_issqn,
            &[
                (1, "Não Retido"),
                (2, "Retido pelo Tomador"),
                (3, "Retido pelo Intermediário"),
                (0, "-"),
                (99, "-"),
            ],
        );
        verificar(
            mapear_tipo_retencao_pis_cofins,
            &[
                (0, "PIS/COFINS/CSLL Não Retidos"),
                (1, "PIS/COFINS Retidos"),
                (2, "PIS/COFINS Não Retidos"),
                (3, "PIS/COFINS/CSLL Retidos"),
                (4, "PIS/COFINS Retidos, CSLL Não Retido"),
                (5, "PIS Retido, COFINS/CSLL Não Retido"),
                (6, "COFINS Retido, PIS/CSLL Não Retido"),
                (7, "PIS Não Retido, COFINS/CSLL Retidos"),
                (8, "PIS/COFINS Não Retidos, CSLL Retido"),
                (9, "COFINS Não Retido, PIS/CSLL Retidos"),
                (99, "-"),
            ],
        );
        verificar(
            mapear_codigo_nao_nif,
            &[
                (1, "Não informado na nota de origem"),
                (2, "Dispensado do NIF"),
                (3, "Não exigência do NIF"),
                (0, "-"),
                (99, "-"),
            ],
        );
    }

    #[test]
    fn tipo_imunidade() {
        assert_eq!(mapear_tipo_imunidade(0), "-");
        assert_eq!(
            mapear_tipo_imunidade(1),
            "Patrimônio, renda ou serviços, uns dos outros (CF88, Art 150, VI, a)"
        );
        assert_eq!(mapear_tipo_imunidade(99), "-");
    }

    #[test]
    fn codigo_ibge_cep() {
        let casos = [
            ("3550308", "01001000", "3550308 / 01.001-000"),
            ("3550308", "", "3550308"),
            ("", "01001000", "01.001-000"),
            ("", "", "-"),
            ("  ", "  ", "-"),
        ];
        for (ibge, cep, esperado) in casos {
            assert_eq!(formatar_codigo_ibge_cep(ibge, cep), esperado);
        }
    }

    #[test]
    fn traco() {
        assert_eq!(traco_se_vazio(""), "-");
        assert_eq!(traco_se_vazio("  "), "-");
        assert_eq!(traco_se_vazio("São Paulo"), "São Paulo");
    }

    #[test]
    fn valor_monetario_ou_traco() {
        assert_eq!(formatar_valor_monetario_ou_traco(None), "-");
        assert_eq!(formatar_valor_monetario_ou_traco(0.0), "-");
        assert_eq!(formatar_valor_monetario_ou_traco(123.45), "R$ 123,45");
    }

    #[test]
    fn valor_monetario() {
        assert_eq!(formatar_valor_monetario(0.0), "R$ 0,00");
        assert_eq!(formatar_valor_monetario(1234.56), "R$ 1.234,56");
        assert_eq!(formatar_valor_monetario(-100.00), "R$ 100,00");
    }

    #[test]
    fn datas() {
        assert_eq!(formatar_data("2026-07-14"), "14/07/2026");
        assert_eq!(formatar_data(""), "-");
        assert_eq!(formatar_data("14-07-2026"), "14-07-2026");
        assert_eq!(formatar_data("  2026-07-14  "), "14/07/2026");
        assert_eq!(
            formatar_data_hora("2026-07-14T10:00:00-03:00"),
            "14/07/2026 10:00:00"
        );
        assert_eq!(formatar_data_hora(""), "-");
        assert_eq!(
            formatar_data_hora("14-07-2026 10:00:00"),
            "14-07-2026 10:00:00"
        );
        assert_eq!(formatar_data_hora("2026-07-14"), "14/07/2026");
    }

    #[test]
    fn datas_com_multibyte_nao_quebram() {
        assert_eq!(formatar_data("2026-07-1é"), "2026-07-1é");
        assert_eq!(
            formatar_data_hora("2026-07-14Tçç:00:00"),
            "2026-07-14Tçç:00:00"
        );
        assert_eq!(formatar_documento("ááá1"), "ááá1");
    }

    #[test]
    fn municipio_uf() {
        assert_eq!(formatar_municipio_uf("São Paulo", "SP"), "São Paulo / SP");
        assert_eq!(formatar_municipio_uf("São Paulo", ""), "São Paulo");
        assert_eq!(formatar_municipio_uf("", "SP"), "SP");
        assert_eq!(formatar_municipio_uf("", ""), "-");
    }

    #[test]
    fn primeiro_nao_vazio_e_endereco() {
        assert_eq!(primeiro_nao_vazio(&["", " ", "x"]), "x");
        assert_eq!(primeiro_nao_vazio(&["", " "]), "-");
        assert_eq!(montar_endereco(&["Rua A", " ", "10 "]), "Rua A, 10");
        assert_eq!(montar_endereco(&["", ""]), "-");
    }
}
