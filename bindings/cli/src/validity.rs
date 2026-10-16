//! `validity`: the Consulta Integrada's two calls (token, then query) built and read by
//! `sunat_sdk::validity`, under one wait for both, so a caller that kills the process after its
//! own margin (the PHP client) still gets `no_verdict` instead of a killed process.

use std::time::{Duration, Instant};

use serde_json::Value;
use sunat_sdk::validity::{self, Answer, Stage};

use crate::config::ApiConnection;
use crate::http::PostError;

/// `post(request, timeout) -> (status, body)`.
pub type PostRest<'a> = &'a dyn Fn(&Value, Duration) -> Result<(u16, Vec<u8>), PostError>;

/// `{"ruc", "document_type", "series", "number", "issue_date", "total", "requester_ruc"?}` →
/// `validity`, `query_error` or `no_verdict` (docs/cli.md). The token never reaches the output.
pub fn validity(input: &Value, conn: &ApiConnection, post: PostRest) -> Result<Value, String> {
    let started = Instant::now();
    let token_call = validity::token_request(&conn.client_id, &conn.client_secret, Some(&conn.token_base))?;
    // The query is checked before the token is spent on it.
    validity::query_request(input, "unused", Some(&conn.query_base))?;

    let token = match post(&token_call, conn.timeout) {
        Ok((status, body)) => validity::token_read(Answer::Http(status, &body))?,
        Err(failure) => return no_answer(failure, Stage::Token),
    };
    let Some(access_token) = token["access_token"].as_str() else {
        return Ok(token);
    };
    let left = conn.timeout.saturating_sub(started.elapsed());
    if left < Duration::from_secs(1) {
        return Ok(validity::timeout(Stage::Query, conn.timeout.as_secs()));
    }
    let query_call = validity::query_request(input, access_token, Some(&conn.query_base))?;
    match post(&query_call, left) {
        Ok((status, body)) => validity::query_read(Answer::Http(status, &body)),
        Err(failure) => no_answer(failure, Stage::Query),
    }
}

/// A timeout is `no_verdict`; any other transport failure is an error.
fn no_answer(failure: PostError, stage: Stage) -> Result<Value, String> {
    match failure {
        PostError::Timeout(seconds) => Ok(validity::timeout(stage, seconds)),
        PostError::Status(code) => {
            let read = if stage == Stage::Token { validity::token_read } else { validity::query_read };
            read(Answer::Http(code, &[]))
        }
        PostError::Other(message) => Err(message),
    }
}

#[cfg(test)]
#[path = "validity_tests.rs"]
mod tests;
