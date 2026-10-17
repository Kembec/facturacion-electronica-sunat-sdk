//! The retention voucher (20) through the binary: `build`, `sign` and `zip` on the first vector of
//! its contract (`fixtures/retention/v1-pen-simple.json`) give a signed XML that verifies and a zip
//! named `RUC-20-R###-N.zip`; a vector SUNAT would reject fails in `build` with its code.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde_json::{json, Value};

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn fixture(case: &str) -> String {
    let path = repository().join("fixtures/retention").join(format!("{case}.json"));
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
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
    let _ = child.stdin.take().unwrap().write_all(stdin.as_bytes());
    child.wait_with_output().unwrap()
}

fn success(output: &Output, what: &str) -> Value {
    assert!(output.status.success(), "{what}: {}", String::from_utf8_lossy(&output.stderr));
    serde_json::from_slice(&output.stdout).unwrap_or_else(|e| panic!("{what}: {e}"))
}

#[test]
fn build_sign_and_zip_a_retention_voucher() {
    let built = success(&run(&["build"], &fixture("v1-pen-simple")), "build");
    let name = built["file_name"].as_str().unwrap();
    assert_eq!(name, "20000000001-20-R001-1");

    let cert = repository().join("tests/vectors/test-cert/test-cert.pem").display().to_string();
    let key = repository().join("tests/vectors/test-cert/test-key.pem").display().to_string();
    let input = json!({ "xml": built["xml"] }).to_string();
    let signed = success(&run(&["sign", "--cert", &cert, "--key", &key], &input), "sign");
    let xml = signed["xml"].as_str().unwrap();
    assert!(sunat_xmldsig::verify(xml.as_bytes()).unwrap().is_valid());

    let input = json!({ "name": name, "xml": xml }).to_string();
    let zipped = success(&run(&["zip"], &input), "zip");
    assert_eq!(zipped["file_name"], "20000000001-20-R001-1.zip");
    let bytes = STANDARD.decode(zipped["zip_base64"].as_str().unwrap()).unwrap();
    assert_eq!(sunat_sdk::unzip_xml(&bytes).unwrap(), xml.as_bytes());
}

#[test]
fn build_rejects_a_rate_that_is_not_the_regimes() {
    let output = run(&["build"], &fixture("v4-rate-6-with-regime-01"));
    assert_eq!(output.status.code(), Some(1));
    let error: Value = serde_json::from_slice(&output.stderr).unwrap();
    let message = error["error"].as_str().unwrap();
    assert!(message.starts_with("$.rate") && message.ends_with("(SUNAT ERR-2619)"), "{message}");
}
