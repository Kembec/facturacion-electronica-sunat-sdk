//! Parsing and small tree helpers shared by signing and verification.

use roxmltree::{Document, Node, ParsingOptions};

use crate::SignError;

pub(crate) const DSIG_NS: &str = "http://www.w3.org/2000/09/xmldsig#";
pub(crate) const EXT_NS: &str =
    "urn:oasis:names:specification:ubl:schema:xsd:CommonExtensionComponents-2";
pub(crate) const XML_NS: &str = "http://www.w3.org/XML/1998/namespace";

pub(crate) const ALG_C14N: &str = "http://www.w3.org/TR/2001/REC-xml-c14n-20010315";
pub(crate) const ALG_RSA_SHA1: &str = "http://www.w3.org/2000/09/xmldsig#rsa-sha1";
pub(crate) const ALG_SHA1: &str = "http://www.w3.org/2000/09/xmldsig#sha1";
pub(crate) const ALG_ENVELOPED: &str = "http://www.w3.org/2000/09/xmldsig#enveloped-signature";

/// The document split into an optional UTF-8 byte order mark and the text the parser sees.
pub(crate) struct Source<'a> {
    pub(crate) bom: &'a str,
    pub(crate) text: &'a str,
}

/// Checks the document is UTF-8 (the only encoding the core emits) and splits off a BOM.
pub(crate) fn source(xml: &[u8]) -> Result<Source<'_>, SignError> {
    let all = std::str::from_utf8(xml)
        .map_err(|_| SignError::UnsupportedEncoding("bytes are not UTF-8".into()))?;
    let (bom, text) = match all.strip_prefix('\u{feff}') {
        Some(rest) => (&all[..3], rest),
        None => ("", all),
    };
    if let Some(encoding) = declared_encoding(text) {
        if !encoding.eq_ignore_ascii_case("utf-8") && !encoding.eq_ignore_ascii_case("utf8") {
            return Err(SignError::UnsupportedEncoding(encoding.into()));
        }
    }
    Ok(Source { bom, text })
}

/// The `encoding` pseudo-attribute of the XML declaration, if any.
fn declared_encoding(text: &str) -> Option<&str> {
    let decl = text.strip_prefix("<?xml")?;
    let decl = &decl[..decl.find("?>")?];
    let after = &decl[decl.find("encoding")? + "encoding".len()..];
    let after = after.trim_start().strip_prefix('=')?.trim_start();
    let quote = after.chars().next().filter(|c| *c == '"' || *c == '\'')?;
    let value = &after[1..];
    Some(&value[..value.find(quote)?])
}

/// Parses with DTDs refused (no entity expansion from untrusted input).
pub(crate) fn parse(text: &str) -> Result<Document<'_>, SignError> {
    let options = ParsingOptions {
        allow_dtd: false,
        ..ParsingOptions::default()
    };
    Document::parse_with_options(text, options).map_err(|e| SignError::InvalidXml(e.to_string()))
}

/// The element's qualified name as written in its start tag.
pub(crate) fn element_qname<'a>(input: &'a str, node: Node) -> &'a str {
    let tag = &input[node.range().start + 1..];
    let end = tag
        .find(|c: char| c.is_ascii_whitespace() || c == '/' || c == '>')
        .unwrap_or(tag.len());
    &tag[..end]
}

pub(crate) fn is_element(node: Node, namespace: &str, local: &str) -> bool {
    node.is_element()
        && node.tag_name().namespace() == Some(namespace)
        && node.tag_name().name() == local
}

/// First child element in the XML-DSig namespace with this local name.
pub(crate) fn dsig_child<'a, 'input>(
    node: Node<'a, 'input>,
    local: &str,
) -> Option<Node<'a, 'input>> {
    node.children().find(|c| is_element(*c, DSIG_NS, local))
}

/// DOM `textContent`: every descendant text node (CDATA included), in document order.
pub(crate) fn text_content(node: Node) -> String {
    node.descendants()
        .filter(|n| n.is_text())
        .filter_map(|n| n.text())
        .collect()
}

/// PHP's `trim()` default character set: " \t\n\r\0\x0B".
pub(crate) fn php_trim(text: &str) -> &str {
    text.trim_matches(|c| matches!(c, ' ' | '\t' | '\n' | '\r' | '\0' | '\u{0B}'))
}

/// Whether `prefix` is bound to `uri` in the element's scope.
pub(crate) fn prefix_bound_to(node: Node, prefix: &str, uri: &str) -> bool {
    node.namespaces()
        .any(|ns| ns.name() == Some(prefix) && ns.uri() == uri)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn declared_encoding_is_read() {
        assert_eq!(
            declared_encoding("<?xml version=\"1.0\" encoding='utf-8'?><a/>"),
            Some("utf-8")
        );
        assert_eq!(declared_encoding("<?xml version=\"1.0\"?><a/>"), None);
        assert_eq!(declared_encoding("<a/>"), None);
    }

    #[test]
    fn other_encodings_are_refused() {
        let xml = b"<?xml version=\"1.0\" encoding=\"ISO-8859-1\"?><a/>";
        assert!(matches!(
            source(xml),
            Err(SignError::UnsupportedEncoding(_))
        ));
    }

    #[test]
    fn qname_is_read_from_the_start_tag() {
        let text = "<p:a xmlns:p=\"u\"><p:b/><c\n/></p:a>";
        let doc = parse(text).unwrap();
        let names: Vec<_> = doc
            .descendants()
            .filter(|n| n.is_element())
            .map(|n| element_qname(text, n))
            .collect();
        assert_eq!(names, ["p:a", "p:b", "c"]);
    }
}
