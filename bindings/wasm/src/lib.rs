//! WebAssembly binding: the CLI's commands in memory, with its input and output shapes
//! (docs/cli.md). No I/O: the caller passes the document, the XML and the PEMs as strings, and does
//! the POST to SUNAT itself with what `soapRequest` returns. Errors are thrown as a JS `Error` whose
//! message is the CLI's `error` text.

mod api;

pub use api::{build_json, cdr_json, sign_json, soap_request_json, soap_response_json, soap_timeout_json, zip_json};
use wasm_bindgen::prelude::*;

/// `document_json` (docs/input-model.md) -> `{"file_name": "...", "xml": "<unsigned XML>"}`.
#[wasm_bindgen]
pub fn build(document_json: &str) -> Result<String, JsError> {
    build_json(document_json).map_err(|message| JsError::new(&message))
}

/// Signs `xml` -> `{"xml": "<signed XML>", "digest_value": "...", "signature_value": "..."}`.
/// `signature_id` defaults to `signatureKG`; the key PEM must be unencrypted RSA.
#[wasm_bindgen]
pub fn sign(xml: &str, cert_pem: &str, key_pem: &str, signature_id: Option<String>) -> Result<String, JsError> {
    sign_json(xml, cert_pem, key_pem, signature_id.as_deref()).map_err(|message| JsError::new(&message))
}

/// `{"name", "xml" | "xml_base64"}` -> `{"file_name", "zip_base64"}`.
#[wasm_bindgen]
pub fn zip(input_json: &str) -> Result<String, JsError> {
    zip_json(input_json).map_err(|message| JsError::new(&message))
}

/// `{"cdr_base64"}` (zip or XML) -> `{"reference_id", "response_code", "description", "notes", "class"}`.
#[wasm_bindgen]
pub fn cdr(input_json: &str) -> Result<String, JsError> {
    cdr_json(input_json).map_err(|message| JsError::new(&message))
}

/// `exchange` is `send`, `summary`, `ticket` or `status` -> `fetch` options for the POST.
#[wasm_bindgen(js_name = soapRequest)]
pub fn soap_request(exchange: &str, input_json: &str, user: &str, password: &str) -> Result<String, JsError> {
    soap_request_json(exchange, input_json, user, password).map_err(|message| JsError::new(&message))
}

/// The HTTP status and body SUNAT answered -> the CLI's output for that exchange.
#[wasm_bindgen(js_name = soapResponse)]
pub fn soap_response(exchange: &str, input_json: &str, http_status: u16, body: &[u8]) -> Result<String, JsError> {
    soap_response_json(exchange, input_json, http_status, body).map_err(|message| JsError::new(&message))
}

/// No answer within `seconds` -> `no_verdict` for ticket and status; thrown for send and summary.
#[wasm_bindgen(js_name = soapTimeout)]
pub fn soap_timeout(exchange: &str, input_json: &str, seconds: u32) -> Result<String, JsError> {
    soap_timeout_json(exchange, input_json, seconds).map_err(|message| JsError::new(&message))
}
