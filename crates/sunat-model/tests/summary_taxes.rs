//! IVAP, ISC, other taxes and the IGV perception on a daily summary line (Resumen Diario sheet, rows 58-75 and
//! 116-148): accepted when SUNAT accepts them, rejected with the code SUNAT answers otherwise.

use serde_json::{json, Value};
use kembec_sunat_model::{Document, ModelError};

const RECEIPTS: &str = include_str!("../../../fixtures/rc-receipts-added.json");

fn first_line(edit: impl FnOnce(&mut Value)) -> Result<Document, ModelError> {
    let mut value: Value = serde_json::from_str(RECEIPTS).unwrap();
    edit(&mut value["lines"][0]);
    Document::from_json(&value.to_string())
}

/// A receipt of 100.00 + IVAP 4.00, under regime 01 (2 %): 104.00 × 2 % = 2.08, cashed 106.08.
fn ivap_line(line: &mut Value) {
    let map = line.as_object_mut().unwrap();
    for key in ["igv", "igv_rate", "taxed"] {
        map.remove(key);
    }
    map.insert("total".into(), json!("104.00"));
    map.insert("taxed".into(), json!("100.00"));
    map.insert("ivap".into(), json!("4.00"));
}

fn perception(line: &mut Value) {
    line["perception"] = json!({ "system_code": "01", "base": "104.00", "amount": "2.08", "total_cashed": "106.08" });
}

fn code(result: Result<Document, ModelError>) -> (Option<&'static str>, String) {
    match result {
        Err(error @ ModelError::Rule { .. }) => {
            let ModelError::Rule { path, .. } = &error else { unreachable!() };
            (error.sunat_code(), path.clone())
        }
        other => panic!("expected a SUNAT rule error, got {other:?}"),
    }
}

#[test]
fn receipt_lines_take_ivap_isc_other_taxes_and_perception() {
    assert!(first_line(ivap_line).is_ok(), "IVAP without igv_rate");
    assert!(first_line(|l| l["isc"] = json!("5.00")).is_ok(), "ISC");
    assert!(first_line(|l| l["other_taxes"] = json!("1.50")).is_ok(), "other taxes");
    assert!(first_line(|l| {
        ivap_line(l);
        perception(l);
    })
    .is_ok(), "IVAP and perception");
    assert!(first_line(|l| {
        l["status"] = json!("3");
        ivap_line(l);
        perception(l);
    })
    .is_ok(), "a voided receipt keeps its perception");
}

#[test]
fn lines_without_the_new_fields_still_need_igv_rate() {
    let result = first_line(|l| {
        l.as_object_mut().unwrap().remove("igv_rate");
    });
    assert_eq!(code(result), (Some("2992"), "$.lines[0].igv_rate".into()));
}

#[test]
fn perception_only_on_a_receipt_not_modified() {
    let on_note = || {
        let mut value: Value = serde_json::from_str(include_str!("../../../fixtures/rc-credit-note-reference.json")).unwrap();
        value["lines"][0]["perception"] = json!({ "system_code": "01", "base": "1.00", "amount": "0.02", "total_cashed": "1.00" });
        Document::from_json(&value.to_string())
    };
    assert_eq!(code(on_note()), (Some("2986"), "$.lines[0].perception".into()));
    let modified = first_line(|l| {
        ivap_line(l);
        perception(l);
        l["status"] = json!("2");
    });
    assert_eq!(code(modified), (Some("2986"), "$.lines[0].perception".into()));
}

#[test]
fn ivap_excludes_the_igv() {
    let with_igv = first_line(|l| {
        ivap_line(l);
        l["igv"] = json!("0.72");
    });
    assert_eq!(code(with_igv), (Some("2355"), "$.lines[0].igv".into()));
    let with_rate = first_line(|l| {
        ivap_line(l);
        l["igv_rate"] = json!("18.00");
    });
    assert_eq!(code(with_rate), (Some("2355"), "$.lines[0].igv_rate".into()));
}

#[test]
fn perception_amounts_follow_the_regime() {
    let off = |field: &str, value: &str| {
        first_line(|l| {
            ivap_line(l);
            perception(l);
            l["perception"][field] = json!(value);
        })
    };
    assert!(off("total_cashed", "107.08").is_ok(), "within 1");
    assert_eq!(code(off("total_cashed", "107.09")), (Some("2608"), "$.lines[0].perception.total_cashed".into()));
    assert_eq!(code(off("amount", "1.04")).0, Some("2608"), "regime 01 is 2 %, not 1 %");
    assert_eq!(code(off("system_code", "09")), (Some("2517"), "$.lines[0].perception.system_code".into()));
    let regime_03 = first_line(|l| {
        ivap_line(l);
        l["perception"] = json!({ "system_code": "03", "base": "104.00", "amount": "0.52", "total_cashed": "104.52" });
    });
    assert!(regime_03.is_ok(), "regime 03 is 0.5 %");
}

#[test]
fn new_amounts_are_positive_with_two_decimals() {
    for (field, value) in [("isc", "0.00"), ("other_taxes", "-1.00"), ("isc", "1.234")] {
        match first_line(|l| l[field] = json!(value)) {
            Err(ModelError::Invalid { path, .. }) => assert_eq!(path, format!("$.lines[0].{field}")),
            other => panic!("{field} {value}: {other:?}"),
        }
    }
    let zero_base = first_line(|l| {
        ivap_line(l);
        perception(l);
        l["perception"]["base"] = json!("0");
    });
    assert!(matches!(zero_base, Err(ModelError::Invalid { path, .. }) if path == "$.lines[0].perception.base"));
    let with_percent = first_line(|l| {
        perception(l);
        l["perception"]["percent"] = json!("2.00");
    });
    assert!(matches!(with_percent, Err(ModelError::Json(_))), "the rate is not input");
}
