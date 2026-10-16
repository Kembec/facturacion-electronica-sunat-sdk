//! Runs `build` and `sign` through the binary on every fixture (vouchers `0[1378]-`, daily summaries
//! `rc-` and void communications `ra-`): the built XML equals
//! `tests/vectors/<case>.unsigned.xml` under inclusive C14N (the vectors come from another
//! serializer: declaration, CDATA and empty elements differ), and signing it with
//! `tests/vectors/test-cert/` gives the `DigestValue` and `SignatureValue` of `<case>.signed.xml`.
//! Also the error paths: stderr JSON, exit 1, empty stdout.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use serde_json::{json, Value};

const EXPECTED_CASES: usize = 28;

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn vectors() -> PathBuf {
    repository().join("tests/vectors")
}

fn cert_path() -> String {
    vectors().join("test-cert/test-cert.pem").display().to_string()
}

fn key_path() -> String {
    vectors().join("test-cert/test-key.pem").display().to_string()
}

fn run(args: &[&str], stdin: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_sunat-sdk"))
        .args(args)
        .env_remove("SUNAT_CERT_PATH")
        .env_remove("SUNAT_KEY_PATH")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    // A command may fail before reading stdin; a broken pipe here is not the test's concern.
    let _ = child.stdin.take().unwrap().write_all(stdin.as_bytes());
    child.wait_with_output().unwrap()
}

fn success(output: &Output, what: &str) -> Value {
    assert!(output.status.success(), "{what}: {}", String::from_utf8_lossy(&output.stderr));
    assert!(output.stderr.is_empty(), "{what}: stderr not empty");
    serde_json::from_slice(&output.stdout).unwrap_or_else(|e| panic!("{what}: {e}"))
}

/// Exit code 1, nothing on stdout, `{"error": <string>}` on stderr; returns the message.
fn failure(output: &Output, what: &str) -> String {
    assert_eq!(output.status.code(), Some(1), "{what}");
    assert!(output.stdout.is_empty(), "{what}: stdout not empty");
    let err: Value = serde_json::from_slice(&output.stderr).unwrap_or_else(|e| panic!("{what}: {e}"));
    err["error"].as_str().unwrap_or_else(|| panic!("{what}: {err}")).to_string()
}

fn cases() -> Vec<String> {
    let mut cases: Vec<String> = std::fs::read_dir(repository().join("fixtures"))
        .unwrap()
        .filter_map(|entry| entry.ok()?.file_name().into_string().ok()?.strip_suffix(".json").map(str::to_owned))
        .collect();
    cases.sort();
    cases
}

fn read(path: PathBuf) -> String {
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn fixture(case: &str) -> String {
    read(repository().join("fixtures").join(format!("{case}.json")))
}

fn element_text<'a>(xml: &'a str, element: &str) -> &'a str {
    let open = format!("<{element}>");
    let start = xml.find(&open).unwrap_or_else(|| panic!("no {element}")) + open.len();
    &xml[start..start + xml[start..].find('<').unwrap()]
}

fn canonical(xml: &str) -> Vec<u8> {
    sunat_xmldsig::canonicalize(xml.as_bytes()).unwrap()
}

#[test]
fn every_fixture_builds_and_signs_like_its_vector() {
    let cases = cases();
    assert_eq!(cases.len(), EXPECTED_CASES, "{cases:?}");
    let (cert, key) = (cert_path(), key_path());
    for case in &cases {
        let built = success(&run(&["build"], &fixture(case)), case);
        let xml = built["xml"].as_str().unwrap();
        let file_name = built["file_name"].as_str().unwrap();
        // `20123456789-01-F001-1`, `20123456789-RC-20261015-1`: the type matches the case prefix.
        assert!(case.starts_with(&file_name[12..14].to_lowercase()), "{case}: {file_name}");
        let unsigned = read(vectors().join(format!("{case}.unsigned.xml")));
        assert!(canonical(xml) == canonical(&unsigned), "{case}: built XML differs from its vector");

        let input = json!({ "xml": xml }).to_string();
        let signed = success(&run(&["sign", "--cert", &cert, "--key", &key], &input), case);
        let expected = read(vectors().join(format!("{case}.signed.xml")));
        assert_eq!(signed["digest_value"], element_text(&expected, "ds:DigestValue"), "{case}");
        assert_eq!(signed["signature_value"], element_text(&expected, "ds:SignatureValue"), "{case}");
        let signed_xml = signed["xml"].as_str().unwrap();
        assert_eq!(element_text(signed_xml, "ds:DigestValue"), signed["digest_value"], "{case}");
        assert!(sunat_xmldsig::verify(signed_xml.as_bytes()).unwrap().is_valid(), "{case}");
    }
}

#[test]
fn sign_reads_paths_from_environment_and_honours_signature_id() {
    let xml = success(&run(&["build"], &fixture(&cases()[0])), "build")["xml"].as_str().unwrap().to_string();
    let mut child = Command::new(env!("CARGO_BIN_EXE_sunat-sdk"))
        .arg("sign")
        .env("SUNAT_CERT_PATH", cert_path())
        .env("SUNAT_KEY_PATH", key_path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let input = json!({ "xml": xml, "signature_id": "otherId" }).to_string();
    child.stdin.take().unwrap().write_all(input.as_bytes()).unwrap();
    let signed = success(&child.wait_with_output().unwrap(), "sign via env");
    let verification = sunat_xmldsig::verify(signed["xml"].as_str().unwrap().as_bytes()).unwrap();
    assert_eq!(verification.signature_id.as_deref(), Some("otherId"));
}

#[test]
fn build_rejects_invalid_json_and_out_of_scope_documents() {
    let message = failure(&run(&["build"], "{not json"), "invalid JSON");
    assert!(message.starts_with("invalid input"), "{message}");

    let mut document: Value = serde_json::from_str(&fixture("01-invoice-taxed-cash")).unwrap();
    document["detraction"] = json!({});
    let message = failure(&run(&["build"], &document.to_string()), "out of scope");
    assert!(message.contains("detraction is out of scope"), "{message}");

    failure(&run(&["build", "--cert", "x"], "{}"), "build with flags");
}

#[test]
fn sign_without_cert_or_key_fails_before_signing() {
    let input = json!({ "xml": "<a/>" }).to_string();
    let key = key_path();
    let cert = cert_path();
    assert_eq!(failure(&run(&["sign", "--key", &key], &input), "no cert"), "missing --cert or SUNAT_CERT_PATH");
    assert_eq!(failure(&run(&["sign", "--cert", &cert], &input), "no key"), "missing --key or SUNAT_KEY_PATH");
    let message = failure(&run(&["sign", "--cert", &cert, "--key", "/nonexistent/key.pem"], &input), "missing file");
    assert!(message.contains("--key") && !message.contains("nonexistent"), "{message}");
}

#[test]
fn unreadable_key_is_an_error_that_never_shows_the_key() {
    let key_pem = read(vectors().join("test-cert/test-key.pem"));
    let broken = key_pem.replacen("-----BEGIN PRIVATE KEY-----\n", "-----BEGIN PRIVATE KEY-----\n!", 1);
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("sunat-sdk-cli");
    std::fs::create_dir_all(&dir).unwrap();
    let broken_path = dir.join("broken-key.pem");
    std::fs::write(&broken_path, &broken).unwrap();

    let xml = success(&run(&["build"], &fixture(&cases()[0])), "build")["xml"].as_str().unwrap().to_string();
    let input = json!({ "xml": xml }).to_string();
    let output = run(&["sign", "--cert", &cert_path(), "--key", &broken_path.display().to_string()], &input);
    let message = failure(&output, "broken key");
    assert!(message.starts_with("invalid private key"), "{message}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    for line in key_pem.lines().filter(|l| !l.starts_with("-----")) {
        assert!(!stderr.contains(&line[1..]), "stderr carries key material");
    }
}
