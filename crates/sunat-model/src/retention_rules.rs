//! SUNAT's validation rules for the retention voucher (20) that can be checked on the document itself
//! (sheet «Retenciones1_0» of SUNAT's validation rules, 24/04/2026): header, parties, regime and
//! totals, in the order of the sheet. The lines are in `retention_lines`. Each rejection carries the
//! error code SUNAT would answer; observations (2900s marked OBSERV, 4000 and above) are not enforced.

use crate::despatch_rules::{blank, digits, digits_between, positive_fits, rule};
use crate::retention_lines::{check_lines, units, SCALE};
use crate::{Decimal, ModelError, Retention};

/// Catalog 23: the retention regime and its rate in percent. A catalog entry, not a parameter: SUNAT
/// rejects a rate other than the regime's (2619), and regime 02 only covers related documents issued
/// up to 28/02/2014 (2985).
pub(crate) const REGIMES: &[(&str, i128)] = &[("01", 3), ("02", 6)];

/// Text up to 1500 characters with no whitespace other than the space (sheet «Retenciones1_0»,
/// RegistrationName of the agent and the supplier).
const NAME_MAX: usize = 1500;

pub(crate) fn check(r: &Retention) -> Result<(), ModelError> {
    check_header(r)?;
    check_issuer(r)?;
    check_supplier(r)?;
    let rate = check_regime(r)?;
    check_totals(r)?;
    check_lines(r, rate)
}

fn check_header(r: &Retention) -> Result<(), ModelError> {
    if r.ubl_version != "2.0" {
        return Err(ModelError::invalid("$.ubl_version", "only \"2.0\" is supported for the retention voucher"));
    }
    if r.document_type != "20" {
        return Err(ModelError::invalid("$.document_type", "must be \"20\""));
    }
    rule(series(&r.series), "1001", "$.series", "must be R and 3 uppercase letters or digits")?;
    rule(digits_between(&r.number, 1, 8), "1001", "$.number", "must be 1 to 8 digits")?;
    if r.note.as_ref().is_some_and(|note| note.chars().count() > 250) {
        return Err(ModelError::invalid("$.note", "up to 250 characters"));
    }
    Ok(())
}

fn check_issuer(r: &Retention) -> Result<(), ModelError> {
    let ruc = &r.issuer.ruc;
    if !(ruc.len() == 11 && digits(ruc)) {
        return Err(ModelError::invalid("$.issuer.ruc", format!("expected 11 digits, got {ruc:?}")));
    }
    rule(!blank(&r.issuer.legal_name), "1037", "$.issuer.legal_name", "required")?;
    rule(name(&r.issuer.legal_name), "1038", "$.issuer.legal_name", "up to 1500 characters, no line breaks or tabs")?;
    let country = r.issuer.address.country_code.as_deref();
    rule(country.is_none_or(|c| c == "PE"), "2548", "$.issuer.address.country_code", "must be \"PE\"")
}

fn check_supplier(r: &Retention) -> Result<(), ModelError> {
    let s = &r.supplier;
    let at = |field: &str| format!("$.supplier.{field}");
    rule(!s.document_number.is_empty(), "2723", at("document_number"), "required")?;
    rule(s.document_number.len() == 11 && digits(&s.document_number), "2724", at("document_number"), "must be a RUC of 11 digits")?;
    rule(s.document_number != r.issuer.ruc, "2620", at("document_number"), "the supplier cannot be the issuer")?;
    rule(!s.document_type.is_empty(), "2516", at("document_type"), "required")?;
    rule(s.document_type == "6", "2511", at("document_type"), "must be 6 (RUC)")?;
    rule(!blank(&s.legal_name), "2134", at("legal_name"), "required")?;
    rule(name(&s.legal_name), "2133", at("legal_name"), "up to 1500 characters, no line breaks or tabs")
}

/// The regime's rate in units (`retention_lines::SCALE`), once `rate` matches it.
fn check_regime(r: &Retention) -> Result<i128, ModelError> {
    let regime = REGIMES.iter().find(|(code, _)| *code == r.regime);
    let Some((_, percent)) = regime else {
        return Err(ModelError::rule("2618", "$.regime", "not a catalog 23 code (\"01\" or \"02\")"));
    };
    let given = rate_units(&r.rate);
    let expected = percent * SCALE;
    rule(given == Some(expected), "2619", "$.rate", "must be the regime's rate (01: 3.00, 02: 6.00)")?;
    Ok(expected)
}

/// `rate` as given, with up to 1 integer digit and 2 decimals (`n(1,2)`).
fn rate_units(rate: &Decimal) -> Option<i128> {
    positive_fits(rate, 1, 2).then(|| units(rate)).flatten()
}

fn check_totals(r: &Retention) -> Result<(), ModelError> {
    if r.lines.is_empty() {
        return Err(ModelError::missing("$.lines"));
    }
    rule(positive_fits(&r.total_retained, 12, 2), "2669", "$.total_retained", "must be positive, up to 12 integer digits and 2 decimals")?;
    let retained: Option<i128> = r.lines.iter().map(|line| units(&line.retention.amount)).sum();
    let reason = "must equal the sum of the lines' retention.amount";
    rule(retained.is_some() && retained == units(&r.total_retained), "2628", "$.total_retained", reason)?;
    rule(positive_fits(&r.total_paid, 12, 2), "2730", "$.total_paid", "must be positive, up to 12 integer digits and 2 decimals")?;
    let rounding = match &r.rounding_amount {
        Some(amount) => units(amount),
        None => Some(0),
    };
    let net: Option<i128> = r.lines.iter().map(|line| units(&line.retention.net_paid)).sum();
    let paid = net.zip(rounding).map(|(net, rounding)| net + rounding);
    let reason = "must equal the sum of the lines' retention.net_paid plus rounding_amount";
    rule(paid.is_some() && paid == units(&r.total_paid), "2629", "$.total_paid", reason)?;
    let Some(amount) = &r.rounding_amount else { return Ok(()) };
    let fits = units(amount).is_some_and(|u| u.abs() <= SCALE);
    rule(fits && decimals(amount) <= 2, "3303", "$.rounding_amount", "absolute value up to 1, with 2 decimals")
}

fn series(text: &str) -> bool {
    text.len() == 4 && text.starts_with('R') && crate::despatch_rules::alnum_upper(&text[1..])
}

fn name(text: &str) -> bool {
    text.chars().count() <= NAME_MAX && !text.chars().any(|c| c.is_whitespace() && c != ' ')
}

fn decimals(value: &Decimal) -> usize {
    value.as_str().split_once('.').map_or(0, |(_, frac)| frac.len())
}
