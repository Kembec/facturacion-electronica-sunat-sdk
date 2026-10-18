//! The only network code of the SDK: one POST over HTTPS with certificate verification (ureq's
//! rustls with webpki roots; there is no switch to disable it), SOAP or REST.

use std::io::Read;
use std::time::Duration;

const MAX_RESPONSE_BYTES: u64 = 16 * 1024 * 1024;

/// Why a POST gave no body to parse; `sunat_sdk::exchange` words the first two. None of them
/// carries request data.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PostError {
    /// SUNAT answered with an HTTP status other than 200 or 500 (500 carries SOAP faults).
    Status(u16),
    /// The whole exchange ran out of time (seconds); whether SUNAT got the request is unknown.
    Timeout(u64),
    /// Any other transport or read failure.
    Other(String),
}

/// Posts `envelope` and returns the body. SOAP faults arrive as HTTP 500 and are returned too,
/// so the caller can parse them. `timeout` bounds the whole exchange; when it runs out the
/// voucher may or may not have reached SUNAT.
pub fn post_soap(endpoint: &str, soap_action: &str, envelope: &str, timeout: Duration) -> Result<Vec<u8>, PostError> {
    let response = agent(timeout)
        .post(endpoint)
        .header("Content-Type", "text/xml; charset=utf-8")
        .header("SOAPAction", format!("\"{soap_action}\""))
        .send(envelope)
        .map_err(|e| failure(e, timeout))?;
    match response.status().as_u16() {
        code if code < 400 || code == 500 => read_body(response),
        code => Err(PostError::Status(code)),
    }
}

/// Sends a REST call built by `sunat_sdk::validity` or `sunat_sdk::gre` (`{"method", "url", "headers",
/// "body"}`; `POST` or `GET`, which carries no body) and returns the HTTP status with its body,
/// whatever the status: the caller decides what each means.
pub fn post_rest(request: &serde_json::Value, timeout: Duration) -> Result<(u16, Vec<u8>), PostError> {
    let url = request["url"].as_str().ok_or_else(|| PostError::Other("request without url".into()))?;
    let method = request["method"].as_str().unwrap_or("POST");
    let agent = agent(timeout);
    let headers = request["headers"].as_object().into_iter().flatten();
    let response = match method {
        "GET" => {
            let mut call = agent.get(url);
            for (name, value) in headers {
                call = call.header(name, value.as_str().unwrap_or_default());
            }
            call.call()
        }
        "POST" => {
            let mut call = agent.post(url);
            for (name, value) in headers {
                call = call.header(name, value.as_str().unwrap_or_default());
            }
            call.send(request["body"].as_str().unwrap_or_default())
        }
        other => return Err(PostError::Other(format!("unsupported method {other}"))),
    }
    .map_err(|e| failure(e, timeout))?;
    let status = response.status().as_u16();
    Ok((status, read_body(response)?))
}

/// One agent per call. A status other than 2xx is not an error of the client: the caller reads the body and decides.
/// The status line is read by `httparse`, which accepts the non-ASCII reason phrase RFC 9112 allows (`obs-text`) and
/// the SUNAT beta uses to carry its rejection text; the SOAP fault is in the body.
fn agent(timeout: Duration) -> ureq::Agent {
    ureq::Agent::config_builder().timeout_global(Some(timeout)).http_status_as_error(false).build().into()
}

fn read_body(response: ureq::http::Response<ureq::Body>) -> Result<Vec<u8>, PostError> {
    let mut body = Vec::new();
    response
        .into_body()
        .into_with_config()
        .limit(MAX_RESPONSE_BYTES)
        .reader()
        .take(MAX_RESPONSE_BYTES)
        .read_to_end(&mut body)
        .map_err(|e| PostError::Other(format!("reading response: {e}")))?;
    Ok(body)
}

fn failure(err: ureq::Error, timeout: Duration) -> PostError {
    if is_timeout(&err) {
        return PostError::Timeout(timeout.as_secs());
    }
    PostError::Other(format!("transport error: {err}"))
}

fn is_timeout(err: &ureq::Error) -> bool {
    match err {
        ureq::Error::Timeout(_) => true,
        ureq::Error::Io(e) => matches!(e.kind(), std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock),
        _ => false,
    }
}

#[cfg(test)]
#[path = "http_tests.rs"]
mod tests;
