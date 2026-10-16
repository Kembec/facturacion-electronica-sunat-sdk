//! The purchase order and the despatch guides a voucher cites. SUNAT answers these only with
//! observations (4233, 4005, 4006) and one error (2364, a guide repeated); beta on 16/10/2026 showed
//! which readings of its validation sheet hold (`docs/input-model.md`), and the SDK refuses what beta
//! observed so a document never reaches SUNAT with a note on it.

use std::collections::HashSet;

use crate::{DespatchReference, ModelError};

pub(crate) fn check(order: Option<&str>, despatches: &[DespatchReference]) -> Result<(), ModelError> {
    if let Some(order) = order {
        check_order(order)?;
    }
    let mut seen = HashSet::new();
    for (index, reference) in despatches.iter().enumerate() {
        let path = |field: &str| format!("$.despatch_references[{index}].{field}");
        if !matches!(reference.document_type.as_str(), "09" | "31") {
            let got = &reference.document_type;
            return Err(ModelError::invalid(path("document_type"), format!("expected \"09\" or \"31\" (SUNAT 4005), got {got:?}")));
        }
        if !despatch_number(&reference.number) {
            let got = &reference.number;
            let expected = "SERIES-NUMBER with a series SUNAT lists (T###, V###, G###, ####, EG01 to EG04, EG07) and 1 to 8 digits (SUNAT 4006)";
            return Err(ModelError::invalid(path("number"), format!("expected {expected}, got {got:?}")));
        }
        if !seen.insert((reference.document_type.as_str(), reference.number.as_str())) {
            return Err(ModelError::invalid(path("number"), "the same guide is cited twice (SUNAT 2364)"));
        }
    }
    Ok(())
}

/// 1 to 20 characters with no space, tab or line break (SUNAT 4233). The sheet says a space is
/// allowed; beta observed 4233 on `OC 1`, `  X` and `OC<tab>1`, and accepted a no-break space.
fn check_order(order: &str) -> Result<(), ModelError> {
    let length = order.chars().count();
    if (1..=20).contains(&length) && !order.chars().any(|c| c.is_ascii_whitespace()) {
        return Ok(());
    }
    Err(ModelError::invalid("$.order_reference", format!("expected 1 to 20 characters with no space, tab or line break (SUNAT 4233), got {order:?}")))
}

/// The series SUNAT's sheet lists for a guide: `T` + 3 alphanumerics (sender's electronic guide),
/// `V` + 3 alphanumerics (carrier's), `G` + 3 digits, 4 digits (paper guide), `EG01` to `EG04` and `EG07`.
/// Beta answered 4006 to `EG05`, to `E1` (the sheet's `[EG0][1-4]` read literally) and to lowercase, and
/// nothing to the rest. The number is 1 to 8 digits; a ninth digit draws 4006. Type and series are not crossed.
pub(crate) fn despatch_number(text: &str) -> bool {
    let Some((series, number)) = text.split_once('-') else { return false };
    let number_ok = (1..=8).contains(&number.len()) && number.bytes().all(|b| b.is_ascii_digit());
    let alphanumeric = |s: &str| s.bytes().all(|b| b.is_ascii_uppercase() || b.is_ascii_digit());
    let series_ok = match series.as_bytes() {
        [b'T' | b'V', rest @ ..] if rest.len() == 3 => alphanumeric(&series[1..]),
        [b'G', rest @ ..] if rest.len() == 3 => rest.iter().all(u8::is_ascii_digit),
        _ => {
            (series.len() == 4 && series.bytes().all(|b| b.is_ascii_digit()))
                || matches!(series, "EG01" | "EG02" | "EG03" | "EG04" | "EG07")
        }
    };
    number_ok && series_ok
}
