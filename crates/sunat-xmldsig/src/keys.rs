//! Certificate and private key loading from PEM text.
//!
//! The certificate PEM may also carry the private key (a single combined file) and the key PEM
//! may also carry certificates: each side only reads the blocks it needs.

use base64::{engine::general_purpose::STANDARD, Engine};
use rsa::pkcs1::DecodeRsaPrivateKey;
use rsa::pkcs8::{DecodePrivateKey, DecodePublicKey};
use rsa::{RsaPrivateKey, RsaPublicKey};
use x509_cert::der::{Decode, Encode};
use x509_cert::Certificate;
use zeroize::Zeroizing;

use crate::SignError;

/// One certificate as it goes into `ds:X509Certificate`.
pub(crate) struct CertificateBlock {
    /// Base64 text as it goes into the signature: the PEM body lines, trimmed and concatenated.
    pub(crate) base64: String,
    pub(crate) public_key: RsaPublicKey,
}

/// Bodies of the PEM blocks labelled `label`, in order. Each body line is trimmed and the lines
/// are concatenated without separators.
fn pem_bodies(pem: &str, label: &str) -> Vec<Zeroizing<String>> {
    let begin = format!("-----BEGIN {label}-----");
    let end = format!("-----END {label}-----");
    let mut bodies = Vec::new();
    let mut current: Option<Zeroizing<String>> = None;
    for line in pem.split('\n') {
        let line = line.trim();
        match current.as_mut() {
            None if line == begin => current = Some(Zeroizing::new(String::new())),
            None => {}
            Some(_) if line == end => bodies.extend(current.take()),
            Some(body) => body.push_str(line),
        }
    }
    bodies
}

/// Every `CERTIFICATE` block of `cert_pem`, in order. At least one is required.
pub(crate) fn load_certificates(cert_pem: &str) -> Result<Vec<CertificateBlock>, SignError> {
    let bodies = pem_bodies(cert_pem, "CERTIFICATE");
    if bodies.is_empty() {
        return Err(SignError::InvalidCertificate(
            "no CERTIFICATE block in the PEM",
        ));
    }
    bodies
        .iter()
        .map(|body| {
            let der = STANDARD
                .decode(body.as_bytes())
                .map_err(|_| SignError::InvalidCertificate("certificate body is not base64"))?;
            Ok(CertificateBlock {
                base64: body.to_string(),
                public_key: certificate_public_key(&der)?,
            })
        })
        .collect()
}

/// RSA public key of a DER X.509 certificate.
pub(crate) fn certificate_public_key(der: &[u8]) -> Result<RsaPublicKey, SignError> {
    let cert = Certificate::from_der(der)
        .map_err(|_| SignError::InvalidCertificate("certificate does not parse as X.509"))?;
    let spki = cert
        .tbs_certificate
        .subject_public_key_info
        .to_der()
        .map_err(|_| SignError::InvalidCertificate("certificate public key does not encode"))?;
    RsaPublicKey::from_public_key_der(&spki)
        .map_err(|_| SignError::InvalidCertificate("certificate key is not RSA"))
}

/// The RSA private key of `key_pem` (`PRIVATE KEY` PKCS#8 or `RSA PRIVATE KEY` PKCS#1).
pub(crate) fn load_private_key(key_pem: &str) -> Result<RsaPrivateKey, SignError> {
    if !pem_bodies(key_pem, "ENCRYPTED PRIVATE KEY").is_empty() {
        return Err(SignError::InvalidPrivateKey(
            "encrypted keys are not supported",
        ));
    }
    if let Some(body) = pem_bodies(key_pem, "PRIVATE KEY").into_iter().next() {
        let der = decode_key_body(&body)?;
        return RsaPrivateKey::from_pkcs8_der(&der)
            .map_err(|_| SignError::InvalidPrivateKey("PKCS#8 key is not a valid RSA key"));
    }
    if let Some(body) = pem_bodies(key_pem, "RSA PRIVATE KEY").into_iter().next() {
        let der = decode_key_body(&body)?;
        return RsaPrivateKey::from_pkcs1_der(&der)
            .map_err(|_| SignError::InvalidPrivateKey("PKCS#1 key is not a valid RSA key"));
    }
    Err(SignError::InvalidPrivateKey(
        "no PRIVATE KEY or RSA PRIVATE KEY block in the PEM",
    ))
}

fn decode_key_body(body: &str) -> Result<Zeroizing<Vec<u8>>, SignError> {
    STANDARD
        .decode(body.as_bytes())
        .map(Zeroizing::new)
        .map_err(|_| SignError::InvalidPrivateKey("key body is not base64 (encrypted PEM?)"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pem_bodies_trims_and_joins_lines() {
        let pem =
            "junk\n-----BEGIN CERTIFICATE-----\r\nQUJD\r\n REVG \n-----END CERTIFICATE-----\n";
        let bodies = pem_bodies(pem, "CERTIFICATE");
        assert_eq!(bodies.len(), 1);
        assert_eq!(bodies[0].as_str(), "QUJDREVG");
    }

    #[test]
    fn missing_blocks_are_errors() {
        assert!(matches!(
            load_certificates("x"),
            Err(SignError::InvalidCertificate(_))
        ));
        assert!(matches!(
            load_private_key("x"),
            Err(SignError::InvalidPrivateKey(_))
        ));
    }
}
