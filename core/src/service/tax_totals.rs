use crate::domain::models::NFSe;
use crate::domain::presentation::format::{formatar_moeda, formatar_percentual};

const PREFIXO: &str = "Totais aproximados dos Tributos cfe. Lei nº 12.741/2012";

/// linha fixa de totais aproximados de tributos (Lei nº 12.741/2012):
/// qualquer total monetário positivo gera a saída em reais; senão, qualquer
/// percentual positivo gera a saída percentual; senão, zeros
pub(super) fn montar_linha_totais_tributos(nota: &NFSe) -> String {
    let totais = &nota.inf_nfse.dps.inf_dps.valores.trib.tot_trib;
    let v = &totais.v_tot_trib;
    if v.v_tot_trib_fed > 0.0 || v.v_tot_trib_est > 0.0 || v.v_tot_trib_mun > 0.0 {
        return linha(
            formatar_moeda(v.v_tot_trib_fed),
            formatar_moeda(v.v_tot_trib_est),
            formatar_moeda(v.v_tot_trib_mun),
        );
    }
    let p = &totais.p_tot_trib;
    if p.p_tot_trib_fed > 0.0 || p.p_tot_trib_est > 0.0 || p.p_tot_trib_mun > 0.0 {
        return linha(
            formatar_percentual(p.p_tot_trib_fed),
            formatar_percentual(p.p_tot_trib_est),
            formatar_percentual(p.p_tot_trib_mun),
        );
    }
    linha("R$ 0,00".into(), "R$ 0,00".into(), "R$ 0,00".into())
}

fn linha(federais: String, estaduais: String, municipais: String) -> String {
    format!("{PREFIXO}: Federais: {federais}; Estaduais: {estaduais}; Municipais: {municipais};")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::models::{PTotTrib, VTotTrib};

    fn nota(v: (f64, f64, f64), p: (f64, f64, f64)) -> NFSe {
        let mut n = NFSe::default();
        let tot = &mut n.inf_nfse.dps.inf_dps.valores.trib.tot_trib;
        tot.v_tot_trib = VTotTrib {
            v_tot_trib_fed: v.0,
            v_tot_trib_est: v.1,
            v_tot_trib_mun: v.2,
        };
        tot.p_tot_trib = PTotTrib {
            p_tot_trib_fed: p.0,
            p_tot_trib_est: p.1,
            p_tot_trib_mun: p.2,
        };
        n
    }

    fn esperado(resto: &str) -> String {
        format!("{PREFIXO}: {resto}")
    }

    #[test]
    fn monetario() {
        assert_eq!(
            montar_linha_totais_tributos(&nota((150.0, 80.0, 50.0), (0.0, 0.0, 0.0))),
            esperado("Federais: R$ 150,00; Estaduais: R$ 80,00; Municipais: R$ 50,00;")
        );
    }

    #[test]
    fn monetario_com_valores_grandes() {
        assert_eq!(
            montar_linha_totais_tributos(&nota((1234.56, 98765.43, 0.0), (0.0, 0.0, 0.0))),
            esperado("Federais: R$ 1.234,56; Estaduais: R$ 98.765,43; Municipais: R$ 0,00;")
        );
    }

    #[test]
    fn porcentagem_fallback() {
        assert_eq!(
            montar_linha_totais_tributos(&nota((0.0, 0.0, 0.0), (5.50, 7.25, 2.00))),
            esperado("Federais: 5,50%; Estaduais: 7,25%; Municipais: 2,00%;")
        );
    }

    #[test]
    fn vazio() {
        assert_eq!(
            montar_linha_totais_tributos(&nota((0.0, 0.0, 0.0), (0.0, 0.0, 0.0))),
            esperado("Federais: R$ 0,00; Estaduais: R$ 0,00; Municipais: R$ 0,00;")
        );
    }

    #[test]
    fn monetario_prioritario_com_um_valor() {
        assert_eq!(
            montar_linha_totais_tributos(&nota((0.0, 0.0, 25.0), (10.0, 0.0, 0.0))),
            esperado("Federais: R$ 0,00; Estaduais: R$ 0,00; Municipais: R$ 25,00;")
        );
    }

    #[test]
    fn porcentagem_somente_um() {
        assert_eq!(
            montar_linha_totais_tributos(&nota((0.0, 0.0, 0.0), (0.0, 0.0, 15.50))),
            esperado("Federais: 0,00%; Estaduais: 0,00%; Municipais: 15,50%;")
        );
    }

    #[test]
    fn monetario_precede_porcentagem() {
        assert_eq!(
            montar_linha_totais_tributos(&nota((100.0, 50.0, 25.0), (10.0, 5.0, 2.5))),
            esperado("Federais: R$ 100,00; Estaduais: R$ 50,00; Municipais: R$ 25,00;")
        );
    }
}
