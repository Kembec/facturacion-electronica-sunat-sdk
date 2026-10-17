//! GRE (despatch advice, 09) by SUNAT's REST API, host-testable: strings in, JSON string out, error as
//! its message. The calls and outcomes are `sunat_sdk::gre`'s, so the npm and Python packages give the
//! same JSON as the CLI's `gre-send` and `gre-ticket` for the same answer. No I/O and no endpoint: the
//! URL bases come from the caller, because SUNAT has no test environment for GRE.

use serde_json::Value;
use sunat_sdk::gre::{self, Answer, Stage};

/// The token request (`grant_type=password`) as `fetch` options with its `url`; `user` is the RUC
/// followed by the SOL user. The body carries the client secret and the SOL password: never log it.
pub fn gre_token_request_json(client_id: &str, client_secret: &str, user: &str, password: &str, token_base: &str) -> Result<String, String> {
    Ok(gre::token_request(client_id, client_secret, user, password, token_base)?.to_string())
}

/// The token answer, `received_at` being the caller's clock in Unix seconds when it arrived ->
/// `{"outcome": "token", "access_token", "token_type", "expires_in", "expires_at"}`, `api_error` or `no_verdict`.
pub fn gre_token_response_json(http_status: u16, body: &[u8], received_at: u64) -> Result<String, String> {
    Ok(gre::token_read(Answer::Http(http_status, body), received_at)?.to_string())
}

/// Whether a `token` outcome (JSON text) can still be used at `now` (Unix seconds): more than ten
/// minutes before its `expires_at`.
pub fn gre_token_usable_json(token_json: &str, now: u64) -> Result<bool, String> {
    Ok(gre::token_usable(&parse(token_json)?, now))
}

/// The send of `{"name": "RUC-09-SERIE-NUMERO", "xml" | "xml_base64"}` (the signed guide) as `fetch`
/// options with its `url`, plus `zip_name` and `hash_zip` to keep with the ticket.
pub fn gre_send_request_json(input_json: &str, access_token: &str, api_base: &str) -> Result<String, String> {
    Ok(gre::send_request(&parse(input_json)?, access_token, api_base)?.to_string())
}

/// The send answer -> `{"outcome": "ticket", "ticket", "received_at"}` or `api_error`. An answer that
/// is not GRE-M's is an error: the guide may have arrived, so the same file is sent again.
pub fn gre_send_response_json(http_status: u16, body: &[u8]) -> Result<String, String> {
    Ok(gre::send_read(Answer::Http(http_status, body))?.to_string())
}

/// The ticket query as `fetch` options with its `url` (a GET, `body` empty).
pub fn gre_ticket_request_json(ticket: &str, access_token: &str, api_base: &str) -> Result<String, String> {
    Ok(gre::ticket_request(ticket, access_token, api_base)?.to_string())
}

/// The ticket answer -> `ticket_status` (0 `processed`, 98 `in_process`, 99 `processed_with_errors`,
/// with the CDR read when there is one), `api_error` or `no_verdict`; every outcome carries `ticket`.
pub fn gre_ticket_response_json(http_status: u16, body: &[u8], ticket: &str) -> Result<String, String> {
    Ok(gre::ticket_read(Answer::Http(http_status, body), ticket)?.to_string())
}

/// No answer within `seconds` for `stage`: `no_verdict` for `token`, and for `ticket` with the
/// `ticket` asked; an error for `send`, whose guide may have arrived.
pub fn gre_timeout_json(stage: &str, seconds: u32, ticket: Option<&str>) -> Result<String, String> {
    let seconds = u64::from(seconds);
    let out: Value = match Stage::from_name(stage)? {
        Stage::Token => gre::timeout(Stage::Token, seconds),
        Stage::Send => gre::send_read(Answer::Timeout(seconds))?,
        Stage::Ticket => gre::ticket_read(Answer::Timeout(seconds), ticket.ok_or("a ticket timeout needs the ticket")?)?,
    };
    Ok(out.to_string())
}

/// Unix seconds from a JS number: whole, not negative and exact as a double. A count in milliseconds
/// (`Date.now()`) is refused, because it would push `expires_at` thousands of years away.
pub fn unix_seconds(value: f64) -> Result<u64, String> {
    const MILLIS_THRESHOLD: f64 = 1e11;
    if !value.is_finite() || value < 0.0 || value.fract() != 0.0 || value >= MILLIS_THRESHOLD {
        return Err(format!("{value} is not a time in whole Unix seconds (Math.floor(Date.now() / 1000))"));
    }
    Ok(value as u64)
}

fn parse(input_json: &str) -> Result<Value, String> {
    serde_json::from_str(input_json).map_err(|e| format!("input is not JSON: {e}"))
}

#[cfg(test)]
#[path = "gre_tests.rs"]
mod tests;
