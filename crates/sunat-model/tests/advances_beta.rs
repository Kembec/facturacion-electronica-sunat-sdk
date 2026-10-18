//! What the SUNAT beta answered (17/10/2026) to the advances vectors, kept in `fixtures/advances/beta/`
//! (billService of the invoice endpoint, public MODDATOS credentials). The SDK's local rules are
//! compared with those answers here, so a change of either side shows up.

use kembec_sunat_model::Document;
use serde_json::Value;

macro_rules! answer {
    ($name:literal) => {
        serde_json::from_str::<Value>(include_str!(concat!("../../../fixtures/advances/beta/", $name, ".json"))).unwrap()
    };
}

macro_rules! local {
    ($name:literal) => {
        Document::from_json(include_str!(concat!("../../../fixtures/advances/", $name, ".json")))
    };
}

fn accepted(answer: &Value) {
    let cdr = &answer["cdr"];
    assert_eq!(cdr["response_code"], "0");
    assert_eq!(cdr["class"], "accepted");
    assert_eq!(cdr["notes"].as_array().unwrap().len(), 0);
}

fn rejected(answer: &Value, code: &str) {
    assert_eq!((answer["class"].as_str(), answer["code"].as_str()), (Some("rejected"), Some(code)));
}

#[test]
fn valid_vectors_were_accepted_with_code_0_and_no_notes() {
    accepted(&answer!("v1-invoice-advance"));
    accepted(&answer!("v1b-invoice-advance-second"));
    accepted(&answer!("v1c-receipt-advance"));
    accepted(&answer!("v2-invoice-final-one-advance"));
    accepted(&answer!("v3-invoice-final-two-advances"));
    accepted(&answer!("v4-receipt-final-advance"));
    for vector in [local!("v1-invoice-advance"), local!("v1b-invoice-advance-second"), local!("v1c-receipt-advance")] {
        vector.unwrap();
    }
    for vector in [local!("v2-invoice-final-one-advance"), local!("v3-invoice-final-two-advances"), local!("v4-receipt-final-advance")] {
        vector.unwrap();
    }
}

/// n1 and n2: the beta rejects with the code the SDK gives locally.
#[test]
fn n1_and_n2_are_rejected_like_the_sdk_says() {
    rejected(&answer!("n1-prepaid-not-sum-of-advances"), "2509");
    assert_eq!(local!("n1-prepaid-not-sum-of-advances").unwrap_err().sunat_code(), Some("2509"));
    rejected(&answer!("n2-igv-on-gross-base"), "3291");
    assert_eq!(local!("n2-igv-on-gross-base").unwrap_err().sunat_code(), Some("3291"));
}

/// n3: the beta does not look the advance up (no 3218), and the SDK cannot either.
#[test]
fn n3_an_advance_that_was_never_issued_is_accepted_by_the_beta_and_by_the_sdk() {
    accepted(&answer!("n3-advance-not-found"));
    local!("n3-advance-not-found").unwrap();
}

/// s1 decides the convention: the IGV of the 3279 total is over the gross base (v2 accepted), and the net
/// version (convention C) is rejected with 3279, as the SDK says.
#[test]
fn s1_the_net_subtotal_is_rejected_with_3279() {
    rejected(&answer!("s1-convention-c"), "3279");
    assert_eq!(local!("s1-convention-c").unwrap_err().sunat_code(), Some("3279"));
}

/// s2: a sale covered in full by the advance (payable 0.00) is accepted, so the SDK does not make it an
/// error (2062 stays for the other documents); its first attempt, without the zero IGV subtotal, got 2638.
#[test]
fn s2_a_payable_of_zero_is_accepted() {
    accepted(&answer!("s2-payable-zero"));
    local!("s2-payable-zero").unwrap();
    let first = answer!("s2-payable-zero.first-attempt-2638");
    assert_eq!(first["code"], "2638");
}

#[test]
fn s3_the_discount_with_factor_and_base_is_accepted() {
    accepted(&answer!("s3-discount-with-factor"));
    local!("s3-discount-with-factor").unwrap();
}
