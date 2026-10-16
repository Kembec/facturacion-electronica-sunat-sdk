//! The commands as functions of JSON in, JSON out. Network goes through a `post` function so the
//! commands are testable without a server.

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use serde_json::{json, Value};
use sunat_sdk::transport::{
    SOAP_ACTION_GET_STATUS, SOAP_ACTION_GET_STATUS_CDR, SOAP_ACTION_SEND_BILL, SOAP_ACTION_SEND_SUMMARY,
};
use sunat_sdk::{
    classify_cdr, classify_code, get_status_cdr_envelope, get_status_envelope,
    parse_get_status_cdr_response, parse_get_status_response, parse_send_bill_response,
    parse_send_summary_response, read_cdr, send_bill_envelope, send_summary_envelope, zip_document,
    zip_file_name, Cdr, SendBillOutcome, SendSummaryOutcome, StatusCdrOutcome, TicketStatusOutcome,
};

use crate::config::Connection;
use crate::http::PostError;

/// `post(endpoint, soap_action, envelope) -> body`.
pub type Post<'a> = &'a dyn Fn(&str, &str, &str) -> Result<Vec<u8>, PostError>;

/// `{"name", "xml" | "xml_base64"}` → `{"file_name", "zip_base64"}`.
pub fn zip(input: &Value) -> Result<Value, String> {
    let (name, zip) = zipped(input)?;
    Ok(json!({ "file_name": zip_file_name(&name), "zip_base64": STANDARD.encode(zip) }))
}

/// Zips the signed XML and sends it with sendBill. Same input as `zip`.
pub fn send(input: &Value, conn: &Connection, post: Post) -> Result<Value, String> {
    let (name, zip) = zipped(input)?;
    let envelope = send_bill_envelope(&conn.user, &conn.password, &zip_file_name(&name), &zip);
    let body = post(&conn.endpoint, SOAP_ACTION_SEND_BILL, &envelope).map_err(|e| e.to_string())?;
    match parse_send_bill_response(&body).map_err(|e| e.to_string())? {
        SendBillOutcome::Cdr(cdr_zip) => Ok(json!({
            "outcome": "cdr",
            "cdr": cdr_json(&read_cdr(&cdr_zip).map_err(|e| e.to_string())?),
            "cdr_zip_base64": STANDARD.encode(cdr_zip),
        })),
        SendBillOutcome::Fault { code, message } => Ok(fault_json(&code, &message)),
    }
}

/// Zips a signed daily summary (RC) or void communication (RA) and sends it with sendSummary.
/// Same input as `zip`, named `RUC-RC|RA-YYYYMMDD-N`. The ticket is the only handle on the
/// outcome: the caller stores it before anything else and asks `ticket` with it.
pub fn summary(input: &Value, conn: &Connection, post: Post) -> Result<Value, String> {
    let (name, zip) = zipped(input)?;
    let envelope = send_summary_envelope(&conn.user, &conn.password, &zip_file_name(&name), &zip);
    let body = post(&conn.endpoint, SOAP_ACTION_SEND_SUMMARY, &envelope).map_err(|e| e.to_string())?;
    match parse_send_summary_response(&body).map_err(|e| e.to_string())? {
        SendSummaryOutcome::Ticket(ticket) => Ok(json!({ "outcome": "ticket", "ticket": ticket })),
        SendSummaryOutcome::Fault { code, message } => Ok(fault_json(&code, &message)),
    }
}

/// `{"ticket"}` → getStatus answer: `state` is `processed` (0), `in_process` (98: ask again with
/// the same ticket, never send again) or `processed_with_errors` (99), with the CDR if attached.
/// An HTTP status or a timeout instead of an answer is `no_verdict`, with the same ticket.
pub fn ticket(input: &Value, conn: &Connection, post: Post) -> Result<Value, String> {
    let ticket = text(input, "ticket")?;
    let envelope = get_status_envelope(&conn.user, &conn.password, ticket).map_err(|e| e.to_string())?;
    let body = match post(&conn.endpoint, SOAP_ACTION_GET_STATUS, &envelope) {
        Ok(body) => body,
        Err(error) => {
            let mut out = no_verdict(error)?;
            out["ticket"] = json!(ticket);
            return Ok(out);
        }
    };
    match parse_get_status_response(&body).map_err(|e| e.to_string())? {
        TicketStatusOutcome::Fault { code, message } => Ok(fault_json(&code, &message)),
        TicketStatusOutcome::Status(status) => {
            let state = match status.status_code.trim() {
                "0" => Value::from("processed"),
                "98" => Value::from("in_process"),
                "99" => Value::from("processed_with_errors"),
                _ => Value::Null,
            };
            let mut out = json!({ "outcome": "ticket_status", "status_code": status.status_code, "state": state });
            if let Some(cdr_zip) = status.cdr {
                out["cdr"] = cdr_json(&read_cdr(&cdr_zip).map_err(|e| e.to_string())?);
                out["cdr_zip_base64"] = json!(STANDARD.encode(cdr_zip));
            }
            Ok(out)
        }
    }
}

/// `{"ruc", "document_type", "series", "number"}` → getStatusCdr answer, with the CDR if attached.
/// An HTTP status or a timeout instead of an answer is `no_verdict`.
pub fn status(input: &Value, conn: &Connection, post: Post) -> Result<Value, String> {
    let number = input["number"]
        .as_u64()
        .or_else(|| input["number"].as_str().and_then(|s| s.parse().ok()))
        .and_then(|n| u32::try_from(n).ok())
        .ok_or("`number` must be a positive integer")?;
    let envelope = get_status_cdr_envelope(
        &conn.user,
        &conn.password,
        text(input, "ruc")?,
        text(input, "document_type")?,
        text(input, "series")?,
        number,
    )
    .map_err(|e| e.to_string())?;
    let body = match post(&conn.endpoint, SOAP_ACTION_GET_STATUS_CDR, &envelope) {
        Ok(body) => body,
        Err(error) => return no_verdict(error),
    };
    match parse_get_status_cdr_response(&body).map_err(|e| e.to_string())? {
        StatusCdrOutcome::Fault { code, message } => Ok(fault_json(&code, &message)),
        StatusCdrOutcome::Status(status) => {
            let mut out = json!({
                "outcome": "status",
                "status_code": status.status_code,
                "status_message": status.status_message,
            });
            if let Some(cdr_zip) = status.cdr {
                out["cdr"] = cdr_json(&read_cdr(&cdr_zip).map_err(|e| e.to_string())?);
                out["cdr_zip_base64"] = json!(STANDARD.encode(cdr_zip));
            }
            Ok(out)
        }
    }
}

/// `{"cdr_base64"}` (zip or XML) → the CDR fields and their class.
pub fn cdr(input: &Value) -> Result<Value, String> {
    let bytes = base64_field(input, "cdr_base64")?;
    Ok(cdr_json(&read_cdr(&bytes).map_err(|e| e.to_string())?))
}

fn zipped(input: &Value) -> Result<(String, Vec<u8>), String> {
    let name = text(input, "name")?.to_string();
    let xml = match input["xml"].as_str() {
        Some(xml) => xml.as_bytes().to_vec(),
        None => base64_field(input, "xml_base64").map_err(|_| "missing `xml` or `xml_base64`".to_string())?,
    };
    let zip = zip_document(&name, &xml).map_err(|e| e.to_string())?;
    Ok((name, zip))
}

fn cdr_json(cdr: &Cdr) -> Value {
    json!({
        "reference_id": cdr.reference_id,
        "response_code": cdr.response_code,
        "description": cdr.description,
        "notes": cdr.notes,
        "class": classify_cdr(&cdr.response_code, &cdr.notes).map(|c| c.as_str()),
    })
}

/// A query that got no answer says nothing about the document: the caller asks again with the
/// same ticket or number, and never sends the document again because of it. An HTTP status
/// (SUNAT has answered 401 to a valid getStatus that succeeded when repeated) or a timeout is
/// that; any other failure stays an error.
fn no_verdict(error: PostError) -> Result<Value, String> {
    let (reason, http_status) = match &error {
        PostError::Status(code) => ("http_status", json!(code)),
        PostError::Timeout(_) => ("timeout", Value::Null),
        PostError::Other(_) => return Err(error.to_string()),
    };
    Ok(json!({ "outcome": "no_verdict", "reason": reason, "http_status": http_status, "message": error.to_string() }))
}

fn fault_json(code: &str, message: &str) -> Value {
    json!({
        "outcome": "fault",
        "code": code,
        "message": message,
        "class": classify_code(code).map(|c| c.as_str()),
    })
}

fn text<'a>(input: &'a Value, field: &str) -> Result<&'a str, String> {
    input[field].as_str().ok_or_else(|| format!("missing string field `{field}`"))
}

fn base64_field(input: &Value, field: &str) -> Result<Vec<u8>, String> {
    STANDARD.decode(text(input, field)?).map_err(|e| format!("`{field}`: {e}"))
}

#[cfg(test)]
#[path = "commands_tests.rs"]
mod tests;
