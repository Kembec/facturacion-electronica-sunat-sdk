//! The battery through the C ABI: every fixture is built and signed with `sunat_sdk_call`, and the
//! XML, `DigestValue` and `SignatureValue` must equal its vector, as `bindings/cli/tests/build_sign.rs`
//! checks through the binary. Also the ABI's own contract: statuses, ownership, the copy for callers
//! that cannot read a pointer, the network path, and concurrent calls.

use std::ffi::{c_char, CStr, CString};
use std::path::{Path, PathBuf};
use std::ptr;

use serde_json::{json, Value};
use sunat_sdk_ffi::*;

const EXPECTED_CASES: usize = 38;

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(path: PathBuf) -> String {
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn test_cert() -> (String, String) {
    let dir = repository().join("tests/vectors/test-cert");
    (dir.join("test-cert.pem").display().to_string(), dir.join("test-key.pem").display().to_string())
}

/// One call through the exported functions; the response is copied out and released.
fn call_raw(request: &[u8]) -> (i32, Value) {
    let request = CString::new(request).unwrap();
    let mut response: *mut c_char = ptr::null_mut();
    let status = unsafe { sunat_sdk_call(request.as_ptr(), &mut response) };
    assert!(!response.is_null());
    let text = unsafe { CStr::from_ptr(response) }.to_str().unwrap().to_owned();
    unsafe { sunat_sdk_free(response) };
    (status, serde_json::from_str(&text).unwrap_or_else(|e| panic!("{e}: {text}")))
}

/// The request is written as text so each fixture reaches the command byte for byte.
fn call(request: &str) -> (i32, Value) {
    call_raw(request.as_bytes())
}

fn ok(request: &str, what: &str) -> Value {
    let (status, value) = call(request);
    assert_eq!(status, SUNAT_SDK_OK, "{what}: {value}");
    value
}

fn error(request: &str) -> String {
    let (status, value) = call(request);
    assert_eq!(status, SUNAT_SDK_ERROR, "{value}");
    value["error"].as_str().unwrap_or_else(|| panic!("{value}")).to_owned()
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
    let mut cases: Vec<String> = std::fs::read_dir(repository().join("fixtures"))
        .unwrap()
        .filter_map(|entry| entry.ok()?.file_name().into_string().ok()?.strip_suffix(".json").map(str::to_owned))
        .collect();
    cases.sort();
    assert_eq!(cases.len(), EXPECTED_CASES, "{cases:?}");
    let (cert, key) = test_cert();
    for case in &cases {
        let fixture = read(repository().join("fixtures").join(format!("{case}.json")));
        let built = ok(&format!(r#"{{"command":"build","input":{fixture}}}"#), case);
        let xml = built["xml"].as_str().unwrap();
        let unsigned = read(repository().join(format!("tests/vectors/{case}.unsigned.xml")));
        assert!(canonical(xml) == canonical(&unsigned), "{case}: built XML differs from its vector");

        let sign = json!({ "command": "sign", "flags": { "cert": cert, "key": key }, "input": { "xml": xml } });
        let signed = ok(&sign.to_string(), case);
        let expected = read(repository().join(format!("tests/vectors/{case}.signed.xml")));
        assert_eq!(signed["digest_value"], element_text(&expected, "ds:DigestValue"), "{case}");
        assert_eq!(signed["signature_value"], element_text(&expected, "ds:SignatureValue"), "{case}");
        assert!(sunat_xmldsig::verify(signed["xml"].as_str().unwrap().as_bytes()).unwrap().is_valid(), "{case}");
    }
}

#[test]
fn the_output_is_the_clis_output() {
    // `zip` then `cdr` round-trip in memory, with the shapes of docs/cli.md.
    let zipped = ok(r#"{"command":"zip","input":{"name":"20000000001-01-F001-1","xml":"<a/>"}}"#, "zip");
    assert_eq!(zipped["file_name"], "20000000001-01-F001-1.zip");
    assert!(zipped["zip_base64"].as_str().is_some_and(|z| !z.is_empty()));
    assert_eq!(error(r#"{"command":"build","flags":{"timeout":3},"input":{}}"#), "`build` takes no flags");
}

#[test]
fn null_pointers_and_bad_text_are_reported_not_crashed() {
    let mut response: *mut c_char = ptr::null_mut();
    assert_eq!(unsafe { sunat_sdk_call(c"{}".as_ptr(), ptr::null_mut()) }, SUNAT_SDK_NO_RESPONSE_SLOT);
    assert_eq!(unsafe { sunat_sdk_call(ptr::null(), &mut response) }, SUNAT_SDK_ERROR);
    let text = unsafe { CStr::from_ptr(response) }.to_str().unwrap().to_owned();
    unsafe { sunat_sdk_free(response) };
    assert_eq!(text, r#"{"error":"request is null"}"#);
    // Latin-1 «ñ», as a VB6 `Declare ... As String` would pass it: rejected, never misread.
    let (status, value) = call_raw(b"{\"command\":\"build\",\"input\":{\"legal_name\":\"Espa\xf1a\"}}");
    assert_eq!((status, value["error"].as_str()), (SUNAT_SDK_ERROR, Some("request is not UTF-8")));
    unsafe { sunat_sdk_free(ptr::null_mut()) };
    assert_eq!(unsafe { sunat_sdk_response_len(ptr::null()) }, 0);
}

#[test]
fn the_copy_serves_callers_that_cannot_read_a_pointer() {
    let mut response: *mut c_char = ptr::null_mut();
    let request = CString::new(r#"{"command":"zip","input":{"name":"20000000001-01-F001-1","xml":"<a>ñ</a>"}}"#).unwrap();
    assert_eq!(unsafe { sunat_sdk_call(request.as_ptr(), &mut response) }, SUNAT_SDK_OK);
    let len = unsafe { sunat_sdk_response_len(response) };
    let expected = unsafe { CStr::from_ptr(response) }.to_bytes().to_vec();
    assert_eq!(len, expected.len());

    let mut small = vec![7 as c_char; len - 1];
    assert_eq!(unsafe { sunat_sdk_response_copy(response, small.as_mut_ptr(), small.len()) }, SUNAT_SDK_BUFFER_TOO_SMALL);
    assert!(small.iter().all(|&b| b == 7), "nothing written on a short buffer");
    let mut exact = vec![0 as c_char; len];
    assert_eq!(unsafe { sunat_sdk_response_copy(response, exact.as_mut_ptr(), len) }, SUNAT_SDK_OK);
    assert_eq!(exact.iter().map(|&b| b as u8).collect::<Vec<u8>>(), expected);
    let mut roomy = vec![7 as c_char; len + 4];
    assert_eq!(unsafe { sunat_sdk_response_copy(response, roomy.as_mut_ptr(), roomy.len()) }, SUNAT_SDK_OK);
    assert_eq!(roomy[len], 0, "NUL after the bytes when there is room");
    assert_eq!(unsafe { sunat_sdk_response_copy(response, ptr::null_mut(), 10) }, SUNAT_SDK_ERROR);
    unsafe { sunat_sdk_free(response) };
}

#[test]
fn network_commands_reach_the_clis_transport() {
    // Plain HTTP is refused before any call, as in the CLI.
    let send = r#"{"command":"send","flags":{"endpoint":"http://127.0.0.1:9","user":"20000000001MODDATOS","password":"x"},"input":{"name":"20000000001-01-F001-1","xml":"<a/>"}}"#;
    assert_eq!(error(send), "endpoint must use https://");
    // Nothing listens on port 9 locally: the HTTPS POST fails in transport, after the envelope was built.
    let refused = send.replace("http://", "https://");
    assert!(error(&refused).starts_with("transport error"), "{refused}");
    // A status query that gets no answer in time is `no_verdict`, exit 0 in the CLI and OK here. The
    // local listener accepts the connection and never answers the TLS handshake.
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let silent = std::thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        std::thread::sleep(std::time::Duration::from_secs(3));
        drop(stream);
    });
    let ticket = json!({
        "command": "ticket",
        "flags": { "endpoint": format!("https://127.0.0.1:{port}/billService"), "user": "20000000001MODDATOS", "password": "x", "timeout": 1 },
        "input": { "ticket": "1700000000000" },
    });
    let (status, value) = call(&ticket.to_string());
    assert_eq!(status, SUNAT_SDK_OK, "{value}");
    assert_eq!((value["outcome"].as_str(), value["reason"].as_str()), (Some("no_verdict"), Some("timeout")), "{value}");
    assert_eq!(value["ticket"], "1700000000000");
    silent.join().unwrap();
}

#[test]
fn concurrent_calls_share_nothing() {
    let fixture = read(repository().join("fixtures/01-invoice-taxed-cash.json"));
    let request = format!(r#"{{"command":"build","input":{fixture}}}"#);
    let expected = ok(&request, "build")["xml"].clone();
    std::thread::scope(|scope| {
        let handles: Vec<_> = (0..8).map(|_| scope.spawn(|| ok(&request, "build")["xml"].clone())).collect();
        for handle in handles {
            assert_eq!(handle.join().unwrap(), expected);
        }
    });
}

#[test]
fn versions_are_static() {
    let version = unsafe { CStr::from_ptr(sunat_sdk_version()) }.to_str().unwrap();
    assert_eq!(version, env!("CARGO_PKG_VERSION"));
    assert_eq!(sunat_sdk_abi_version(), SUNAT_SDK_ABI_VERSION);
}
