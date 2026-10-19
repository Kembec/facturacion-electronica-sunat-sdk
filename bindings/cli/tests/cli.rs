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
    // A command may fail before reading stdin; a broken pipe here is not the test's concern.
    let _ = child.stdin.take().unwrap().write_all(stdin.as_bytes());
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

#[test]
fn gre_send_refuses_a_plain_http_base_before_any_call_and_echoes_no_secret() {
    let flags = [
        "--client-id", "c04864ce-f454", "--client-secret", "client-secret-x", "--user", "20000000001MODUSER",
        "--password", "sol-secret-x", "--token-endpoint", "https://auth.invalid/v1/clientessol", "--endpoint", "http://cpe.invalid/v1",
    ];
    let out = run(&[&["gre-send"][..], &flags[..]].concat(), r#"{"name":"20000000001-09-T001-1","xml":"<x/>"}"#);
    assert!(!out.status.success() && out.stdout.is_empty());
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.contains("--endpoint must use https://"));
    assert!(!stderr.contains("sol-secret-x") && !stderr.contains("client-secret-x"));
}

#[test]
fn version_flags_print_one_line_with_the_workspace_version_and_exit_0() {
    for flag in ["--version", "-V"] {
        let out = run(&[flag], "");
        assert!(out.status.success(), "{flag}");
        assert_eq!(String::from_utf8(out.stdout).unwrap(), format!("sunat-sdk {}\n", env!("CARGO_PKG_VERSION")));
        assert!(out.stderr.is_empty(), "{flag}");
    }
}

#[test]
fn no_arguments_still_print_the_usage_and_exit_1() {
    let out = run(&[], "");
    assert_eq!(out.status.code(), Some(1));
    assert!(out.stdout.is_empty());
    let err: serde_json::Value = serde_json::from_slice(&out.stderr).unwrap();
    assert!(err["error"].as_str().unwrap().starts_with("usage: sunat-sdk <build|"));
}
