//! Replays GRE REST exchanges labelled by source (`tests/data/gre/README.md`): `manual-*` printed in
//! GRE-M, `derived-*` built from its field lists, `fork-*` from Greenter's tests (secondary source).
//! SUNAT has no test environment for GRE: none of these proves acceptance by SUNAT.

use super::*;

const TOKEN_BASE: &str = "https://auth.example.test/v1/clientessol";
const API_BASE: &str = "https://api.example.test/v1/";
const SIGNED: &str = "<?xml version=\"1.0\" encoding=\"utf-8\"?><DespatchAdvice/>";

fn data(name: &str) -> Vec<u8> {
    let path = format!("{}/tests/data/gre/{name}.json", env!("CARGO_MANIFEST_DIR"));
    std::fs::read(&path).unwrap_or_else(|e| panic!("{path}: {e}"))
}

fn guide() -> Value {
    json!({ "name": "20000000001-09-T001-1", "xml": SIGNED })
}

#[test]
fn token_request_is_gre_m_password_grant_with_secrets_encoded() {
    let request = token_request("c04864ce-f454", "s/e=c", "20000000001MODUSER", "p&ss", TOKEN_BASE).unwrap();
    assert_eq!(request["method"], "POST");
    assert_eq!(request["url"], "https://auth.example.test/v1/clientessol/c04864ce-f454/oauth2/token/");
    assert_eq!(request["headers"]["Content-Type"], "application/x-www-form-urlencoded");
    assert_eq!(
        request["body"],
        "grant_type=password&scope=https%3A%2F%2Fapi-cpe.sunat.gob.pe&client_id=c04864ce-f454&client_secret=s%2Fe%3Dc&username=20000000001MODUSER&password=p%26ss"
    );
    assert!(token_request("a/b", "s", "20000000001U", "p", TOKEN_BASE).is_err());
    assert!(token_request("abc", "", "20000000001U", "p", TOKEN_BASE).is_err());
    assert!(token_request("abc", "s", "20000000001U", "", TOKEN_BASE).is_err());
    for user in ["2000000000", "20000000001", "2000000000XU", "20000000001U/x"] {
        assert!(token_request("abc", "s", user, "p", TOKEN_BASE).unwrap_err().contains("RUC"), "{user}");
    }
    assert!(token_request("abc", "s", "20000000001U", "p", "").is_err());
}

#[test]
fn token_read_gives_expiry_from_the_callers_clock() {
    let token = token_read(Answer::Http(200, &data("derived-token-200")), 1_000).unwrap();
    assert_eq!(token["outcome"], "token");
    assert_eq!(token["access_token"], "fake.token.for-tests");
    assert_eq!((token["expires_in"].as_u64(), token["expires_at"].as_u64()), (Some(3600), Some(4600)));
    assert!(token_usable(&token, 1_000));
    assert!(token_usable(&token, 3_999));
    assert!(!token_usable(&token, 4_000), "renewed 10 minutes before expiry");
    assert!(!token_usable(&json!({ "access_token": "x" }), 0), "no expiry, no reuse");
    assert!(token_read(Answer::Http(200, b"{}"), 0).is_err());
}

#[test]
fn token_refused_is_an_api_error_and_silence_no_verdict() {
    let refused = token_read(Answer::Http(401, &data("derived-error-401")), 0).unwrap();
    assert_eq!((refused["outcome"].as_str(), refused["stage"].as_str()), (Some("api_error"), Some("token")));
    assert_eq!((refused["general_error"].as_str(), refused["code"].as_str()), (Some("unauthorized"), Some("401")));
    let html = token_read(Answer::Http(502, b"<html/>"), 0).unwrap();
    assert_eq!((html["outcome"].as_str(), html["http_status"].as_u64()), (Some("no_verdict"), Some(502)));
    let late = token_read(Answer::Timeout(30), 0).unwrap();
    assert_eq!((late["outcome"].as_str(), late["reason"].as_str()), (Some("no_verdict"), Some("timeout")));
}

#[test]
fn send_request_zips_the_guide_and_hashes_the_zip() {
    let request = send_request(&guide(), "tok", API_BASE).unwrap();
    assert_eq!(request["method"], "POST");
    assert_eq!(request["url"], "https://api.example.test/v1/contribuyente/gem/comprobantes/20000000001-09-T001-1");
    assert_eq!(request["headers"]["Authorization"], "Bearer tok");
    assert_eq!(request["headers"]["Content-Type"], "application/json");
    let body: Value = serde_json::from_str(request["body"].as_str().unwrap()).unwrap();
    let file = &body["archivo"];
    assert_eq!(file["nomArchivo"], "20000000001-09-T001-1.zip");
    let zip = STANDARD.decode(file["arcGreZip"].as_str().unwrap()).unwrap();
    let hash: String = Sha256::digest(&zip).iter().map(|b| format!("{b:02x}")).collect();
    assert_eq!(file["hashZip"], json!(hash));
    assert_eq!(request["hash_zip"], json!(hash));
    assert_eq!(hash.len(), 64);
    assert_eq!(sunat_transport::unzip_xml(&zip).unwrap(), SIGNED.as_bytes());

    let mut encoded = guide();
    encoded["xml"] = Value::Null;
    encoded["xml_base64"] = json!(STANDARD.encode(SIGNED));
    assert_eq!(send_request(&encoded, "tok", API_BASE).unwrap()["hash_zip"].as_str().map(str::len), Some(64));
}

#[test]
fn send_request_refuses_what_is_not_a_09() {
    let with_name = |name: &str| send_request(&json!({ "name": name, "xml": SIGNED }), "tok", API_BASE);
    assert!(with_name("20000000001-31-V001-1").unwrap_err().contains("RUC-09"), "31 is outside this client");
    assert!(with_name("20000000001-01-F001-1").is_err());
    assert!(with_name("2000000000-09-T001-1").is_err());
    assert!(with_name("20000000001-09-T001-1/../x").is_err());
    assert!(send_request(&guide(), "", API_BASE).is_err());
    assert!(send_request(&json!({ "name": "20000000001-09-T001-1" }), "tok", API_BASE).is_err());
}

#[test]
fn send_read_gives_the_ticket_from_the_fork_exchange() {
    let out = send_read(Answer::Http(200, &data("fork-send-200"))).unwrap();
    assert_eq!(out, json!({ "outcome": "ticket", "ticket": "a.b.c", "received_at": "2023-01-10T20:10:50" }));
}

#[test]
fn send_rejected_carries_the_validation_errors_in_either_shape() {
    let fork = send_read(Answer::Http(422, &data("fork-send-422"))).unwrap();
    assert_eq!((fork["outcome"].as_str(), fork["stage"].as_str()), (Some("api_error"), Some("send")));
    assert_eq!(fork["errors"], json!([{ "code": "501", "description": "El valor de codCpe no permitido o no valido" }]));
    let server = send_read(Answer::Http(500, &data("manual-error-500"))).unwrap();
    assert_eq!((server["general_error"].as_str(), server["code"].as_str()), (Some("internal_server_error"), Some("500")));
    assert!(server.get("exc").is_none(), "the server trace is not copied");
}

#[test]
fn send_without_an_answer_is_an_error_because_the_guide_may_have_arrived() {
    assert!(send_read(Answer::Timeout(90)).unwrap_err().contains("may have arrived"));
    assert!(send_read(Answer::Http(502, b"<html/>")).unwrap_err().contains("may have arrived"));
    assert!(send_read(Answer::Http(200, b"{}")).unwrap_err().contains("numTicket"));
}

#[test]
fn ticket_request_is_a_get_on_the_ticket() {
    let request = ticket_request("a.b.c", "tok", API_BASE).unwrap();
    assert_eq!(request["method"], "GET");
    assert_eq!(request["url"], "https://api.example.test/v1/contribuyente/gem/comprobantes/envios/a.b.c");
    assert_eq!(request["headers"]["Authorization"], "Bearer tok");
    assert!(request["headers"].get("Content-Type").is_none());
    for bad in ["", "..", "a/b", "a?b", "a b"] {
        assert!(ticket_request(bad, "tok", API_BASE).is_err(), "{bad}");
    }
}

#[test]
fn ticket_in_process_is_98_and_never_a_reason_to_send_again() {
    let out = ticket_read(Answer::Http(200, &data("fork-ticket-98")), "a.b.c").unwrap();
    assert_eq!(
        out,
        json!({ "outcome": "ticket_status", "status_code": "98", "state": "in_process", "cdr_generated": false, "ticket": "a.b.c" })
    );
}

#[test]
fn ticket_with_cdr_reads_it_with_the_sdk_reader() {
    let out = ticket_read(Answer::Http(200, &data("fork-ticket-0")), "a.b.c").unwrap();
    assert_eq!((out["state"].as_str(), out["cdr_generated"].as_bool()), (Some("processed"), Some(true)));
    assert_eq!(out["cdr"]["reference_id"], "T001-1");
    assert_eq!(out["cdr"]["response_code"], "0");
    assert_eq!(out["cdr"]["class"], "observed", "note 4398 on an accepted guide");
    let zip = STANDARD.decode(out["cdr_zip_base64"].as_str().unwrap()).unwrap();
    assert!(zip.starts_with(b"PK\x03\x04"));
}

#[test]
fn ticket_rejected_is_99_with_the_error_and_its_class() {
    let out = ticket_read(Answer::Http(200, &data("fork-ticket-99")), "a.b.c").unwrap();
    assert_eq!((out["state"].as_str(), out["cdr_generated"].as_bool()), (Some("processed_with_errors"), Some(false)));
    assert_eq!(out["error"], json!({ "code": "1345", "description": "El RUC no valido", "class": "exception" }));
    assert!(out.get("cdr").is_none());
}

#[test]
fn ticket_errors_follow_gre_m_and_silence_is_no_verdict() {
    let manual = ticket_read(Answer::Http(422, &data("manual-error-422-ticket")), "x").unwrap();
    assert_eq!((manual["outcome"].as_str(), manual["stage"].as_str()), (Some("api_error"), Some("ticket")));
    assert_eq!(manual["errors"], json!([{ "code": "166", "description": "Código de ticket no enviado." }]));
    assert_eq!(manual["ticket"], "x");
    let late = ticket_read(Answer::Timeout(30), "x").unwrap();
    assert_eq!((late["outcome"].as_str(), late["stage"].as_str(), late["ticket"].as_str()), (Some("no_verdict"), Some("ticket"), Some("x")));
    let gateway = ticket_read(Answer::Http(504, b""), "x").unwrap();
    assert_eq!(gateway["outcome"], "no_verdict");
    assert!(ticket_read(Answer::Http(200, b"[]"), "x").is_err());
    let broken = br#"{"codRespuesta":"0","indCdrGenerado":"1"}"#;
    assert!(ticket_read(Answer::Http(200, broken), "x").unwrap_err().contains("arcCdr"));
    assert_eq!(timeout(Stage::Send, 5)["stage"], "send");
    assert_eq!(Stage::from_name("ticket"), Ok(Stage::Ticket));
}
