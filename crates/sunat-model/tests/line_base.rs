//! SUNAT 3272 and 4294: without ISC, the IGV base of a line (`igv_base`, `cbc:TaxableAmount`) equals its
//! value (`line_value`, `cbc:LineExtensionAmount`) to the cent. Error 3272 in an invoice and in a note
//! that modifies one; observation 4294 in a receipt and in a note that modifies one, rejected locally.

use kembec_sunat_model::{Document, ModelError};
use serde_json::{json, Value};

const INVOICE: &str = include_str!("../../../fixtures/01-invoice-taxed-cash.json");
const RECEIPT: &str = include_str!("../../../fixtures/03-receipt-consumer-dni.json");
const CREDIT_NOTE: &str = include_str!("../../../fixtures/07-credit-note-partial-return.json");
const DEBIT_NOTE: &str = include_str!("../../../fixtures/08-debit-note-value-increase.json");

fn rule(base: &str, edit: impl FnOnce(&mut Value)) -> (&'static str, String) {
    let mut value: Value = serde_json::from_str(base).unwrap();
    edit(&mut value);
    match Document::from_json(&value.to_string()) {
        Err(ModelError::Rule { code, path, .. }) => (code, path),
        other => panic!("expected a SUNAT rule, got {other:?}"),
    }
}

fn base_off_by_a_cent(v: &mut Value) {
    let value: f64 = v["lines"][0]["line_value"].as_str().unwrap().parse().unwrap();
    v["lines"][0]["igv_base"] = json!(format!("{:.2}", value - 0.01));
}

#[test]
fn invoice_line_base_other_than_its_value_3272() {
    assert_eq!(rule(INVOICE, base_off_by_a_cent), ("3272", "$.lines[0].igv_base".into()));
}

#[test]
fn receipt_line_base_other_than_its_value_4294() {
    assert_eq!(rule(RECEIPT, base_off_by_a_cent), ("4294", "$.lines[0].igv_base".into()));
}

#[test]
fn notes_take_3272_on_an_invoice_and_4294_on_a_receipt() {
    for note in [CREDIT_NOTE, DEBIT_NOTE] {
        assert_eq!(rule(note, base_off_by_a_cent).0, "3272");
        let on_receipt = |v: &mut Value| {
            v["note"]["affected_document_type"] = json!("03");
            v["note"]["affected_document_number"] = json!("B001-1");
            base_off_by_a_cent(v);
        };
        assert_eq!(rule(note, on_receipt).0, "4294");
    }
}

#[test]
fn the_same_amount_written_with_other_decimals_passes() {
    let mut value: Value = serde_json::from_str(INVOICE).unwrap();
    let line_value = value["lines"][0]["line_value"].as_str().unwrap().to_string();
    value["lines"][0]["igv_base"] = json!(format!("{line_value}0"));
    Document::from_json(&value.to_string()).unwrap();
}
