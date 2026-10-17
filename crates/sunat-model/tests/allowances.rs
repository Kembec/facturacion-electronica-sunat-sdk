//! One test per rule of the discounts and charges contract (§5): each edits a valid vector of
//! `fixtures/allowances/` and expects the SUNAT code the sheet answers.

use kembec_sunat_model::{Document, ModelError};
use serde_json::{json, Value};

const GLOBAL_02: &str = include_str!("../../../fixtures/allowances/inv-01-descuento-global-02-afecta-base.json");
const GLOBAL_03: &str = include_str!("../../../fixtures/allowances/inv-02-descuento-global-03-no-afecta-base.json");
const GLOBAL_46: &str = include_str!("../../../fixtures/allowances/inv-04-recargo-consumo-46-fuera-de-base.json");
const FISE: &str = include_str!("../../../fixtures/allowances/inv-05-fise-45.json");
const LINE_47: &str = include_str!("../../../fixtures/allowances/inv-06-cargo-linea-47-afecta-base.json");

fn edited(base: &str, edit: impl FnOnce(&mut Value)) -> Result<Document, ModelError> {
    let mut value: Value = serde_json::from_str(base).unwrap();
    edit(&mut value);
    Document::from_json(&value.to_string())
}

fn code(base: &str, edit: impl FnOnce(&mut Value)) -> Option<&'static str> {
    edited(base, edit).expect_err("rejected").sunat_code()
}

fn receipt(v: &mut Value) {
    v["document_type"] = json!("03");
    v["series"] = json!("B001");
}

fn invalid_mentions(result: Result<Document, ModelError>, code: &str) {
    match result {
        Err(ModelError::Invalid { reason, .. }) => assert!(reason.contains(code), "{reason}"),
        other => panic!("{code}: {other:?}"),
    }
}

#[test]
fn every_valid_vector_parses() {
    for base in [GLOBAL_02, GLOBAL_03, GLOBAL_46, FISE, LINE_47] {
        Document::from_json(base).unwrap();
    }
}

#[test]
fn global_codes_outside_the_global_set() {
    assert_eq!(code(GLOBAL_02, |v| v["global_adjustments"][0]["code"] = json!("99")), Some("3071"));
    assert_eq!(code(GLOBAL_02, |v| v["global_adjustments"][0]["code"] = json!("00")), Some("3071"));
    assert_eq!(
        code(GLOBAL_02, |v| {
            receipt(v);
            v["global_adjustments"][0]["code"] = json!("01")
        }),
        Some("4291")
    );
    assert_eq!(code(GLOBAL_02, |v| v["global_adjustments"][0]["code"] = json!("")), Some("3072"));
    match edited(GLOBAL_02, |v| v["global_adjustments"][0]["code"] = json!("04")) {
        Err(ModelError::OutOfScope { .. }) => {}
        other => panic!("advance 04: {other:?}"),
    }
    let retention = edited(GLOBAL_02, |v| v["global_adjustments"][0]["code"] = json!("62"));
    assert!(matches!(retention, Err(ModelError::Invalid { .. })), "{retention:?}");
}

#[test]
fn line_codes_outside_the_line_set() {
    let charge = |c: &'static str| move |v: &mut Value| v["lines"][0]["charges"][0]["code"] = json!(c);
    assert_eq!(code(LINE_47, charge("")), Some("3073"));
    assert_eq!(code(LINE_47, charge("49")), Some("4268"));
    assert_eq!(code(LINE_47, charge("98")), Some("2954"));
    assert!(matches!(edited(LINE_47, charge("00")), Err(ModelError::Invalid { .. })), "a discount in charges");
}

#[test]
fn formats_carry_their_sunat_codes() {
    let global = |field: &'static str, value: &'static str| move |v: &mut Value| v["global_adjustments"][0][field] = json!(value);
    invalid_mentions(edited(GLOBAL_02, global("factor", "0.123456")), "3025");
    invalid_mentions(edited(GLOBAL_02, global("amount", "25.001")), "2968");
    invalid_mentions(edited(GLOBAL_02, global("base_amount", "-250.00")), "3016");
    let line = |field: &'static str, value: &'static str| move |v: &mut Value| v["lines"][0]["charges"][0][field] = json!(value);
    invalid_mentions(edited(LINE_47, line("factor", "1000")), "3052");
    invalid_mentions(edited(LINE_47, line("amount", "0")), "2955");
    invalid_mentions(edited(LINE_47, line("base_amount", "1234567890123")), "3053");
    invalid_mentions(edited(GLOBAL_03, |v| v["totals"]["allowance_total"] = json!("12.505")), "2065");
    invalid_mentions(edited(GLOBAL_46, |v| v["totals"]["charge_total"] = json!("-25.00")), "2064");
}

#[test]
fn amount_is_base_times_factor_within_one() {
    let global = |amount: &'static str| move |v: &mut Value| v["global_adjustments"][0]["amount"] = json!(amount);
    // 25.99 is within 1 of 25.00; the totals then differ by less than 1 too.
    assert!(edited(GLOBAL_46, global("25.99")).is_ok());
    assert_eq!(code(GLOBAL_46, global("26.01")), Some("3307"));
    assert_eq!(
        code(GLOBAL_46, |v| {
            receipt(v);
            global("27.00")(v)
        }),
        Some("4322")
    );
    let line = |v: &mut Value| v["lines"][0]["charges"][0]["factor"] = json!("0.06");
    assert_eq!(code(LINE_47, line), Some("3290"));
    assert_eq!(
        code(LINE_47, |v| {
            receipt(v);
            line(v)
        }),
        Some("4289")
    );
}

#[test]
fn fise_needs_a_base_and_an_amount_above_zero() {
    assert_eq!(code(FISE, |v| v["global_adjustments"][0]["base_amount"] = json!("0.00")), Some("3092"));
    assert_eq!(code(FISE, |v| v["global_adjustments"][0]["amount"] = json!("0")), Some("3074"));
}

#[test]
fn line_value_carries_the_line_charge_47() {
    let gross = |v: &mut Value| {
        v["lines"][0]["line_value"] = json!("200.00");
        v["lines"][0]["igv_base"] = json!("200.00");
    };
    assert_eq!(code(LINE_47, gross), Some("3271"));
}

#[test]
fn totals_51_and_52() {
    assert_eq!(code(GLOBAL_03, |v| v["totals"].as_object_mut().unwrap().remove("allowance_total").map(drop).unwrap()), Some("3300"));
    assert_eq!(code(GLOBAL_46, |v| v["totals"]["charge_total"] = json!("23.00")), Some("3301"));
    assert_eq!(
        code(GLOBAL_46, |v| {
            receipt(v);
            v["totals"]["charge_total"] = json!("23.00")
        }),
        Some("4308")
    );
}

#[test]
fn payable_adds_charges_and_subtracts_allowances() {
    assert_eq!(code(GLOBAL_03, |v| v["totals"]["payable"] = json!("295.00")), Some("3280"));
    assert_eq!(code(GLOBAL_46, |v| v["totals"]["payable"] = json!("295.00")), Some("3280"));
}

#[test]
fn base_igv_sale_value_and_subtotal_are_net_of_02() {
    assert_eq!(code(GLOBAL_02, |v| v["totals"]["taxed"] = json!("250.00")), Some("3277"));
    assert_eq!(code(GLOBAL_02, |v| v["totals"]["igv"] = json!("45.00")), Some("3291"));
    assert_eq!(code(GLOBAL_02, |v| v["totals"]["sale_value"] = json!("250.00")), Some("3278"));
    let gross_subtotal = |v: &mut Value| {
        v["totals"]["subtotal"] = json!("290.50");
        v["totals"]["payable"] = json!("290.50");
    };
    assert_eq!(code(GLOBAL_02, gross_subtotal), Some("3279"));
    assert_eq!(code(GLOBAL_02, |v| { receipt(v); v["totals"]["taxed"] = json!("250.00") }), Some("4299"));
    assert_eq!(code(GLOBAL_02, |v| { receipt(v); v["totals"]["igv"] = json!("45.00") }), Some("4290"));
}

#[test]
fn global_02_with_two_igv_rates_is_out_of_scope() {
    let two_rates = |v: &mut Value| {
        let line = &mut v["lines"][1];
        line["igv_rate"] = json!("10.50");
        line["igv"] = json!("5.25");
    };
    match edited(GLOBAL_02, two_rates) {
        Err(ModelError::OutOfScope { feature, .. }) => assert!(feature.contains("two IGV rates"), "{feature}"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn notes_take_no_global_adjustments_nor_line_charges() {
    const NOTE: &str = include_str!("../../../fixtures/07-credit-note-void.json");
    let adjustment = json!([{ "code": "02", "factor": "0.1", "amount": "1", "base_amount": "10" }]);
    assert!(edited(NOTE, |v| v["global_adjustments"] = adjustment.clone()).is_err());
    let charge = json!([{ "code": "47", "factor": "0.1", "amount": "1", "base_amount": "10" }]);
    match edited(NOTE, |v| v["lines"][0]["charges"] = charge) {
        Err(ModelError::OutOfScope { feature, .. }) => assert_eq!(feature, "line charges in credit and debit notes"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn charge_indicator_follows_the_code() {
    let document = Document::from_json(LINE_47).unwrap();
    let Document::Invoice(invoice) = &document else { panic!("invoice") };
    assert!(invoice.voucher.lines[0].charges[0].is_charge());
    let document = Document::from_json(GLOBAL_02).unwrap();
    let Document::Invoice(invoice) = &document else { panic!("invoice") };
    assert!(!invoice.global_adjustments[0].is_charge());
}
