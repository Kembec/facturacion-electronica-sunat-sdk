//! Reading of the CDR (constancia de recepción), an UBL ApplicationResponse.
//! The paths follow the ApplicationResponse structure of SUNAT's programmer's manual (SEE 2.1,
//! annex A.2): `cbc:Note` below the root and `cac:DocumentResponse/cac:Response` with
//! `ReferenceID`, `ResponseCode` and `Description`.
//!
//! An OSE's CDR (SUNAT's OSE technical annex) is read by the same function: it names the document in
//! `cac:DocumentResponse/cac:DocumentReference/cbc:ID` instead of `ReferenceID`, and writes each
//! observation as a `cac:Response/cac:Status` with `StatusReasonCode` and `StatusReason`. Each status
//! enters `notes` as `"<code> - <reason>"`, the form of SUNAT's notes, so classification and
//! diagnosis read both answers alike.

use crate::archive::unzip_xml;
use crate::xml::{decode, nodes, Node};
use crate::TransportError;

/// What the CDR says about the document. `response_code` is kept as SUNAT writes it (e.g. `0109`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cdr {
    pub reference_id: String,
    pub response_code: String,
    pub description: String,
    pub notes: Vec<String>,
}

impl Cdr {
    /// `response_code` as a number, for classification.
    pub fn response_code_number(&self) -> Option<u32> {
        self.response_code.parse().ok()
    }
}

/// Reads a CDR from its zip (as SUNAT returns it) or from its XML.
pub fn read_cdr(bytes: &[u8]) -> Result<Cdr, TransportError> {
    let xml = if bytes.starts_with(b"PK\x03\x04") { unzip_xml(bytes)? } else { bytes.to_vec() };
    let nodes = nodes(&decode(&xml)?)?;
    let response_code = at(&nodes, &["DocumentResponse", "Response", "ResponseCode"])
        .ok_or(TransportError::MissingElement("ResponseCode"))?;
    let mut notes: Vec<String> = nodes
        .iter()
        .filter(|n| n.path.len() == 2 && n.path[1] == "Note")
        .map(|n| n.text.clone())
        .collect();
    notes.extend(status_notes(&nodes));
    let reference_id = at(&nodes, &["DocumentResponse", "Response", "ReferenceID"])
        .or_else(|| at(&nodes, &["DocumentResponse", "DocumentReference", "ID"]))
        .unwrap_or_default();
    Ok(Cdr {
        reference_id,
        response_code,
        description: at(&nodes, &["DocumentResponse", "Response", "Description"]).unwrap_or_default(),
        notes,
    })
}

/// One `"<code> - <reason>"` per `DocumentResponse/Response/Status`, in document order. Nodes come
/// in closing order, so a status's children precede it; a status with only one of the two gives it
/// alone, and an empty one gives nothing.
fn status_notes(nodes: &[Node]) -> Vec<String> {
    const STATUS: [&str; 3] = ["DocumentResponse", "Response", "Status"];
    let mut out = Vec::new();
    let (mut code, mut reason) = (String::new(), String::new());
    for node in nodes {
        if is_at(node, &STATUS) {
            let note: Vec<&str> = [code.as_str(), reason.as_str()].into_iter().filter(|t| !t.is_empty()).collect();
            if !note.is_empty() {
                out.push(note.join(" - "));
            }
            code.clear();
            reason.clear();
            continue;
        }
        if node.path.len() != STATUS.len() + 2 || !is_at_prefix(node, &STATUS) {
            continue;
        }
        match node.path[STATUS.len() + 1].as_str() {
            "StatusReasonCode" if code.is_empty() => code = node.text.clone(),
            "StatusReason" if reason.is_empty() => reason = node.text.clone(),
            _ => {}
        }
    }
    out
}

fn is_at(node: &Node, path: &[&str]) -> bool {
    node.path.len() == path.len() + 1 && is_at_prefix(node, path)
}

fn is_at_prefix(node: &Node, path: &[&str]) -> bool {
    node.path.len() > path.len() && node.path[1..].iter().zip(path).all(|(a, b)| a == b)
}

/// Text of the first element at `path` below the root (the root name is not checked).
fn at(nodes: &[Node], path: &[&str]) -> Option<String> {
    nodes
        .iter()
        .find(|n| is_at(n, path))
        .map(|n| n.text.clone())
        .filter(|t| !t.is_empty())
}
