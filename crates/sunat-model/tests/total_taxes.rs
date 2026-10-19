//! `totals.total_taxes` against IGV + IVAP + ICBPER in every invoice, receipt and note: 3294 and 4301, within 1 (one currency unit, as `within_one_cents` and SUNAT's sheet).
//! A note answers by the document it modifies: 3294 on an invoice (01), 4301 on anything else.

use kembec_sunat_model::{Document, ModelError};
use serde_json::Value;

const INVOICE: &str = include_str!("../../../fixtures/01-invoice-taxed-cash.json");
const RECEIPT: &str = include_str!("../../../fixtures/03-receipt-consumer-dni.json");
const CREDIT_NOTE: &str = include_str!("../../../fixtures/07-credit-note-partial-return.json");
const DEBIT_NOTE: &str = include_str!("../../../fixtures/08-debit-note-value-increase.json");

fn with_total_taxes(base: &str, total: &str) -> Result<Document, ModelError> {
    let mut value: Value = serde_json::from_str(base).unwrap();
    value["totals"]["total_taxes"] = Value::String(total.into());
    Document::from_json(&value.to_string())
}

fn code(base: &str, cents: i64) -> Option<&'static str> {
    let value: Value = serde_json::from_str(base).unwrap();
    let igv: i64 = value["totals"]["igv"].as_str().unwrap().replace('.', "").parse().unwrap();
    let total = igv + cents;
    let total = format!("{}.{:02}", total / 100, total % 100);
    with_total_taxes(base, &total).err().map(|e| e.sunat_code().expect("SUNAT code"))
}

#[test]
fn invoice_one_unit_and_a_cent_off_is_3294() {
    assert_eq!(code(INVOICE, 101), Some("3294"));
    assert_eq!(code(INVOICE, -101), Some("3294"));
}

#[test]
fn invoice_one_unit_off_passes() {
    for delta in [-100, 0, 100] {
        assert_eq!(code(INVOICE, delta), None);
    }
}

#[test]
fn receipt_one_unit_and_a_cent_off_is_4301() {
    assert_eq!(code(RECEIPT, 101), Some("4301"));
    assert_eq!(code(RECEIPT, 100), None);
}

/// The note, retargeted to a receipt (03) when `affected` says so, with `total_taxes` = IGV + `cents`.
fn note(base: &str, affected: &str, cents: i64) -> Result<Document, ModelError> {
    let mut value: Value = serde_json::from_str(base).unwrap();
    if affected == "03" {
        value["series"] = Value::String(value["series"].as_str().unwrap().replacen('F', "B", 1));
        value["note"]["affected_document_type"] = Value::String("03".into());
        value["note"]["affected_document_number"] = Value::String("B001-1".into());
    }
    let igv: i64 = value["totals"]["igv"].as_str().unwrap().replace('.', "").parse().unwrap();
    let total = igv + cents;
    value["totals"]["total_taxes"] = Value::String(format!("{}.{:02}", total / 100, total % 100));
    Document::from_json(&value.to_string())
}

fn rule(result: Result<Document, ModelError>) -> (&'static str, String) {
    match result {
        Err(ModelError::Rule { code, path, .. }) => (code, path),
        other => panic!("expected a SUNAT rule, got {other:?}"),
    }
}

#[test]
fn note_one_unit_off_passes() {
    for base in [CREDIT_NOTE, DEBIT_NOTE] {
        for affected in ["01", "03"] {
            for delta in [-100, 0, 100] {
                assert!(note(base, affected, delta).is_ok(), "{affected} {delta}");
            }
        }
    }
}

#[test]
fn note_on_an_invoice_one_unit_and_a_cent_off_is_3294() {
    for base in [CREDIT_NOTE, DEBIT_NOTE] {
        for delta in [-101, 101] {
            assert_eq!(rule(note(base, "01", delta)), ("3294", "$.totals.total_taxes".to_string()));
        }
    }
}

#[test]
fn note_on_a_receipt_one_unit_and_a_cent_off_is_4301() {
    for base in [CREDIT_NOTE, DEBIT_NOTE] {
        for delta in [-101, 101] {
            assert_eq!(rule(note(base, "03", delta)), ("4301", "$.totals.total_taxes".to_string()));
        }
    }
}
