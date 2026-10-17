use std::path::{Path, PathBuf};

use serde_json::Value;

use super::*;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(relative: &str) -> String {
    let path = root().join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn cases() -> Vec<String> {
    let mut cases: Vec<String> = std::fs::read_dir(root().join("fixtures"))
        .unwrap()
        .filter_map(|entry| entry.unwrap().file_name().into_string().unwrap().strip_suffix(".json").map(str::to_owned))
        .collect();
    cases.sort();
    assert_eq!(cases.len(), 38, "fixtures");
    cases
}

fn parse(output: &str) -> Value {
    serde_json::from_str(output).unwrap()
}

fn cert() -> String {
    read("tests/vectors/test-cert/test-cert.pem")
}

fn key() -> String {
    read("tests/vectors/test-cert/test-key.pem")
}

#[test]
fn build_then_sign_reproduces_every_vector() {
    for case in cases() {
        let built = parse(&build_json(&read(&format!("fixtures/{case}.json"))).unwrap());
        let xml = built["xml"].as_str().unwrap();
        let type_code = built["file_name"].as_str().unwrap().split('-').nth(1).unwrap().to_lowercase();
        assert!(case.starts_with(&type_code), "{case}: file_name document type");

        let signed = parse(&sign_json(xml, &cert(), &key(), None).unwrap());
        let expected = read(&format!("tests/vectors/{case}.signed.xml"));
        let expected_digest = element_text(&expected, "ds:DigestValue").unwrap();
        let expected_signature = element_text(&expected, "ds:SignatureValue").unwrap();
        assert_eq!(signed["digest_value"], expected_digest.as_str(), "{case}: digest");
        assert_eq!(signed["signature_value"], expected_signature.as_str(), "{case}: signature");
        assert!(signed["xml"].as_str().unwrap().contains("Id=\"signatureKG\""), "{case}: default id");
    }
}

#[test]
fn sign_uses_the_given_signature_id() {
    let case = &cases()[0];
    let xml = read(&format!("tests/vectors/{case}.unsigned.xml"));
    let signed = parse(&sign_json(&xml, &cert(), &key(), Some("customSig")).unwrap());
    assert!(signed["xml"].as_str().unwrap().contains("Id=\"customSig\""));
}

#[test]
fn build_rejects_invalid_json() {
    assert!(build_json("{not json").is_err());
}

#[test]
fn sign_error_does_not_carry_key_material() {
    let case = &cases()[0];
    let xml = read(&format!("tests/vectors/{case}.unsigned.xml"));
    let broken_key = key().replace("PRIVATE KEY", "BROKEN KEY");
    let message = sign_json(&xml, &cert(), &broken_key, None).unwrap_err();
    let key_line = key().lines().nth(1).unwrap().to_owned();
    assert!(!message.contains(&key_line[1..]), "error must not carry key material");
}

#[test]
fn soap_request_is_fetch_options_and_soap_response_matches_the_cli() {
    let doc = r#"{"name": "20100066603-01-F001-1", "xml": "<Invoice/>"}"#;
    let post = parse(&soap_request_json("send", doc, "20100066603MODDATOS", "moddatos").unwrap());
    assert_eq!(post["method"], "POST");
    assert_eq!(post["headers"], serde_json::json!({ "Content-Type": "text/xml; charset=utf-8", "SOAPAction": "\"urn:sendBill\"" }));
    assert!(post["body"].as_str().unwrap().contains("<fileName>20100066603-01-F001-1.zip</fileName>"));

    let cdr_zip = std::fs::read(root().join("crates/sunat-transport/tests/data/R-20150147718-01-FT01-982.zip")).unwrap();
    let cdr_base64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &cdr_zip);
    let answer = format!("<S:Envelope xmlns:S=\"s\"><S:Body><n:sendBillResponse xmlns:n=\"n\"><applicationResponse>{cdr_base64}</applicationResponse></n:sendBillResponse></S:Body></S:Envelope>");
    let sent = parse(&soap_response_json("send", doc, 200, answer.as_bytes()).unwrap());
    assert_eq!((sent["outcome"].as_str(), sent["cdr"]["class"].as_str()), (Some("cdr"), Some("rejected")));
    let read_alone = parse(&cdr_json(&format!(r#"{{"cdr_base64": "{cdr_base64}"}}"#)).unwrap());
    assert_eq!(read_alone, sent["cdr"]);

    let ticket = r#"{"ticket": "1760620000123"}"#;
    let unauthorized = parse(&soap_response_json("ticket", ticket, 401, b"").unwrap());
    assert_eq!((unauthorized["outcome"].as_str(), unauthorized["ticket"].as_str()), (Some("no_verdict"), Some("1760620000123")));
    assert_eq!(parse(&soap_timeout_json("status", "{}", 45).unwrap())["reason"], "timeout");
    assert!(soap_timeout_json("send", doc, 45).unwrap_err().contains("outcome is unknown"));
    assert!(soap_request_json("zip", doc, "u", "p").is_err());
    assert_eq!(parse(&zip_json(doc).unwrap())["file_name"], "20100066603-01-F001-1.zip");
}

#[test]
fn validity_steps_are_fetch_options_and_outcomes_like_the_cli() {
    let token_call = parse(&validity_token_request_json("abc-123", "s3cr=t", None).unwrap());
    assert_eq!(token_call["url"], "https://api-seguridad.sunat.gob.pe/v1/clientesextranet/abc-123/oauth2/token/");
    assert!(token_call["body"].as_str().unwrap().ends_with("client_secret=s3cr%3Dt"));
    let token = parse(&validity_token_response_json(200, br#"{"access_token":"t0k","token_type":"JWT","expires_in":3600}"#).unwrap());
    assert_eq!(token["access_token"], "t0k");

    let input = r#"{"ruc":"20100066603","document_type":"03","series":"B001","number":7,"issue_date":"2026-10-15","total":"118.00"}"#;
    let query = parse(&validity_request_json(input, "t0k", Some("https://api.test/x")).unwrap());
    assert_eq!(query["url"], "https://api.test/x/20100066603/validarcomprobante");
    assert_eq!(query["headers"]["Authorization"], "Bearer t0k");
    let found = parse(&validity_response_json(200, br#"{"success":true,"message":"ok","data":{"estadoCp":"0"}}"#).unwrap());
    assert_eq!((found["outcome"].as_str(), found["state"].as_str()), (Some("validity"), Some("not_reported")));
    assert_eq!(parse(&validity_response_json(401, b"").unwrap())["stage"], "query");
    assert_eq!(parse(&validity_timeout_json("token", 30).unwrap())["reason"], "timeout");
    assert!(validity_timeout_json("send", 30).is_err());
    assert!(validity_request_json(r#"{"ruc":"1"}"#, "t0k", None).is_err());
}

/// The 2800 the beta answered on 17/10/2026 to a boleta (`test/data/beta-2800-fault.xml`, HTTP 200).
fn beta_2800() -> Vec<u8> {
    std::fs::read(root().join("bindings/wasm/test/data/beta-2800-fault.xml")).unwrap()
}

/// getStatusCdr with the manual's observed CDR (annex C.1), its first note turned into a 4287.
fn status_observed_4287() -> Vec<u8> {
    let cdr = read("crates/sunat-transport/tests/data/R-20150147718-01-FA01-981.xml")
        .replace("4031 - Debe indicar el nombre comercial", "4287 - El precio unitario de la operacion que se informa difiere de los calculos realizados en base a la cantidad, valor unitario y descuentos");
    let content = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, cdr.as_bytes());
    format!("<S:Envelope xmlns:S=\"s\"><S:Body><n:getStatusCdrResponse xmlns:n=\"n\"><statusCdr><statusCode>0004</statusCode><content>{content}</content><statusMessage>La constancia existe</statusMessage></statusCdr></n:getStatusCdrResponse></S:Body></S:Envelope>").into_bytes()
}

const BOLETA: &str = r#"{"name": "20100066603-03-B001-1", "xml": "<Invoice/>"}"#;
const BOLETA_STATUS: &str = r#"{"ruc": "20100066603", "document_type": "03", "series": "B001", "number": 1}"#;

#[test]
fn soap_response_diagnoses_the_beta_2800_on_the_customer_document_type() {
    let sent = parse(&soap_response_json("send", BOLETA, 200, &beta_2800()).unwrap());
    assert_eq!((sent["outcome"].as_str(), sent["code"].as_str()), (Some("fault"), Some("2800")));
    let expected = serde_json::json!([{
        "code": "2800",
        "detail": { "node": "cbc:ID/schemeID", "value": "1" },
        "fields": [{
            "input_path": "customer.document_type",
            "xpath": "/Invoice/cac:AccountingCustomerParty/cac:Party/cac:PartyIdentification/cbc:ID@schemeID",
            "certainty": "fuente"
        }]
    }]);
    assert_eq!(sent["diagnosis"], expected);
    let alone = parse(&diagnose_json("03", "2800", sent["message"].as_str().unwrap()));
    assert_eq!(alone, expected[0]);
}

#[test]
fn soap_response_diagnoses_each_note_and_leaves_answers_without_codes_alone() {
    let observed = parse(&soap_response_json("status", BOLETA_STATUS, 200, &status_observed_4287()).unwrap());
    assert_eq!(observed["cdr"]["class"], "observed");
    assert_eq!(observed["diagnosis"][0]["code"], "4287");
    assert_eq!(observed["diagnosis"][0]["fields"][0]["input_path"], "lines[].unit_price");
    assert_eq!(observed["diagnosis"][1], serde_json::json!({ "code": "4001", "detail": null, "fields": [] }));

    let unauthorized = parse(&soap_response_json("ticket", r#"{"ticket": "1"}"#, 401, b"").unwrap());
    assert!(unauthorized.get("diagnosis").is_none());
    assert!(parse(&soap_timeout_json("status", BOLETA_STATUS, 45).unwrap()).get("diagnosis").is_none());
    assert_eq!(parse(&diagnose_json("01", "9999", "x")), serde_json::json!({ "code": "9999", "detail": null, "fields": [] }));
}

/// The PHP binding's tests feed these files as the CLI's output: they must be what the core gives.
#[test]
fn php_fixtures_are_the_cli_output_for_the_same_answers() {
    let fixture = |name: &str| parse(&read(&format!("bindings/php/tests/data/{name}.json")));
    assert_eq!(fixture("send-beta-2800"), parse(&soap_response_json("send", BOLETA, 200, &beta_2800()).unwrap()));
    assert_eq!(fixture("status-observed-4287"), parse(&soap_response_json("status", BOLETA_STATUS, 200, &status_observed_4287()).unwrap()));
}
