//! SUNAT's REST «Consulta Integrada» (Manual de Consulta Integrada de Comprobante de Pago por Servicio
//! Web v2.0, 22/11/2018) without I/O: an OAuth token by `client_credentials`, then one POST that says
//! whether SUNAT has a voucher and in which state (`estadoCp`). Like `exchange`, each step is a
//! `*_request` that gives the HTTP call and a `*_read` that turns the answer into the outcome, so the
//! CLI, the WebAssembly and the Python bindings print the same JSON for the same answer (docs/cli.md).
//!
//! SUNAT offers no test environment for this API: there is only production, with the credentials of
//! «Credenciales de API SUNAT» in SOL. The tests replay the answers the manual prints and the error
//! shapes derived from them (`tests/data/validity/`).

use serde_json::{json, Map, Value};

use crate::validity_input::{query_body, requester_ruc, validate_client_id};

/// Base of the token URL: `<base>/<client_id>/oauth2/token/` (manual §B).
pub const TOKEN_BASE: &str = "https://api-seguridad.sunat.gob.pe/v1/clientesextranet";
/// Base of the query URL: `<base>/<RUC that asks>/validarcomprobante` (manual §C).
pub const QUERY_BASE: &str = "https://api.sunat.gob.pe/v1/contribuyente/contribuyentes";
/// Fixed `scope` of the token request (manual §B).
pub const SCOPE: &str = "https://api.sunat.gob.pe/v1/contribuyente/contribuyentes";

/// Which of the two calls an outcome without a verdict comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    Token,
    Query,
}

impl Stage {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Token => "token",
            Self::Query => "query",
        }
    }

    /// `token` or `query`.
    pub fn from_name(name: &str) -> Result<Self, String> {
        match name {
            "token" => Ok(Self::Token),
            "query" => Ok(Self::Query),
            _ => Err(format!("unknown stage `{name}`: expected token or query")),
        }
    }
}

/// What came back for one call.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Answer<'a> {
    /// An HTTP response, whatever its status.
    Http(u16, &'a [u8]),
    /// The wait ran out after this many seconds.
    Timeout(u64),
}

/// The token request as `{"method", "url", "headers", "body"}`; `base` defaults to [`TOKEN_BASE`].
/// The body carries the client secret: never log it.
pub fn token_request(client_id: &str, client_secret: &str, base: Option<&str>) -> Result<Value, String> {
    validate_client_id(client_id)?;
    if client_secret.is_empty() {
        return Err("missing client secret".into());
    }
    let url = format!("{}/{client_id}/oauth2/token/", base.unwrap_or(TOKEN_BASE).trim_end_matches('/'));
    let body = form(&[
        ("grant_type", "client_credentials"),
        ("scope", SCOPE),
        ("client_id", client_id),
        ("client_secret", client_secret),
    ]);
    let headers = json!({ "Content-Type": "application/x-www-form-urlencoded" });
    Ok(json!({ "method": "POST", "url": url, "headers": headers, "body": body }))
}

/// `{"outcome": "token", "access_token", "token_type", "expires_in"}` for an HTTP 200 that carries a
/// token; `no_verdict` (stage `token`) for any other status or a timeout. The token is a secret.
pub fn token_read(answer: Answer) -> Result<Value, String> {
    let (status, body) = match answer {
        Answer::Timeout(_) => return Ok(no_verdict(Stage::Token, answer)),
        Answer::Http(status, body) => (status, body),
    };
    if status != 200 {
        return Ok(no_verdict(Stage::Token, answer));
    }
    let parsed: Value = serde_json::from_slice(body).map_err(|_| "the token answer is not JSON".to_string())?;
    let token = parsed["access_token"].as_str().filter(|t| !t.is_empty());
    let token = token.ok_or("the token answer has no `access_token`")?;
    Ok(json!({
        "outcome": "token",
        "access_token": token,
        "token_type": parsed["token_type"].as_str(),
        "expires_in": parsed["expires_in"].as_u64(),
    }))
}

/// The query for `input` (`{"ruc", "document_type", "series", "number", "issue_date", "total",
/// "requester_ruc"?}`) as `{"method", "url", "headers", "body"}`; `base` defaults to [`QUERY_BASE`].
/// The `Authorization` header carries the token: never log it.
pub fn query_request(input: &Value, access_token: &str, base: Option<&str>) -> Result<Value, String> {
    if access_token.is_empty() {
        return Err("missing access token".into());
    }
    let body = query_body(input)?;
    let url = format!("{}/{}/validarcomprobante", base.unwrap_or(QUERY_BASE).trim_end_matches('/'), requester_ruc(input)?);
    let headers = json!({ "Content-Type": "application/json", "Authorization": format!("Bearer {access_token}") });
    Ok(json!({ "method": "POST", "url": url, "headers": headers, "body": body.to_string() }))
}

/// The outcome of the query:
/// - `validity`: SUNAT answered `success: true`; `state` is `not_reported` (0), `accepted` (1),
///   `voided` (2), `printed_authorized` (3) or `printed_not_authorized` (4), null for any other
///   code or none. `observations` say where the data sent differs from SUNAT's (an accepted voucher
///   can come with a different issue date).
/// - `query_error`: SUNAT refused the query itself (`success: false`, with HTTP 200, 400 or 422).
///   It says nothing about the voucher.
/// - `no_verdict` (stage `query`): any other HTTP status (401 included) or the wait ran out.
pub fn query_read(answer: Answer) -> Result<Value, String> {
    let (status, body) = match answer {
        Answer::Timeout(_) => return Ok(no_verdict(Stage::Query, answer)),
        Answer::Http(status, body) => (status, body),
    };
    let parsed = serde_json::from_slice::<Value>(body).ok().filter(Value::is_object);
    match (status, parsed) {
        (200, Some(parsed)) if parsed["success"] == json!(true) => Ok(validity_json(&parsed)),
        (200 | 400 | 422, Some(parsed)) if parsed["success"] == json!(false) => Ok(query_error(status, &parsed)),
        (200, _) => Err("the validity answer is neither a result nor an error".into()),
        _ => Ok(no_verdict(Stage::Query, answer)),
    }
}

/// `no_verdict` for `stage` after `seconds` without an answer.
pub fn timeout(stage: Stage, seconds: u64) -> Value {
    no_verdict(stage, Answer::Timeout(seconds))
}

/// The name of an `estadoCp` code (manual, annex). `not_reported` is the manual's «no informado»: it
/// does not prove the voucher never arrived (a summary's ticket may still be in process), so it is
/// never a reason to issue the operation with another number. 3 and 4 are for printed vouchers,
/// whose series start with a digit; for an electronic series they are a state outside the contract.
pub fn state_name(code: &str) -> Option<&'static str> {
    match code {
        "0" => Some("not_reported"),
        "1" => Some("accepted"),
        "2" => Some("voided"),
        "3" => Some("printed_authorized"),
        "4" => Some("printed_not_authorized"),
        _ => None,
    }
}

fn validity_json(parsed: &Value) -> Value {
    let data = parsed["data"].as_object().cloned().unwrap_or_default();
    let state_code = code(&data, "estadoCp");
    let observations: Vec<Value> = data
        .iter()
        .find(|(key, _)| key.eq_ignore_ascii_case("observaciones"))
        .and_then(|(_, value)| value.as_array())
        .map(|list| list.iter().filter_map(Value::as_str).map(|s| json!(s)).collect())
        .unwrap_or_default();
    json!({
        "outcome": "validity",
        "state_code": state_code,
        "state": state_code.as_deref().and_then(state_name),
        "taxpayer_state_code": code(&data, "estadoRuc"),
        "domicile_condition_code": code(&data, "condDomiRuc"),
        "observations": observations,
        "message": parsed["message"].as_str().unwrap_or_default().trim(),
    })
}

fn query_error(status: u16, parsed: &Value) -> Value {
    json!({
        "outcome": "query_error",
        "http_status": status,
        "error_code": parsed["errorCode"].as_str().map(str::to_string).or_else(|| parsed["errorCode"].as_i64().map(|n| n.to_string())),
        "message": parsed["message"].as_str().unwrap_or_default().trim(),
    })
}

/// A code SUNAT sends as a string (`"1"`) or, per the manual's table, as a number.
fn code(data: &Map<String, Value>, field: &str) -> Option<String> {
    match data.get(field)? {
        Value::String(s) if !s.trim().is_empty() => Some(s.trim().to_string()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

fn no_verdict(stage: Stage, answer: Answer) -> Value {
    let (reason, http_status, message) = match answer {
        Answer::Http(code, _) => ("http_status", json!(code), format!("SUNAT answered HTTP {code} to the {} request", stage.as_str())),
        Answer::Timeout(seconds) => ("timeout", Value::Null, format!("timed out after {seconds} s waiting for the {} answer", stage.as_str())),
    };
    json!({ "outcome": "no_verdict", "stage": stage.as_str(), "reason": reason, "http_status": http_status, "message": message })
}

/// `application/x-www-form-urlencoded`: unreserved bytes as they are, everything else `%XX`.
fn form(pairs: &[(&str, &str)]) -> String {
    let encode = |text: &str| -> String {
        text.bytes()
            .map(|b| match b {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => (b as char).to_string(),
                _ => format!("%{b:02X}"),
            })
            .collect()
    };
    pairs.iter().map(|(k, v)| format!("{}={}", encode(k), encode(v))).collect::<Vec<_>>().join("&")
}

#[cfg(test)]
#[path = "validity_tests.rs"]
mod tests;
