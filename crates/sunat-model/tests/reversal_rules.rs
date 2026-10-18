//! The reversal summary (RR) of a purchase settlement (04): its vector (`fixtures/self-billed/v3-reversal`)
//! and one test per «Reversión:» rule of the contract the SDK checks before sending, each expecting
//! SUNAT's code. `n3` (communication 7 days after a settlement issued 11/10: 6 after a CDR of the next day) is checked only here.

use kembec_sunat_model::{Document, ModelError};
use serde_json::{json, Value};

const V3: &str = include_str!("../../../fixtures/self-billed/v3-reversal.json");
const N3: &str = include_str!("../../../fixtures/self-billed/n3-reversal-7-days-2958.json");

fn v3(edit: impl FnOnce(&mut Value)) -> Result<Document, ModelError> {
    let mut value: Value = serde_json::from_str(V3).unwrap();
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

#[test]
fn the_vector_is_a_reversal_named_rr() {
    let document = Document::from_json(V3).unwrap();
    assert!(matches!(document, Document::Reversal(_)));
    assert_eq!(document.file_name(), "20000000001-RR-20261018-1");
    assert!(document.voucher().is_none());
}

#[test]
fn a_void_communication_keeps_its_ra_name() {
    let mut value: Value = serde_json::from_str(V3).unwrap();
    value["kind"] = json!("voided");
    value["lines"][0] = json!({ "document_type": "01", "series": "F001", "number": "1", "reason": "ERROR" });
    let document = Document::from_json(&value.to_string()).unwrap();
    assert_eq!(document.file_name(), "20000000001-RA-20261018-1");
}

#[test]
fn document_type_other_than_04() {
    for other in ["01", "03", "20"] {
        expect_code(v3(|v| v["lines"][0]["document_type"] = json!(other)), "2308");
    }
}

#[test]
fn series_not_l_and_three() {
    for bad in ["F001", "L01", "L0011", "l001", "0001"] {
        expect_code(v3(|v| v["lines"][0]["series"] = json!(bad)), "2345");
    }
}

#[test]
fn number_not_one_to_eight_digits() {
    for bad in ["", "123456789", "1A"] {
        expect_code(v3(|v| v["lines"][0]["number"] = json!(bad)), "2312");
    }
}

#[test]
fn repeated_line() {
    expect_code(
        v3(|v| {
            let mut again = v["lines"][0].clone();
            again["number"] = json!("0001");
            v["lines"].as_array_mut().unwrap().push(again);
        }),
        "2348",
    );
}

#[test]
fn reason_blank_or_short() {
    expect_code(v3(|v| v["lines"][0]["reason"] = json!("  ")), "2315");
    expect_code(v3(|v| v["lines"][0]["reason"] = json!("NO")), "4203");
    assert!(v3(|v| v["lines"][0]["reason"] = json!("ERR")).is_ok());
}

#[test]
fn generation_after_communication() {
    expect_code(v3(|v| v["generation_date"] = json!("2026-10-19")), "2671");
}

#[test]
fn correlative_over_five_digits() {
    expect_code(v3(|v| v["number"] = json!("123456")), "2220");
    expect_code(v3(|v| v["number"] = json!("")), "2220");
}

/// SUNAT counts 5 days from the settlement's CDR, which may be the day after its issue: the SDK rejects
/// only past 6 days from `generation_date`, and its message names both sources.
#[test]
fn more_than_six_days_after_the_settlement() {
    assert!(v3(|v| v["generation_date"] = json!("2026-10-12")).is_ok(), "6 days must pass");
    let error = Document::from_json(N3).unwrap_err();
    assert_eq!(error.sunat_code(), Some("2958"));
    let message = error.to_string();
    assert!(message.contains("5 days of its CDR") && message.contains("RS 244-2019") && message.contains("7"), "{message}");
}

#[test]
fn n3_is_one_day_past_the_tolerance() {
    let mut value: Value = serde_json::from_str(N3).unwrap();
    value["generation_date"] = json!("2026-10-12");
    assert!(Document::from_json(&value.to_string()).is_ok());
}
