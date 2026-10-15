//! Inclusive Canonical XML 1.0 without comments (`REC-xml-c14n-20010315`), rendered the way
//! libxml2 renders it (`xmlC14NDocSaveTo`), the canonicalizer behind the compatibility vectors.
//!
//! Two node sets are needed: the whole document minus the signature (the `Reference URI=""`
//! with the enveloped transform) and the `SignedInfo` subtree. For a subtree, the apex element
//! carries every namespace in scope (inherited ones included) and the `xml:*` attributes of its
//! ancestors; a descendant only carries the namespaces its parent does not already have.

use std::collections::BTreeMap;

use roxmltree::{Document, Node, NodeId, NodeType};

use crate::xml::{element_qname, XML_NS};

/// In-scope namespaces by prefix; `""` is the default namespace (absent when undeclared).
type NsMap<'a> = BTreeMap<&'a str, &'a str>;

/// One attribute ready to render: sort key (namespace URI, local name), qname and value.
struct Attr<'a> {
    namespace: &'a str,
    local: &'a str,
    qname: String,
    value: &'a str,
}

/// C14N of the whole document, without the subtree rooted at `excluded`.
pub(crate) fn canonicalize_document(doc: &Document, excluded: Option<NodeId>) -> Vec<u8> {
    let mut c = Canonicalizer {
        input: doc.input_text(),
        excluded,
        out: String::new(),
    };
    let mut after_root = false;
    for child in doc.root().children() {
        match child.node_type() {
            NodeType::Element => {
                c.element(child, &NsMap::new(), Vec::new());
                after_root = true;
            }
            NodeType::PI if after_root => {
                c.out.push('\n');
                c.pi(child);
            }
            NodeType::PI => {
                c.pi(child);
                c.out.push('\n');
            }
            _ => {}
        }
    }
    c.out.into_bytes()
}

/// C14N of the subtree rooted at `element`, as the apex of a document subset.
pub(crate) fn canonicalize_subtree(doc: &Document, element: Node) -> Vec<u8> {
    let mut c = Canonicalizer {
        input: doc.input_text(),
        excluded: None,
        out: String::new(),
    };
    c.element(element, &NsMap::new(), inherited_xml_attributes(element));
    c.out.into_bytes()
}

/// `xml:*` attributes of the ancestors, nearest first, that the element does not set itself.
fn inherited_xml_attributes<'a>(element: Node<'a, '_>) -> Vec<Attr<'a>> {
    let mut found: Vec<Attr<'a>> = Vec::new();
    let own = |local: &str| {
        element
            .attributes()
            .any(|a| a.namespace() == Some(XML_NS) && a.name() == local)
    };
    for ancestor in element.ancestors().skip(1).filter(|n| n.is_element()) {
        for attr in ancestor
            .attributes()
            .filter(|a| a.namespace() == Some(XML_NS))
        {
            if own(attr.name()) || found.iter().any(|f| f.local == attr.name()) {
                continue;
            }
            found.push(Attr {
                namespace: XML_NS,
                local: attr.name(),
                qname: format!("xml:{}", attr.name()),
                value: attr.value(),
            });
        }
    }
    found
}

fn in_scope<'a>(node: Node<'a, '_>) -> NsMap<'a> {
    let mut scope = NsMap::new();
    for ns in node.namespaces() {
        let prefix = ns.name().unwrap_or("");
        if prefix == "xml" || (prefix.is_empty() && ns.uri().is_empty()) {
            continue;
        }
        scope.insert(prefix, ns.uri());
    }
    scope
}

struct Canonicalizer<'i> {
    input: &'i str,
    excluded: Option<NodeId>,
    out: String,
}

impl<'i> Canonicalizer<'i> {
    fn element<'a>(
        &mut self,
        node: Node<'a, 'i>,
        parent_scope: &NsMap<'a>,
        mut attrs: Vec<Attr<'a>>,
    ) {
        if Some(node.id()) == self.excluded {
            return;
        }
        let qname = element_qname(self.input, node);
        let scope = in_scope(node);

        self.out.push('<');
        self.out.push_str(qname);
        self.namespaces(&scope, parent_scope);

        for attr in node.attributes() {
            attrs.push(Attr {
                namespace: attr.namespace().unwrap_or(""),
                local: attr.name(),
                qname: self.input[attr.range_qname()].to_string(),
                value: attr.value(),
            });
        }
        attrs.sort_by(|a, b| (a.namespace, a.local).cmp(&(b.namespace, b.local)));
        for attr in &attrs {
            self.out.push(' ');
            self.out.push_str(&attr.qname);
            self.out.push_str("=\"");
            escape_attribute(&mut self.out, attr.value);
            self.out.push('"');
        }
        self.out.push('>');

        for child in node.children() {
            match child.node_type() {
                NodeType::Element => self.element(child, &scope, Vec::new()),
                NodeType::Text => escape_text(&mut self.out, child.text().unwrap_or("")),
                NodeType::PI => self.pi(child),
                _ => {}
            }
        }

        self.out.push_str("</");
        self.out.push_str(qname);
        self.out.push('>');
    }

    /// Namespace declarations not already in effect in the parent, default first, then by prefix.
    fn namespaces(&mut self, scope: &NsMap, parent_scope: &NsMap) {
        let mut render: NsMap = scope
            .iter()
            .filter(|(prefix, uri)| parent_scope.get(*prefix) != Some(*uri))
            .map(|(p, u)| (*p, *u))
            .collect();
        if !scope.contains_key("") && parent_scope.contains_key("") {
            render.insert("", "");
        }
        for (prefix, uri) in render {
            if prefix.is_empty() {
                self.out.push_str(" xmlns=");
            } else {
                self.out.push_str(" xmlns:");
                self.out.push_str(prefix);
                self.out.push('=');
            }
            write_quoted_like_libxml2(&mut self.out, uri);
        }
    }

    fn pi(&mut self, node: Node) {
        let Some(pi) = node.pi() else { return };
        self.out.push_str("<?");
        self.out.push_str(pi.target);
        if let Some(value) = pi.value.filter(|v| !v.is_empty()) {
            self.out.push(' ');
            self.out.push_str(&value.replace('\r', "&#xD;"));
        }
        self.out.push_str("?>");
    }
}

fn escape_text(out: &mut String, text: &str) {
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '\r' => out.push_str("&#xD;"),
            _ => out.push(c),
        }
    }
}

fn escape_attribute(out: &mut String, value: &str) {
    for c in value.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '"' => out.push_str("&quot;"),
            '\t' => out.push_str("&#x9;"),
            '\n' => out.push_str("&#xA;"),
            '\r' => out.push_str("&#xD;"),
            _ => out.push(c),
        }
    }
}

/// libxml2 writes namespace URIs with `xmlBufWriteQuotedString`, not with C14N escaping: double
/// quotes unless the value has `"`, single quotes if it has `"` but no `'`, else `"` as `&quot;`.
fn write_quoted_like_libxml2(out: &mut String, value: &str) {
    match (value.contains('"'), value.contains('\'')) {
        (false, _) => {
            out.push('"');
            out.push_str(value);
            out.push('"');
        }
        (true, false) => {
            out.push('\'');
            out.push_str(value);
            out.push('\'');
        }
        (true, true) => {
            out.push('"');
            out.push_str(&value.replace('"', "&quot;"));
            out.push('"');
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::xml::parse;

    fn doc_c14n(text: &str) -> String {
        String::from_utf8(canonicalize_document(&parse(text).unwrap(), None)).unwrap()
    }

    #[test]
    fn sorts_namespaces_and_attributes_and_expands_empty_elements() {
        let text = "<?xml version=\"1.0\"?>\n<r b=\"2\" xmlns:z=\"uz\" a=\"1\" xmlns=\"ud\" z:c=\"3\"><e/></r>\n";
        assert_eq!(
            doc_c14n(text),
            "<r xmlns=\"ud\" xmlns:z=\"uz\" a=\"1\" b=\"2\" z:c=\"3\"><e></e></r>"
        );
    }

    #[test]
    fn drops_comments_keeps_pis_and_escapes() {
        let text =
            "<?pi a?><!-- c --><r a=\"&lt;&quot;&#9;\"><![CDATA[<&>]]>&#13;<!-- x --></r><?pi b?>";
        assert_eq!(
            doc_c14n(text),
            "<?pi a?>\n<r a=\"&lt;&quot;&#x9;\">&lt;&amp;&gt;&#xD;</r>\n<?pi b?>"
        );
    }

    #[test]
    fn subtree_carries_inherited_namespaces_and_xml_attributes() {
        let text = "<r xmlns=\"ud\" xmlns:p=\"up\" xml:lang=\"es\"><p:s><p:i xmlns:p=\"up\"><p:j/></p:i></p:s></r>";
        let doc = parse(text).unwrap();
        let apex = doc
            .descendants()
            .find(|n| n.has_tag_name(("up", "i")))
            .unwrap();
        let out = String::from_utf8(canonicalize_subtree(&doc, apex)).unwrap();
        assert_eq!(
            out,
            "<p:i xmlns=\"ud\" xmlns:p=\"up\" xml:lang=\"es\"><p:j></p:j></p:i>"
        );
    }

    #[test]
    fn undeclared_default_namespace_is_rendered_once() {
        let text = "<r xmlns=\"ud\"><a xmlns=\"\"><b/></a></r>";
        assert_eq!(
            doc_c14n(text),
            "<r xmlns=\"ud\"><a xmlns=\"\"><b></b></a></r>"
        );
    }
}
