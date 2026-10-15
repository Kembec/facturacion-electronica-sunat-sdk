//! `build` and `sign`: a phase 1 document to unsigned UBL XML, and that XML to its signed form.

use serde_json::{json, Value};
use sunat_model::Document;

const DEFAULT_SIGNATURE_ID: &str = sunat_ubl::DEFAULT_SIGNATURE_ID;

/// One document as JSON text (`docs/input-model.md`) → `{"file_name", "xml"}`.
pub fn build(raw: &str) -> Result<Value, String> {
    let document = Document::from_json(raw).map_err(|e| e.to_string())?;
    let xml = sunat_ubl::build(&document).map_err(|e| e.to_string())?;
    let xml = String::from_utf8(xml).map_err(|_| "built XML is not UTF-8".to_string())?;
    Ok(json!({ "file_name": document.file_name(), "xml": xml }))
}

/// `{"xml", "signature_id"?}` → `{"xml", "digest_value", "signature_value"}`. Errors come from
/// `sunat-xmldsig`, whose messages never carry key material.
pub fn sign(input: &Value, cert_pem: &str, key_pem: &str) -> Result<Value, String> {
    let xml = input["xml"].as_str().ok_or("missing string field `xml`")?;
    let signature_id = match &input["signature_id"] {
        Value::Null => DEFAULT_SIGNATURE_ID,
        Value::String(id) => id.as_str(),
        _ => return Err("`signature_id` must be a string".into()),
    };
    let signed = sunat_xmldsig::sign_with_id(xml.as_bytes(), cert_pem, key_pem, signature_id)
        .map_err(|e| e.to_string())?;
    let signed = String::from_utf8(signed).map_err(|_| "signed XML is not UTF-8".to_string())?;
    let (digest_value, signature_value) = signature_values(&signed, signature_id)?;
    Ok(json!({ "xml": signed, "digest_value": digest_value, "signature_value": signature_value }))
}

/// `DigestValue` and `SignatureValue` of the `ds:Signature` with `Id = signature_id`, as written
/// by `sunat-xmldsig` (no whitespace inside either element).
fn signature_values(signed: &str, signature_id: &str) -> Result<(String, String), String> {
    let anchor = format!(" Id=\"{signature_id}\">");
    let start = signed.find(&anchor).ok_or("signature not found in the signed XML")?;
    let signature = &signed[start..];
    Ok((element_text(signature, "ds:DigestValue")?, element_text(signature, "ds:SignatureValue")?))
}

fn element_text(xml: &str, element: &str) -> Result<String, String> {
    let open = format!("<{element}>");
    let close = format!("</{element}>");
    let start = xml.find(&open).ok_or_else(|| format!("{element} not found"))? + open.len();
    let end = xml[start..].find(&close).ok_or_else(|| format!("{element} not closed"))?;
    Ok(xml[start..start + end].to_string())
}
