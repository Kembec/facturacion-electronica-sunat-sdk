//! Format observations on the despatch advice's free text (sheet «Guía-Remitente2_0»): one test of
//! the borders per row, each refused with its field and SUNAT code, never rewritten. SUNAT has no
//! test environment for despatch advices: the sheet is the only source.

use kembec_sunat_model::{Document, ModelError};
use serde_json::{json, Value};

/// Reason 01 (sale), private transport, two vehicles and a driver.
const SALE: &str = include_str!("../../../fixtures/09-despatch-sale.json");
/// Reason 04 (between establishments of the same company), public transport.
const BETWEEN: &str = include_str!("../../../fixtures/09-despatch-between-establishments.json");

const PARTY: &str = r#"{"document_type": "6", "document_number": "20444555666", "legal_name": "TERCERO DEMO S.A."}"#;

/// What each row allows besides its length.
#[derive(Clone, Copy, PartialEq)]
enum Kind {
    /// Any character and the space; no other whitespace; not only spaces (refused by this row, or
    /// first by the error that requires the field).
    Name,
    /// Any character and the space; no other whitespace; only spaces allowed.
    Spaced,
    /// Like `Spaced`, with at least 3 letters (R!172), so only spaces are refused too.
    Letters,
    /// Any whitespace, line breaks included (R!449).
    AnyWhitespace,
}

fn set(base: &str, prepare: fn(&mut Value), pointer: &str, value: &str) -> Result<Document, ModelError> {
    let mut json: Value = serde_json::from_str(base).unwrap();
    prepare(&mut json);
    *json.pointer_mut(pointer).unwrap() = json!(value);
    Document::from_json(&json.to_string())
}

fn refused(base: &str, prepare: fn(&mut Value), pointer: &str, path: &str, needle: &str, value: &str) {
    match set(base, prepare, pointer, value) {
        Err(ModelError::Invalid { path: got, reason }) => {
            assert_eq!(got, path, "{value:?}");
            assert!(reason.contains(needle), "{value:?}: {reason}");
        }
        other => panic!("{path} {value:?}: {other:?}"),
    }
}

fn accepted(base: &str, prepare: fn(&mut Value), pointer: &str, value: &str) {
    let document = set(base, prepare, pointer, value).unwrap_or_else(|e| panic!("{pointer} {value:?}: {e}"));
    document.validate().unwrap();
}

fn same(_: &mut Value) {}

fn assert_borders(base: &str, prepare: fn(&mut Value), pointer: &str, path: &str, code: &str, min: usize, max: usize, kind: Kind) {
    let code = &format!("SUNAT {code}");
    let filler = |n: usize| "Ñ".repeat(n);
    for value in [filler(min), filler(max), "AB C".into(), "AB\u{a0}C".into()] {
        accepted(base, prepare, pointer, &value);
    }
    let mut bad = vec![filler(max + 1)];
    if min > 1 {
        bad.push(filler(min - 1));
    }
    let breaks = ['\n', '\r', '\t'].map(|ws| format!("AB{ws}C"));
    if kind == Kind::AnyWhitespace {
        breaks.iter().for_each(|value| accepted(base, prepare, pointer, value));
    } else {
        bad.extend(breaks);
    }
    let spaces = " ".repeat(min.max(3));
    match kind {
        Kind::Name | Kind::Letters => assert!(set(base, prepare, pointer, &spaces).is_err(), "{path}: only spaces"),
        Kind::Spaced => accepted(base, prepare, pointer, &spaces),
        Kind::AnyWhitespace => {}
    }
    for value in bad {
        refused(base, prepare, pointer, path, code, &value);
    }
}

#[test]
fn note_4186() {
    assert_borders(SALE, same, "/note", "$.note", "4186", 1, 250, Kind::Name);
    refused(SALE, same, "/note", "$.note", "not only spaces (SUNAT 4186)", "  ");
    accepted(SALE, same, "/note", "");
}

#[test]
fn issuer_legal_name_4338() {
    assert_borders(SALE, same, "/issuer/legal_name", "$.issuer.legal_name", "4338", 1, 250, Kind::Name);
}

#[test]
fn related_document_description_4372() {
    let (pointer, path) = ("/related_documents/0/description", "$.related_documents[0].description");
    assert_borders(SALE, same, pointer, path, "4372", 1, 120, Kind::Name);
    refused(SALE, same, pointer, path, "not only spaces (SUNAT 4372)", " ");
    accepted(SALE, same, pointer, "");
}

#[test]
fn recipient_legal_name_4152() {
    assert_borders(SALE, same, "/recipient/legal_name", "$.recipient.legal_name", "4152", 1, 250, Kind::Name);
}

fn with_seller(v: &mut Value) {
    v["seller"] = serde_json::from_str(PARTY).unwrap();
}

fn with_buyer(v: &mut Value) {
    v["buyer"] = serde_json::from_str(PARTY).unwrap();
}

#[test]
fn seller_legal_name_4106_with_any_reason() {
    assert_borders(SALE, with_seller, "/seller/legal_name", "$.seller.legal_name", "4106", 1, 250, Kind::Name);
    refused(SALE, with_seller, "/seller/legal_name", "$.seller.legal_name", "not only spaces (SUNAT 4106)", " ");
}

#[test]
fn buyer_legal_name_4381_with_any_reason() {
    assert_borders(SALE, with_buyer, "/buyer/legal_name", "$.buyer.legal_name", "4381", 1, 250, Kind::Name);
    refused(SALE, with_buyer, "/buyer/legal_name", "$.buyer.legal_name", "not only spaces (SUNAT 4381)", " ");
    accepted(SALE, with_buyer, "/buyer/legal_name", "");
}

fn reason_13(v: &mut Value) {
    v["shipment"]["reason_code"] = json!("13");
}

#[test]
fn reason_description_4190_length_for_reason_13() {
    let (pointer, path) = ("/shipment/reason_description", "$.shipment.reason_description");
    assert_borders(SALE, reason_13, pointer, path, "4190", 3, 100, Kind::Letters);
    // Other reasons: SUNAT reads it only with reason 13.
    accepted(SALE, same, pointer, "A\nB");
}

#[test]
fn reason_description_4190_three_letters_for_reason_13() {
    let (pointer, path) = ("/shipment/reason_description", "$.shipment.reason_description");
    for value in ["123", "A1 2-3", "AB 1"] {
        refused(SALE, reason_13, pointer, path, "at least 3 letters (SUNAT 4190)", value);
    }
    accepted(SALE, reason_13, pointer, "ÁÑo");
    accepted(SALE, same, pointer, "123");
}

#[test]
fn carrier_legal_name_4165_in_public_transport() {
    let (pointer, path) = ("/shipment/carrier/legal_name", "$.shipment.carrier.legal_name");
    assert_borders(BETWEEN, same, pointer, path, "4165", 3, 250, Kind::Name);
}

#[test]
fn carrier_mtc_registration_4392() {
    let (pointer, path) = ("/shipment/carrier/mtc_registration", "$.shipment.carrier.mtc_registration");
    for value in ["A", "0A", &"Z".repeat(20)] {
        accepted(BETWEEN, same, pointer, value);
    }
    let too_long = "Z".repeat(21);
    for value in ["000", "MTC 15", "mtc15", "MTC\n15", "MTC\t15", "MTC-15", "MTCÑ", &too_long] {
        refused(BETWEEN, same, pointer, path, "SUNAT 4392", value);
    }
    accepted(BETWEEN, same, pointer, "");
}

#[test]
fn carrier_text_is_not_read_with_m1_or_l_vehicles() {
    let m1 = |v: &mut Value| v["shipment"]["m1_or_l_vehicle"] = json!(true);
    accepted(BETWEEN, m1, "/shipment/carrier/mtc_registration", "mtc 15");
    accepted(BETWEEN, m1, "/shipment/carrier/legal_name", "AB");
}

fn second_driver(v: &mut Value) {
    let mut driver = v["shipment"]["drivers"][0].clone();
    driver["document_number"] = json!("12345678");
    v["shipment"]["drivers"].as_array_mut().unwrap().push(driver);
}

#[test]
fn main_driver_names_4409() {
    for field in ["first_name", "family_name"] {
        let (pointer, path) = (format!("/shipment/drivers/0/{field}"), format!("$.shipment.drivers[0].{field}"));
        assert_borders(SALE, same, &pointer, &path, "4409", 1, 250, Kind::Name);
    }
}

#[test]
fn secondary_driver_names_4409() {
    for field in ["first_name", "family_name"] {
        let (pointer, path) = (format!("/shipment/drivers/1/{field}"), format!("$.shipment.drivers[1].{field}"));
        assert_borders(SALE, second_driver, &pointer, &path, "4409", 1, 250, Kind::Name);
    }
}

#[test]
fn origin_address_4076() {
    let (pointer, path) = ("/shipment/origin/address", "$.shipment.origin.address");
    assert_borders(SALE, same, pointer, path, "4076", 3, 500, Kind::Name);
}

#[test]
fn destination_address_4068() {
    let (pointer, path) = ("/shipment/destination/address", "$.shipment.destination.address");
    assert_borders(SALE, same, pointer, path, "4068", 3, 500, Kind::Name);
}

#[test]
fn line_description_4084_allows_line_breaks() {
    let (pointer, path) = ("/lines/0/description", "$.lines[0].description");
    assert_borders(SALE, same, pointer, path, "4084", 3, 500, Kind::AnyWhitespace);
    accepted(SALE, same, pointer, "Caja\n\tcuadernos\r\n");
}

#[test]
fn line_product_code_4085() {
    let (pointer, path) = ("/lines/0/product_code", "$.lines[0].product_code");
    assert_borders(SALE, same, pointer, path, "4085", 1, 30, Kind::Spaced);
    accepted(SALE, same, pointer, "");
}

#[test]
fn an_error_answers_before_the_observation() {
    // A blank name is error 2761, not observation 4152.
    let result = set(SALE, same, "/recipient/legal_name", "   ");
    assert_eq!(result.unwrap_err().sunat_code(), Some("2761"));
}

#[test]
fn the_value_is_refused_not_rewritten() {
    let document = set(SALE, same, "/shipment/origin/address", " AV. EJEMPLO  123 ").unwrap();
    let Document::Despatch(despatch) = document else { panic!("not a despatch advice") };
    assert_eq!(despatch.shipment.origin.address, " AV. EJEMPLO  123 ");
}
