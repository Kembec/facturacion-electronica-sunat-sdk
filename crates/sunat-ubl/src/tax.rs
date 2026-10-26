//! Catalog 05 (tax schemes) and how catalog 07 (IGV affectation) maps onto it.

/// A tax scheme of catalog 05: code, name and UN/ECE 5153 type code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TaxScheme {
    pub(crate) id: &'static str,
    pub(crate) name: &'static str,
    pub(crate) type_code: &'static str,
}

pub(crate) const IGV: TaxScheme = TaxScheme { id: "1000", name: "IGV", type_code: "VAT" };
pub(crate) const UNAFFECTED: TaxScheme = TaxScheme { id: "9998", name: "INA", type_code: "FRE" };
pub(crate) const EXEMPT: TaxScheme = TaxScheme { id: "9997", name: "EXO", type_code: "VAT" };
pub(crate) const FREE: TaxScheme = TaxScheme { id: "9996", name: "GRA", type_code: "FRE" };
pub(crate) const EXPORT: TaxScheme = TaxScheme { id: "9995", name: "EXP", type_code: "FRE" };
pub(crate) const IVAP: TaxScheme = TaxScheme { id: "1016", name: "IVAP", type_code: "VAT" };
pub(crate) const ISC: TaxScheme = TaxScheme { id: "2000", name: "ISC", type_code: "EXC" };
/// Other taxes (9999): the Resumen Diario sheet fixes no name; `OTROS` as in catalog 05 and Greenter.
pub(crate) const OTHER_TAXES: TaxScheme = TaxScheme { id: "9999", name: "OTROS", type_code: "OTH" };
/// Plastic bags tax: a subtotal without `TaxableAmount` or `Percent` (SUNAT 3003, 2992).
pub(crate) const ICBPER: TaxScheme = TaxScheme { id: "7152", name: "ICBPER", type_code: "OTH" };

/// Scheme of a line by its catalog 07 affectation: onerous taxed, exempt, unaffected and export
/// operations keep their own scheme, rice under the IVAP (17) goes under IVAP, and every free transfer
/// (11-16, 21, 31-37) goes under GRA.
pub(crate) fn scheme_for_affectation(code: &str) -> TaxScheme {
    match code {
        "10" => IGV,
        "20" => EXEMPT,
        "30" => UNAFFECTED,
        "40" => EXPORT,
        "17" => IVAP,
        _ => FREE,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn free_transfers_share_the_free_scheme() {
        for code in ["11", "12", "13", "14", "15", "16", "21", "31", "32", "33", "34", "35", "36", "37"] {
            assert_eq!(scheme_for_affectation(code), FREE, "{code}");
        }
        assert_eq!(scheme_for_affectation("10"), IGV);
        assert_eq!(scheme_for_affectation("20"), EXEMPT);
        assert_eq!(scheme_for_affectation("30"), UNAFFECTED);
        assert_eq!(scheme_for_affectation("40"), EXPORT);
        assert_eq!(scheme_for_affectation("17"), IVAP);
    }
}
