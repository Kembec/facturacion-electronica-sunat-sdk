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
