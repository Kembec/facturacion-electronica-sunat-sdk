use std::fmt;

/// Errors of [`crate::sign`] and [`crate::verify`].
///
/// Messages never carry key material or document content beyond parser positions.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SignError {
    /// The document is not UTF-8 or declares another encoding.
    UnsupportedEncoding(String),
    /// The document is not well-formed XML (parser message with position).
    InvalidXml(String),
    /// The certificate PEM has no usable `CERTIFICATE` block, or it does not parse.
    InvalidCertificate(&'static str),
    /// The key PEM has no usable private key, or it is encrypted or not RSA.
    InvalidPrivateKey(&'static str),
    /// The private key does not belong to the first certificate of the certificate PEM.
    KeyCertificateMismatch,
    /// No `ds:Signature` element in the document.
    SignatureNotFound,
    /// The `ds:Signature` element lacks a required part or has an invalid value.
    MalformedSignature(&'static str),
    /// An algorithm or reference outside the SUNAT profile.
    UnsupportedAlgorithm(String),
    /// The cryptographic operation failed.
    Crypto(&'static str),
}

impl fmt::Display for SignError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedEncoding(enc) => write!(f, "unsupported document encoding: {enc}"),
            Self::InvalidXml(msg) => write!(f, "invalid XML: {msg}"),
            Self::InvalidCertificate(why) => write!(f, "invalid certificate: {why}"),
            Self::InvalidPrivateKey(why) => write!(f, "invalid private key: {why}"),
            Self::KeyCertificateMismatch => {
                write!(f, "the private key does not match the first certificate")
            }
            Self::SignatureNotFound => write!(f, "no ds:Signature element found"),
            Self::MalformedSignature(why) => write!(f, "malformed signature: {why}"),
            Self::UnsupportedAlgorithm(what) => write!(f, "unsupported: {what}"),
            Self::Crypto(why) => write!(f, "cryptographic failure: {why}"),
        }
    }
}

impl std::error::Error for SignError {}
