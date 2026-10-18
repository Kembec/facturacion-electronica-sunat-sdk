//! The purchase settlement (04): the vectors of its contract (`fixtures/self-billed/`) and one test per
//! SUNAT validation rule the SDK checks before sending. Each rule test edits the first vector so that
//! it breaks exactly that rule and expects SUNAT's code.

use kembec_sunat_model::{DateTime, Document, ModelError};
use serde_json::{json, Value};

/// 0501, two taxed lines, income tax retention 1.5 %.
const V1: &str = include_str!("../../../fixtures/self-billed/v1-taxed-purchase.json");
/// 0503, gold, retention 4 %, the four properties.
const V2: &str = include_str!("../../../fixtures/self-billed/v2-gold-retention-4.json");
const N1: &str = include_str!("../../../fixtures/self-billed/n1-seller-with-ruc-2800.json");
const N2: &str = include_str!("../../../fixtures/self-billed/n2-no-operation-place-2458.json");

fn with(base: &str, edit: impl FnOnce(&mut Value)) -> Result<Document, ModelError> {
    let mut value: Value = serde_json::from_str(base).unwrap();
    edit(&mut value);
    Document::from_json(&value.to_string())
}

fn expect_code(result: Result<Document, ModelError>, code: &str) {
    match result {
        Err(error) if error.sunat_code() == Some(code) => {
            assert!(error.to_string().ends_with(&format!("(SUNAT ERR-{code})")), "{error}");
        }
        other => panic!("expected SUNAT {code}, got {other:?}"),
    }
}

fn v1(edit: impl FnOnce(&mut Value)) -> Result<Document, ModelError> {
    with(V1, edit)
}

#[test]
fn the_valid_vectors_pass() {
    for (json, name) in [(V1, "20000000001-04-L001-1"), (V2, "20000000001-04-L001-2")] {
        let document = Document::from_json(json).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(document.file_name(), name);
        assert!(document.voucher().is_none());
    }
}

#[test]
fn the_negative_vectors_fail_with_their_code() {
    expect_code(Document::from_json(N1), "2800");
    expect_code(Document::from_json(N2), "2458");
}

#[test]
fn operation_type() {
    expect_code(v1(|d| d["operation_type"] = json!("0101")), "3206");
    assert!(matches!(v1(|d| d["operation_type"] = json!("0502")), Err(ModelError::OutOfScope { .. })));
}

#[test]
fn series_and_number() {
    expect_code(v1(|d| d["series"] = json!("F001")), "1001");
    expect_code(v1(|d| d["series"] = json!("0001")), "1001");
    expect_code(v1(|d| d["number"] = json!("123456789")), "1001");
}

#[test]
fn currency() {
    expect_code(v1(|d| d["currency"] = json!("soles")), "3088");
}

#[test]
fn seller_identity() {
    expect_code(v1(|d| d["seller"]["document_type"] = json!("6")), "2800");
    expect_code(v1(|d| d["seller"]["document_number"] = json!("")), "2014");
    expect_code(v1(|d| d["seller"]["document_number"] = json!("4012345")), "2802");
    expect_code(v1(|d| {
        d["seller"]["document_type"] = json!("4");
        d["seller"]["document_number"] = json!("AB 123");
    }), "2802");
    expect_code(v1(|d| {
        d["seller"]["document_type"] = json!("7");
        d["seller"]["document_number"] = json!("P1234567890123456");
    }), "2802");
}

#[test]
fn seller_name() {
    expect_code(v1(|d| d["seller"]["legal_name"] = json!(" ")), "2021");
    expect_code(v1(|d| d["seller"]["legal_name"] = json!("A".repeat(1501))), "2022");
}

#[test]
fn seller_address() {
    expect_code(v1(|d| d["seller"]["address"]["street"] = Value::Null), "2452");
    expect_code(v1(|d| d["seller"]["address"]["street"] = json!("AB")), "2593");
    expect_code(v1(|d| d["seller"]["address"]["street"] = json!(" CALLE 1")), "2593");
    expect_code(v1(|d| d["seller"]["address"]["ubigeo"] = Value::Null), "2453");
    expect_code(v1(|d| d["seller"]["address"]["address_type"] = Value::Null), "2456");
    expect_code(v1(|d| d["seller"]["address"]["address_type"] = json!("X")), "2457");
}

#[test]
fn operation_place() {
    expect_code(v1(|d| {
        d.as_object_mut().unwrap().remove("operation_place");
    }), "2458");
    expect_code(v1(|d| d["operation_place"]["street"] = Value::Null), "2454");
    expect_code(v1(|d| d["operation_place"]["street"] = json!("\tAV. 1")), "2778");
    expect_code(v1(|d| d["operation_place"]["ubigeo"] = Value::Null), "2455");
    expect_code(v1(|d| d["operation_place"]["ubigeo"] = json!("LIMA")), "2917");
    expect_code(v1(|d| d["operation_place"]["location_type"] = Value::Null), "2458");
    expect_code(v1(|d| d["operation_place"]["location_type"] = json!("1")), "2459");
}

#[test]
fn product_code() {
    expect_code(v1(|d| d["lines"][0]["sunat_product_code"] = Value::Null), "3506");
    expect_code(v1(|d| d["lines"][0]["sunat_product_code"] = json!("1114")), "3496");
}

#[test]
fn quantity_unit_description_and_prices() {
    expect_code(v1(|d| d["lines"][0]["quantity"] = json!("0")), "2024");
    expect_code(v1(|d| d["lines"][0]["quantity"] = json!("-1")), "2025");
    expect_code(v1(|d| d["lines"][0]["unit"] = json!("")), "2883");
    expect_code(v1(|d| d["lines"][0]["description"] = json!("")), "2026");
    expect_code(v1(|d| d["lines"][0]["description"] = json!("x".repeat(501))), "2027");
    expect_code(v1(|d| d["lines"][0]["unit_value"] = json!("0")), "2369");
    expect_code(v1(|d| d["lines"][0]["unit_price"] = Value::Null), "2028");
    expect_code(v1(|d| d["lines"][0]["unit_price"] = json!("-4.13")), "2367");
}

#[test]
fn igv_of_the_line() {
    expect_code(v1(|d| d["lines"][0]["igv"] = json!("128.00")), "3103");
    expect_code(v1(|d| d["lines"][0]["affectation"] = json!("20")), "3110");
    expect_code(v1(|d| d["lines"][0]["affectation"] = json!("40")), "2040");
    expect_code(v1(|d| d["lines"][0]["igv_rate"] = json!("-18")), "3102");
}

#[test]
fn income_tax_retention_of_the_line() {
    expect_code(v1(|d| d["lines"][0]["income_tax_retention"] = Value::Null), "2464");
    expect_code(v1(|d| d["lines"][0]["income_tax_retention"]["amount"] = json!("12.00")), "2464");
    expect_code(v1(|d| d["lines"][0]["income_tax_retention"]["amount"] = json!("-10.50")), "2033");
    expect_code(v1(|d| d["lines"][0]["income_tax_retention"]["rate"] = json!("3.00")), "4237");
}

#[test]
fn gold_properties() {
    for (index, code) in ["2466", "2467", "2468", "2469"].iter().enumerate() {
        expect_code(with(V2, |d| {
            d["lines"][0]["item_properties"].as_array_mut().unwrap().remove(index);
        }), code);
    }
    expect_code(with(V2, |d| d["lines"][0]["item_properties"][1]["value"] = json!("")), "3064");
}

#[test]
fn totals() {
    expect_code(v1(|d| d["totals"]["total_taxes"] = json!("232.05")), "4301");
    expect_code(v1(|d| d["totals"]["sale_value"] = json!("1200.00")), "4309");
    expect_code(v1(|d| d["totals"]["income_tax_retention_base"] = json!("1000.00")), "4345");
    expect_code(v1(|d| d["totals"]["income_tax_retention"] = json!("20.00")), "4346");
    expect_code(v1(|d| d["totals"]["subtotal"] = json!("1420.00")), "4310");
    expect_code(v1(|d| d["totals"]["payable"] = json!("1190.00")), "4312");
    expect_code(v1(|d| d["totals"]["rounding"] = json!("1.50")), "4314");
}

#[test]
fn send_deadline() {
    let document = Document::from_json(V1).unwrap();
    let Document::SelfBilled(settlement) = document else { panic!("not a settlement") };
    let at = |text: &str| text.parse::<DateTime>().unwrap();
    settlement.check_send_deadline(at("2026-10-19T23:00:00-05:00")).unwrap();
    expect_code(settlement.check_send_deadline(at("2026-10-20T08:00:00-05:00")).map(|_| Document::from_json(V1).unwrap()), "2108");
    expect_code(settlement.check_send_deadline(at("2026-10-17T08:00:00-05:00")).map(|_| Document::from_json(V1).unwrap()), "2329");
}
