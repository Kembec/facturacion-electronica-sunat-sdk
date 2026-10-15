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
