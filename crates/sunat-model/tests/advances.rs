//! One test per rule of the advances contract: each edits a valid vector of `fixtures/advances/` and
//! expects the SUNAT code the sheet answers. The totals follow the convention the beta accepted
//! (`fixtures/advances/beta/`, `advances_beta.rs`).

use kembec_sunat_model::{Document, ModelError};
use serde_json::{json, Value};

const V1: &str = include_str!("../../../fixtures/advances/v1-invoice-advance.json");
const V1B: &str = include_str!("../../../fixtures/advances/v1b-invoice-advance-second.json");
const V1C: &str = include_str!("../../../fixtures/advances/v1c-receipt-advance.json");
const V2: &str = include_str!("../../../fixtures/advances/v2-invoice-final-one-advance.json");
const V3: &str = include_str!("../../../fixtures/advances/v3-invoice-final-two-advances.json");
const V4: &str = include_str!("../../../fixtures/advances/v4-receipt-final-advance.json");
const N1: &str = include_str!("../../../fixtures/advances/n1-prepaid-not-sum-of-advances.json");
const N2: &str = include_str!("../../../fixtures/advances/n2-igv-on-gross-base.json");
const N3: &str = include_str!("../../../fixtures/advances/n3-advance-not-found.json");
const S3: &str = include_str!("../../../fixtures/advances/s3-discount-with-factor.json");

fn edited(base: &str, edit: impl FnOnce(&mut Value)) -> Result<Document, ModelError> {
    let mut value: Value = serde_json::from_str(base).unwrap();
    edit(&mut value);
    Document::from_json(&value.to_string())
}

fn code(base: &str, edit: impl FnOnce(&mut Value)) -> Option<&'static str> {
    edited(base, edit).expect_err("rejected").sunat_code()
}

fn out_of_scope(result: Result<Document, ModelError>) -> String {
    match result {
        Err(ModelError::OutOfScope { path, .. }) => path,
        other => panic!("{other:?}"),
    }
}

#[test]
fn every_valid_vector_parses() {
    for base in [V1, V1B, V1C, V2, V3, V4, S3, N3] {
        Document::from_json(base).unwrap();
    }
}

#[test]
fn the_negative_vectors_fail_with_their_code() {
    assert_eq!(Document::from_json(N1).unwrap_err().sunat_code(), Some("2509"));
    assert_eq!(Document::from_json(N2).unwrap_err().sunat_code(), Some("3291"));
}

#[test]
fn document_type_and_number_of_the_advance() {
    assert_eq!(code(V2, |v| v["advances"][0]["document_type"] = json!("01")), Some("2505"));
    assert_eq!(code(V2, |v| v["advances"][0]["document_number"] = json!("B001-8101")), Some("2521"));
    assert_eq!(code(V2, |v| v["advances"][0]["document_number"] = json!("F001-123456789")), Some("2521"));
    assert_eq!(code(V2, |v| v["advances"][0]["document_number"] = json!("F001")), Some("2521"));
    assert_eq!(code(V4, |v| v["advances"][0]["document_number"] = json!("F001-8100")), Some("2521"));
    for accepted in ["E001-5", "0001-5", "F1A9-12345678"] {
        edited(V2, |v| v["advances"][0]["document_number"] = json!(accepted)).unwrap();
    }
    for accepted in ["EB01-5", "0001-5", "B1A9-1"] {
        edited(V4, |v| v["advances"][0]["document_number"] = json!(accepted)).unwrap();
    }
}

#[test]
fn issuer_ruc_of_the_advance() {
    assert_eq!(code(V2, |v| v["advances"][0]["issuer_ruc"] = json!("2000000000")), Some("3217"));
    edited(V2, |v| v["advances"][0]["issuer_ruc"] = json!("20100066603")).unwrap();
}

#[test]
fn amounts_of_the_advance() {
    assert_eq!(code(V2, |v| v["advances"][0]["paid_amount"] = json!("0")), Some("2503"));
    assert_eq!(code(V2, |v| v["advances"][0]["paid_amount"] = json!("590.001")), Some("2503"));
    assert_eq!(code(V2, |v| v["advances"][0]["value"] = json!("-500.00")), Some("2968"));
    assert_eq!(code(V2, |v| v["advances"][0]["value"] = json!("500.001")), Some("2968"));
}

#[test]
fn affectation_of_the_advance_picks_the_discount() {
    let rejected = edited(V2, |v| v["advances"][0]["affectation"] = json!("40")).unwrap_err();
    assert!(matches!(rejected, ModelError::Invalid { .. }), "{rejected:?}");
}

#[test]
fn factor_and_base_go_together_and_multiply_to_the_value() {
    assert_eq!(code(S3, |v| v["advances"][0]["factor"] = json!(null)), Some("3307"));
    assert_eq!(code(S3, |v| v["advances"][0]["value"] = json!("503.00")), Some("3307"));
    assert_eq!(
        code(S3, |v| {
            v["document_type"] = json!("03");
            v["series"] = json!("B001");
            v["advances"][0]["value"] = json!("503.00")
        }),
        Some("4322")
    );
    edited(S3, |v| v["advances"][0]["value"] = json!("500.99")).unwrap();
}

#[test]
fn prepaid_total_is_required_and_is_the_sum_of_what_was_paid() {
    assert_eq!(code(V2, |v| { v["totals"].as_object_mut().unwrap().remove("prepaid"); }), Some("3220"));
    assert_eq!(code(V2, |v| v["totals"]["prepaid"] = json!("0")), Some("3220"));
    assert_eq!(code(V3, |v| v["totals"]["prepaid"] = json!("590.00")), Some("2509"));
    assert_eq!(code(V1, |v| v["totals"]["prepaid"] = json!("1.00")), Some("2509"));
}

#[test]
fn the_same_advance_twice_is_rejected_locally() {
    let twice = edited(V3, |v| v["advances"][1]["document_number"] = json!("F001-8101")).unwrap_err();
    assert!(matches!(twice, ModelError::Invalid { .. }), "{twice:?}");
}

#[test]
fn totals_net_of_the_advance_for_an_invoice() {
    assert_eq!(code(V2, |v| v["totals"]["taxed"] = json!("2250.00")), Some("3277"));
    assert_eq!(code(V2, |v| v["totals"]["igv"] = json!("405.00")), Some("3291"));
    assert_eq!(code(V2, |v| v["totals"]["sale_value"] = json!("1750.00")), Some("3278"));
    // The sheet checks 3280 before 3279, so the payable follows the wrong subtotal: this is convention C of the probe s1.
    assert_eq!(code(V2, |v| { v["totals"]["subtotal"] = json!("2565.00"); v["totals"]["payable"] = json!("1975.00") }), Some("3279"));
    assert_eq!(code(V2, |v| v["totals"]["payable"] = json!("2655.00")), Some("3280"));
    assert_eq!(code(V3, |v| v["totals"]["payable"] = json!("2065.00")), Some("3280"));
}

#[test]
fn totals_net_of_the_advance_for_a_receipt() {
    assert_eq!(code(V4, |v| v["totals"]["taxed"] = json!("300.00")), Some("4299"));
    assert_eq!(code(V4, |v| v["totals"]["igv"] = json!("54.00")), Some("4290"));
    assert_eq!(code(V4, |v| v["totals"]["sale_value"] = json!("200.00")), Some("4309"));
    assert_eq!(code(V4, |v| { v["totals"]["subtotal"] = json!("300.00"); v["totals"]["payable"] = json!("182.00") }), Some("4310"));
    assert_eq!(code(V4, |v| v["totals"]["payable"] = json!("354.00")), Some("4312"));
}

/// A sale of one exempt line (1000.00) and one unaffected line (400.00), each with its own advance.
fn exempt_and_unaffected(v: &mut Value) {
    let line = |affectation: &str, description: &str, value: &str| {
        json!({
            "unit": "NIU", "quantity": "1", "description": description, "unit_value": value, "igv_base": value, "igv_rate": "0.00",
            "igv": "0.00", "affectation": affectation, "total_taxes": "0.00", "unit_price": value, "line_value": value
        })
    };
    v["lines"] = json!([line("20", "Servicio exonerado", "1000.00"), line("30", "Servicio inafecto", "400.00")]);
    v["advances"] = json!([
        { "document_type": "02", "document_number": "F001-8101", "paid_amount": "300.00", "affectation": "20", "value": "300.00" },
        { "document_type": "02", "document_number": "F001-8103", "paid_amount": "100.00", "affectation": "30", "value": "100.00" }
    ]);
    v["totals"] = json!({
        "exempt": "700.00", "unaffected": "300.00", "igv": "0.00", "total_taxes": "0.00", "sale_value": "1400.00",
        "subtotal": "1400.00", "prepaid": "400.00", "payable": "1000.00"
    });
}

#[test]
fn exempt_and_unaffected_advances_come_off_their_own_base() {
    edited(V2, exempt_and_unaffected).unwrap();
    assert_eq!(code(V2, |v| { exempt_and_unaffected(v); v["totals"]["exempt"] = json!("1000.00") }), Some("3275"));
    assert_eq!(code(V2, |v| { exempt_and_unaffected(v); v["totals"]["unaffected"] = json!("400.00") }), Some("3274"));
    let receipt = |v: &mut Value| {
        exempt_and_unaffected(v);
        v["document_type"] = json!("03");
        v["series"] = json!("B001");
    };
    edited(V2, receipt).unwrap();
    assert_eq!(code(V2, |v| { receipt(v); v["totals"]["exempt"] = json!("1000.00") }), Some("4297"));
    assert_eq!(code(V2, |v| { receipt(v); v["totals"]["unaffected"] = json!("400.00") }), Some("4296"));
}

#[test]
fn advances_do_not_go_in_global_adjustments() {
    for adjustment in ["04", "05", "06"] {
        let adjustments = json!([{ "code": adjustment, "factor": "0.2", "amount": "500.00", "base_amount": "2500.00" }]);
        assert_eq!(out_of_scope(edited(V2, |v| v["global_adjustments"] = adjustments)), "$.global_adjustments[0].code");
    }
}

#[test]
fn notes_take_no_advances() {
    const NOTE: &str = include_str!("../../../fixtures/07-credit-note-void.json");
    assert_eq!(out_of_scope(edited(NOTE, |v| v["advances"] = json!([{ "document_type": "02", "document_number": "F001-1", "paid_amount": "1.00", "affectation": "10", "value": "1.00" }]))), "$.advances");
    assert_eq!(out_of_scope(edited(NOTE, |v| v["totals"]["prepaid"] = json!("1.00"))), "$.totals.prepaid");
}

#[test]
fn two_igv_rates_with_an_advance_are_out_of_scope() {
    assert_eq!(out_of_scope(edited(V2, |v| v["lines"][1]["igv_rate"] = json!("10.50"))), "$.advances");
}

#[test]
fn prepayments_stays_rejected_as_a_synonym() {
    assert_eq!(out_of_scope(edited(V2, |v| v["prepayments"] = json!([]))), "$.prepayments");
}

#[test]
fn a_document_without_advances_keeps_its_totals_unchecked() {
    edited(V2, |v| {
        v["advances"] = json!([]);
        v["totals"].as_object_mut().unwrap().remove("prepaid");
        v["totals"]["taxed"] = json!("2250.00");
        v["totals"]["igv"] = json!("405.00");
        v["totals"]["total_taxes"] = json!("405.00");
        v["totals"]["payable"] = json!("2655.00");
    })
    .unwrap();
}
