//! Verification of the first `ds:Signature` of a document within the SUNAT signing profile:
//! inclusive C14N 1.0, RSA-SHA1, one `Reference` to the whole document (`URI=""`) with the
//! enveloped-signature transform and a SHA1 digest.
//!
//! It proves integrity against the certificate embedded in `KeyInfo`. Whether that certificate is
//! trusted (issuer, validity, owner's RUC) is the caller's decision: it is returned for that.

use base64::{engine::general_purpose::STANDARD, Engine};
use roxmltree::Node;
use sha1::{Digest, Sha1};

use crate::xml::{self, ALG_C14N, ALG_ENVELOPED, ALG_RSA_SHA1, ALG_SHA1, DSIG_NS};
use crate::{c14n, keys, SignError};

/// Outcome of [`verify`] for a structurally valid signature.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Verification {
    /// The document, minus the signature, still hashes to `DigestValue`.
    pub digest_matches: bool,
    /// `SignatureValue` is valid over the canonical `SignedInfo` for the embedded certificate.
    pub signature_matches: bool,
    /// `Id` of the `ds:Signature` element, if any.
    pub signature_id: Option<String>,
    /// DER of the first `X509Certificate`, the one the signature was checked against.
    pub certificate_der: Vec<u8>,
}

impl Verification {
    /// Both the digest and the signature hold.
    pub fn is_valid(&self) -> bool {
        self.digest_matches && self.signature_matches
    }
}

/// Verifies the first `ds:Signature` of `xml`.
///
/// A tampered document or signature returns `Ok` with [`Verification::is_valid`] false; a missing
/// or malformed signature, or one outside the supported profile, returns an error.
pub fn verify(xml: &[u8]) -> Result<Verification, SignError> {
    let source = xml::source(xml)?;
    let doc = xml::parse(source.text)?;
    let signature = doc
        .descendants()
        .find(|n| xml::is_element(*n, DSIG_NS, "Signature"))
        .ok_or(SignError::SignatureNotFound)?;
    let signed_info = xml::dsig_child(signature, "SignedInfo")
        .ok_or(SignError::MalformedSignature("missing SignedInfo"))?;
    let expected_digest = check_profile(signed_info)?;

    let signature_value = decode_base64(
        xml::dsig_child(signature, "SignatureValue"),
        "missing SignatureValue",
    )?;
    let certificate_der = decode_base64(
        xml::dsig_child(signature, "KeyInfo")
            .and_then(|k| xml::dsig_child(k, "X509Data"))
            .and_then(|d| xml::dsig_child(d, "X509Certificate")),
        "missing KeyInfo/X509Data/X509Certificate",
    )?;
    let public_key = keys::certificate_public_key(&certificate_der)?;

    let digest = Sha1::digest(c14n::canonicalize_document(&doc, Some(signature.id())));
    let digest_matches = digest.as_slice() == expected_digest.as_slice();

    let canonical_signed_info = c14n::canonicalize_subtree(&doc, signed_info);
    let signature_matches = public_key.verify_sha1(&canonical_signed_info, &signature_value);

    Ok(Verification {
        digest_matches,
        signature_matches,
        signature_id: signature.attribute("Id").map(str::to_string),
        certificate_der,
    })
}

/// Checks `SignedInfo` against the supported profile and returns the decoded `DigestValue`.
fn check_profile(signed_info: Node) -> Result<Vec<u8>, SignError> {
    expect_algorithm(signed_info, "CanonicalizationMethod", ALG_C14N)?;
    expect_algorithm(signed_info, "SignatureMethod", ALG_RSA_SHA1)?;

    let mut references = signed_info
        .children()
        .filter(|c| xml::is_element(*c, DSIG_NS, "Reference"));
    let reference = references
        .next()
        .ok_or(SignError::MalformedSignature("missing Reference"))?;
    if references.next().is_some() {
        return Err(SignError::UnsupportedAlgorithm(
            "more than one Reference".into(),
        ));
    }
    if let Some(uri) = reference.attribute("URI").filter(|u| !u.is_empty()) {
        return Err(SignError::UnsupportedAlgorithm(format!(
            "Reference URI {uri:?}"
        )));
    }
    let transforms = xml::dsig_child(reference, "Transforms")
        .map(|t| {
            t.children()
                .filter(|c| xml::is_element(*c, DSIG_NS, "Transform"))
                .collect()
        })
        .unwrap_or_else(Vec::new);
    if !transforms
        .iter()
        .any(|t| t.attribute("Algorithm") == Some(ALG_ENVELOPED))
    {
        return Err(SignError::UnsupportedAlgorithm(
            "Reference without enveloped transform".into(),
        ));
    }
    for transform in transforms {
        let algorithm = transform.attribute("Algorithm").unwrap_or("");
        if algorithm != ALG_ENVELOPED && algorithm != ALG_C14N {
            return Err(SignError::UnsupportedAlgorithm(format!(
                "Transform {algorithm}"
            )));
        }
    }
    expect_algorithm(reference, "DigestMethod", ALG_SHA1)?;
    decode_base64(
        xml::dsig_child(reference, "DigestValue"),
        "missing DigestValue",
    )
}

fn expect_algorithm(parent: Node, element: &'static str, expected: &str) -> Result<(), SignError> {
    let node = xml::dsig_child(parent, element).ok_or(SignError::MalformedSignature(element))?;
    match node.attribute("Algorithm") {
        Some(found) if found == expected => Ok(()),
        found => Err(SignError::UnsupportedAlgorithm(format!(
            "{element} {}",
            found.unwrap_or("")
        ))),
    }
}

/// Base64 text content of an element, whitespace (line breaks) ignored.
fn decode_base64(node: Option<Node>, missing: &'static str) -> Result<Vec<u8>, SignError> {
    let node = node.ok_or(SignError::MalformedSignature(missing))?;
    let compact: String = xml::text_content(node)
        .chars()
        .filter(|c| !c.is_ascii_whitespace())
        .collect();
    STANDARD
        .decode(compact.as_bytes())
        .map_err(|_| SignError::MalformedSignature("value is not base64"))
}
