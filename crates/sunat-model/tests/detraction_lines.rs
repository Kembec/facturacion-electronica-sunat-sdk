//! Detraction on hydrobiological resources (1002, code 004) and freight transport (1004, code 027):
//! the line data SUNAT requires on every line, and nothing more.

use kembec_sunat_model::{Document, ModelError};
use serde_json::{json, Value};

const DETRACTION: &str = include_str!("../../../fixtures/01-invoice-detraction.json");

fn fishing() -> Value {
    json!({
        "vessel_registration": "CO-12345-PM", "vessel_name": "SANTA ROSA I", "species": "ANCHOVETA",
        "landing_place": "MUELLE DE CHIMBOTE", "landing_date": "2026-10-20", "quantity": "12.50"
    })
}

fn freight() -> Value {
    json!({
        "origin": { "ubigeo": "040101", "address": "CALLE MERCADERES 45, AREQUIPA" },
        "destination": { "ubigeo": "150101", "address": "AV. ARGENTINA 123, LIMA" },
        "trip_detail": "AREQUIPA - LIMA, CARGA GENERAL",
        "reference_value": "1500.00", "effective_load_value": "1400.00", "nominal_load_value": "1600.00"
    })
}

/// The 1001 vector turned into `operation` with `code`, its line data on every line (two lines).
fn document(operation: &str, code: &str, edit: impl FnOnce(&mut Value)) -> Result<Document, ModelError> {
    let mut value: Value = serde_json::from_str(DETRACTION).unwrap();
    value["operation_type"] = json!(operation);
    value["detraction"]["code"] = json!(code);
    let second = value["lines"][0].clone();
    value["lines"].as_array_mut().unwrap().push(second);
    for line in value["lines"].as_array_mut().unwrap() {
        match operation {
            "1002" => line["fishing"] = fishing(),
            "1004" => line["freight"] = freight(),
            _ => {}
        }
    }
    edit(&mut value);
    Document::from_json(&value.to_string())
}

fn rule(result: Result<Document, ModelError>) -> (&'static str, String) {
    match result {
        Err(ModelError::Rule { code, path, .. }) => (code, path),
        other => panic!("expected a SUNAT rule, got {other:?}"),
    }
}

fn invalid(result: Result<Document, ModelError>) -> (String, String) {
    match result {
        Err(ModelError::Invalid { path, reason }) => (path, reason),
        other => panic!("expected Invalid, got {other:?}"),
    }
}

#[test]
fn invoice_and_receipt_take_1002_and_1004_with_their_line_data() {
    for (operation, code) in [("1002", "004"), ("1004", "027")] {
        let invoice = document(operation, code, |_| {});
        assert!(invoice.is_ok(), "{operation}: {invoice:?}");
        let receipt = document(operation, code, |v| {
            v["document_type"] = json!("03");
            v["series"] = json!("B001");
        });
        assert!(receipt.is_ok(), "receipt {operation}: {receipt:?}");
    }
    let parsed = document("1002", "004", |_| {}).unwrap();
    let fishing = parsed.voucher().unwrap().lines[1].fishing.clone().unwrap();
    assert_eq!(fishing.quantity.unwrap().as_str(), "12.50");
}

#[test]
fn fishing_1002_requires_every_field_on_every_line() {
    assert_eq!(rule(document("1002", "004", |v| { v["lines"][0]["fishing"].as_object_mut().unwrap().remove("quantity"); })), ("3133", "$.lines[0].fishing.quantity".into()));
    assert_eq!(rule(document("1002", "004", |v| { v["lines"][1].as_object_mut().unwrap().remove("fishing"); })), ("3063", "$.lines[1].fishing".into()));
    for (field, code) in [("vessel_registration", "3063"), ("vessel_name", "3130"), ("species", "3131"), ("landing_place", "3132"), ("landing_date", "3134")] {
        let got = rule(document("1002", "004", |v| { v["lines"][1]["fishing"].as_object_mut().unwrap().remove(field); }));
        assert_eq!(got, (code, format!("$.lines[1].fishing.{field}")));
        assert_eq!(rule(document("1002", "004", |v| v["lines"][1]["fishing"][field] = json!(" "))).0, code, "blank {field}");
    }
    let (path, _) = invalid(document("1002", "004", |v| v["lines"][0]["fishing"]["landing_date"] = json!("20/10/2026")));
    assert_eq!(path, "$.lines[0].fishing.landing_date");
}

#[test]
fn freight_1004_requires_the_trip_and_its_three_values() {
    assert_eq!(rule(document("1004", "027", |v| { v["lines"][0]["freight"].as_object_mut().unwrap().remove("nominal_load_value"); })), ("3126", "$.lines[0].freight.nominal_load_value".into()));
    assert_eq!(rule(document("1004", "027", |v| { v["lines"][1].as_object_mut().unwrap().remove("freight"); })).0, "3116");
    let cases: [(fn(&mut Value), &str); 7] = [
        (|f| { f["origin"].as_object_mut().unwrap().remove("ubigeo"); }, "3116"),
        (|f| { f["origin"].as_object_mut().unwrap().remove("address"); }, "3117"),
        (|f| { f["destination"].as_object_mut().unwrap().remove("ubigeo"); }, "3118"),
        (|f| { f["destination"].as_object_mut().unwrap().remove("address"); }, "3119"),
        (|f| { f.as_object_mut().unwrap().remove("trip_detail"); }, "3120"),
        (|f| { f.as_object_mut().unwrap().remove("reference_value"); }, "3124"),
        (|f| { f.as_object_mut().unwrap().remove("effective_load_value"); }, "3125"),
    ];
    for (edit, code) in cases {
        assert_eq!(rule(document("1004", "027", |v| edit(&mut v["lines"][1]["freight"]))).0, code);
    }
    for amount in ["0", "-5.00", "1500.005", "1234567890123.00"] {
        let (path, reason) = invalid(document("1004", "027", |v| v["lines"][0]["freight"]["effective_load_value"] = json!(amount)));
        assert_eq!(path, "$.lines[0].freight.effective_load_value", "{amount}");
        assert!(reason.contains("3122"), "{reason}");
    }
    // SUNAT does not check 01 against 02 and 03: neither does the SDK.
    assert!(document("1004", "027", |v| v["lines"][0]["freight"]["reference_value"] = json!("10.00")).is_ok());
}

#[test]
fn code_3129_follows_the_operation_type() {
    for (operation, wrong) in [("1004", "037"), ("1002", "027"), ("1004", "004")] {
        let (path, reason) = invalid(document(operation, wrong, |_| {}));
        assert_eq!(path, "$.detraction.code");
        assert!(reason.contains("3129"), "{operation} {wrong}: {reason}");
    }
}

#[test]
fn line_data_outside_its_operation_type_is_rejected_locally() {
    let (path, _) = invalid(document("0101", "037", |v| {
        v.as_object_mut().unwrap().remove("detraction");
        v["lines"][0]["fishing"] = fishing();
    }));
    assert_eq!(path, "$.lines[0].fishing");
    assert_eq!(invalid(document("1002", "004", |v| v["lines"][1]["freight"] = freight())).0, "$.lines[1].freight");
    assert_eq!(invalid(document("1001", "037", |v| v["lines"][0]["fishing"] = fishing())).0, "$.lines[0].fishing");
    assert_eq!(invalid(document("1004", "027", |v| v["lines"][0]["fishing"] = fishing())).0, "$.lines[0].fishing");
    let mut note: Value = serde_json::from_str(include_str!("../../../fixtures/07-credit-note-void.json")).unwrap();
    note["lines"][0]["freight"] = freight();
    match Document::from_json(&note.to_string()) {
        Err(ModelError::Invalid { path, .. }) => assert_eq!(path, "$.lines[0].freight"),
        other => panic!("note with freight: {other:?}"),
    }
}
