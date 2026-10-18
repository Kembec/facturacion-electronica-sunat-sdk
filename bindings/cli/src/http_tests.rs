//! `post_soap` against a local plain-HTTP server (the only plain HTTP in the repository, bound to 127.0.0.1, in
//! tests): no network. The SUNAT beta answers a rejected summary with its message, in ISO-8859-1 or UTF-8, in the status
//! line of an HTTP 200 and the SOAP fault in the body.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::time::Duration;

use serde_json::json;

use super::post_soap;
use crate::commands::{status, summary, ticket};
use crate::config::Connection;

const FAULT: &str = "<e:Envelope xmlns:e=\"s\"><e:Body><e:Fault><faultcode>e:Client.2992</faultcode><faultstring>Error en la linea: 1. Tasa del tributo faltante para c\u{f3}digo 1000.</faultstring></e:Fault></e:Body></e:Envelope>";

/// Serves one request with `status_line` (raw bytes) and `body`; returns the endpoint.
fn serve_once(status_line: &'static [u8], body: &'static str) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = format!("http://{}/billService", listener.local_addr().unwrap());
    std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut request = [0u8; 8192];
        let _ = stream.read(&mut request);
        let mut reply = status_line.to_vec();
        reply.extend_from_slice(format!("\r\nContent-Type: text/xml\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len()).as_bytes());
        reply.extend_from_slice(body.as_bytes());
        stream.write_all(&reply).unwrap();
    });
    endpoint
}

fn connection(endpoint: String) -> Connection {
    Connection { endpoint, user: "20100066603MODDATOS".into(), password: "pw-secret".into(), timeout: Duration::from_secs(10) }
}

// "HTTP/1.1 200 " + the beta's phrase, with the non-ASCII "c\u{f3}digo" as UTF-8 (the bytes the beta sends).
const BETA_LINE: &[u8] = "HTTP/1.1 200 Error en la linea: 1. Tasa del tributo faltante para código 1000.: 2992 (nodo: \"/\" valor: \"\")".as_bytes();
const LATIN1_LINE: &[u8] = b"HTTP/1.1 200 Error en la linea: 1. Tasa del tributo faltante para c\xf3digo 1000.: 2992";

fn assert_fault_2992(out: &serde_json::Value) {
    assert_eq!(out["outcome"], "fault", "{out}");
    assert_eq!(out["code"], "2992");
    assert_eq!(out["diagnosis"][0]["code"], "2992");
}

#[test]
fn summary_reads_the_fault_from_a_non_ascii_status_line() {
    for line in [BETA_LINE, LATIN1_LINE] {
        let conn = connection(serve_once(line, FAULT));
        let post = |endpoint: &str, action: &str, envelope: &str| post_soap(endpoint, action, envelope, conn.timeout);
        let out = summary(&json!({ "name": "20100066603-RC-20261017-1", "xml": "<a/>" }), &conn, &post).unwrap();
        assert_fault_2992(&out);
    }
}

#[test]
fn ticket_and_status_read_the_body_of_a_non_ascii_status_line() {
    let conn = connection(serve_once(BETA_LINE, FAULT));
    let post = |endpoint: &str, action: &str, envelope: &str| post_soap(endpoint, action, envelope, conn.timeout);
    let out = ticket(&json!({ "ticket": "202610170001" }), &conn, &post).unwrap();
    assert_eq!(out["code"], "2992", "{out}");

    let conn = connection(serve_once(BETA_LINE, FAULT));
    let post = |endpoint: &str, action: &str, envelope: &str| post_soap(endpoint, action, envelope, conn.timeout);
    let input = json!({ "ruc": "20100066603", "document_type": "03", "series": "B001", "number": 1 });
    let out = status(&input, &conn, &post).unwrap();
    assert_eq!(out["code"], "2992", "{out}");
}

#[test]
fn a_status_other_than_200_and_500_still_gives_no_body() {
    let endpoint = serve_once(b"HTTP/1.1 401 Unauthorized", "");
    let err = post_soap(&endpoint, "urn:getStatus", "<a/>", Duration::from_secs(10)).unwrap_err();
    assert_eq!(err, super::PostError::Status(401));
}
