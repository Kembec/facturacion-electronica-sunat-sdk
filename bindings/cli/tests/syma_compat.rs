//! What a consumer with Syma's storage needs from the binary: one combined PEM (certificate and key
//! in the same file) passed as both `--cert` and `--key`, and an HTTP wait it can bound below its
//! queue worker's timeout. No request reaches SUNAT: the endpoint is a local socket that never
//! answers.

use std::io::Write;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

use serde_json::{json, Value};

fn vectors() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/vectors")
}

fn read(path: PathBuf) -> String {
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn run(args: &[&str], envs: &[(&str, &str)], stdin: &str) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_sunat-sdk"));
    command.args(args).env_clear();
    for (key, value) in envs {
        command.env(key, value);
    }
    let mut child = command.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().unwrap();
    let _ = child.stdin.take().unwrap().write_all(stdin.as_bytes());
    child.wait_with_output().unwrap()
}

fn element_text<'a>(xml: &'a str, element: &str) -> &'a str {
    let start = xml.find(&format!("<{element}>")).unwrap() + element.len() + 2;
    let end = xml[start..].find(&format!("</{element}>")).unwrap();
    &xml[start..start + end]
}

/// The combined file in the layouts a PKCS#12 export or a hand-made concatenation produce: key
/// first or certificate first, LF or CRLF, `Bag Attributes` lines and blank lines around blocks.
fn combined_layouts() -> Vec<(&'static str, String)> {
    let cert = read(vectors().join("test-cert/test-cert.pem"));
    let key = read(vectors().join("test-cert/test-key.pem"));
    let bagged = format!("Bag Attributes\n    localKeyID: 01 00 00 00\nsubject=/CN=test\n{cert}\nBag Attributes\n    localKeyID: 01 00 00 00\n{key}\n");
    vec![
        ("cert then key", format!("{cert}{key}")),
        ("key then cert", format!("{key}\n\n{cert}")),
        ("CRLF", format!("{cert}{key}").replace('\n', "\r\n")),
        ("openssl pkcs12 -nodes", bagged),
    ]
}

#[test]
fn one_combined_pem_serves_as_cert_and_key_and_signs_like_the_vector() {
    let case = "01-invoice-taxed-cash";
    let unsigned = read(vectors().join(format!("{case}.unsigned.xml")));
    let expected = read(vectors().join(format!("{case}.signed.xml")));
    let dir = std::env::temp_dir().join(format!("sunat-sdk-combined-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let input = json!({ "xml": unsigned }).to_string();
    for (layout, text) in combined_layouts() {
        let path = dir.join("combined.pem");
        std::fs::write(&path, text).unwrap();
        let path = path.display().to_string();
        let out = run(&["sign", "--cert", &path, "--key", &path], &[], &input);
        assert!(out.status.success(), "{layout}: {}", String::from_utf8_lossy(&out.stderr));
        let signed: Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(signed["digest_value"], element_text(&expected, "ds:DigestValue"), "{layout}");
        assert_eq!(signed["signature_value"], element_text(&expected, "ds:SignatureValue"), "{layout}");
        // One certificate in the file, one in KeyInfo: the key block is never copied there.
        assert_eq!(signed["xml"].as_str().unwrap().matches("<ds:X509Certificate>").count(), 1, "{layout}");
        let by_env = run(&["sign"], &[("SUNAT_CERT_PATH", &path), ("SUNAT_KEY_PATH", &path)], &input);
        assert_eq!(serde_json::from_slice::<Value>(&by_env.stdout).unwrap()["signature_value"], signed["signature_value"], "{layout}");
    }
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn send_gives_up_after_the_configured_http_wait() {
    // Accepts the TCP connection and never speaks TLS: the client waits until its timeout.
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = format!("https://127.0.0.1:{}/billService", listener.local_addr().unwrap().port());
    let holder = std::thread::spawn(move || {
        let held: Vec<_> = listener.incoming().take(2).collect();
        std::thread::sleep(Duration::from_secs(8));
        drop(held);
    });
    let input = json!({ "name": "20100066603-01-F001-1", "xml": "<Invoice/>" }).to_string();
    let creds = [("SUNAT_SOL_USER", "20100066603MODDATOS"), ("SUNAT_SOL_PASSWORD", "moddatos")];

    let started = Instant::now();
    let out = run(&["send", "--endpoint", &endpoint, "--timeout", "1"], &creds, &input);
    let elapsed = started.elapsed();
    assert_eq!(out.status.code(), Some(1));
    let error: Value = serde_json::from_slice(&out.stderr).unwrap();
    assert!(error["error"].as_str().unwrap().contains("timed out after 1 s"), "{error}");
    assert!(elapsed >= Duration::from_millis(900) && elapsed < Duration::from_secs(6), "{elapsed:?}");

    let mut envs = creds.to_vec();
    envs.push(("SUNAT_HTTP_TIMEOUT_SECONDS", "2"));
    let started = Instant::now();
    let out = run(&["send", "--endpoint", &endpoint], &envs, &input);
    assert!(String::from_utf8_lossy(&out.stderr).contains("timed out after 2 s"));
    assert!(started.elapsed() >= Duration::from_millis(1900));
    holder.join().unwrap();
}
