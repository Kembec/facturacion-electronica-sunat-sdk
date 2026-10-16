//! The SUNAT web-service calls as JSON in, JSON out, without I/O: `request` gives the SOAP envelope
//! to POST and `read` turns what came back into the outcome. The CLI and the WebAssembly binding go
//! through here, so they print the same JSON for the same answer (docs/cli.md).

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use serde_json::{json, Value};
use sunat_transport::{
    get_status_cdr_envelope, get_status_envelope, parse_get_status_cdr_response,
    parse_get_status_response, parse_send_bill_response, parse_send_summary_response, read_cdr,
    send_bill_envelope, send_summary_envelope, zip_document, zip_file_name, Cdr, SendBillOutcome,
    SendSummaryOutcome, StatusCdrOutcome, TicketStatusOutcome, SOAP_ACTION_GET_STATUS,
    SOAP_ACTION_GET_STATUS_CDR, SOAP_ACTION_SEND_BILL, SOAP_ACTION_SEND_SUMMARY,
};

use crate::{classify_cdr, classify_code};

/// One web-service call, named as the CLI command that makes it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Exchange {
    /// `sendBill`: input `{"name", "xml" | "xml_base64"}`.
    Send,
    /// `sendSummary` (RC, RA): same input as `Send`, named `RUC-RC|RA-YYYYMMDD-N`. The ticket is the
    /// only handle on the outcome: store it before anything else, then ask `Ticket` with it.
    Summary,
    /// `getStatus`: input `{"ticket"}`.
    Ticket,
    /// `getStatusCdr`: input `{"ruc", "document_type", "series", "number"}`.
    Status,
}

impl Exchange {
    /// `send`, `summary`, `ticket` or `status`.
    pub fn from_name(name: &str) -> Result<Self, String> {
        match name {
            "send" => Ok(Self::Send),
            "summary" => Ok(Self::Summary),
            "ticket" => Ok(Self::Ticket),
            "status" => Ok(Self::Status),
            _ => Err(format!("unknown exchange `{name}`: expected send, summary, ticket or status")),
        }
    }

    /// The `SOAPAction` header value, without quotes.
    pub fn soap_action(self) -> &'static str {
        match self {
            Self::Send => SOAP_ACTION_SEND_BILL,
            Self::Summary => SOAP_ACTION_SEND_SUMMARY,
            Self::Ticket => SOAP_ACTION_GET_STATUS,
            Self::Status => SOAP_ACTION_GET_STATUS_CDR,
        }
    }

    /// A query only asks; a send that got no answer may have reached SUNAT.
    fn is_query(self) -> bool {
        matches!(self, Self::Ticket | Self::Status)
    }
}

/// What came back for one POST.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reply<'a> {
    /// The body of an HTTP 200, or of a 500, which is how SOAP faults arrive.
    Body(&'a [u8]),
    /// Any other HTTP status.
    HttpStatus(u16),
    /// The wait ran out after this many seconds; whether SUNAT got the request is unknown.
    Timeout(u64),
}

impl<'a> Reply<'a> {
    /// The reply for an HTTP response: its body for 200 and 500, its status otherwise.
    pub fn from_http(status: u16, body: &'a [u8]) -> Self {
        match status {
            200 | 500 => Self::Body(body),
            _ => Self::HttpStatus(status),
        }
    }
}

/// The SOAP envelope of `exchange` for `input`, authenticated with `user` (RUC + SOL user) and
/// `password`. POST it with `Content-Type: text/xml; charset=utf-8` and `SOAPAction:
/// "<soap_action>"`.
pub fn request(exchange: Exchange, input: &Value, user: &str, password: &str) -> Result<String, String> {
    match exchange {
        Exchange::Send => {
            let (name, zip) = zipped(input)?;
            Ok(send_bill_envelope(user, password, &zip_file_name(&name), &zip))
        }
        Exchange::Summary => {
            let (name, zip) = zipped(input)?;
            Ok(send_summary_envelope(user, password, &zip_file_name(&name), &zip))
        }
        Exchange::Ticket => get_status_envelope(user, password, text(input, "ticket")?).map_err(|e| e.to_string()),
        Exchange::Status => get_status_cdr_envelope(
            user,
            password,
            text(input, "ruc")?,
            text(input, "document_type")?,
            text(input, "series")?,
            number(input)?,
        )
        .map_err(|e| e.to_string()),
    }
}

/// The outcome of `exchange` for `reply`; `input` is the one given to `request`.
///
/// A query without an answer (an HTTP status, or the wait ran out) is `no_verdict`: it says
/// nothing about the document, so the caller asks again and never sends again because of it
/// (SUNAT has answered 401 to a valid getStatus that succeeded when repeated). For a send the same
/// cases are errors, since the document may have arrived.
pub fn read(exchange: Exchange, input: &Value, reply: Reply) -> Result<Value, String> {
    let body = match reply {
        Reply::Body(body) => body,
        _ if !exchange.is_query() => return Err(no_answer_message(reply)),
        _ => {
            let mut out = no_verdict(reply);
            if exchange == Exchange::Ticket {
                out["ticket"] = json!(text(input, "ticket")?);
            }
            return Ok(out);
        }
    };
    match exchange {
        Exchange::Send => read_send(body),
        Exchange::Summary => read_summary(body),
        Exchange::Ticket => read_ticket(body),
        Exchange::Status => read_status(body),
    }
}

/// `{"name", "xml" | "xml_base64"}` → `{"file_name", "zip_base64"}`.
pub fn zip(input: &Value) -> Result<Value, String> {
    let (name, zip) = zipped(input)?;
    Ok(json!({ "file_name": zip_file_name(&name), "zip_base64": STANDARD.encode(zip) }))
}

/// `{"cdr_base64"}` (zip or XML) → the CDR fields and their class.
pub fn cdr(input: &Value) -> Result<Value, String> {
    let bytes = base64_field(input, "cdr_base64")?;
    Ok(cdr_json(&read_cdr(&bytes).map_err(|e| e.to_string())?))
}

fn read_send(body: &[u8]) -> Result<Value, String> {
    match parse_send_bill_response(body).map_err(|e| e.to_string())? {
        SendBillOutcome::Cdr(cdr_zip) => Ok(json!({
            "outcome": "cdr",
            "cdr": cdr_json(&read_cdr(&cdr_zip).map_err(|e| e.to_string())?),
            "cdr_zip_base64": STANDARD.encode(&cdr_zip),
        })),
        SendBillOutcome::Fault { code, message } => Ok(fault_json(&code, &message)),
    }
}

fn read_summary(body: &[u8]) -> Result<Value, String> {
    match parse_send_summary_response(body).map_err(|e| e.to_string())? {
        SendSummaryOutcome::Ticket(ticket) => Ok(json!({ "outcome": "ticket", "ticket": ticket })),
        SendSummaryOutcome::Fault { code, message } => Ok(fault_json(&code, &message)),
    }
}

/// `state` is `processed` (0), `in_process` (98: ask again with the same ticket, never send again)
/// or `processed_with_errors` (99), with the CDR if attached.
fn read_ticket(body: &[u8]) -> Result<Value, String> {
    match parse_get_status_response(body).map_err(|e| e.to_string())? {
        TicketStatusOutcome::Fault { code, message } => Ok(fault_json(&code, &message)),
        TicketStatusOutcome::Status(status) => {
            let state = match status.status_code.trim() {
                "0" => Value::from("processed"),
                "98" => Value::from("in_process"),
                "99" => Value::from("processed_with_errors"),
                _ => Value::Null,
            };
            let mut out = json!({ "outcome": "ticket_status", "status_code": status.status_code, "state": state });
            attach_cdr(&mut out, status.cdr)?;
            Ok(out)
        }
    }
}

fn read_status(body: &[u8]) -> Result<Value, String> {
    match parse_get_status_cdr_response(body).map_err(|e| e.to_string())? {
        StatusCdrOutcome::Fault { code, message } => Ok(fault_json(&code, &message)),
        StatusCdrOutcome::Status(status) => {
            let mut out = json!({
                "outcome": "status",
                "status_code": status.status_code,
                "status_message": status.status_message,
            });
            attach_cdr(&mut out, status.cdr)?;
            Ok(out)
        }
    }
}

fn attach_cdr(out: &mut Value, cdr_zip: Option<Vec<u8>>) -> Result<(), String> {
    if let Some(cdr_zip) = cdr_zip {
        out["cdr"] = cdr_json(&read_cdr(&cdr_zip).map_err(|e| e.to_string())?);
        out["cdr_zip_base64"] = json!(STANDARD.encode(cdr_zip));
    }
    Ok(())
}

fn no_verdict(reply: Reply) -> Value {
    let (reason, http_status) = match reply {
        Reply::HttpStatus(code) => ("http_status", json!(code)),
        _ => ("timeout", Value::Null),
    };
    json!({ "outcome": "no_verdict", "reason": reason, "http_status": http_status, "message": no_answer_message(reply) })
}

fn no_answer_message(reply: Reply) -> String {
    match reply {
        Reply::HttpStatus(code) => format!("SUNAT answered HTTP {code}"),
        Reply::Timeout(seconds) => format!("timed out after {seconds} s waiting for SUNAT; the outcome is unknown"),
        Reply::Body(_) => String::new(),
    }
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

fn number(input: &Value) -> Result<u32, String> {
    input["number"]
        .as_u64()
        .or_else(|| input["number"].as_str().and_then(|s| s.parse().ok()))
        .and_then(|n| u32::try_from(n).ok())
        .ok_or_else(|| "`number` must be a positive integer".to_string())
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
#[path = "exchange_tests.rs"]
mod tests;
