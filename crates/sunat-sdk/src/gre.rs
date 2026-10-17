//! SUNAT's REST API for the despatch advice (GRE, type 09) without I/O: an OAuth token by `password`,
//! the POST of the zipped guide with its `hashZip`, and the GET of the ticket until there is a CDR.
//! Like `validity`, each step is a `*_request` that gives the HTTP call and a `*_read` that turns the
//! answer into the outcome, so every binding prints the same JSON for the same answer (docs/cli.md).
//!
//! Sources. Authentication, the error bodies and the general errors of annex I come from SUNAT's
//! «Manual de servicios web, plataforma nueva GRE» (GRE-M, 05/10/2022, §1.2, §1.3 and annex I). GRE-M
//! does not describe the send and the ticket: their paths and fields come from the MIT client
//! `greenter/gre-api` (`CpeApi`, `CpeDocumentArchivo`, `CpeResponse`, `StatusResponse`) and its use in
//! `greenter/ws` (`GreSender`), a secondary source. SUNAT offers no test environment for GRE: nothing
//! here has been accepted by SUNAT, and the tests replay exchanges labelled by their source
//! (`tests/data/gre/README.md`).
//!
//! The URL bases always come from the caller: this module holds no endpoint.

use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use sunat_transport::{read_cdr, zip_document, zip_file_name};

use crate::classify::classify_code;
use crate::exchange::cdr_json;
use crate::validity_input::validate_client_id;
pub use crate::validity::Answer;

/// Fixed `scope` of the token request (GRE-M §1.2 b).
pub const SCOPE: &str = "https://api-cpe.sunat.gob.pe";
/// A token closer than this to its expiry is renewed before a call (the secondary source renews 10
/// minutes ahead; GRE-M says a token lasts what `expires_in` says, today one hour).
pub const RENEW_MARGIN_SECONDS: u64 = 600;

/// Which of the three calls an outcome comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    Token,
    Send,
    Ticket,
}

impl Stage {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Token => "token",
            Self::Send => "send",
            Self::Ticket => "ticket",
        }
    }

    /// `token`, `send` or `ticket`.
    pub fn from_name(name: &str) -> Result<Self, String> {
        match name {
            "token" => Ok(Self::Token),
            "send" => Ok(Self::Send),
            "ticket" => Ok(Self::Ticket),
            _ => Err(format!("unknown stage `{name}`: expected token, send or ticket")),
        }
    }
}

/// The token request as `{"method", "url", "headers", "body"}` to `<token_base>/<client_id>/oauth2/token/`
/// (GRE-M §1.2 b: `…/v1/clientessol`). `username` is the RUC followed by the SOL user. GRE-M's header
/// table says `Content-Type: application/json` while it asks for a form body; the request sends the
/// form type, as the secondary source does. The body carries both secrets: never log it.
pub fn token_request(client_id: &str, client_secret: &str, username: &str, password: &str, token_base: &str) -> Result<Value, String> {
    validate_client_id(client_id)?;
    validate_username(username)?;
    if client_secret.is_empty() || password.is_empty() {
        return Err("missing client secret or SOL password".into());
    }
    let url = format!("{}/{client_id}/oauth2/token/", base(token_base)?);
    let body = form(&[
        ("grant_type", "password"),
        ("scope", SCOPE),
        ("client_id", client_id),
        ("client_secret", client_secret),
        ("username", username),
        ("password", password),
    ]);
    let headers = json!({ "Content-Type": "application/x-www-form-urlencoded", "Accept": "application/json" });
    Ok(json!({ "method": "POST", "url": url, "headers": headers, "body": body }))
}

/// `{"outcome": "token", "access_token", "token_type", "expires_in", "expires_at"}` for an HTTP 200 with
/// a token; `expires_at` is `received_at` (Unix seconds when the answer arrived, from the caller's
/// clock) plus `expires_in`. A JSON error body is `api_error`; anything else, `no_verdict`.
pub fn token_read(answer: Answer, received_at: u64) -> Result<Value, String> {
    let (status, body) = match answer {
        Answer::Timeout(_) => return Ok(no_verdict(Stage::Token, answer)),
        Answer::Http(status, body) => (status, body),
    };
    if status != 200 {
        return Ok(error_or_no_verdict(Stage::Token, answer));
    }
    let parsed: Value = serde_json::from_slice(body).map_err(|_| "the token answer is not JSON".to_string())?;
    let token = parsed["access_token"].as_str().filter(|t| !t.is_empty());
    let token = token.ok_or("the token answer has no `access_token`")?;
    let expires_in = parsed["expires_in"].as_u64().or_else(|| parsed["expires_in"].as_str().and_then(|s| s.parse().ok()));
    Ok(json!({
        "outcome": "token",
        "access_token": token,
        "token_type": parsed["token_type"].as_str(),
        "expires_in": expires_in,
        "expires_at": expires_in.map(|s| received_at.saturating_add(s)),
    }))
}

/// Whether a `token` outcome can still be used at `now` (Unix seconds): more than
/// [`RENEW_MARGIN_SECONDS`] before `expires_at`. A token without `expires_at` is not reused.
pub fn token_usable(token: &Value, now: u64) -> bool {
    let present = token["access_token"].as_str().is_some_and(|t| !t.is_empty());
    let expires_at = token["expires_at"].as_u64();
    present && expires_at.is_some_and(|at| at > now.saturating_add(RENEW_MARGIN_SECONDS))
}

/// The send of `input` (`{"name": "RUC-09-SERIE-NUMERO", "xml" | "xml_base64"}`, the signed guide) as
/// `{"method", "url", "headers", "body"}`, plus `zip_name` and `hash_zip` so the caller can keep them
/// with the ticket. Body: `{"archivo": {"nomArchivo", "arcGreZip", "hashZip"}}`, the zip in base64 and
/// its SHA-256 in lowercase hex (secondary source). The `Authorization` header carries the token.
pub fn send_request(input: &Value, access_token: &str, api_base: &str) -> Result<Value, String> {
    require_token(access_token)?;
    let name = text(input, "name")?;
    despatch_name(name)?;
    let xml = match input["xml"].as_str() {
        Some(xml) => xml.as_bytes().to_vec(),
        None => STANDARD
            .decode(input["xml_base64"].as_str().ok_or("missing `xml` or `xml_base64`")?)
            .map_err(|e| format!("`xml_base64`: {e}"))?,
    };
    let zip = zip_document(name, &xml).map_err(|e| e.to_string())?;
    let hash_zip: String = Sha256::digest(&zip).iter().map(|b| format!("{b:02x}")).collect();
    let zip_name = zip_file_name(name);
    let body = json!({ "archivo": { "nomArchivo": zip_name, "arcGreZip": STANDARD.encode(&zip), "hashZip": hash_zip } });
    Ok(json!({
        "method": "POST",
        "url": format!("{}/contribuyente/gem/comprobantes/{name}", base(api_base)?),
        "headers": bearer(access_token, true),
        "body": body.to_string(),
        "zip_name": zip_name,
        "hash_zip": hash_zip,
    }))
}

/// `{"outcome": "ticket", "ticket", "received_at"}` for an HTTP 200 with `numTicket`: store the ticket
/// before anything else, it is the only handle on the outcome. A JSON error body is `api_error`. No
/// answer or a body that is not GRE-M's is an error, as for `exchange` sends: the guide may have
/// arrived, so the same file is sent again and never the operation with another number.
pub fn send_read(answer: Answer) -> Result<Value, String> {
    let (status, body) = match answer {
        Answer::Timeout(seconds) => return Err(format!("timed out after {seconds} s waiting for SUNAT; the guide may have arrived")),
        Answer::Http(status, body) => (status, body),
    };
    let parsed = object(body);
    if status == 200 {
        let parsed = parsed.ok_or("the send answer is not JSON")?;
        let ticket = parsed.get("numTicket").and_then(Value::as_str).filter(|t| !t.is_empty()).ok_or("the send answer has no `numTicket`")?;
        return Ok(json!({ "outcome": "ticket", "ticket": ticket, "received_at": parsed.get("fecRecepcion").and_then(Value::as_str) }));
    }
    match parsed {
        Some(parsed) => Ok(api_error(Stage::Send, status, &parsed)),
        None => Err(format!("SUNAT answered HTTP {status} to the send; the guide may have arrived")),
    }
}

/// The ticket query as `{"method": "GET", "url", "headers", "body": ""}` to
/// `<api_base>/contribuyente/gem/comprobantes/envios/<ticket>` (secondary source).
pub fn ticket_request(ticket: &str, access_token: &str, api_base: &str) -> Result<Value, String> {
    require_token(access_token)?;
    let valid = !ticket.is_empty() && ticket.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'.' || b == b'_');
    if !valid || ticket.bytes().all(|b| b == b'.') {
        return Err("the ticket must be the letters, digits, `-`, `.` and `_` SUNAT returned".into());
    }
    let url = format!("{}/contribuyente/gem/comprobantes/envios/{ticket}", base(api_base)?);
    Ok(json!({ "method": "GET", "url": url, "headers": bearer(access_token, false), "body": "" }))
}

/// The outcome of the ticket query; every outcome carries `ticket`:
/// - `ticket_status`: `status_code` (`codRespuesta`) and `state`: `processed` (0), `in_process` (98: ask
///   again with the same ticket, never send again) or `processed_with_errors` (99), null otherwise;
///   `cdr_generated` (`indCdrGenerado` = 1), the `cdr` read with the SDK's reader and `cdr_zip_base64`
///   when there is one, and `error` (`numError`, `desError`, its class) when SUNAT sends it.
/// - `api_error`: SUNAT answered a GRE-M error body (annex I or 422 with `errors`).
/// - `no_verdict`: no answer or a body that is not GRE-M's. Ask again.
pub fn ticket_read(answer: Answer, ticket: &str) -> Result<Value, String> {
    let mut out = match answer {
        Answer::Http(200, body) => status_json(&object(body).ok_or("the ticket answer is not JSON")?)?,
        _ => error_or_no_verdict(Stage::Ticket, answer),
    };
    out["ticket"] = json!(ticket);
    Ok(out)
}

/// `no_verdict` for `stage` after `seconds` without an answer (a send without answer is an error:
/// [`send_read`]).
pub fn timeout(stage: Stage, seconds: u64) -> Value {
    no_verdict(stage, Answer::Timeout(seconds))
}

/// Name and annex I text of a general error, by HTTP status (GRE-M annex I).
pub fn general_error(status: u16) -> Option<(&'static str, &'static str)> {
    Some(match status {
        400 => ("bad_request", "El Request no puede ser entendido por el Servidor debido a errores de Sintaxis"),
        401 => ("unauthorized", "Fallo en la autenticación del Cliente"),
        403 => ("forbidden", "El Cliente no tiene autorización para acceder al Recurso"),
        404 => ("not_found", "El Recurso Solicitado no puede ser encontrado"),
        405 => ("not_allowed", "El Método HTTP utilizado en el Request no es soportado por el Recurso"),
        406 => ("not_acceptable", "El Recurso no puede responder al Cliente en el Media Type solicitado"),
        415 => ("unsupported_media_type", "La Entidad en el Body del Request está en un Media Type que no es soportado"),
        422 => ("unprocessable_entity", "Se presentaron errores de validacion que impidieron completar el Request"),
        500 => ("internal_server_error", "Se presento una condición inesperada que impidió completar el Request"),
        503 => ("service_unavailable", "El Servidor no está disponible temporalmente o está muy ocupado"),
        _ => return None,
    })
}

fn status_json(parsed: &Map<String, Value>) -> Result<Value, String> {
    let status_code = code_of(parsed.get("codRespuesta")).ok_or("the ticket answer has no `codRespuesta`")?;
    let state = match status_code.as_str() {
        "0" => json!("processed"),
        "98" => json!("in_process"),
        "99" => json!("processed_with_errors"),
        _ => Value::Null,
    };
    let generated = code_of(parsed.get("indCdrGenerado")).as_deref() == Some("1");
    let mut out = json!({ "outcome": "ticket_status", "status_code": status_code, "state": state, "cdr_generated": generated });
    if let Some(error) = parsed.get("error").and_then(Value::as_object) {
        let number = code_of(error.get("numError"));
        out["error"] = json!({
            "code": number,
            "description": error.get("desError").and_then(Value::as_str).unwrap_or_default().trim(),
            "class": number.as_deref().and_then(classify_code).map(|c| c.as_str()),
        });
    }
    if generated {
        let encoded = parsed.get("arcCdr").and_then(Value::as_str).ok_or("`indCdrGenerado` is 1 without `arcCdr`")?;
        let cdr_zip = STANDARD.decode(encoded.trim()).map_err(|e| format!("`arcCdr`: {e}"))?;
        out["cdr"] = cdr_json(&read_cdr(&cdr_zip).map_err(|e| e.to_string())?);
        out["cdr_zip_base64"] = json!(STANDARD.encode(cdr_zip));
    }
    Ok(out)
}

/// GRE-M §1.3: `{cod, msg, exc}`, and for 422 an `errors` list. The manual prints the items as
/// `{codError, desError}`; the secondary source reads `{cod, msg}`: both are accepted. `exc` (a server
/// trace) is not copied.
fn api_error(stage: Stage, status: u16, parsed: &Map<String, Value>) -> Value {
    let item = |e: &Value| {
        let item_code = code_of(e.get("codError")).or_else(|| code_of(e.get("cod")));
        let text = e.get("desError").or_else(|| e.get("msg")).and_then(Value::as_str).unwrap_or_default().trim().to_string();
        json!({ "code": item_code, "description": text })
    };
    let errors: Vec<Value> = parsed.get("errors").and_then(Value::as_array).map(|l| l.iter().map(item).collect()).unwrap_or_default();
    json!({
        "outcome": "api_error",
        "stage": stage.as_str(),
        "http_status": status,
        "general_error": general_error(status).map(|(name, _)| name),
        "code": code_of(parsed.get("cod")),
        "message": parsed.get("msg").and_then(Value::as_str).unwrap_or_default().trim(),
        "errors": errors,
    })
}

/// A GRE-M error body (`cod` or `errors`) is `api_error`; anything else is `no_verdict`.
fn error_or_no_verdict(stage: Stage, answer: Answer) -> Value {
    if let Answer::Http(status, body) = answer {
        if let Some(parsed) = object(body).filter(|p| p.contains_key("cod") || p.contains_key("errors")) {
            return api_error(stage, status, &parsed);
        }
    }
    no_verdict(stage, answer)
}

fn no_verdict(stage: Stage, answer: Answer) -> Value {
    let (reason, http_status, message) = match answer {
        Answer::Http(code, _) => ("http_status", json!(code), format!("SUNAT answered HTTP {code} to the {} request", stage.as_str())),
        Answer::Timeout(seconds) => ("timeout", Value::Null, format!("timed out after {seconds} s waiting for the {} answer", stage.as_str())),
    };
    json!({ "outcome": "no_verdict", "stage": stage.as_str(), "reason": reason, "http_status": http_status, "message": message })
}

fn despatch_name(name: &str) -> Result<(), String> {
    let parts: Vec<&str> = name.split('-').collect();
    let ruc_ok = parts.first().is_some_and(|r| r.len() == 11 && r.bytes().all(|b| b.is_ascii_digit()));
    if parts.len() != 4 || !ruc_ok || parts[1] != "09" {
        return Err("`name` must be RUC-09-SERIE-NUMERO (the sender's despatch advice)".into());
    }
    Ok(())
}

fn validate_username(username: &str) -> Result<(), String> {
    let (ruc, user) = username.split_at(username.len().min(11));
    if ruc.len() != 11 || !ruc.bytes().all(|b| b.is_ascii_digit()) || user.is_empty() || !user.bytes().all(|b| b.is_ascii_alphanumeric()) {
        return Err("the user must be the RUC (11 digits) followed by the SOL user".into());
    }
    Ok(())
}

fn require_token(access_token: &str) -> Result<(), String> {
    if access_token.is_empty() {
        return Err("missing access token".into());
    }
    Ok(())
}

fn base(base: &str) -> Result<&str, String> {
    let trimmed = base.trim_end_matches('/');
    if trimmed.is_empty() {
        return Err("missing URL base".into());
    }
    Ok(trimmed)
}

fn bearer(access_token: &str, json_body: bool) -> Value {
    let mut headers = json!({ "Accept": "application/json", "Authorization": format!("Bearer {access_token}") });
    if json_body {
        headers["Content-Type"] = json!("application/json");
    }
    headers
}

fn object(body: &[u8]) -> Option<Map<String, Value>> {
    match serde_json::from_slice::<Value>(body).ok()? {
        Value::Object(map) => Some(map),
        _ => None,
    }
}

/// A code SUNAT may send as a string or a number.
fn code_of(value: Option<&Value>) -> Option<String> {
    match value? {
        Value::String(s) if !s.trim().is_empty() => Some(s.trim().to_string()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

fn text<'a>(input: &'a Value, field: &str) -> Result<&'a str, String> {
    input[field].as_str().ok_or_else(|| format!("missing string field `{field}`"))
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
#[path = "gre_tests.rs"]
mod tests;
