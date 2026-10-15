//! Host-testable core of the binding: strings in, JSON string out, error as its message.

use serde_json::json;
use sunat_model::Document;
use sunat_xmldsig::DEFAULT_SIGNATURE_ID;

/// Renders the unsigned XML of a document given as JSON.
pub fn build_json(document_json: &str) -> Result<String, String> {
    let document = Document::from_json(document_json).map_err(|e| e.to_string())?;
    let xml = sunat_ubl::build(&document).map_err(|e| e.to_string())?;
    let xml = String::from_utf8(xml).map_err(|e| e.to_string())?;
    Ok(json!({ "file_name": document.file_name(), "xml": xml }).to_string())
}

/// Signs an unsigned XML and reports the `DigestValue` and `SignatureValue` it carries.
pub fn sign_json(xml: &str, cert_pem: &str, key_pem: &str, signature_id: Option<&str>) -> Result<String, String> {
    let id = signature_id.unwrap_or(DEFAULT_SIGNATURE_ID);
    let signed = sunat_xmldsig::sign_with_id(xml.as_bytes(), cert_pem, key_pem, id).map_err(|e| e.to_string())?;
    let signed = String::from_utf8(signed).map_err(|e| e.to_string())?;
    let digest_value = element_text(&signed, "ds:DigestValue")?;
    let signature_value = element_text(&signed, "ds:SignatureValue")?;
    Ok(json!({ "xml": signed, "digest_value": digest_value, "signature_value": signature_value }).to_string())
}

/// Text of the first `<element>` (no attributes) in `xml`, without whitespace.
fn element_text(xml: &str, element: &str) -> Result<String, String> {
    let open = format!("<{element}>");
    let start = xml.find(&open).ok_or_else(|| format!("signed XML has no {element}"))? + open.len();
    let end = xml[start..].find('<').ok_or_else(|| format!("unterminated {element}"))?;
    Ok(xml[start..start + end].split_whitespace().collect())
}

#[cfg(test)]
#[path = "api_tests.rs"]
mod tests;
