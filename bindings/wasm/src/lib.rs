//! WebAssembly binding: `build` and `sign` in memory, with the input and output shapes of the CLI
//! (docs/cli.md). No I/O: the caller passes the document, the XML and the PEMs as strings. Errors
//! are thrown as a JS `Error` whose message is the CLI's `error` text.

mod api;

pub use api::{build_json, sign_json};
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
