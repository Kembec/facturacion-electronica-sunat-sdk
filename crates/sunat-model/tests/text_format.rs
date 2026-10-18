//! Format observations on free text (4092, 4094 to 4098, 4338, 4234, 4269): refused with the field,
//! never rewritten. Beta observed 4094 on a receipt whose issuer street had a line break (18/10/2026).

use kembec_sunat_model::{Document, ModelError};

const INVOICE: &str = include_str!("../../../fixtures/01-invoice-taxed-cash.json");
const RECEIPT: &str = include_str!("../../../fixtures/03-receipt-consumer-dni.json");
const CREDIT_NOTE: &str = include_str!("../../../fixtures/07-credit-note-void.json");
const DEBIT_NOTE: &str = include_str!("../../../fixtures/08-debit-note-value-increase.json");

fn set(base: &str, pointer: &str, value: &str) -> Result<Document, ModelError> {
    let mut json: serde_json::Value = serde_json::from_str(base).unwrap();
    *json.pointer_mut(pointer).unwrap() = serde_json::json!(value);
    Document::from_json(&json.to_string())
}

fn refused(base: &str, pointer: &str, path: &str, code: &str, value: &str) {
    match set(base, pointer, value) {
        Err(ModelError::Invalid { path: got, reason }) => {
            assert_eq!(got, path, "{value:?}");
            assert!(reason.contains(&format!("SUNAT {code}")), "{reason}");
        }
        other => panic!("{path} {value:?}: {other:?}"),
    }
}

/// Every whitespace SUNAT names, and the others of Java's `\s`; the space and a no-break space pass.
fn assert_borders(base: &str, pointer: &str, path: &str, code: &str, min: usize, max: usize) {
    let filler = |n: usize| "Ñ".repeat(n);
    for value in [filler(min), filler(max), "AV. LOS ÁLAMOS 1".chars().take(max).collect(), format!("A\u{a0}B{}", filler(min.saturating_sub(3)))] {
        assert!(set(base, pointer, &value).is_ok(), "{path} {value:?}");
    }
    let mut bad = vec![filler(max + 1)];
    if min > 1 {
        bad.push(filler(min - 1));
    }
    for ws in ['\n', '\r', '\t', '\u{0B}', '\u{0C}'] {
        bad.push(format!("AB{ws}C"));
    }
    for value in bad {
        refused(base, pointer, path, code, &value);
    }
    assert!(set(base, pointer, "").is_ok(), "{path}: an empty value writes no tag");
}

#[test]
fn issuer_text_fields_follow_the_sheet_in_every_voucher() {
    let fields = [
        ("/issuer/address/street", "$.issuer.address.street", "4094", 3, 200),
        ("/issuer/address/urbanization", "$.issuer.address.urbanization", "4095", 1, 25),
        ("/issuer/address/province", "$.issuer.address.province", "4096", 1, 30),
        ("/issuer/address/department", "$.issuer.address.department", "4097", 1, 30),
        ("/issuer/address/district", "$.issuer.address.district", "4098", 1, 30),
    ];
    for base in [INVOICE, RECEIPT, CREDIT_NOTE, DEBIT_NOTE] {
        for (pointer, path, code, min, max) in fields {
            assert_borders(base, pointer, path, code, min, max);
        }
    }
}

#[test]
fn issuer_names_take_one_character_in_invoices_and_three_in_notes() {
    for (base, min) in [(INVOICE, 1), (RECEIPT, 1), (CREDIT_NOTE, 3), (DEBIT_NOTE, 3)] {
        assert_borders(base, "/issuer/trade_name", "$.issuer.trade_name", "4092", min, 1500);
        for value in ["AB\nC", "AB\tC", &"Ñ".repeat(1501)] {
            refused(base, "/issuer/legal_name", "$.issuer.legal_name", "4338", value);
        }
    }
    refused(CREDIT_NOTE, "/issuer/legal_name", "$.issuer.legal_name", "4338", "AB");
    assert!(set(INVOICE, "/issuer/legal_name", "AB").is_ok());
}

#[test]
fn product_code_is_4269_in_invoices_and_4234_in_notes() {
    for (base, code) in [(INVOICE, "4269"), (RECEIPT, "4269"), (CREDIT_NOTE, "4234"), (DEBIT_NOTE, "4234")] {
        assert_borders(base, "/lines/0/product_code", "$.lines[0].product_code", code, 1, 30);
    }
}

#[test]
fn the_value_is_refused_not_rewritten() {
    let document = set(INVOICE, "/issuer/address/street", "AV. EJEMPLO  123 ").unwrap();
    assert_eq!(document.voucher().unwrap().issuer.address.street.as_deref(), Some("AV. EJEMPLO  123 "));
}
