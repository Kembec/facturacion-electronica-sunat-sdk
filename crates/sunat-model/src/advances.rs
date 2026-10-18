//! Advances (anticipos) of an invoice or receipt: the entries (`advances[]`) and the prepaid total, in
//! the order of SUNAT's sheet: 2505, 2521, 3217, 2503, 2968, 3307, then 3220 and 2509. The totals the
//! discounts 04, 05 and 06 change are checked in `allowance_totals`.
//!
//! A receipt answers with the same codes as an invoice for these rules, except the factor (3307 an
//! invoice, 4322 a receipt). Amounts are compared in millionths, never as floating point.

use crate::allowances::{cents, half_up_cents, show_cents, within_one_cents, Codes};
use crate::retention_lines::{units, SCALE};
use crate::validate::fits;
use crate::{Advance, Decimal, Invoice, ModelError};

/// The 99 payment identifiers `an..2` allows (SUNAT 3212).
const MAX_ADVANCES: usize = 99;

/// What the advances take off the document, in millionths: the base of the discounts 04, 05 and 06,
/// and the total prepaid.
#[derive(Default)]
pub(crate) struct Sums {
    pub(crate) taxed: i128,
    pub(crate) exempt: i128,
    pub(crate) unaffected: i128,
    pub(crate) paid: i128,
}

/// Sums of the entries; an amount too long to read exactly counts as zero (its format error comes first).
pub(crate) fn sums(invoice: &Invoice) -> Sums {
    let mut sums = Sums::default();
    for advance in &invoice.advances {
        let value = units(&advance.value).unwrap_or(0);
        match advance.discount_code() {
            Some("04") => sums.taxed += value,
            Some("05") => sums.exempt += value,
            Some("06") => sums.unaffected += value,
            _ => {}
        }
        sums.paid += units(&advance.paid_amount).unwrap_or(0);
    }
    sums
}

pub(crate) fn check(invoice: &Invoice, codes: &Codes) -> Result<(), ModelError> {
    let totals = &invoice.voucher.totals;
    if invoice.advances.is_empty() && totals.prepaid.is_none() {
        return Ok(());
    }
    if invoice.advances.len() > MAX_ADVANCES {
        return Err(ModelError::invalid("$.advances", format!("at most {MAX_ADVANCES} advances (SUNAT 3212)")));
    }
    for (index, advance) in invoice.advances.iter().enumerate() {
        check_entry(&format!("$.advances[{index}]"), advance, codes)?;
    }
    check_duplicates(&invoice.advances)?;
    check_prepaid(invoice)
}

fn check_entry(path: &str, advance: &Advance, codes: &Codes) -> Result<(), ModelError> {
    let invoice = advance.document_type == "02";
    let at = |field: &str| format!("{path}.{field}");
    if !matches!(advance.document_type.as_str(), "02" | "03") {
        return Err(ModelError::rule("2505", at("document_type"), "expected \"02\" (invoice) or \"03\" (receipt) of the advance"));
    }
    let expected = if invoice { "F123-1, E001-1 or 0001-1" } else { "B123-1, EB01-1 or 0001-1" };
    if !document_number(&advance.document_number, invoice) {
        let reason = format!("expected the SERIES-NUMBER of the advance, like {expected}, got {:?}", advance.document_number);
        return Err(ModelError::rule("2521", at("document_number"), reason));
    }
    if advance.discount_code().is_none() {
        let reason = format!("expected \"10\" (taxed, discount 04), \"20\" (exempt, 05) or \"30\" (unaffected, 06), got {:?}", advance.affectation);
        return Err(ModelError::invalid(at("affectation"), reason));
    }
    if let Some(ruc) = &advance.issuer_ruc {
        let ok = ruc.len() == 11 && ruc.bytes().all(|b| b.is_ascii_digit());
        if !ok {
            return Err(ModelError::rule("3217", at("issuer_ruc"), format!("expected the 11 digits of a RUC, got {ruc:?}")));
        }
    }
    positive(&at("paid_amount"), &advance.paid_amount, 12, 2, "2503")?;
    positive(&at("value"), &advance.value, 12, 2, "2968")?;
    check_factor(path, advance, codes)
}

/// `base_amount` and `factor` go together; `value` = `base_amount` × `factor`, within 1.
fn check_factor(path: &str, advance: &Advance, codes: &Codes) -> Result<(), ModelError> {
    let (base, factor) = match (&advance.base_amount, &advance.factor) {
        (None, None) => return Ok(()),
        (Some(base), Some(factor)) => (base, factor),
        _ => return Err(ModelError::rule(codes.global_factor, format!("{path}.factor"), "base_amount and factor go together")),
    };
    positive(&format!("{path}.base_amount"), base, 12, 2, "3016")?;
    positive(&format!("{path}.factor"), factor, 3, 5, "3025")?;
    let (Some(value), Some(base), Some(factor)) = (units(&advance.value), units(base), units(factor)) else { return Ok(()) };
    let expected = half_up_cents(base * factor, SCALE * SCALE);
    if within_one_cents(cents(value), expected) {
        return Ok(());
    }
    let reason = format!("value must be base_amount × factor ({}), within 1", show_cents(expected));
    Err(ModelError::rule(codes.global_factor, format!("{path}.value"), reason))
}

/// The same advance twice would be discounted twice; the sheet does not check it.
fn check_duplicates(advances: &[Advance]) -> Result<(), ModelError> {
    for (index, advance) in advances.iter().enumerate() {
        let key = (&advance.document_type, &advance.document_number, &advance.issuer_ruc);
        if advances[..index].iter().any(|other| (&other.document_type, &other.document_number, &other.issuer_ruc) == key) {
            let reason = format!("{} {} is already in the list", advance.document_type, advance.document_number);
            return Err(ModelError::invalid(format!("$.advances[{index}].document_number"), reason));
        }
    }
    Ok(())
}

/// 3220: advances need a prepaid total above zero; 2509: it is the sum of what was paid.
fn check_prepaid(invoice: &Invoice) -> Result<(), ModelError> {
    let prepaid = invoice.voucher.totals.prepaid.as_ref().and_then(units).unwrap_or(0);
    if !invoice.advances.is_empty() && prepaid <= 0 {
        return Err(ModelError::rule("3220", "$.totals.prepaid", "a document with advances needs totals.prepaid above zero"));
    }
    let paid = sums(invoice).paid;
    if cents(prepaid) == cents(paid) {
        return Ok(());
    }
    let reason = format!("must be the sum of advances[].paid_amount ({})", show_cents(cents(paid)));
    Err(ModelError::rule("2509", "$.totals.prepaid", reason))
}

fn positive(path: &str, value: &Decimal, int_max: usize, frac_max: usize, code: &'static str) -> Result<(), ModelError> {
    if !value.is_negative() && !value.is_zero() && fits(value, int_max, frac_max) {
        return Ok(());
    }
    let reason = format!("expected a positive amount with up to {int_max} integer digits and {frac_max} decimals (SUNAT {code}), got {:?}", value.as_str());
    Err(ModelError::rule(code, path, reason))
}

/// `F` plus 3 letters or digits, `E001` or up to 4 digits for an invoice; `B` plus 3, `EB01` or up to
/// 4 digits for a receipt; then a dash and 1 to 8 digits.
fn document_number(text: &str, invoice: bool) -> bool {
    let Some((series, number)) = text.split_once('-') else { return false };
    let numbered = (1..=8).contains(&number.len()) && number.bytes().all(|b| b.is_ascii_digit());
    let alnum = |s: &str| s.bytes().all(|b| b.is_ascii_uppercase() || b.is_ascii_digit());
    let own = if invoice { series.len() == 4 && series.starts_with('F') && alnum(&series[1..]) || series == "E001" } else { series.len() == 4 && series.starts_with('B') && alnum(&series[1..]) || series == "EB01" };
    let legacy = (1..=4).contains(&series.len()) && series.bytes().all(|b| b.is_ascii_digit());
    numbered && (own || legacy)
}
