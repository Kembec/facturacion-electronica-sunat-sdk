//! Host-testable core of the binding: strings in, JSON string out, error as its message.

use serde_json::{json, Value};
use sunat_model::Document;
use sunat_sdk::exchange::{self, Exchange, Reply};
use sunat_sdk::validity::{self, Answer, Stage};
use sunat_xmldsig::DEFAULT_SIGNATURE_ID;

/// Renders the unsigned XML of a document given as JSON.
pub fn build_json(document_json: &str) -> Result<String, String> {
    let document = Document::from_json(document_json).map_err(|e| e.to_string())?;
    let xml = sunat_ubl::build(&document).map_err(|e| e.to_string())?;
    let xml = String::from_utf8(xml).map_err(|e| e.to_string())?;
    Ok(json!({ "file_name": document.file_name(), "xml": xml }).to_string())
}

/// Signs an unsigned XML and reports the `DigestValue` and `SignatureValue` it carries.
pub fn sign_json(xml: &str, cert_pem: &str, key_pem: &str, signature_id: Option<&str>) -> Result<String, String> {
    let id = signature_id.unwrap_or(DEFAULT_SIGNATURE_ID);
    let signed = sunat_xmldsig::sign_with_id(xml.as_bytes(), cert_pem, key_pem, id).map_err(|e| e.to_string())?;
    let signed = String::from_utf8(signed).map_err(|e| e.to_string())?;
    let digest_value = element_text(&signed, "ds:DigestValue")?;
    let signature_value = element_text(&signed, "ds:SignatureValue")?;
    Ok(json!({ "xml": signed, "digest_value": digest_value, "signature_value": signature_value }).to_string())
}

/// `{"name", "xml" | "xml_base64"}` -> `{"file_name", "zip_base64"}`, as the CLI's `zip`.
pub fn zip_json(input_json: &str) -> Result<String, String> {
    Ok(exchange::zip(&parse(input_json)?)?.to_string())
}

/// `{"cdr_base64"}` (zip or XML) -> the CDR fields and their class, as the CLI's `cdr`.
pub fn cdr_json(input_json: &str) -> Result<String, String> {
    Ok(exchange::cdr(&parse(input_json)?)?.to_string())
}

/// The POST of `exchange` (send, summary, ticket, status) for the CLI's input, as `fetch` options:
/// `{"method": "POST", "headers": {"Content-Type", "SOAPAction"}, "body": "<envelope>"}`.
pub fn soap_request_json(exchange_name: &str, input_json: &str, user: &str, password: &str) -> Result<String, String> {
    let kind = Exchange::from_name(exchange_name)?;
    let envelope = exchange::request(kind, &parse(input_json)?, user, password)?;
    let headers = json!({ "Content-Type": "text/xml; charset=utf-8", "SOAPAction": format!("\"{}\"", kind.soap_action()) });
    Ok(json!({ "method": "POST", "headers": headers, "body": envelope }).to_string())
}

/// The outcome for an HTTP response to that POST: the CLI's output for the same answer.
pub fn soap_response_json(exchange_name: &str, input_json: &str, http_status: u16, body: &[u8]) -> Result<String, String> {
    let kind = Exchange::from_name(exchange_name)?;
    Ok(exchange::read(kind, &parse(input_json)?, Reply::from_http(http_status, body))?.to_string())
}

/// The outcome when the wait ran out after `seconds`: `no_verdict` for ticket and status, an
/// error for send and summary, whose document may have arrived.
pub fn soap_timeout_json(exchange_name: &str, input_json: &str, seconds: u32) -> Result<String, String> {
    let kind = Exchange::from_name(exchange_name)?;
    Ok(exchange::read(kind, &parse(input_json)?, Reply::Timeout(u64::from(seconds)))?.to_string())
}

/// The Consulta Integrada's token request as `fetch` options with its `url`
/// (`{"method", "url", "headers", "body"}`); `base` defaults to SUNAT's. The body carries the secret.
pub fn validity_token_request_json(client_id: &str, client_secret: &str, base: Option<&str>) -> Result<String, String> {
    Ok(validity::token_request(client_id, client_secret, base)?.to_string())
}

/// The token answer -> `{"outcome": "token", "access_token", "token_type", "expires_in"}` or `no_verdict`.
pub fn validity_token_response_json(http_status: u16, body: &[u8]) -> Result<String, String> {
    Ok(validity::token_read(Answer::Http(http_status, body))?.to_string())
}

/// The query for the CLI's `validity` input as `fetch` options with its `url`; `base` defaults to SUNAT's.
pub fn validity_request_json(input_json: &str, access_token: &str, base: Option<&str>) -> Result<String, String> {
    Ok(validity::query_request(&parse(input_json)?, access_token, base)?.to_string())
}

/// The query answer -> the CLI's `validity` output: `validity`, `query_error` or `no_verdict`.
pub fn validity_response_json(http_status: u16, body: &[u8]) -> Result<String, String> {
    Ok(validity::query_read(Answer::Http(http_status, body))?.to_string())
}

/// No answer within `seconds` for `stage` (`token` or `query`) -> `no_verdict`.
pub fn validity_timeout_json(stage: &str, seconds: u32) -> Result<String, String> {
    Ok(validity::timeout(Stage::from_name(stage)?, u64::from(seconds)).to_string())
}

fn parse(input_json: &str) -> Result<Value, String> {
    serde_json::from_str(input_json).map_err(|e| format!("input is not JSON: {e}"))
}

/// Text of the first `<element>` (no attributes) in `xml`, without whitespace.
fn element_text(xml: &str, element: &str) -> Result<String, String> {
    let open = format!("<{element}>");
    let start = xml.find(&open).ok_or_else(|| format!("signed XML has no {element}"))? + open.len();
    let end = xml[start..].find('<').ok_or_else(|| format!("unterminated {element}"))?;
    Ok(xml[start..start + end].split_whitespace().collect())
}

#[cfg(test)]
#[path = "api_tests.rs"]
mod tests;
