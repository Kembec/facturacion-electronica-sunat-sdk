//! Enveloped XML-DSig for SUNAT documents, in the profile of SUNAT's programmer's manual and
//! UBL 2.1 guides, reproducing the compatibility vectors (`tests/vectors/*.signed.xml`):
//!
//! - RSA-SHA1 signature, SHA1 digest, inclusive Canonical XML 1.0 without comments;
//! - a single `Reference URI=""` with the enveloped-signature transform;
//! - `ds:Signature Id="…"` (default [`DEFAULT_SIGNATURE_ID`]) appended to the first
//!   `ext:ExtensionContent` with no text content (or to the root element when there is none);
//! - `KeyInfo/X509Data/X509Certificate` with every certificate of the certificate PEM.
//!
//! Signing is deterministic: for the same input document and key, `DigestValue` and
//! `SignatureValue` are byte-identical to the vectors'. The input bytes are kept as they are and
//! the signature is spliced in, so any document C14N-equivalent to a vector gets the same values.
//!
//! Sans-IO: this crate only transforms bytes. Private key material is never formatted into errors,
//! logs or `Debug` output.

mod c14n;
mod error;
mod keys;
mod sign;
mod verify;
mod xml;

pub use error::SignError;
pub use sign::{sign, sign_with_id};
pub use verify::{verify, Verification};

/// `Id` attribute of the `ds:Signature` element when none is given: the one the compatibility
/// vectors use, and the one `sunat-ubl` references from `cac:Signature` by default.
pub const DEFAULT_SIGNATURE_ID: &str = "signatureKG";

/// Inclusive Canonical XML 1.0 (without comments) of a whole UTF-8 document.
pub fn canonicalize(xml: &[u8]) -> Result<Vec<u8>, SignError> {
    let source = xml::source(xml)?;
    let doc = xml::parse(source.text)?;
    Ok(c14n::canonicalize_document(&doc, None))
}
