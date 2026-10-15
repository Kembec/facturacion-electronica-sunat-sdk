//! Signing.
//!
//! The document is hashed before the signature exists; the signature goes as the last child of
//! the first `ext:ExtensionContent` whose text content is blank, the canonical `SignedInfo` is
//! signed once it sits in the document, and every certificate of the PEM goes into `X509Data`.
//! The input bytes are kept and the signature is spliced in; `xmlns:ds` is declared on the
//! signature only when no ancestor already binds `ds` to the XML-DSig namespace.

use base64::{engine::general_purpose::STANDARD, Engine};
use rand_core::OsRng;
use roxmltree::{Document, Node};
use rsa::pkcs1v15::SigningKey;
use rsa::signature::{RandomizedSigner, SignatureEncoding};
use sha1::{Digest, Sha1};

use crate::keys::{self, CertificateBlock};
use crate::xml::{self, ALG_C14N, ALG_ENVELOPED, ALG_RSA_SHA1, ALG_SHA1, DSIG_NS, EXT_NS};
use crate::{c14n, SignError, DEFAULT_SIGNATURE_ID};

/// Signs `xml` (UTF-8) with an enveloped XML-DSig whose `Id` is [`DEFAULT_SIGNATURE_ID`].
///
/// `cert_pem` holds the signing certificate first (more `CERTIFICATE` blocks are copied into
/// `X509Data` in order); `key_pem` holds its RSA private key, unencrypted (`PRIVATE KEY` or
/// `RSA PRIVATE KEY`). Both may be the same combined PEM text.
pub fn sign(xml: &[u8], cert_pem: &str, key_pem: &str) -> Result<Vec<u8>, SignError> {
    sign_with_id(xml, cert_pem, key_pem, DEFAULT_SIGNATURE_ID)
}

/// [`sign`] with another `Id` for `ds:Signature` (it must match the document's `cac:Signature`
/// reference). The `Id` must be an XML name without `"` or `&`.
pub fn sign_with_id(xml: &[u8], cert_pem: &str, key_pem: &str, signature_id: &str) -> Result<Vec<u8>, SignError> {
    if !is_plain_id(signature_id) {
        return Err(SignError::MalformedSignature("signature id must be a non-empty XML name"));
    }
    let certificates = keys::load_certificates(cert_pem)?;
    let key = keys::load_private_key(key_pem)?;
    if key.to_public_key() != certificates[0].public_key {
        return Err(SignError::KeyCertificateMismatch);
    }

    let source = xml::source(xml)?;
    let doc = xml::parse(source.text)?;
    let digest = STANDARD.encode(Sha1::digest(c14n::canonicalize_document(&doc, None)));

    let target = signature_parent(&doc);
    let splice = Splice::at(source.text, target);
    let declare_ds = !xml::prefix_bound_to(target, "ds", DSIG_NS);
    let head = signature_head(declare_ds, signature_id, &digest);
    let tail = signature_tail(&certificates);

    let unsigned = splice.assemble(&head, "", &tail);
    let signed_info = canonical_signed_info(&unsigned, splice.signature_offset())?;
    let signature = SigningKey::<Sha1>::new(key)
        .try_sign_with_rng(&mut OsRng, &signed_info)
        .map_err(|_| SignError::Crypto("RSA-SHA1 signing failed"))?;
    let value = STANDARD.encode(signature.to_bytes());

    let mut out = String::with_capacity(xml.len() + head.len() + tail.len() + value.len());
    out.push_str(source.bom);
    out.push_str(&splice.assemble(&head, &value, &tail));
    Ok(out.into_bytes())
}

/// The first blank `ext:ExtensionContent`, else the root element.
fn signature_parent<'a, 'i>(doc: &'a Document<'i>) -> Node<'a, 'i> {
    doc.descendants()
        .find(|n| {
            xml::is_element(*n, EXT_NS, "ExtensionContent")
                && xml::php_trim(&xml::text_content(*n)).is_empty()
        })
        .unwrap_or_else(|| doc.root_element())
}

/// Where the signature goes in the source text: right before the parent's end tag, or in place
/// of the `/>` of an empty-element tag.
struct Splice<'i> {
    text: &'i str,
    cut: usize,
    /// For an empty-element tag: (end of the tag, qname to close it with).
    self_closing: Option<(usize, &'i str)>,
}

impl<'i> Splice<'i> {
    fn at(text: &'i str, parent: Node<'_, 'i>) -> Self {
        let range = parent.range();
        let element = &text[range.clone()];
        if element.ends_with("/>") {
            let qname = xml::element_qname(text, parent);
            return Self {
                text,
                cut: range.end - 2,
                self_closing: Some((range.end, qname)),
            };
        }
        // The last `<` of an element is the start of its end tag.
        let end_tag = element.rfind('<').unwrap_or(element.len());
        Self {
            text,
            cut: range.start + end_tag,
            self_closing: None,
        }
    }

    fn signature_offset(&self) -> usize {
        self.cut + usize::from(self.self_closing.is_some())
    }

    fn assemble(&self, head: &str, value: &str, tail: &str) -> String {
        let mut out =
            String::with_capacity(self.text.len() + head.len() + value.len() + tail.len() + 64);
        out.push_str(&self.text[..self.cut]);
        if self.self_closing.is_some() {
            out.push('>');
        }
        out.push_str(head);
        out.push_str(value);
        out.push_str(tail);
        match self.self_closing {
            Some((end, qname)) => {
                out.push_str("</");
                out.push_str(qname);
                out.push('>');
                out.push_str(&self.text[end..]);
            }
            None => out.push_str(&self.text[self.cut..]),
        }
        out
    }
}

fn is_plain_id(id: &str) -> bool {
    let mut chars = id.chars();
    chars.next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
}

/// From `<ds:Signature …>` through the opening `<ds:SignatureValue>`, with the namespace
/// declaration before `Id` and empty elements as `<x/>`, as in the vectors.
fn signature_head(declare_ds: bool, signature_id: &str, digest: &str) -> String {
    let ns = if declare_ds {
        format!(" xmlns:ds=\"{DSIG_NS}\"")
    } else {
        String::new()
    };
    format!(
        "<ds:Signature{ns} Id=\"{signature_id}\"><ds:SignedInfo>\
         <ds:CanonicalizationMethod Algorithm=\"{ALG_C14N}\"/>\
         <ds:SignatureMethod Algorithm=\"{ALG_RSA_SHA1}\"/>\
         <ds:Reference URI=\"\"><ds:Transforms><ds:Transform Algorithm=\"{ALG_ENVELOPED}\"/></ds:Transforms>\
         <ds:DigestMethod Algorithm=\"{ALG_SHA1}\"/><ds:DigestValue>{digest}</ds:DigestValue>\
         </ds:Reference></ds:SignedInfo><ds:SignatureValue>"
    )
}

fn signature_tail(certificates: &[CertificateBlock]) -> String {
    let mut tail = String::from("</ds:SignatureValue><ds:KeyInfo><ds:X509Data>");
    for cert in certificates {
        tail.push_str("<ds:X509Certificate>");
        tail.push_str(&cert.base64);
        tail.push_str("</ds:X509Certificate>");
    }
    tail.push_str("</ds:X509Data></ds:KeyInfo></ds:Signature>");
    tail
}

/// Canonical `SignedInfo` of the signature that starts at `offset` of the assembled document.
fn canonical_signed_info(assembled: &str, offset: usize) -> Result<Vec<u8>, SignError> {
    let doc = xml::parse(assembled)?;
    let signature = doc
        .descendants()
        .find(|n| n.is_element() && n.range().start == offset)
        .filter(|n| xml::is_element(*n, DSIG_NS, "Signature"))
        .ok_or(SignError::MalformedSignature(
            "inserted signature not found",
        ))?;
    let signed_info = xml::dsig_child(signature, "SignedInfo")
        .ok_or(SignError::MalformedSignature("missing SignedInfo"))?;
    Ok(c14n::canonicalize_subtree(&doc, signed_info))
}
