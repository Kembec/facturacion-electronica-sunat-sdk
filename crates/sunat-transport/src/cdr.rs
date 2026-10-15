//! Reading of the CDR (constancia de recepción), an UBL ApplicationResponse.
//! The paths follow the ApplicationResponse structure of SUNAT's programmer's manual (SEE 2.1,
//! annex A.2): `cbc:Note` below the root and `cac:DocumentResponse/cac:Response` with
//! `ReferenceID`, `ResponseCode` and `Description`.

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
    let notes = nodes
        .iter()
        .filter(|n| n.path.len() == 2 && n.path[1] == "Note")
        .map(|n| n.text.clone())
        .collect();
    Ok(Cdr {
        reference_id: at(&nodes, &["DocumentResponse", "Response", "ReferenceID"]).unwrap_or_default(),
        response_code,
        description: at(&nodes, &["DocumentResponse", "Response", "Description"]).unwrap_or_default(),
        notes,
    })
}

/// Text of the first element at `path` below the root (the root name is not checked).
fn at(nodes: &[Node], path: &[&str]) -> Option<String> {
    nodes
        .iter()
        .find(|n| n.path.len() == path.len() + 1 && n.path[1..].iter().zip(path).all(|(a, b)| a == b))
        .map(|n| n.text.clone())
        .filter(|t| !t.is_empty())
}
