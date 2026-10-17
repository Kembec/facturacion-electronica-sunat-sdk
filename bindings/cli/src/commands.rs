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
    let out = exchange::read(kind, input, reply)?;
    Ok(sunat_sdk::diagnosis::with_diagnosis(kind, input, out))
}

#[cfg(test)]
#[path = "commands_tests.rs"]
mod tests;
