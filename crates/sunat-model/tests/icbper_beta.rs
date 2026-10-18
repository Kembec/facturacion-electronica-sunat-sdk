//! What the SUNAT beta answered (17/10/2026) to the ICBPER vectors, kept in `fixtures/icbper/beta/`
//! (billService of the invoice endpoint, public MODDATOS credentials). The SDK's local rules are
//! compared with those answers here, so a change of either side shows up.

use kembec_sunat_model::Document;
use serde_json::Value;

macro_rules! answer {
    ($name:literal) => {
        serde_json::from_str::<Value>(include_str!(concat!("../../../fixtures/icbper/beta/", $name, ".json"))).unwrap()
    };
}

fn cdr(answer: &Value) -> &Value {
    answer.get("cdr").unwrap_or_else(|| &answer["ticket_status"]["cdr"])
}

fn accepted(answer: &Value) {
    let cdr = cdr(answer);
    assert_eq!(cdr["response_code"], "0");
    assert_eq!(cdr["class"], "accepted");
    assert_eq!(cdr["notes"].as_array().unwrap().len(), 0);
}

#[test]
fn valid_vectors_were_accepted_with_code_0_and_no_notes() {
    accepted(&answer!("v1-invoice-bags"));
    accepted(&answer!("v2-receipt-bags"));
    accepted(&answer!("v3-receipt-free-bags"));
    accepted(&answer!("v4-credit-note-bags"));
    accepted(&answer!("v5-rc-icbper"));
}

/// n1: the beta accepts the invoice and observes the bag line with 4318, as the SDK predicts.
#[test]
fn n1_is_observed_with_4318_like_the_sdk_says() {
    let a = answer!("n1-amount-not-unit-times-quantity");
    assert_eq!(a["cdr"]["class"], "observed");
    assert!(a["cdr"]["notes"][0].as_str().unwrap().starts_with("4318"));
    let local = Document::from_json(include_str!("../../../fixtures/icbper/n1-amount-not-unit-times-quantity.json"));
    assert_eq!(local.unwrap_err().sunat_code(), Some("4318"));
}

/// n2: the beta rejects with 3306, as the SDK says.
#[test]
fn n2_is_rejected_with_3306_like_the_sdk_says() {
    let a = answer!("n2-total-not-sum-of-lines");
    assert_eq!((a["class"].as_str(), a["code"].as_str()), (Some("rejected"), Some("3306")));
    let local = Document::from_json(include_str!("../../../fixtures/icbper/n2-total-not-sum-of-lines.json"));
    assert_eq!(local.unwrap_err().sunat_code(), Some("3306"));
}

/// n3: a receipt with the 2019 rate (0.10) in 2026 was accepted with no note, so the beta does not apply 4237;
/// the SDK keeps rejecting it (decided so: the sheet lists 4237 and production may apply it).
#[test]
fn n3_stale_rate_is_accepted_by_the_beta_and_still_rejected_by_the_sdk() {
    accepted(&answer!("n3-stale-rate"));
    let local = Document::from_json(include_str!("../../../fixtures/icbper/n3-stale-rate.json"));
    assert_eq!(local.unwrap_err().sunat_code(), Some("4237"));
}
