use std::path::PathBuf;

use serde_json::{json, Value};

use super::*;
use crate::{parse_send_bill_response, SendBillOutcome};

fn data(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/data/error-fields").join(name)
}

fn paths(diagnosis: &Diagnosis) -> Vec<Option<&str>> {
    diagnosis.fields.iter().map(|f| f.input_path.as_deref()).collect()
}

#[test]
fn beta_2800_points_at_the_customer_document_type() {
    // The faultstring of a beta rejection (17/10/2026), as SUNAT sent it.
    let message = std::fs::read_to_string(data("faultstring-2800.txt")).unwrap();
    let diagnosis = diagnose("01", "2800", message.trim());
    assert_eq!(diagnosis.detail, Some(Detail { node: "cbc:ID/schemeID".into(), value: "1".into() }));
    assert_eq!(paths(&diagnosis), [Some("customer.document_type")]);
    assert_eq!(
        diagnosis.fields[0].xpath.as_deref(),
        Some("/Invoice/cac:AccountingCustomerParty/cac:Party/cac:PartyIdentification/cbc:ID@schemeID")
    );
    assert_eq!(diagnosis.fields[0].certainty, "fuente");
    assert_eq!(
        diagnosis.to_json(),
        json!({
            "code": "2800",
            "detail": { "node": "cbc:ID/schemeID", "value": "1" },
            "fields": [{
                "input_path": "customer.document_type",
                "xpath": "/Invoice/cac:AccountingCustomerParty/cac:Party/cac:PartyIdentification/cbc:ID@schemeID",
                "certainty": "fuente"
            }]
        })
    );
}

#[test]
fn beta_2335_without_a_node_gives_every_row_of_its_code() {
    let body = std::fs::read(data("fault-2335.xml")).unwrap();
    let SendBillOutcome::Fault { code, message } = parse_send_bill_response(&body).unwrap() else {
        panic!("fault-2335.xml is a fault");
    };
    let diagnosis = diagnose("03", &code, &message);
    assert_eq!(diagnosis.code, "2335");
    assert_eq!(diagnosis.detail, None, "{message}");
    let all: Vec<&Row> = rows().iter().filter(|r| r.code == "2335" && r.document_types.iter().any(|t| t == "03")).collect();
    assert!(!all.is_empty());
    assert_eq!(diagnosis.fields, all.iter().map(|r| r.field.clone()).collect::<Vec<_>>());
    assert_eq!(paths(&diagnosis), [Some("sdk")], "the signature is the SDK's, not a field of the input");
}

#[test]
fn an_observation_note_in_the_rules_points_at_its_field() {
    let cdr = json!({ "response_code": "0", "description": "aceptada", "notes": ["4287 - El precio unitario difiere", "sin código"] });
    let diagnosis = diagnosis_json("03", &json!({ "outcome": "cdr", "cdr": cdr })).unwrap();
    assert_eq!(diagnosis.as_array().unwrap().len(), 1, "code 0 and a note without a code are not diagnosed");
    assert_eq!(diagnosis[0]["code"], "4287");
    assert_eq!(diagnosis[0]["fields"][0]["input_path"], "lines[].unit_price");
}

#[test]
fn a_rejection_and_its_notes_are_each_diagnosed_and_other_outcomes_are_not() {
    let rejected = json!({ "outcome": "cdr", "cdr": { "response_code": "2800", "description": "x", "notes": ["4287 - y"] } });
    let codes: Vec<Value> = diagnosis_json("03", &rejected).unwrap().as_array().unwrap().iter().map(|d| d["code"].clone()).collect();
    assert_eq!(codes, [json!("2800"), json!("4287")]);
    assert_eq!(diagnosis_json("01", &json!({ "outcome": "ticket", "ticket": "1" })), None);
    assert_eq!(diagnosis_json("01", &json!({ "outcome": "ticket_status", "status_code": "98" })), None);
    let accepted = json!({ "outcome": "status", "cdr": { "response_code": "0", "notes": [] } });
    assert_eq!(diagnosis_json("01", &accepted), Some(json!([])));
}

#[test]
fn a_node_that_matches_no_row_keeps_every_candidate_and_unknown_types_have_none() {
    let rows_2800 = diagnose("01", "2800", "").fields;
    let other = diagnose("01", "2800", r#"(nodo: "cbc:Other" valor: "x")"#);
    assert_eq!(other.fields, rows_2800);
    assert_eq!(other.detail.unwrap().node, "cbc:Other");
    assert!(diagnose("RC", "2800", "").fields.is_empty());
    assert!(diagnose("09", "2800", "").fields.is_empty());
    assert_eq!(diagnose("01", "109", "").fields, diagnose("01", "0109", "").fields, "codes are compared as numbers");
    assert!(!diagnose("01", "0109", "").fields.is_empty());
    assert_eq!(diagnose("01", "2800", r#"(nodo: "cbc:ID/schemeID")"#).detail, None, "a node without a value is no detail");
}

#[test]
fn nodes_match_with_or_without_the_at_sign() {
    for node in ["cbc:ID/schemeID", "cbc:ID@schemeID", "cbc:ID/@schemeID"] {
        assert!(ends_in(Some("/Invoice/cac:Party/cac:PartyIdentification/cbc:ID@schemeID"), node), "{node}");
    }
    assert!(!ends_in(Some("/Invoice/cbc:ID@schemeID"), "D/schemeID"), "whole steps only");
    assert!(!ends_in(None, "cbc:ID"));
    assert!(!ends_in(Some("/Invoice/cbc:ID"), ""));
}

/// Fails when `data/error-fields.json` is not what `data/generate-error-fields.py` makes of its source:
/// regenerate it with `python3 -I crates/sunat-sdk/data/generate-error-fields.py`.
#[test]
fn error_fields_match_their_source() {
    let source: Vec<Value> = serde_json::from_slice(&std::fs::read(data("error-fields.source.json")).unwrap()).unwrap();
    let embedded: Value = serde_json::from_str(ERROR_FIELDS).unwrap();
    let mut expected = Vec::new();
    for row in &source {
        let mut groups: Vec<(Value, Vec<Value>)> = Vec::new();
        for document_type in row["document_types"].as_array().unwrap() {
            let own = &row[format!("xpath_{}", document_type.as_str().unwrap()).as_str()];
            let xpath = if own.is_string() { own.clone() } else { row["xpath"].clone() };
            match groups.iter_mut().find(|(x, _)| *x == xpath) {
                Some((_, types)) => types.push(document_type.clone()),
                None => groups.push((xpath, vec![document_type.clone()])),
            }
        }
        for (xpath, document_types) in groups {
            let xpath = if xpath.as_str().is_some_and(|x| !x.starts_with('(')) { xpath } else { Value::Null };
            expected.push(json!({
                "code": row["code"],
                "document_types": document_types,
                "xpath": xpath,
                "input_path": row["input_path"],
                "also": row["also"],
                "certainty": row["certainty"],
            }));
        }
    }
    assert_eq!(source.len(), 711, "the source table of 26.08.2026 and the 9 reversal (RR) rows");
    assert_eq!(embedded, Value::Array(expected), "data/error-fields.json drifted from its source: regenerate it");
}

/// The reversal summary (RR) of a purchase settlement: `summary` names it `RUC-RR-...`, so its codes are looked
/// up under `RR`, and each one the SDK checks points at its `kind: "reversal"` field.
#[test]
fn reversal_codes_point_at_their_fields() {
    let expected = [
        ("2220", "number"),
        ("2671", "generation_date"),
        ("2308", "lines[].document_type"),
        ("2345", "lines[].series"),
        ("2958", "communication_date"),
        ("2312", "lines[].number"),
        ("2348", "lines[].number"),
        ("2315", "lines[].reason"),
        ("4203", "lines[].reason"),
    ];
    for (code, field) in expected {
        assert_eq!(paths(&diagnose("RR", code, "")), vec![Some(field)], "{code}");
    }
    assert!(diagnose("01", "2958", "").fields.is_empty(), "RR rows stay under RR");
}
