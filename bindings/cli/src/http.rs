//! The only network code of the SDK: one SOAP POST over HTTPS with certificate verification
//! (ureq's rustls with webpki roots; there is no switch to disable it).

use std::io::Read;
use std::time::Duration;

const MAX_RESPONSE_BYTES: u64 = 16 * 1024 * 1024;

/// Posts `envelope` and returns the body. SOAP faults arrive as HTTP 500 and are returned too,
/// so the caller can parse them; other errors carry no request data.
pub fn post_soap(endpoint: &str, soap_action: &str, envelope: &str) -> Result<Vec<u8>, String> {
    let agent = ureq::AgentBuilder::new().timeout(Duration::from_secs(90)).build();
    let response = agent
        .post(endpoint)
        .set("Content-Type", "text/xml; charset=utf-8")
        .set("SOAPAction", &format!("\"{soap_action}\""))
        .send_string(envelope);
    let response = match response {
        Ok(response) => response,
        Err(ureq::Error::Status(500, response)) => response,
        Err(ureq::Error::Status(code, _)) => return Err(format!("SUNAT answered HTTP {code}")),
        Err(ureq::Error::Transport(err)) => return Err(format!("transport error: {}", err.kind())),
    };
    let mut body = Vec::new();
    response
        .into_reader()
        .take(MAX_RESPONSE_BYTES)
        .read_to_end(&mut body)
        .map_err(|e| format!("reading response: {e}"))?;
    Ok(body)
}
