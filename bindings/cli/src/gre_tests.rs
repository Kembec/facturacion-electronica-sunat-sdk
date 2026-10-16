//! The two calls of each GRE command against recorded answers (`crates/sunat-sdk/tests/data/gre/`); no
//! network is touched.

use std::cell::RefCell;

use super::*;

const DATA: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../crates/sunat-sdk/tests/data/gre/");

fn data(name: &str) -> Vec<u8> {
    std::fs::read(format!("{DATA}{name}.json")).unwrap()
}

fn conn(timeout: u64) -> GreConnection {
    GreConnection {
        client_id: "c04864ce-f454".into(),
        client_secret: "secret-value".into(),
        user: "20000000001MODUSER".into(),
        password: "sol-password".into(),
        token_base: "https://auth.test/v1/clientessol".into(),
        api_base: "https://cpe.test/v1".into(),
        timeout: Duration::from_secs(timeout),
    }
}

fn guide() -> Value {
    json!({ "name": "20000000001-09-T001-1", "xml": "<DespatchAdvice/>" })
}

/// Answers each call with the next recorded body and keeps the requests.
fn replay<'a>(calls: &'a RefCell<Vec<Value>>, answers: &'a [(u16, Vec<u8>)]) -> impl Fn(&Value, Duration) -> Result<(u16, Vec<u8>), PostError> + 'a {
    move |request, _| {
        calls.borrow_mut().push(request.clone());
        Ok(answers[calls.borrow().len() - 1].clone())
    }
}

#[test]
fn send_asks_for_a_token_then_posts_the_guide() {
    let calls = RefCell::new(Vec::new());
    let answers = [(200, data("derived-token-200")), (200, data("fork-send-200"))];
    let out = send(&guide(), &conn(90), &replay(&calls, &answers)).unwrap();
    assert_eq!((out["outcome"].as_str(), out["ticket"].as_str()), (Some("ticket"), Some("a.b.c")));
    assert_eq!(out["zip_name"], "20000000001-09-T001-1.zip");
    assert_eq!(out["hash_zip"].as_str().map(str::len), Some(64));
    let calls = calls.borrow();
    assert!(calls[0]["body"].as_str().unwrap().contains("grant_type=password"));
    assert_eq!(calls[1]["headers"]["Authorization"], "Bearer fake.token.for-tests");
    let printed = out.to_string();
    assert!(!printed.contains("fake.token") && !printed.contains("sol-password") && !printed.contains("secret-value"));
}

#[test]
fn ticket_reads_in_process_and_rejected() {
    for (answer, state) in [("fork-ticket-98", "in_process"), ("fork-ticket-99", "processed_with_errors"), ("fork-ticket-0", "processed")] {
        let calls = RefCell::new(Vec::new());
        let answers = [(200, data("derived-token-200")), (200, data(answer))];
        let out = ticket(&json!({ "ticket": "a.b.c" }), &conn(90), &replay(&calls, &answers)).unwrap();
        assert_eq!((out["state"].as_str(), out["ticket"].as_str()), (Some(state), Some("a.b.c")), "{answer}");
        assert_eq!(calls.borrow()[1]["method"], "GET");
    }
}

#[test]
fn a_refused_token_stops_before_the_guide_is_sent() {
    let calls = RefCell::new(Vec::new());
    let answers = [(401, data("derived-error-401"))];
    let out = send(&guide(), &conn(90), &replay(&calls, &answers)).unwrap();
    assert_eq!((out["outcome"].as_str(), out["stage"].as_str()), (Some("api_error"), Some("token")));
    assert_eq!(calls.borrow().len(), 1);
    let out = ticket(&json!({ "ticket": "t" }), &conn(90), &replay(&RefCell::new(Vec::new()), &answers)).unwrap();
    assert_eq!((out["stage"].as_str(), out["ticket"].as_str()), (Some("token"), Some("t")));
}

#[test]
fn silence_after_a_send_is_an_error_and_after_a_ticket_no_verdict() {
    let late = |stage: usize| {
        let count = RefCell::new(0);
        move |_: &Value, _: Duration| -> Result<(u16, Vec<u8>), PostError> {
            *count.borrow_mut() += 1;
            if *count.borrow() == stage { Err(PostError::Timeout(30)) } else { Ok((200, data("derived-token-200"))) }
        }
    };
    assert!(send(&guide(), &conn(30), &late(2)).unwrap_err().contains("may have arrived"));
    let out = ticket(&json!({ "ticket": "t" }), &conn(30), &late(2)).unwrap();
    assert_eq!((out["outcome"].as_str(), out["stage"].as_str()), (Some("no_verdict"), Some("ticket")));
    let out = send(&guide(), &conn(30), &late(1)).unwrap();
    assert_eq!((out["outcome"].as_str(), out["stage"].as_str()), (Some("no_verdict"), Some("token")));
}

#[test]
fn a_bad_input_spends_no_call() {
    let never = |_: &Value, _: Duration| -> Result<(u16, Vec<u8>), PostError> { panic!("no call expected") };
    assert!(send(&json!({ "name": "20000000001-31-V001-1", "xml": "<x/>" }), &conn(90), &never).is_err());
    assert!(ticket(&json!({ "ticket": "a/b" }), &conn(90), &never).is_err());
    assert!(ticket(&json!({}), &conn(90), &never).is_err());
}
