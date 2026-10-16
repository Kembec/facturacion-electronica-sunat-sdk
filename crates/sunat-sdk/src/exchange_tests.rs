use super::*;

const TICKET_IN_PROCESS: &[u8] = br#"<S:Envelope xmlns:S="http://schemas.xmlsoap.org/soap/envelope/"><S:Body><ns2:getStatusResponse xmlns:ns2="http://service.sunat.gob.pe"><status><statusCode>98</statusCode></status></ns2:getStatusResponse></S:Body></S:Envelope>"#;
const FAULT: &[u8] = b"<e:Envelope xmlns:e=\"s\"><e:Body><e:Fault><faultcode>e:Client.0109</faultcode><faultstring>El sistema no puede responder su solicitud</faultstring></e:Fault></e:Body></e:Envelope>";

#[test]
fn names_and_soap_actions_match_the_cli() {
    let names = ["send", "summary", "ticket", "status"];
    let actions: Vec<_> = names.iter().map(|n| Exchange::from_name(n).unwrap().soap_action()).collect();
    assert_eq!(actions, ["urn:sendBill", "urn:sendSummary", "urn:getStatus", "urn:getStatusCdr"]);
    assert!(Exchange::from_name("zip").unwrap_err().contains("expected send, summary, ticket or status"));
}

#[test]
fn http_200_and_500_carry_a_body_and_any_other_status_does_not() {
    assert_eq!(Reply::from_http(200, b"x"), Reply::Body(b"x"));
    assert_eq!(Reply::from_http(500, b"x"), Reply::Body(b"x"));
    assert_eq!(Reply::from_http(401, b"x"), Reply::HttpStatus(401));
    assert_eq!(Reply::from_http(503, b""), Reply::HttpStatus(503));
}

#[test]
fn request_builds_each_envelope_and_rejects_missing_fields() {
    let doc = json!({ "name": "20100066603-01-F001-1", "xml": "<Invoice/>" });
    let send = request(Exchange::Send, &doc, "20100066603MODDATOS", "moddatos").unwrap();
    assert!(send.contains("<fileName>20100066603-01-F001-1.zip</fileName>") && send.contains("20100066603MODDATOS"));
    let ticket = request(Exchange::Ticket, &json!({ "ticket": "1760620000123" }), "u", "p").unwrap();
    assert!(ticket.contains("<ticket>1760620000123</ticket>"));
    let by_number = json!({ "ruc": "20100066603", "document_type": "03", "series": "B001", "number": "7" });
    assert!(request(Exchange::Status, &by_number, "u", "p").unwrap().contains("<numeroComprobante>7</numeroComprobante>"));

    assert!(request(Exchange::Send, &json!({ "name": "20100066603-01-F001-1" }), "u", "p").is_err());
    assert!(request(Exchange::Status, &json!({ "ruc": "20100066603", "document_type": "03", "series": "B001", "number": -1 }), "u", "p").is_err());
}

#[test]
fn read_parses_a_body_and_words_the_no_answer_cases() {
    let by_ticket = json!({ "ticket": "1760620000123" });
    let processing = read(Exchange::Ticket, &by_ticket, Reply::Body(TICKET_IN_PROCESS)).unwrap();
    assert_eq!(processing, json!({ "outcome": "ticket_status", "status_code": "98", "state": "in_process" }));
    assert_eq!(read(Exchange::Send, &json!({}), Reply::Body(FAULT)).unwrap()["class"], "exception");

    let unauthorized = read(Exchange::Ticket, &by_ticket, Reply::HttpStatus(401)).unwrap();
    assert_eq!(
        unauthorized,
        json!({ "outcome": "no_verdict", "reason": "http_status", "http_status": 401, "message": "SUNAT answered HTTP 401", "ticket": "1760620000123" })
    );
    let timed_out = read(Exchange::Status, &json!({}), Reply::Timeout(45)).unwrap();
    assert_eq!((timed_out["reason"].as_str(), timed_out["http_status"].is_null()), (Some("timeout"), true));

    assert_eq!(read(Exchange::Summary, &json!({}), Reply::HttpStatus(503)).unwrap_err(), "SUNAT answered HTTP 503");
    assert_eq!(
        read(Exchange::Send, &json!({}), Reply::Timeout(45)).unwrap_err(),
        "timed out after 45 s waiting for SUNAT; the outcome is unknown"
    );
}
