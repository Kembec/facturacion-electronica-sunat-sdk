use kembec_sunat_model::{Document, ModelError};

const INVOICE: &str = include_str!("../../../fixtures/01-invoice-ivap.json");
const CREDIT: &str = include_str!("../../../fixtures/07-credit-note-ivap.json");
const DEBIT: &str = include_str!("../../../fixtures/08-debit-note-ivap.json");
const IGV_INVOICE: &str = include_str!("../../../fixtures/01-invoice-taxed-cash.json");

fn edited(base: &str, edit: impl FnOnce(&mut serde_json::Value)) -> Result<Document, ModelError> {
    let mut value: serde_json::Value = serde_json::from_str(base).unwrap();
    edit(&mut value);
    Document::from_json(&value.to_string())
}

fn path_of(result: Result<Document, ModelError>) -> String {
    match result {
        Err(ModelError::Invalid { path, .. } | ModelError::Missing { path }) => path,
        other => panic!("{other:?}"),
    }
}

#[test]
fn ivap_fixtures_are_accepted() {
    for json in [INVOICE, include_str!("../../../fixtures/03-receipt-ivap.json"), CREDIT, DEBIT] {
        let document = Document::from_json(json).unwrap();
        assert_eq!(document.voucher().unwrap().totals.ivap.as_ref().map(|d| d.as_str()), Some("14.00"));
    }
}

#[test]
fn every_line_takes_17_or_none_does() {
    let mixed = edited(INVOICE, |v| {
        let mut igv = v["lines"][0].clone();
        igv["affectation"] = serde_json::json!("10");
        v["lines"].as_array_mut().unwrap().push(igv);
    });
    assert_eq!(path_of(mixed), "$.lines[1].affectation");
}

#[test]
fn ivap_totals_go_exactly_with_ivap_lines() {
    let no_base = edited(INVOICE, |v| {
        v["totals"].as_object_mut().unwrap().remove("ivap_base");
    });
    assert_eq!(path_of(no_base), "$.totals.ivap_base");
    let no_tax = edited(INVOICE, |v| {
        v["totals"].as_object_mut().unwrap().remove("ivap");
    });
    assert_eq!(path_of(no_tax), "$.totals.ivap");
    let igv_with_ivap_totals = edited(IGV_INVOICE, |v| v["totals"]["ivap_base"] = serde_json::json!("10.00"));
    assert_eq!(path_of(igv_with_ivap_totals), "$.totals.ivap_base");
}

#[test]
fn notes_on_ivap_lines_take_type_12() {
    for (base, rejected) in [(CREDIT, "07"), (DEBIT, "02")] {
        let other_type = edited(base, |v| v["note"]["reason_code"] = serde_json::json!(rejected));
        assert_eq!(path_of(other_type), "$.note.reason_code");
    }
}
