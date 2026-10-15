//! Minimal XML writer that emits canonical form directly (Canonical XML 1.0): no whitespace
//! between elements, empty elements as a start and an end tag, and the escapes C14N uses. The
//! bytes after the declaration are therefore their own canonical form.

pub(crate) struct XmlWriter {
    out: String,
    open: Vec<&'static str>,
}

impl XmlWriter {
    pub(crate) fn new() -> Self {
        Self { out: String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n"), open: Vec::new() }
    }

    /// Opens `name` with `attrs` in the given order (callers pass them already in C14N order).
    pub(crate) fn open(&mut self, name: &'static str, attrs: &[(&str, &str)]) {
        self.out.push('<');
        self.out.push_str(name);
        for (key, value) in attrs {
            self.out.push(' ');
            self.out.push_str(key);
            self.out.push_str("=\"");
            escape_attribute(&mut self.out, value);
            self.out.push('"');
        }
        self.out.push('>');
        self.open.push(name);
    }

    pub(crate) fn close(&mut self) {
        let name = self.open.pop().expect("close without open");
        self.out.push_str("</");
        self.out.push_str(name);
        self.out.push('>');
    }

    /// `<name attrs>text</name>`.
    pub(crate) fn leaf(&mut self, name: &'static str, attrs: &[(&str, &str)], text: &str) {
        self.open(name, attrs);
        escape_text(&mut self.out, text);
        self.close();
    }

    /// `<name>text</name>` without attributes.
    pub(crate) fn text(&mut self, name: &'static str, text: &str) {
        self.leaf(name, &[], text);
    }

    /// Writes the element only when there is a non-empty value.
    pub(crate) fn text_opt(&mut self, name: &'static str, text: Option<&str>) {
        if let Some(text) = text.filter(|t| !t.is_empty()) {
            self.text(name, text);
        }
    }

    pub(crate) fn finish(self) -> Vec<u8> {
        debug_assert!(self.open.is_empty(), "unclosed elements: {:?}", self.open);
        self.out.into_bytes()
    }
}

fn escape_text(out: &mut String, text: &str) {
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '\r' => out.push_str("&#xD;"),
            _ => out.push(ch),
        }
    }
}

fn escape_attribute(out: &mut String, text: &str) {
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '"' => out.push_str("&quot;"),
            '\t' => out.push_str("&#x9;"),
            '\n' => out.push_str("&#xA;"),
            '\r' => out.push_str("&#xD;"),
            _ => out.push(ch),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_like_canonical_xml() {
        let mut w = XmlWriter::new();
        w.leaf("a", &[("k", "x\"<&\n")], "1 < 2 & 3 > 0\r");
        let out = String::from_utf8(w.finish()).unwrap();
        assert!(out.ends_with("<a k=\"x&quot;&lt;&amp;&#xA;\">1 &lt; 2 &amp; 3 &gt; 0&#xD;</a>"));
    }

    #[test]
    fn empty_element_has_an_end_tag() {
        let mut w = XmlWriter::new();
        w.open("e", &[]);
        w.close();
        assert!(String::from_utf8(w.finish()).unwrap().ends_with("<e></e>"));
    }
}
