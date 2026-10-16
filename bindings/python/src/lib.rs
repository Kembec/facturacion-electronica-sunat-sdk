//! Python binding (`sunat_sdk._native`): the npm package's in-memory commands (bindings/wasm/src/api.rs)
//! behind PyO3, with the same JSON text in and out. No I/O: the pure-Python layer in
//! `python/sunat_sdk` parses the JSON and does the POST to SUNAT. Errors are raised as
//! `SunatSdkError` whose message is the CLI's `error` text.

use pyo3::create_exception;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use sunat_sdk_wasm as api;

create_exception!(_native, SunatSdkError, PyValueError, "The SDK rejected the input; the message says why.");

fn raise(message: String) -> PyErr {
    SunatSdkError::new_err(message)
}

/// `document_json` (docs/input-model.md) -> `{"file_name", "xml"}` as JSON text.
#[pyfunction]
fn build(py: Python<'_>, document_json: &str) -> PyResult<String> {
    py.detach(|| api::build_json(document_json)).map_err(raise)
}

/// Signs `xml` -> `{"xml", "digest_value", "signature_value"}` as JSON text.
#[pyfunction]
#[pyo3(signature = (xml, cert_pem, key_pem, signature_id=None))]
fn sign(py: Python<'_>, xml: &str, cert_pem: &str, key_pem: &str, signature_id: Option<&str>) -> PyResult<String> {
    py.detach(|| api::sign_json(xml, cert_pem, key_pem, signature_id)).map_err(raise)
}

/// `{"name", "xml" | "xml_base64"}` -> `{"file_name", "zip_base64"}` as JSON text.
#[pyfunction]
fn zip(py: Python<'_>, input_json: &str) -> PyResult<String> {
    py.detach(|| api::zip_json(input_json)).map_err(raise)
}

/// `{"cdr_base64"}` -> the CDR fields and their class as JSON text.
#[pyfunction]
fn cdr(py: Python<'_>, input_json: &str) -> PyResult<String> {
    py.detach(|| api::cdr_json(input_json)).map_err(raise)
}

/// The POST of `exchange` (send, summary, ticket, status) as `{"method", "headers", "body"}`.
#[pyfunction]
fn soap_request(exchange: &str, input_json: &str, user: &str, password: &str) -> PyResult<String> {
    api::soap_request_json(exchange, input_json, user, password).map_err(raise)
}

/// The HTTP status and body SUNAT answered -> the CLI's output for that exchange.
#[pyfunction]
fn soap_response(py: Python<'_>, exchange: &str, input_json: &str, http_status: u16, body: &[u8]) -> PyResult<String> {
    py.detach(|| api::soap_response_json(exchange, input_json, http_status, body)).map_err(raise)
}

/// No answer within `seconds` -> `no_verdict` for ticket and status; raised for send and summary.
#[pyfunction]
fn soap_timeout(exchange: &str, input_json: &str, seconds: u32) -> PyResult<String> {
    api::soap_timeout_json(exchange, input_json, seconds).map_err(raise)
}

/// Consulta Integrada token request as `{"method", "url", "headers", "body"}`.
#[pyfunction]
#[pyo3(signature = (client_id, client_secret, base=None))]
fn validity_token_request(client_id: &str, client_secret: &str, base: Option<&str>) -> PyResult<String> {
    api::validity_token_request_json(client_id, client_secret, base).map_err(raise)
}

/// The token answer -> `token` or `no_verdict`.
#[pyfunction]
fn validity_token_response(http_status: u16, body: &[u8]) -> PyResult<String> {
    api::validity_token_response_json(http_status, body).map_err(raise)
}

/// The query for the CLI's `validity` input, with the token, as `{"method", "url", "headers", "body"}`.
#[pyfunction]
#[pyo3(signature = (input_json, access_token, base=None))]
fn validity_request(input_json: &str, access_token: &str, base: Option<&str>) -> PyResult<String> {
    api::validity_request_json(input_json, access_token, base).map_err(raise)
}

/// The query answer -> the CLI's `validity` output.
#[pyfunction]
fn validity_response(http_status: u16, body: &[u8]) -> PyResult<String> {
    api::validity_response_json(http_status, body).map_err(raise)
}

/// No answer within `seconds` for `stage` (`token` or `query`) -> `no_verdict`.
#[pyfunction]
fn validity_timeout(stage: &str, seconds: u32) -> PyResult<String> {
    api::validity_timeout_json(stage, seconds).map_err(raise)
}

#[pymodule]
fn _native(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("SunatSdkError", m.py().get_type::<SunatSdkError>())?;
    m.add_function(wrap_pyfunction!(build, m)?)?;
    m.add_function(wrap_pyfunction!(sign, m)?)?;
    m.add_function(wrap_pyfunction!(zip, m)?)?;
    m.add_function(wrap_pyfunction!(cdr, m)?)?;
    m.add_function(wrap_pyfunction!(soap_request, m)?)?;
    m.add_function(wrap_pyfunction!(soap_response, m)?)?;
    m.add_function(wrap_pyfunction!(soap_timeout, m)?)?;
    m.add_function(wrap_pyfunction!(validity_token_request, m)?)?;
    m.add_function(wrap_pyfunction!(validity_token_response, m)?)?;
    m.add_function(wrap_pyfunction!(validity_request, m)?)?;
    m.add_function(wrap_pyfunction!(validity_response, m)?)?;
    m.add_function(wrap_pyfunction!(validity_timeout, m)?)?;
    Ok(())
}
