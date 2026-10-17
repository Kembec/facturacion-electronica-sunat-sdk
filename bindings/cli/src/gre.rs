//! `gre-send` and `gre-ticket`: the GRE REST calls (token, then send or ticket) built and read by
//! `sunat_sdk::gre`, under one wait for both, like `validity`. A process asks for its own token: the
//! token is never printed, so it cannot be reused across calls of the CLI.

use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};
use sunat_sdk::gre::{self, Answer, Stage};

use crate::config::GreConnection;
use crate::http::PostError;
use crate::validity::PostRest;

/// `{"name": "RUC-09-SERIE-NUMERO", "xml" | "xml_base64"}` (the signed guide) → `ticket` (with
/// `zip_name` and `hash_zip`), `api_error`, or `no_verdict` of the token stage (docs/cli.md). No
/// answer to the send itself is an error: the guide may have arrived.
pub fn send(input: &Value, conn: &GreConnection, post: PostRest) -> Result<Value, String> {
    // The send is checked before the token is spent on it.
    gre::send_request(input, "unused", &conn.api_base)?;
    let (access_token, left) = match token(conn, post)? {
        Ok(granted) => granted,
        Err(outcome) => return Ok(outcome),
    };
    let call = gre::send_request(input, &access_token, &conn.api_base)?;
    let mut out = match post(&call, left) {
        Ok((status, body)) => gre::send_read(Answer::Http(status, &body))?,
        Err(PostError::Timeout(seconds)) => return gre::send_read(Answer::Timeout(seconds)),
        Err(PostError::Status(code)) => gre::send_read(Answer::Http(code, &[]))?,
        Err(PostError::Other(message)) => return Err(message),
    };
    if out["outcome"] == "ticket" {
        out["zip_name"] = call["zip_name"].clone();
        out["hash_zip"] = call["hash_zip"].clone();
    }
    Ok(out)
}

/// `{"ticket"}` → `ticket_status`, `api_error` or `no_verdict` (docs/cli.md); all carry `ticket`.
pub fn ticket(input: &Value, conn: &GreConnection, post: PostRest) -> Result<Value, String> {
    let ticket = input["ticket"].as_str().ok_or("missing string field `ticket`")?;
    gre::ticket_request(ticket, "unused", &conn.api_base)?;
    let (access_token, left) = match token(conn, post)? {
        Ok(granted) => granted,
        Err(mut outcome) => {
            outcome["ticket"] = json!(ticket);
            return Ok(outcome);
        }
    };
    let call = gre::ticket_request(ticket, &access_token, &conn.api_base)?;
    match post(&call, left) {
        Ok((status, body)) => gre::ticket_read(Answer::Http(status, &body), ticket),
        Err(PostError::Timeout(seconds)) => gre::ticket_read(Answer::Timeout(seconds), ticket),
        Err(PostError::Status(code)) => gre::ticket_read(Answer::Http(code, &[]), ticket),
        Err(PostError::Other(message)) => Err(message),
    }
}

/// The access token and the wait left for the second call, or the token stage's outcome.
fn token(conn: &GreConnection, post: PostRest) -> Result<Result<(String, Duration), Value>, String> {
    let started = Instant::now();
    let call = gre::token_request(&conn.client_id, &conn.client_secret, &conn.user, &conn.password, &conn.token_base)?;
    let outcome = match post(&call, conn.timeout) {
        Ok((status, body)) => gre::token_read(Answer::Http(status, &body), now())?,
        Err(PostError::Timeout(seconds)) => gre::timeout(Stage::Token, seconds),
        Err(PostError::Status(code)) => gre::token_read(Answer::Http(code, &[]), now())?,
        Err(PostError::Other(message)) => return Err(message),
    };
    let Some(access_token) = outcome["access_token"].as_str() else {
        return Ok(Err(outcome));
    };
    let left = conn.timeout.saturating_sub(started.elapsed());
    if left < Duration::from_secs(1) {
        return Ok(Err(gre::timeout(Stage::Token, conn.timeout.as_secs())));
    }
    Ok(Ok((access_token.to_string(), left)))
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or_default()
}

#[cfg(test)]
#[path = "gre_tests.rs"]
mod tests;
