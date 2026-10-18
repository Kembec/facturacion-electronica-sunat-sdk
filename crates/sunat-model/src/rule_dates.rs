//! SUNAT rules that take effect on a date, as versioned data of the SDK and not as loose constants.
//! A rule applies by the date its norm names: `reference_date` for a daily summary, `issue_date` for
//! a voucher or a note. Before the cut the old rule stands, from it the new one. A `pista` row has
//! no norm behind it (only SUNAT's rules sheet or a secondary source): the SDK never rejects on it,
//! so a date nobody confirmed cannot turn a valid document into a rejected one.

/// Where a date comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Basis {
    /// A resolution that says so.
    Norm,
    /// No norm found: not enforced.
    Lead,
}

/// One dated rule. `effective_from` is `YYYY-MM-DD`, Lima.
pub(crate) struct RuleEffectivity {
    pub(crate) rule: &'static str,
    pub(crate) effective_from: &'static str,
    pub(crate) basis: Basis,
    /// Provenance, for whoever reads the table (and its test); no code branches on it.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) source: &'static str,
}

// RS 143-2026/SUNAT, art. 3.4 rewrites the first final provision of RS 048-2026/SUNAT: all of it is
// in force on 01/01/2027, except 4.3, 4.4, the 4th repealing provision and 8th (b), on 01/04/2027
// (none of those is a rule here). Its annexes are the ones that amend RS 097-2012.
pub(crate) const FREE_SALES_06_09: &str = "free-sales-06-09";
pub(crate) const DEBIT_NOTE_13: &str = "debit-note-13-penalties";
pub(crate) const PRODUCT_CODE_3496: &str = "product-code-3496";

pub(crate) const RULES: &[RuleEffectivity] = &[
    RuleEffectivity {
        rule: FREE_SALES_06_09,
        effective_from: "2027-01-01",
        basis: Basis::Norm,
        source: "annex IX (catalog 11: 06-09; 6th repealing provision removes 05) and annex X (field 18, nodes 25) of RS 048-2026; in force by RS 143-2026 art. 3.4",
    },
    RuleEffectivity {
        rule: DEBIT_NOTE_13,
        effective_from: "2027-01-01",
        basis: Basis::Norm,
        source: "annex IX (catalog 10: 13) and annex VI note (8) of RS 048-2026 (ERR-3507, ERR-2524); in force by RS 143-2026 art. 3.4",
    },
    RuleEffectivity {
        rule: PRODUCT_CODE_3496,
        effective_from: "2027-01-01",
        basis: Basis::Lead,
        source: "Nubefact only; SUNAT's sheet says 2026-08-01; no norm names the product code",
    },
];

/// The row of a rule. A rule missing from the table is a programming error.
pub(crate) fn rule(name: &str) -> &'static RuleEffectivity {
    RULES.iter().find(|r| r.rule == name).unwrap_or_else(|| panic!("rule {name} is not in the table"))
}

/// Whether the SDK enforces the rule on `date` (`YYYY-MM-DD`): its date has come and a norm backs it.
pub(crate) fn applies(name: &str, date: &str) -> bool {
    let row = rule(name);
    row.basis == Basis::Norm && row.effective_from <= date
}

/// The date a rejection names.
pub(crate) fn from(name: &str) -> &'static str {
    rule(name).effective_from
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn norm_rules_apply_from_their_date_and_not_before() {
        for name in [FREE_SALES_06_09, DEBIT_NOTE_13] {
            assert!(!applies(name, "2026-12-31"), "{name}");
            assert!(applies(name, "2027-01-01"), "{name}");
            assert!(applies(name, "2027-06-30"), "{name}");
        }
    }

    #[test]
    fn a_lead_is_never_enforced_and_its_date_moves_only_with_its_source() {
        // Moving this date or promoting it to a norm needs the later SUNAT sheet (26.08.2026) or a
        // resolution: change `source` with it.
        let row = rule(PRODUCT_CODE_3496);
        assert_eq!((row.effective_from, row.basis), ("2027-01-01", Basis::Lead));
        assert!(row.source.starts_with("Nubefact only"));
        assert!(!applies(PRODUCT_CODE_3496, "2030-01-01"));
    }

    #[test]
    fn every_row_has_a_valid_date_and_a_source() {
        for row in RULES {
            assert_eq!(row.effective_from.len(), 10, "{}", row.rule);
            assert!(!row.source.is_empty());
        }
    }
}
