//! Minimal namespace-agnostic reader: every element's own text, keyed by its path of local names.

use quick_xml::events::Event;
use quick_xml::Reader;

use crate::TransportError;

/// An element with the local names from the root down to it, and its direct text.
pub(crate) struct Node {
    pub path: Vec<String>,
    pub text: String,
}

/// Decodes XML bytes to text: ISO-8859-1 when the declaration says so (SUNAT CDRs), UTF-8 otherwise.
pub(crate) fn decode(bytes: &[u8]) -> Result<String, TransportError> {
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
    if declares_latin1(bytes) {
        return Ok(bytes.iter().map(|&b| b as char).collect());
    }
    String::from_utf8(bytes.to_vec()).map_err(|e| TransportError::Xml(e.to_string()))
}

fn declares_latin1(bytes: &[u8]) -> bool {
    if !bytes.starts_with(b"<?xml") {
        return false;
    }
    let end = bytes.windows(2).position(|w| w == b"?>").unwrap_or(0);
    let declaration = String::from_utf8_lossy(&bytes[..end]).to_ascii_lowercase();
    declaration.contains("iso-8859-1") || declaration.contains("latin1")
}

/// All elements of the document in closing order.
pub(crate) fn nodes(xml: &str) -> Result<Vec<Node>, TransportError> {
    let mut reader = Reader::from_str(xml);
    let mut stack: Vec<(String, String)> = Vec::new();
    let mut out = Vec::new();
    let err = |e: quick_xml::Error| TransportError::Xml(e.to_string());
    loop {
        match reader.read_event().map_err(err)? {
            Event::Start(e) => stack.push((local(e.local_name().as_ref()), String::new())),
            Event::Empty(e) => {
                let mut path: Vec<String> = stack.iter().map(|(n, _)| n.clone()).collect();
                path.push(local(e.local_name().as_ref()));
                out.push(Node { path, text: String::new() });
            }
            Event::Text(t) => append(&mut stack, &t.unescape().map_err(err)?),
            Event::CData(c) => append(&mut stack, &String::from_utf8_lossy(&c.into_inner())),
            Event::End(_) => {
                let path: Vec<String> = stack.iter().map(|(n, _)| n.clone()).collect();
                let Some((_, text)) = stack.pop() else {
                    return Err(TransportError::Xml("unbalanced end tag".into()));
                };
                out.push(Node { path, text: text.trim().to_string() });
            }
            Event::Eof => break,
            _ => {}
        }
    }
    if !stack.is_empty() {
        return Err(TransportError::Xml("unexpected end of document".into()));
    }
    Ok(out)
}

/// Text of the first element whose path ends with `suffix`, if not empty.
pub(crate) fn text_by_suffix(nodes: &[Node], suffix: &[&str]) -> Option<String> {
    nodes
        .iter()
        .find(|n| ends_with(&n.path, suffix) && !n.text.is_empty())
        .map(|n| n.text.clone())
}

pub(crate) fn has_suffix(nodes: &[Node], suffix: &[&str]) -> bool {
    nodes.iter().any(|n| ends_with(&n.path, suffix))
}

fn ends_with(path: &[String], suffix: &[&str]) -> bool {
    path.len() >= suffix.len()
        && path[path.len() - suffix.len()..].iter().zip(suffix).all(|(a, b)| a == b)
}

fn append(stack: &mut [(String, String)], text: &str) {
    if let Some((_, buf)) = stack.last_mut() {
        buf.push_str(text);
    }
}

fn local(name: &[u8]) -> String {
    String::from_utf8_lossy(name).into_owned()
}
