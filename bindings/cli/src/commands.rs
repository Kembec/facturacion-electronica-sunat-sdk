//! The network commands: the envelope and the reading of the answer come from
//! `sunat_sdk::exchange`, shared with the WebAssembly binding; the POST goes through a `post`
//! function so the commands are testable without a server. The CLI adds `diagnosis` to the outcome:
//! the input field each code in the answer points at (`sunat_sdk::diagnosis`).

use serde_json::Value;
use sunat_sdk::exchange::{self, Exchange, Reply};

pub use sunat_sdk::exchange::{cdr, zip};

use crate::config::Connection;
use crate::http::PostError;

/// `post(endpoint, soap_action, envelope) -> body`.
pub type Post<'a> = &'a dyn Fn(&str, &str, &str) -> Result<Vec<u8>, PostError>;

/// Zips the signed XML and sends it with sendBill. Same input as `zip`.
pub fn send(input: &Value, conn: &Connection, post: Post) -> Result<Value, String> {
    call(Exchange::Send, input, conn, post)
}

/// Zips a signed daily summary (RC) or void communication (RA) and sends it with sendSummary.
pub fn summary(input: &Value, conn: &Connection, post: Post) -> Result<Value, String> {
    call(Exchange::Summary, input, conn, post)
}

/// `{"ticket"}` → getStatus answer, or `no_verdict` with the same ticket.
pub fn ticket(input: &Value, conn: &Connection, post: Post) -> Result<Value, String> {
    call(Exchange::Ticket, input, conn, post)
}

/// `{"ruc", "document_type", "series", "number"}` → getStatusCdr answer, or `no_verdict`.
pub fn status(input: &Value, conn: &Connection, post: Post) -> Result<Value, String> {
    call(Exchange::Status, input, conn, post)
}

fn call(kind: Exchange, input: &Value, conn: &Connection, post: Post) -> Result<Value, String> {
    let envelope = exchange::request(kind, input, &conn.user, &conn.password)?;
    let answer = post(&conn.endpoint, kind.soap_action(), &envelope);
    let reply = match &answer {
        Ok(body) => Reply::Body(body),
        Err(PostError::Status(code)) => Reply::HttpStatus(*code),
        Err(PostError::Timeout(seconds)) => Reply::Timeout(*seconds),
        Err(PostError::Other(message)) => return Err(message.clone()),
    };
    let mut out = exchange::read(kind, input, reply)?;
    if let Some(diagnosis) = sunat_sdk::diagnosis::diagnosis_json(&document_type(kind, input), &out) {
        out["diagnosis"] = diagnosis;
    }
    Ok(out)
}

/// The document type the codes are looked up for: the second part of the file name for `send` and
/// `summary` (`RUC-01-F001-1`, `RUC-RC-...`), the `document_type` field for `status`. A `ticket`
/// only answers for a summary or void communication, whose codes the table has no rows for.
fn document_type(kind: Exchange, input: &Value) -> String {
    let field = |name: &str| input[name].as_str().unwrap_or_default().to_string();
    match kind {
        Exchange::Send | Exchange::Summary => field("name").split('-').nth(1).unwrap_or_default().to_string(),
        Exchange::Status => field("document_type"),
        Exchange::Ticket => String::new(),
    }
}

#[cfg(test)]
#[path = "commands_tests.rs"]
mod tests;
