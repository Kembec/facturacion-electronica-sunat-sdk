//! Runs the binary for the offline commands and for argument errors; no network is touched.

use std::io::Write;
use std::process::{Command, Output, Stdio};

fn run(args: &[&str], stdin: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_sunat-sdk"))
        .args(args)
        .env_remove("SUNAT_ENDPOINT")
        .env_remove("SUNAT_SOL_USER")
        .env_remove("SUNAT_SOL_PASSWORD")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(stdin.as_bytes()).unwrap();
    child.wait_with_output().unwrap()
}

fn stdout_json(output: &Output) -> serde_json::Value {
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn zip_then_cdr_round_trip_through_stdin_and_stdout() {
    let out = run(&["zip"], r#"{"name":"20100066603-03-B001-10","xml":"<Invoice/>"}"#);
    assert!(out.status.success());
    assert_eq!(stdout_json(&out)["file_name"], "20100066603-03-B001-10.zip");

    let cdr_xml = "<ar:ApplicationResponse xmlns:ar=\"a\" xmlns:cac=\"c\" xmlns:cbc=\"b\"><cac:DocumentResponse><cac:Response><cbc:ReferenceID>B001-10</cbc:ReferenceID><cbc:ResponseCode>4031</cbc:ResponseCode><cbc:Description>Aceptada</cbc:Description></cac:Response></cac:DocumentResponse></ar:ApplicationResponse>";
    use base64::Engine;
    let input = serde_json::json!({ "cdr_base64": base64::engine::general_purpose::STANDARD.encode(cdr_xml) });
    let out = run(&["cdr"], &input.to_string());
    assert!(out.status.success());
    assert_eq!(stdout_json(&out)["class"], "observed");
}

#[test]
fn send_without_credentials_fails_before_reading_or_sending() {
    let out = run(&["send", "--endpoint", "https://sunat.invalid/billService"], "{}");
    assert!(!out.status.success());
    let err: serde_json::Value = serde_json::from_slice(&out.stderr).unwrap();
    assert_eq!(err["error"], "missing --user or SUNAT_SOL_USER");
}

#[test]
fn plain_http_endpoint_is_refused_and_password_is_not_echoed() {
    let out = run(&["send", "--endpoint", "http://sunat.invalid", "--user", "20100066603U", "--password", "pw-secret"], "{}");
    assert!(!out.status.success());
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.contains("https://") && !stderr.contains("pw-secret"));
}
