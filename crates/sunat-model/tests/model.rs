use sunat_model::{Document, ModelError};

const BASE: &str = include_str!("../../../fixtures/01-invoice-taxed-cash.json");

fn with(edit: impl FnOnce(&mut serde_json::Value)) -> Result<Document, ModelError> {
    let mut value: serde_json::Value = serde_json::from_str(BASE).unwrap();
    edit(&mut value);
    Document::from_json(&value.to_string())
}

#[test]
fn file_name_is_ruc_type_series_number() {
    assert_eq!(Document::from_json(BASE).unwrap().file_name(), "20123456789-01-F001-1");
}

#[test]
fn out_of_scope_features_get_named_errors() {
    let cases: [(&str, fn(&mut serde_json::Value)); 5] = [
        ("detraction", |v| v["detraction"] = serde_json::json!({})),
        ("perception", |v| v["perception"] = serde_json::json!({})),
        ("global discounts", |v| v["discounts"] = serde_json::json!([])),
        ("ICBPER", |v| v["lines"][0]["icbper"] = serde_json::json!("0.50")),
        ("ISC", |v| v["totals"]["isc"] = serde_json::json!("1.00")),
    ];
    for (feature, edit) in cases {
        match with(edit) {
            Err(ModelError::OutOfScope { feature: f, .. }) => assert_eq!(f, feature),
            other => panic!("{feature}: {other:?}"),
        }
    }
    let ivap = with(|v| v["lines"][0]["affectation"] = serde_json::json!("17"));
    assert!(matches!(ivap, Err(ModelError::OutOfScope { .. })));
}

#[test]
fn kind_and_type_must_agree() {
    assert!(with(|v| v["document_type"] = serde_json::json!("07")).is_err());
    assert!(with(|v| v["kind"] = serde_json::json!("note")).is_err());
    assert!(with(|v| v["unknown"] = serde_json::json!(1)).is_err());
    assert!(with(|v| v["totals"]["payable"] = serde_json::json!(218.0)).is_err());
}

const SUMMARY: &str = include_str!("../../../fixtures/rc-credit-note-reference.json");
const VOIDED: &str = include_str!("../../../fixtures/ra-invoice.json");

fn edited(base: &str, edit: impl FnOnce(&mut serde_json::Value)) -> Result<Document, ModelError> {
    let mut value: serde_json::Value = serde_json::from_str(base).unwrap();
    edit(&mut value);
    Document::from_json(&value.to_string())
}

#[test]
fn summary_and_voided_file_names_carry_their_dates() {
    assert_eq!(Document::from_json(SUMMARY).unwrap().file_name(), "20123456789-RC-20261015-2");
    assert_eq!(Document::from_json(VOIDED).unwrap().file_name(), "20123456789-RA-20261015-1");
    assert!(Document::from_json(SUMMARY).unwrap().voucher().is_none());
}

#[test]
fn summary_lines_are_checked() {
    let invalid: [(&str, fn(&mut serde_json::Value)); 8] = [
        ("$.lines[0].reference_document", |v| v["lines"][0]["document_type"] = serde_json::json!("03")),
        ("$.lines[0].reference_document", |v| {
            v["lines"][0].as_object_mut().unwrap().remove("reference_document");
        }),
        ("$.lines[0].document_type", |v| v["lines"][0]["document_type"] = serde_json::json!("01")),
        ("$.lines[0].status", |v| v["lines"][0]["status"] = serde_json::json!("4")),
        ("$.lines[0].document_number", |v| v["lines"][0]["document_number"] = serde_json::json!("B0013")),
        ("$.number", |v| v["number"] = serde_json::json!("123456")),
        ("$.reference_date", |v| v["reference_date"] = serde_json::json!("2026-10-16")),
        ("$.lines", |v| v["lines"] = serde_json::json!([])),
    ];
    for (path, edit) in invalid {
        match edited(SUMMARY, edit) {
            Err(ModelError::Invalid { path: p, .. } | ModelError::Missing { path: p }) => assert_eq!(p, path),
            other => panic!("{path}: {other:?}"),
        }
    }
    assert!(edited(SUMMARY, |v| v["lines"][0]["unknown"] = serde_json::json!(1)).is_err());
    assert!(edited(SUMMARY, |v| v["generation_date"] = serde_json::json!("2026-10-15T00:00:00-05:00")).is_err());
}

#[test]
fn summary_line_taxes_out_of_scope_get_named_errors() {
    for (key, feature) in [("isc", "ISC"), ("icbper", "ICBPER"), ("perception", "perception"), ("ivap", "IVAP")] {
        match edited(SUMMARY, |v| v["lines"][0][key] = serde_json::json!("1.00")) {
            Err(ModelError::OutOfScope { feature: f, .. }) => assert_eq!(f, feature),
            other => panic!("{key}: {other:?}"),
        }
    }
}

#[test]
fn void_communication_is_checked() {
    let invalid: [(&str, fn(&mut serde_json::Value)); 4] = [
        ("$.lines[0].document_type", |v| v["lines"][0]["document_type"] = serde_json::json!("03")),
        ("$.lines[0].reason", |v| v["lines"][0]["reason"] = serde_json::json!("")),
        ("$.lines[0].series", |v| v["lines"][0]["series"] = serde_json::json!("F01")),
        ("$.generation_date", |v| v["generation_date"] = serde_json::json!("2026-10-16")),
    ];
    for (path, edit) in invalid {
        match edited(VOIDED, edit) {
            Err(ModelError::Invalid { path: p, .. } | ModelError::Missing { path: p }) => assert_eq!(p, path),
            other => panic!("{path}: {other:?}"),
        }
    }
}
