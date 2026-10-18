//! `totals.total_taxes` against IGV + IVAP + ICBPER in every invoice and receipt: 3294 and 4301, within 1 (one currency unit, as `within_one_cents` and SUNAT's sheet).

use kembec_sunat_model::{Document, ModelError};
use serde_json::Value;

const INVOICE: &str = include_str!("../../../fixtures/01-invoice-taxed-cash.json");
const RECEIPT: &str = include_str!("../../../fixtures/03-receipt-consumer-dni.json");

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
