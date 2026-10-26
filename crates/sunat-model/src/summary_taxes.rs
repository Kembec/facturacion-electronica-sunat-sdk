//! IVAP (1016), ISC (2000), other taxes (9999) and the IGV perception of a daily summary line, checked only when the
//! line carries them: a line without them validates as before. Resumen Diario sheet of SUNAT's validation rules,
//! rows 58-75 (perception) and 116-148 (taxes); only what that sheet answers with an ERROR is rejected here. The
//! perception's rate comes from catalog 22 (Catálogos sheet, rows 330-334): SUNAT checks it (2891), so the caller
//! picks the regime and the SDK the rate.

use crate::allowances::{cents, half_up_cents, show_cents, within_one_cents};
use crate::retention_lines::{units, SCALE};
use crate::validate::{check_format, fits};
use crate::{Decimal, ModelError, SummaryLine, SummaryPerception};

const PEN: &str = "PEN";
/// Catalog 22 and each regime's rate, in hundredths of a percent.
const PERCEPTION_REGIMES: &[(&str, i128)] = &[("01", 200), ("02", 100), ("03", 50)];
/// Hundredths of a percent in one unit.
const RATE_SCALE: i128 = 10_000;

pub(crate) fn check(index: usize, line: &SummaryLine, currency: &str) -> Result<(), ModelError> {
    let path = |field: &str| format!("$.lines[{index}].{field}");
    if let Some(ivap) = &line.ivap {
        positive(&path("ivap"), ivap, "2048")?;
        // One IGV-or-IVAP tax per line: 1000 and 1016 together repeat it (2355); `igv_rate` only renders with 1000.
        for (field, present) in [("igv", line.igv.is_some()), ("igv_rate", line.igv_rate.is_some())] {
            if present {
                return Err(ModelError::rule("2355", path(field), "a line with ivap (tax 1016) carries no IGV (tax 1000)"));
            }
        }
    }
    for (field, value) in [("isc", &line.isc), ("other_taxes", &line.other_taxes)] {
        if let Some(value) = value {
            positive(&path(field), value, "2048")?;
        }
    }
    match &line.perception {
        Some(perception) => check_perception(index, line, perception, currency),
        None => Ok(()),
    }
}

fn check_perception(index: usize, line: &SummaryLine, perception: &SummaryPerception, currency: &str) -> Result<(), ModelError> {
    let path = |field: &str| format!("$.lines[{index}].perception.{field}");
    if line.document_type != "03" || line.status == "2" {
        let reason = "applies only to a receipt (03) whose status is not 2";
        return Err(ModelError::rule("2986", format!("$.lines[{index}].perception"), reason));
    }
    let Some(rate) = regime_rate(&perception.system_code) else {
        return Err(ModelError::rule("2517", path("system_code"), "expected catalog 22: \"01\", \"02\" or \"03\""));
    };
    if currency != PEN {
        return Err(ModelError::rule("2685", "$.currency", "a perception is reported in PEN"));
    }
    positive(&path("amount"), &perception.amount, "2893")?;
    positive(&path("total_cashed"), &perception.total_cashed, "2895")?;
    positive(&path("base"), &perception.base, "2897")?;
    let expected = half_up_cents(amount(&perception.base) * rate, SCALE * RATE_SCALE);
    within_one(&path("amount"), &perception.amount, expected, "base × the regime's rate")?;
    let expected = cents(amount(&line.total) + amount(&perception.amount));
    within_one(&path("total_cashed"), &perception.total_cashed, expected, "total + amount")
}

/// The rate SUNAT expects for a perception regime (2891), as the text a writer renders (`"2.00"`); `None` outside
/// catalog 22. The writer reads it from here so the table that validates is the one that writes.
pub fn perception_percent(system_code: &str) -> Option<String> {
    regime_rate(system_code).map(|rate| format!("{}.{:02}", rate / 100, rate % 100))
}

fn regime_rate(system_code: &str) -> Option<i128> {
    PERCEPTION_REGIMES.iter().find(|(code, _)| *code == system_code).map(|&(_, rate)| rate)
}

fn positive(path: &str, value: &Decimal, code: &str) -> Result<(), ModelError> {
    let ok = !value.is_negative() && !value.is_zero() && fits(value, 12, 2);
    let expected = format!("a positive amount with up to 12 integer digits and 2 decimals (SUNAT {code})");
    check_format(path, value.as_str(), &expected, |_| ok)
}

/// SUNAT 2608, plus or minus 1 currency unit; `expected` in cents.
fn within_one(path: &str, given: &Decimal, expected: i128, formula: &str) -> Result<(), ModelError> {
    if within_one_cents(cents(amount(given)), expected) {
        return Ok(());
    }
    Err(ModelError::rule("2608", path, format!("must be {formula} ({}), within 1", show_cents(expected))))
}

/// In millionths; an amount too long to read exactly counts as zero.
fn amount(value: &Decimal) -> i128 {
    units(value).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::perception_percent;

    #[test]
    fn each_regime_renders_its_catalog_22_rate() {
        assert_eq!(perception_percent("01").as_deref(), Some("2.00"));
        assert_eq!(perception_percent("02").as_deref(), Some("1.00"));
        assert_eq!(perception_percent("03").as_deref(), Some("0.50"));
        assert_eq!(perception_percent("04"), None);
    }
}
