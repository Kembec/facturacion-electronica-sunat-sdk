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
        // The ra-/rc- summaries are phase 2, not build input.
        .filter(|case| ["01-", "03-", "07-", "08-"].iter().any(|prefix| case.starts_with(prefix)))
        .collect();
    cases.sort();
    assert_eq!(cases.len(), 12, "fixtures");
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
        let type_code = built["file_name"].as_str().unwrap().split('-').nth(1).unwrap().to_owned();
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
