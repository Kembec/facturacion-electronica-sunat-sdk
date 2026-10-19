//! The line rules of invoices and receipts the SDK checks before signing, each at the edge of the
//! sheet's tolerance of 1 (2026-04-24 sheet, «Factura2_0» and «Boleta2_0»): the line value (3271,
//! receipt 4288), the unit price (3270, receipt 4287) and the line IGV (3103 in both).

use kembec_sunat_model::{Document, ModelError};
use serde_json::{json, Value};

const RECEIPT: &str = include_str!("../../../fixtures/03-receipt-line-discount.json");
const INVOICE: &str = include_str!("../../../fixtures/01-invoice-taxed-cash.json");
const FREE: &str = include_str!("../../../fixtures/03-receipt-free-line.json");

fn edited(base: &str, edit: impl FnOnce(&mut Value)) -> Result<Document, ModelError> {
    let mut value: Value = serde_json::from_str(base).unwrap();
    edit(&mut value);
    Document::from_json(&value.to_string())
}

fn line(base: &str, field: &str, value: &str) -> Result<Document, ModelError> {
    edited(base, |v| v["lines"][0][field] = json!(value))
}

fn code(result: Result<Document, ModelError>) -> Option<&'static str> {
    result.expect_err("rejected").sunat_code()
}

#[test]
fn the_receipt_with_a_line_discount_still_passes() {
    Document::from_json(RECEIPT).unwrap();
    Document::from_json(INVOICE).unwrap();
    Document::from_json(FREE).unwrap();
}

/// 16.95 × 3 − 5.09 = 45.76: a `unit_value` of 17.2833 gives 46.7599 (0.9999 off), 17.2834 gives
/// 46.7602 (1.0002 off).
#[test]
fn receipt_line_value_4288() {
    line(RECEIPT, "unit_value", "17.2833").unwrap();
    assert_eq!(code(line(RECEIPT, "unit_value", "17.2834")), Some("4288"));
    let net = edited(RECEIPT, |v| {
        v["lines"][0]["unit_value"] = json!("15.253333");
    });
    assert_eq!(code(net), Some("4288"), "a unit_value already net of the discount 00");
    let invoice = edited(RECEIPT, |v| {
        v["document_type"] = json!("01");
        v["series"] = json!("F001");
        v["customer"]["document_type"] = json!("6");
        v["customer"]["document_number"] = json!("20100070970");
        v["lines"][0]["unit_value"] = json!("17.2834");
    });
    assert_eq!(code(invoice), Some("3271"));
}

/// (45.76 + 8.24) ÷ 3 = 18.00 on the receipt; (100.00 + 18.00) ÷ 2 = 59.00 on the invoice.
#[test]
fn unit_price_4287_and_3270() {
    line(RECEIPT, "unit_price", "19.00").unwrap();
    line(RECEIPT, "unit_price", "17.00").unwrap();
    line(RECEIPT, "unit_price", "18.0000000001").unwrap();
    assert_eq!(code(line(RECEIPT, "unit_price", "19.01")), Some("4287"));
    assert_eq!(code(line(RECEIPT, "unit_price", "16.99")), Some("4287"));
    line(INVOICE, "unit_price", "60.00").unwrap();
    assert_eq!(code(line(INVOICE, "unit_price", "60.01")), Some("3270"));
    assert_eq!(code(line(INVOICE, "unit_price", "57.9999999999")), Some("3270"));
}

/// A discount 01 does not reduce `line_value` but comes off the price the buyer pays.
#[test]
fn unit_price_takes_off_the_discount_01() {
    let discounted = |price: &str| {
        edited(INVOICE, |v| {
            v["lines"][0]["discounts"] = json!([{ "code": "01", "factor": "0.05", "amount": "5.90", "base_amount": "118.00" }]);
            v["lines"][0]["unit_price"] = json!(price);
            v["totals"]["allowance_total"] = json!("5.90");
            v["totals"]["payable"] = json!("142.10");
        })
    };
    discounted("56.05").unwrap();
    assert_eq!(code(discounted("59.00")), Some("3270"), "(100 + 18 − 5.90) ÷ 2 = 56.05");
}

#[test]
fn a_free_line_keeps_its_unit_price_unchecked() {
    edited(FREE, |v| v["lines"][1]["unit_price"] = json!("0.00")).unwrap();
}

/// 45.76 × 18 % = 8.2368 → 8.24 on the receipt; 100.00 × 18 % = 18.00 on the invoice.
#[test]
fn line_igv_3103() {
    line(RECEIPT, "igv", "9.24").unwrap();
    line(RECEIPT, "igv", "7.24").unwrap();
    assert_eq!(code(line(RECEIPT, "igv", "9.25")), Some("3103"));
    assert_eq!(code(line(RECEIPT, "igv", "7.23")), Some("3103"));
    line(INVOICE, "igv", "19.00").unwrap();
    assert_eq!(code(line(INVOICE, "igv", "19.01")), Some("3103"));
    assert_eq!(code(edited(FREE, |v| v["lines"][1]["igv"] = json!("16.99"))), Some("3103"), "a free taxed line (13)");
}
