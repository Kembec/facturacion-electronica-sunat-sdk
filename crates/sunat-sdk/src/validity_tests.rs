//! Replays SUNAT's answers. `manual-*` files are the ones the Consulta Integrada manual v2.0 prints
//! (pp. 6, 8-9 and 12; the token is replaced by a fake one); `derived-*` change one field of those
//! (the `estadoCp` of each state, a numeric code as the manual's table types it, a 401). There is no
//! test environment to record more from.

use super::*;

fn data(name: &str) -> Vec<u8> {
    let path = format!("{}/tests/data/validity/{name}.json", env!("CARGO_MANIFEST_DIR"));
    std::fs::read(&path).unwrap_or_else(|e| panic!("{path}: {e}"))
}

fn query() -> Value {
    json!({ "ruc": "20100066603", "document_type": "01", "series": "E001", "number": "749", "issue_date": "2018-06-29", "total": "236.00" })
}

#[test]
fn token_request_follows_the_manual_and_encodes_the_secret() {
    let request = token_request("c04864ce-f454-48e1-b41d-8730e575b905", "OpzHz4pn8D1/rRrv5iDklA==", None).unwrap();
    assert_eq!(request["method"], "POST");
    assert_eq!(request["url"], "https://api-seguridad.sunat.gob.pe/v1/clientesextranet/c04864ce-f454-48e1-b41d-8730e575b905/oauth2/token/");
    assert_eq!(request["headers"]["Content-Type"], "application/x-www-form-urlencoded");
    assert_eq!(
        request["body"],
        "grant_type=client_credentials&scope=https%3A%2F%2Fapi.sunat.gob.pe%2Fv1%2Fcontribuyente%2Fcontribuyentes&client_id=c04864ce-f454-48e1-b41d-8730e575b905&client_secret=OpzHz4pn8D1%2FrRrv5iDklA%3D%3D"
    );
    let local = token_request("abc", "s", Some("http://127.0.0.1:9/base/")).unwrap();
    assert_eq!(local["url"], "http://127.0.0.1:9/base/abc/oauth2/token/");
    assert!(token_request("a/b", "s", None).is_err());
    assert!(token_request("abc", "", None).is_err());
}

#[test]
fn token_read_gives_the_token_or_no_verdict() {
    let token = token_read(Answer::Http(200, &data("manual-token-200"))).unwrap();
    assert_eq!(
        token,
        json!({ "outcome": "token", "access_token": "test-access-token-not-a-real-jwt", "token_type": "JWT", "expires_in": 3600 })
    );
    let refused = token_read(Answer::Http(401, &data("derived-token-401"))).unwrap();
    assert_eq!(
        refused,
        json!({ "outcome": "no_verdict", "stage": "token", "reason": "http_status", "http_status": 401, "message": "SUNAT answered HTTP 401 to the token request" })
    );
    let late = token_read(Answer::Timeout(30)).unwrap();
    assert_eq!((late["stage"].as_str(), late["reason"].as_str()), (Some("token"), Some("timeout")));
    assert!(token_read(Answer::Http(200, b"<html/>")).is_err());
    assert!(token_read(Answer::Http(200, b"{\"token_type\":\"JWT\"}")).is_err());
}

#[test]
fn query_request_is_the_manual_example() {
    let request = query_request(&query(), "test-token", None).unwrap();
    assert_eq!(request["url"], "https://api.sunat.gob.pe/v1/contribuyente/contribuyentes/20100066603/validarcomprobante");
    assert_eq!(request["headers"], json!({ "Content-Type": "application/json", "Authorization": "Bearer test-token" }));
    let body: Value = serde_json::from_str(request["body"].as_str().unwrap()).unwrap();
    assert_eq!(
        body,
        json!({ "numRuc": "20100066603", "codComp": "01", "numeroSerie": "E001", "numero": "749", "fechaEmision": "29/06/2018", "monto": "236.00" })
    );
    assert!(query_request(&query(), "", None).is_err());
}

#[test]
fn each_state_code_has_its_name() {
    let expected = [("0", "not_reported"), ("2", "voided"), ("3", "printed_authorized"), ("4", "printed_not_authorized")];
    for (code, name) in expected {
        let out = query_read(Answer::Http(200, &data(&format!("derived-query-{code}")))).unwrap();
        assert_eq!((out["outcome"].as_str(), out["state_code"].as_str(), out["state"].as_str()), (Some("validity"), Some(code), Some(name)));
        assert_eq!(out["observations"], json!([]));
    }
}

#[test]
fn the_manual_answer_keeps_its_observations() {
    let out = query_read(Answer::Http(200, &data("manual-query-1"))).unwrap();
    assert_eq!(out["state"], "accepted");
    assert_eq!((out["taxpayer_state_code"].as_str(), out["domicile_condition_code"].as_str()), (Some("00"), Some("00")));
    assert_eq!(out["message"], "Operation Success!");
    assert_eq!(out["observations"].as_array().unwrap().len(), 2);
    assert!(out["observations"][0].as_str().unwrap().contains("Fecha de emisión"));

    let numeric = query_read(Answer::Http(200, &data("derived-query-1-numeric"))).unwrap();
    assert_eq!((numeric["state_code"].as_str(), numeric["state"].as_str()), (Some("1"), Some("accepted")));
    assert_eq!(numeric["observations"], json!(["- x"]));

    let unknown = query_read(Answer::Http(200, &data("derived-query-unknown"))).unwrap();
    assert_eq!((unknown["state_code"].as_str(), unknown["state"].is_null()), (Some("7"), true));
}

#[test]
fn a_refused_query_is_an_error_about_the_query_not_the_voucher() {
    let out = query_read(Answer::Http(422, &data("manual-query-422"))).unwrap();
    assert_eq!((out["outcome"].as_str(), out["http_status"].as_u64(), out["error_code"].as_str()), (Some("query_error"), Some(422), Some("99")));
    assert!(out["message"].as_str().unwrap().starts_with("Unexpected character"));
    let as_200 = query_read(Answer::Http(200, b"{\"success\":false,\"message\":\"x\",\"errorCode\":12}")).unwrap();
    assert_eq!((as_200["outcome"].as_str(), as_200["error_code"].as_str()), (Some("query_error"), Some("12")));
}

#[test]
fn without_an_answer_there_is_no_verdict() {
    let unauthorized = query_read(Answer::Http(401, &data("derived-query-401"))).unwrap();
    assert_eq!(
        unauthorized,
        json!({ "outcome": "no_verdict", "stage": "query", "reason": "http_status", "http_status": 401, "message": "SUNAT answered HTTP 401 to the query request" })
    );
    for status in [403, 404, 429, 500, 502, 503] {
        assert_eq!(query_read(Answer::Http(status, b"{\"success\":false}")).unwrap()["outcome"], "no_verdict", "{status}");
    }
    assert_eq!(query_read(Answer::Http(422, b"<html/>")).unwrap()["outcome"], "no_verdict");
    assert_eq!(timeout(Stage::Query, 45), query_read(Answer::Timeout(45)).unwrap());
    assert_eq!(timeout(Stage::Query, 45)["message"], "timed out after 45 s waiting for the query answer");
    assert!(query_read(Answer::Http(200, b"<html/>")).is_err());
}
