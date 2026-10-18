//! What the SUNAT beta answered (18/10/2026) to the export vectors, kept in `fixtures/export/beta/`
//! (billService of the invoice endpoint, public MODDATOS credentials). The SDK's local rules are
//! compared with those answers here, so a change of either side shows up.

use kembec_sunat_model::Document;
use serde_json::Value;

macro_rules! answer {
    ($name:literal) => {
        serde_json::from_str::<Value>(include_str!(concat!("../../../fixtures/export/beta/", $name, ".json"))).unwrap()
    };
}

macro_rules! local {
    ($name:literal) => {
        Document::from_json(include_str!(concat!("../../../fixtures/export/", $name, ".json")))
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

/// 0200 goods, 0201 SaaS and web hosting (the beta did not answer 3097 for the test RUC), 0208 consulting and
/// 0202 lodging with the ten guest properties: accepted with code 0 and no notes.
#[test]
fn valid_vectors_were_accepted_with_code_0_and_no_notes() {
    accepted(&answer!("v1-invoice-export-goods-0200"));
    accepted(&answer!("v2-invoice-export-saas-hosting-0201"));
    accepted(&answer!("v3-invoice-export-consulting-0208"));
    accepted(&answer!("v4-invoice-export-lodging-0202"));
    for vector in [local!("v1-invoice-export-goods-0200"), local!("v2-invoice-export-saas-hosting-0201"), local!("v3-invoice-export-consulting-0208"), local!("v4-invoice-export-lodging-0202")] {
        vector.unwrap();
    }
}

/// n2 and n3: the beta rejects with the code the SDK gives locally.
#[test]
fn n2_and_n3_are_rejected_like_the_sdk_says() {
    rejected(&answer!("n2-0200-customer-with-ruc"), "2800");
    assert_eq!(local!("n2-0200-customer-with-ruc").unwrap_err().sunat_code(), Some("2800"));
    rejected(&answer!("n3-0202-without-stay-days"), "3145");
    assert_eq!(local!("n3-0202-without-stay-days").unwrap_err().sunat_code(), Some("3145"));
}

/// n1: the beta accepted 0208 without the country of use, but the sheet (Factura2_0 row 75) makes 3098 an
/// error, so the SDK keeps rejecting it: production may apply the row the beta skips.
#[test]
fn n1_without_country_is_accepted_by_the_beta_and_rejected_by_the_sdk() {
    accepted(&answer!("n1-0208-without-country"));
    assert_eq!(local!("n1-0208-without-country").unwrap_err().sunat_code(), Some("3098"));
}

/// 0203 shipping, 0206 cargo services, 0204 foreign vessel, 0205 tour package and 0207 electricity to a ZED
/// customer with RUC (sent in that order, 18/10/2026 01:47): accepted with code 0 and no notes.
#[test]
fn the_five_types_without_a_vector_before_were_accepted_with_code_0_and_no_notes() {
    accepted(&answer!("v5-invoice-export-shipping-0203"));
    accepted(&answer!("v8-invoice-export-cargo-services-0206"));
    accepted(&answer!("v6-invoice-export-foreign-vessel-0204"));
    accepted(&answer!("v7-invoice-export-tour-package-0205"));
    accepted(&answer!("v9-invoice-export-electricity-zed-0207"));
    for vector in [
        local!("v5-invoice-export-shipping-0203"),
        local!("v6-invoice-export-foreign-vessel-0204"),
        local!("v7-invoice-export-tour-package-0205"),
        local!("v8-invoice-export-cargo-services-0206"),
        local!("v9-invoice-export-electricity-zed-0207"),
    ] {
        vector.unwrap();
    }
}

fn v3_with_country(number: &str, country: &str) -> Result<Document, kembec_sunat_model::ModelError> {
    let mut value: Value = serde_json::from_str(include_str!("../../../fixtures/export/v3-invoice-export-consulting-0208.json")).unwrap();
    value["number"] = Value::from(number);
    value["service_use_country"] = Value::from(country);
    Document::from_json(&value.to_string())
}

/// Country probes on v3 (0208): `RS`, in ISO 3166-1 but not on the page the SUNAT sheet cites, was
/// accepted; `AN`, withdrawn from ISO in 2010 but on that page, was rejected with 3099 («Valor no se
/// encuentra en el catalogo: 04»). The beta's catalog 04 is ISO in force; the SDK still takes the union.
#[test]
fn the_beta_accepted_rs_and_rejected_an_while_the_sdk_takes_both() {
    accepted(&answer!("s1-0208-country-rs"));
    v3_with_country("8210", "RS").unwrap();
    rejected(&answer!("s2-0208-country-an"), "3099");
    v3_with_country("8211", "AN").unwrap();
}
