//! Debit note 13 «Penalidades» (catalog 10), by the date of its `issue_date`: until 2026-12-31 penalties
//! go under `03` and `13` does not exist; from 2027-01-01 `13` is the code (`03` answers 2524) and the
//! note is issued only on unaffected operations (3507; RS 048-2026 annex VI note 8). Dates in `rule_dates`.

use crate::rule_dates::{applies, from, DEBIT_NOTE_13};
use crate::{Decimal, DocumentType, ModelError, Note};

/// Catalog 07 unaffected operations (30 to 37); exports (40), exempt (20, 21) and taxed ones are not.
fn is_unaffected(code: &str) -> bool {
    matches!(code, "30" | "31" | "32" | "33" | "34" | "35" | "36" | "37")
}

fn given(amount: &Option<Decimal>) -> bool {
    amount.as_ref().is_some_and(|a| !a.is_zero())
}

pub(crate) fn check(note: &Note) -> Result<(), ModelError> {
    let voucher = &note.voucher;
    if voucher.document_type != DocumentType::DebitNote {
        return Ok(());
    }
    let code = note.reference.reason_code.as_str();
    let date = voucher.issue_date.to_lima().date_string();
    let since = from(DEBIT_NOTE_13);
    let in_force = applies(DEBIT_NOTE_13, &date);
    match (code, in_force) {
        ("13", false) => {
            let reason = format!("catalog 10 has no code 13 until {since}, and issue_date is {date}: use \"03\" until then");
            Err(ModelError::invalid("$.note.reason_code", reason))
        }
        ("03", true) => {
            let reason = format!("penalties are code \"13\" from {since} (SUNAT 2524), and issue_date is {date}");
            Err(ModelError::rule("2524", "$.note.reason_code", reason))
        }
        ("13", true) => check_unaffected(note),
        _ => Ok(()),
    }
}

/// 3507: no line outside the unaffected operations, and none of the totals SUNAT names (taxed, exempt, export).
fn check_unaffected(note: &Note) -> Result<(), ModelError> {
    let voucher = &note.voucher;
    if let Some(index) = voucher.lines.iter().position(|line| !is_unaffected(&line.affectation)) {
        let reason = "a debit note 13 (penalties) takes unaffected operations only, affectation 30 to 37";
        return Err(ModelError::rule("3507", format!("$.lines[{index}].affectation"), reason));
    }
    let totals = &voucher.totals;
    for (field, amount) in [("taxed", &totals.taxed), ("exempt", &totals.exempt), ("export", &totals.export)] {
        if given(amount) {
            let reason = "a debit note 13 (penalties) carries no taxed, exempt or export total";
            return Err(ModelError::rule("3507", format!("$.totals.{field}"), reason));
        }
    }
    Ok(())
}
