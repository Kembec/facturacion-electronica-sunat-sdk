//! The class and `diagnosis` the SDK gives an OSE's answers: synthetic vectors in `fixtures/ose/`
//! (see its README), read by the transport like SUNAT's.

use std::fs;
use std::path::{Path, PathBuf};

use kembec_sunat_sdk::diagnosis::diagnosis_json;
use kembec_sunat_sdk::transport::{parse_send_bill_response, read_cdr, SendBillOutcome};
use kembec_sunat_sdk::{classify_cdr, classify_code, ResponseClass};
use serde_json::json;

fn fixture(name: &str) -> Vec<u8> {
    let path: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/ose").join(name);
    fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn cdr_class(name: &str) -> Option<ResponseClass> {
    let cdr = read_cdr(&fixture(name)).unwrap();
    classify_cdr(&cdr.response_code, &cdr.notes)
}

fn fault_class(name: &str) -> (String, Option<ResponseClass>) {
    let SendBillOutcome::Fault { code, .. } = parse_send_bill_response(&fixture(name)).unwrap() else {
        panic!("{name} is a fault");
    };
    let class = classify_code(&code);
    (code, class)
}

#[test]
fn accepted_cdrs_are_accepted() {
    assert_eq!(cdr_class("ose-cdr-invoice-accepted.xml"), Some(ResponseClass::Accepted));
    assert_eq!(cdr_class("ose-cdr-summary-accepted.xml"), Some(ResponseClass::Accepted));
}

#[test]
fn an_observation_in_response_status_is_observed_and_diagnosed() {
    assert_eq!(cdr_class("ose-cdr-invoice-observed.xml"), Some(ResponseClass::Observed));
    let cdr = read_cdr(&fixture("ose-cdr-invoice-observed.xml")).unwrap();
    let outcome = json!({
        "outcome": "cdr",
        "cdr": { "response_code": cdr.response_code, "description": cdr.description, "notes": cdr.notes },
    });
    let diagnosis = diagnosis_json("01", &outcome).unwrap();
    let codes: Vec<&str> = diagnosis.as_array().unwrap().iter().filter_map(|d| d["code"].as_str()).collect();
    assert_eq!(codes, ["4252"]);
}

/// The annex allows only code 0 in an OSE's CDR; whether it matches what was sent is not checked here.
#[test]
fn a_nonzero_code_is_not_accepted() {
    let class = cdr_class("ose-cdr-nonzero-code.xml");
    assert_eq!(class, Some(ResponseClass::Unknown));
    assert!(!class.unwrap().is_valid_document());
}

#[test]
fn rejection_2108_is_rejected_as_an_ose_or_sunat_writes_it() {
    assert_eq!(fault_class("ose-fault-rejected.xml"), ("2108".into(), Some(ResponseClass::Rejected)));
    assert_eq!(fault_class("sunat-fault-contrast.xml"), ("2108".into(), Some(ResponseClass::Rejected)));
}

#[test]
fn an_ose_code_of_its_own_is_unknown_never_accepted_or_a_known_rejection() {
    let (_, class) = fault_class("ose-fault-alpha-code.xml");
    assert_eq!(class, Some(ResponseClass::Unknown));
    assert!(!class.unwrap().is_valid_document());
}
