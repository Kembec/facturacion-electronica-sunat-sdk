//! The binding's GRE layer replays the core's exchanges (crates/sunat-sdk/tests/data/gre, sources in
//! its README) and must print what the core prints for each. None of them proves acceptance by SUNAT.

use std::path::Path;

use serde_json::{json, Value};

use super::*;

const TOKEN_BASE: &str = "https://auth.example.test/v1/clientessol";
const API_BASE: &str = "https://api.example.test/v1";
const GUIDE: &str = r#"{"name": "20000000001-09-T001-1", "xml": "<DespatchAdvice/>"}"#;

fn data(name: &str) -> Vec<u8> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../../crates/sunat-sdk/tests/data/gre/{name}.json"));
    std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn parse(output: &str) -> Value {
    serde_json::from_str(output).unwrap()
}

#[test]
fn token_steps_match_the_core() {
    let call = parse(&gre_token_request_json("abc-123", "s/e=c", "20000000001MODUSER", "p&ss", TOKEN_BASE).unwrap());
    assert_eq!(call, gre::token_request("abc-123", "s/e=c", "20000000001MODUSER", "p&ss", TOKEN_BASE).unwrap());
    assert_eq!(call["url"], "https://auth.example.test/v1/clientessol/abc-123/oauth2/token/");
    assert!(gre_token_request_json("abc", "s", "20000000001", "p", TOKEN_BASE).unwrap_err().contains("RUC"));

    let token = parse(&gre_token_response_json(200, &data("derived-token-200"), 1_000).unwrap());
    assert_eq!((token["outcome"].as_str(), token["expires_at"].as_u64()), (Some("token"), Some(4_600)));
    assert!(gre_token_usable_json(&token.to_string(), 3_999).unwrap());
    assert!(!gre_token_usable_json(&token.to_string(), 4_000).unwrap());
    let refused = parse(&gre_token_response_json(401, &data("derived-error-401"), 0).unwrap());
    assert_eq!((refused["outcome"].as_str(), refused["general_error"].as_str()), (Some("api_error"), Some("unauthorized")));
    let late = parse(&gre_timeout_json("token", 30, None).unwrap());
    assert_eq!((late["outcome"].as_str(), late["stage"].as_str()), (Some("no_verdict"), Some("token")));
}

#[test]
fn send_steps_match_the_core() {
    let call = parse(&gre_send_request_json(GUIDE, "tok", API_BASE).unwrap());
    assert_eq!(call, gre::send_request(&parse(GUIDE), "tok", API_BASE).unwrap());
    assert_eq!(call["url"], "https://api.example.test/v1/contribuyente/gem/comprobantes/20000000001-09-T001-1");
    assert_eq!(call["hash_zip"].as_str().map(str::len), Some(64));
    assert!(gre_send_request_json(r#"{"name": "20000000001-31-V001-1", "xml": "<x/>"}"#, "tok", API_BASE).is_err());

    let ticket = parse(&gre_send_response_json(200, &data("fork-send-200")).unwrap());
    assert_eq!(ticket, json!({ "outcome": "ticket", "ticket": "a.b.c", "received_at": "2023-01-10T20:10:50" }));
    let rejected = parse(&gre_send_response_json(422, &data("fork-send-422")).unwrap());
    assert_eq!(rejected["errors"], json!([{ "code": "501", "description": "El valor de codCpe no permitido o no valido" }]));
    let server = parse(&gre_send_response_json(500, &data("manual-error-500")).unwrap());
    assert_eq!((server["stage"].as_str(), server["general_error"].as_str()), (Some("send"), Some("internal_server_error")));
    assert!(gre_send_response_json(502, b"<html/>").unwrap_err().contains("may have arrived"));
    assert!(gre_timeout_json("send", 90, None).unwrap_err().contains("may have arrived"));
}

#[test]
fn ticket_outcomes_are_the_cores_for_0_98_and_99() {
    let call = parse(&gre_ticket_request_json("a.b.c", "tok", API_BASE).unwrap());
    assert_eq!((call["method"].as_str(), call["body"].as_str()), (Some("GET"), Some("")));
    assert!(gre_ticket_request_json("a/b", "tok", API_BASE).is_err());

    for (name, state) in [("fork-ticket-0", "processed"), ("fork-ticket-98", "in_process"), ("fork-ticket-99", "processed_with_errors")] {
        let out = parse(&gre_ticket_response_json(200, &data(name), "a.b.c").unwrap());
        assert_eq!(out, gre::ticket_read(Answer::Http(200, &data(name)), "a.b.c").unwrap(), "{name}");
        assert_eq!((out["state"].as_str(), out["ticket"].as_str()), (Some(state), Some("a.b.c")), "{name}");
    }
    let rejected = parse(&gre_ticket_response_json(200, &data("fork-ticket-99"), "a.b.c").unwrap());
    assert_eq!(rejected["error"], json!({ "code": "1345", "description": "El RUC no valido", "class": "exception" }));
    let processed = parse(&gre_ticket_response_json(200, &data("fork-ticket-0"), "a.b.c").unwrap());
    assert_eq!((processed["cdr"]["reference_id"].as_str(), processed["cdr"]["class"].as_str()), (Some("T001-1"), Some("observed")));

    let missing = parse(&gre_ticket_response_json(422, &data("manual-error-422-ticket"), "x").unwrap());
    assert_eq!((missing["outcome"].as_str(), missing["errors"][0]["code"].as_str()), (Some("api_error"), Some("166")));
    let late = parse(&gre_timeout_json("ticket", 30, Some("a.b.c")).unwrap());
    assert_eq!((late["outcome"].as_str(), late["ticket"].as_str()), (Some("no_verdict"), Some("a.b.c")));
    assert!(gre_timeout_json("ticket", 30, None).is_err());
    assert!(gre_timeout_json("query", 30, None).is_err());
}

#[test]
fn unix_seconds_refuses_what_is_not_whole_seconds() {
    assert_eq!(unix_seconds(1_760_620_000.0), Ok(1_760_620_000));
    for bad in [-1.0, 1.5, f64::NAN, f64::INFINITY, 1_760_620_000_000.0] {
        assert!(unix_seconds(bad).is_err(), "{bad}");
    }
}
