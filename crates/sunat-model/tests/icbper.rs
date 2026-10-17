//! One test per ICBPER rule of the contract (§5): each edits a valid vector of `fixtures/icbper/`
//! and expects the SUNAT code the sheet answers for that document type.

use kembec_sunat_model::{Document, ModelError};
use serde_json::{json, Value};

const INVOICE: &str = include_str!("../../../fixtures/icbper/v1-invoice-bags.json");
const RECEIPT: &str = include_str!("../../../fixtures/icbper/v2-receipt-bags.json");
const FREE: &str = include_str!("../../../fixtures/icbper/v3-receipt-free-bags.json");
const CREDIT_NOTE: &str = include_str!("../../../fixtures/icbper/v4-credit-note-bags.json");
const SUMMARY: &str = include_str!("../../../fixtures/icbper/v5-rc-icbper.json");
const N1: &str = include_str!("../../../fixtures/icbper/n1-amount-not-unit-times-quantity.json");
const N2: &str = include_str!("../../../fixtures/icbper/n2-total-not-sum-of-lines.json");
const N3: &str = include_str!("../../../fixtures/icbper/n3-stale-rate.json");

fn edited(base: &str, edit: impl FnOnce(&mut Value)) -> Result<Document, ModelError> {
    let mut value: Value = serde_json::from_str(base).unwrap();
    edit(&mut value);
    Document::from_json(&value.to_string())
}

fn code(base: &str, edit: impl FnOnce(&mut Value)) -> &'static str {
    match edited(base, edit) {
        Err(error) => error.sunat_code().unwrap_or_else(|| panic!("no SUNAT code: {error}")),
        Ok(_) => panic!("accepted"),
    }
}

fn invalid_mentions(result: Result<Document, ModelError>, code: &str) {
    match result {
        Err(ModelError::Invalid { reason, .. }) => assert!(reason.contains(code), "{reason}"),
        other => panic!("{code}: {other:?}"),
    }
}

/// The bag line of the vector: the last one.
fn bag(v: &mut Value) -> &mut Value {
    let lines = v["lines"].as_array_mut().unwrap();
    lines.last_mut().unwrap()
}

#[test]
fn the_five_valid_vectors_parse() {
    for base in [INVOICE, RECEIPT, FREE, CREDIT_NOTE, SUMMARY] {
        Document::from_json(base).unwrap();
    }
}

#[test]
fn the_three_invalid_vectors_fail_with_their_codes() {
    let code = |base: &str| Document::from_json(base).err().and_then(|e| e.sunat_code()).expect("rejected with a code");
    assert_eq!(code(N1), "4318");
    assert_eq!(code(N2), "3306");
    assert_eq!(code(N3), "4237");
}

#[test]
fn amount_format_2033_and_total_format_2048() {
    invalid_mentions(edited(INVOICE, |v| bag(v)["icbper"] = json!("0")), "2033");
    invalid_mentions(edited(INVOICE, |v| bag(v)["icbper"] = json!("2.001")), "2033");
    invalid_mentions(edited(INVOICE, |v| v["totals"]["icbper"] = json!("-2.00")), "2048");
    invalid_mentions(edited(SUMMARY, |v| v["lines"][0]["icbper"] = json!("0.00")), "2048");
}

#[test]
fn bags_are_a_whole_quantity_from_1_to_99999_2892() {
    assert_eq!(code(INVOICE, |v| bag(v)["quantity"] = json!("4.5")), "2892");
    assert_eq!(code(INVOICE, |v| bag(v)["quantity"] = json!("100000")), "2892");
    assert_eq!(code(INVOICE, |v| bag(v)["quantity"] = json!("0")), "2892");
    invalid_mentions(edited(INVOICE, |v| bag(v)["icbper_factor"] = json!("0.123456")), "2892");
}

#[test]
fn factor_is_required_and_not_zero_3238() {
    assert_eq!(code(INVOICE, |v| bag(v).as_object_mut().unwrap().remove("icbper_factor").map(drop).unwrap()), "3238");
    assert_eq!(code(RECEIPT, |v| bag(v)["icbper_factor"] = json!("0.00")), "3238");
    let factor_alone = edited(INVOICE, |v| {
        bag(v).as_object_mut().unwrap().remove("icbper");
    });
    assert!(matches!(factor_alone, Err(ModelError::Invalid { .. })), "{factor_alone:?}");
}

#[test]
fn factor_is_the_rate_of_the_year_of_issue_4237() {
    assert_eq!(code(INVOICE, |v| bag(v)["icbper_factor"] = json!("0.40")), "4237");
    assert_eq!(code(CREDIT_NOTE, |v| v["lines"][0]["icbper_factor"] = json!("0.10")), "4237");
    // 31/12/2022 23:30 in Lima is still 2022: S/ 0.40, and 4 bags make 1.60.
    let in_2022 = edited(INVOICE, |v| {
        v["issue_date"] = json!("2022-12-31T23:30:00-05:00");
        bag(v)["icbper_factor"] = json!("0.40");
        bag(v)["icbper"] = json!("1.60");
        bag(v)["total_taxes"] = json!("1.64");
        v["totals"]["icbper"] = json!("1.60");
        v["totals"]["total_taxes"] = json!("19.64");
        v["totals"]["subtotal"] = json!("119.84");
        v["totals"]["payable"] = json!("119.84");
    });
    in_2022.unwrap();
    // The same instant written in UTC is 2023-01-01 04:30Z: still Lima's 2022.
    let utc = edited(INVOICE, |v| v["issue_date"] = json!("2023-01-01T04:30:00Z"));
    assert_eq!(utc.err().and_then(|e| e.sunat_code()), Some("4237"));
}

#[test]
fn amount_is_factor_times_bags_4318() {
    assert_eq!(code(RECEIPT, |v| bag(v)["icbper"] = json!("1.00")), "4318");
}

#[test]
fn not_in_force_before_august_2019_2949() {
    assert_eq!(code(INVOICE, |v| v["issue_date"] = json!("2019-07-31T10:00:00-05:00")), "2949");
}

#[test]
fn total_is_the_sum_of_the_lines_3306_or_4321() {
    assert_eq!(code(RECEIPT, |v| {
        v["totals"]["icbper"] = json!("1.00");
        v["totals"]["total_taxes"] = json!("10.03");
    }), "4321");
    assert_eq!(code(CREDIT_NOTE, |v| {
        v["totals"]["icbper"] = json!("0.50");
        v["totals"]["total_taxes"] = json!("0.52");
    }), "3306");
    let missing = edited(INVOICE, |v| {
        v["totals"].as_object_mut().unwrap().remove("icbper");
    });
    assert!(matches!(missing, Err(ModelError::Missing { .. })), "{missing:?}");
    let without_lines = edited(INVOICE, |v| {
        let line = bag(v).as_object_mut().unwrap();
        line.remove("icbper");
        line.remove("icbper_factor");
    });
    assert!(matches!(without_lines, Err(ModelError::Invalid { .. })), "{without_lines:?}");
}

#[test]
fn line_taxes_include_icbper_3292_or_4293() {
    assert_eq!(code(INVOICE, |v| bag(v)["total_taxes"] = json!("0.04")), "3292");
    assert_eq!(code(RECEIPT, |v| bag(v)["total_taxes"] = json!("0.03")), "4293");
}

#[test]
fn total_taxes_include_icbper_3294_or_4301() {
    assert_eq!(code(INVOICE, |v| v["totals"]["total_taxes"] = json!("18.04")), "3294");
    assert_eq!(code(FREE, |v| v["totals"]["total_taxes"] = json!("9.00")), "4301");
}

#[test]
fn subtotal_and_payable_include_icbper() {
    let total = |value: &'static str| move |v: &mut Value| v["totals"]["subtotal"] = json!(value);
    assert_eq!(code(INVOICE, total("118.24")), "3279");
    assert_eq!(code(RECEIPT, total("58.68")), "4310");
    assert_eq!(code(INVOICE, |v| v["totals"]["payable"] = json!("118.24")), "3280");
    assert_eq!(code(CREDIT_NOTE, |v| v["totals"]["payable"] = json!("0.02")), "3280");
}

#[test]
fn icbper_outside_pen_is_out_of_scope() {
    for base in [INVOICE, SUMMARY] {
        match edited(base, |v| v["currency"] = json!("USD")) {
            Err(ModelError::OutOfScope { feature, .. }) => assert!(feature.contains("PEN"), "{feature}"),
            other => panic!("{other:?}"),
        }
    }
}
