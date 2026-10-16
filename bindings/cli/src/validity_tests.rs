use std::cell::RefCell;

use serde_json::json;

use super::*;

const TOKEN: &[u8] = include_bytes!("../../../crates/sunat-sdk/tests/data/validity/manual-token-200.json");
const ACCEPTED: &[u8] = include_bytes!("../../../crates/sunat-sdk/tests/data/validity/manual-query-1.json");

fn conn(timeout: u64) -> ApiConnection {
    ApiConnection {
        client_id: "c04864ce-f454-48e1-b41d-8730e575b905".into(),
        client_secret: "secret-value".into(),
        token_base: "https://seguridad.test/v1/clientesextranet".into(),
        query_base: "https://api.test/v1/contribuyente/contribuyentes".into(),
        timeout: Duration::from_secs(timeout),
    }
}

fn input() -> Value {
    json!({ "ruc": "20100066603", "document_type": "03", "series": "B001", "number": 7, "issue_date": "2026-10-15T10:30:00-05:00", "total": "118.00" })
}

#[test]
fn asks_for_a_token_then_queries_with_it() {
    let calls = RefCell::new(Vec::new());
    let post = |request: &Value, _: Duration| -> Result<(u16, Vec<u8>), PostError> {
        calls.borrow_mut().push(request.clone());
        Ok((200, if calls.borrow().len() == 1 { TOKEN.to_vec() } else { ACCEPTED.to_vec() }))
    };
    let out = validity(&input(), &conn(90), &post).unwrap();
    assert_eq!((out["outcome"].as_str(), out["state"].as_str()), (Some("validity"), Some("accepted")));
    let calls = calls.borrow();
    assert_eq!(calls[0]["url"], "https://seguridad.test/v1/clientesextranet/c04864ce-f454-48e1-b41d-8730e575b905/oauth2/token/");
    assert_eq!(calls[1]["url"], "https://api.test/v1/contribuyente/contribuyentes/20100066603/validarcomprobante");
    assert_eq!(calls[1]["headers"]["Authorization"], "Bearer test-access-token-not-a-real-jwt");
    assert!(!out.to_string().contains("test-access-token") && !out.to_string().contains("secret-value"));
}

#[test]
fn a_bad_input_spends_no_call() {
    let post = |_: &Value, _: Duration| -> Result<(u16, Vec<u8>), PostError> { panic!("no call expected") };
    let mut bad = input();
    bad["document_type"] = json!("09");
    assert!(validity(&bad, &conn(90), &post).unwrap_err().contains("document_type"));
}

#[test]
fn a_401_or_a_timeout_on_either_call_is_no_verdict() {
    let refused = |_: &Value, _: Duration| -> Result<(u16, Vec<u8>), PostError> { Ok((401, b"{}".to_vec())) };
    let out = validity(&input(), &conn(90), &refused).unwrap();
    assert_eq!((out["outcome"].as_str(), out["stage"].as_str(), out["http_status"].as_u64()), (Some("no_verdict"), Some("token"), Some(401)));

    let seen = RefCell::new(0);
    let late_query = |_: &Value, timeout: Duration| -> Result<(u16, Vec<u8>), PostError> {
        *seen.borrow_mut() += 1;
        if *seen.borrow() == 1 { Ok((200, TOKEN.to_vec())) } else { Err(PostError::Timeout(timeout.as_secs())) }
    };
    let out = validity(&input(), &conn(30), &late_query).unwrap();
    assert_eq!((out["stage"].as_str(), out["reason"].as_str()), (Some("query"), Some("timeout")));

    let broken = |_: &Value, _: Duration| -> Result<(u16, Vec<u8>), PostError> { Err(PostError::Other("transport error: Dns".into())) };
    assert_eq!(validity(&input(), &conn(30), &broken).unwrap_err(), "transport error: Dns");
}

#[test]
fn the_query_gets_what_is_left_of_the_wait() {
    let waits = RefCell::new(Vec::new());
    let post = |_: &Value, timeout: Duration| -> Result<(u16, Vec<u8>), PostError> {
        waits.borrow_mut().push(timeout);
        if waits.borrow().len() == 1 {
            std::thread::sleep(Duration::from_millis(1100));
            Ok((200, TOKEN.to_vec()))
        } else {
            Ok((200, ACCEPTED.to_vec()))
        }
    };
    validity(&input(), &conn(5), &post).unwrap();
    let waits = waits.borrow();
    assert_eq!(waits[0], Duration::from_secs(5));
    assert!(waits[1] < Duration::from_secs(4), "{:?}", waits[1]);
}
