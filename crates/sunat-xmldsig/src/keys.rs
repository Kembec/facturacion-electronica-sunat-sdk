//! Certificate and private key loading from PEM text.
//!
//! The certificate PEM may also carry the private key (a single combined file) and the key PEM
//! may also carry certificates: each side only reads the blocks it needs.

use base64::{engine::general_purpose::STANDARD, Engine};
use x509_cert::der::Decode;
use x509_cert::Certificate;
use zeroize::Zeroizing;

use crate::rsa_sha1::{PrivateKey, PublicKey};
use crate::SignError;

/// One certificate as it goes into `ds:X509Certificate`.
pub(crate) struct CertificateBlock {
    /// Base64 text as it goes into the signature: the PEM body lines, trimmed and concatenated.
    pub(crate) base64: String,
    pub(crate) public_key: PublicKey,
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
pub(crate) fn certificate_public_key(der: &[u8]) -> Result<PublicKey, SignError> {
    let cert = Certificate::from_der(der)
        .map_err(|_| SignError::InvalidCertificate("certificate does not parse as X.509"))?;
    let spki = &cert.tbs_certificate.subject_public_key_info;
    if spki.algorithm.oid != pkcs1::ALGORITHM_OID {
        return Err(SignError::InvalidCertificate("certificate key is not RSA"));
    }
    spki.subject_public_key
        .as_bytes()
        .and_then(PublicKey::from_pkcs1_der)
        .ok_or(SignError::InvalidCertificate("certificate key is not RSA"))
}

/// The RSA private key of `key_pem` (`PRIVATE KEY` PKCS#8 or `RSA PRIVATE KEY` PKCS#1).
pub(crate) fn load_private_key(key_pem: &str) -> Result<PrivateKey, SignError> {
    if !pem_bodies(key_pem, "ENCRYPTED PRIVATE KEY").is_empty() {
        return Err(SignError::InvalidPrivateKey(
            "encrypted keys are not supported",
        ));
    }
    if let Some(body) = pem_bodies(key_pem, "PRIVATE KEY").into_iter().next() {
        let der = decode_key_body(&body)?;
        return PrivateKey::from_pkcs8_der(&der)
            .ok_or(SignError::InvalidPrivateKey("PKCS#8 key is not a valid RSA key"));
    }
    if let Some(body) = pem_bodies(key_pem, "RSA PRIVATE KEY").into_iter().next() {
        let der = decode_key_body(&body)?;
        return PrivateKey::from_pkcs1_der(&der)
            .ok_or(SignError::InvalidPrivateKey("PKCS#1 key is not a valid RSA key"));
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

    const CERT: &str = include_str!("../../../tests/vectors/test-cert/test-cert.pem");
    const KEY: &str = include_str!("../../../tests/vectors/test-cert/test-key.pem");

    /// The test key as `RSA PRIVATE KEY` (PKCS#1): the inner key of its PKCS#8 wrapper.
    fn pkcs1_key() -> String {
        let der = STANDARD.decode(pem_bodies(KEY, "PRIVATE KEY")[0].as_bytes()).unwrap();
        let inner = pkcs8::PrivateKeyInfo::try_from(der.as_slice()).unwrap().private_key.to_vec();
        let body = STANDARD.encode(inner);
        let lines: Vec<&str> = body.as_bytes().chunks(64).map(|c| std::str::from_utf8(c).unwrap()).collect();
        format!("-----BEGIN RSA PRIVATE KEY-----\n{}\n-----END RSA PRIVATE KEY-----\n", lines.join("\n"))
    }

    /// The closed list of combined-PEM differences that load to the same certificate and key:
    /// block order, LF or CRLF, text outside blocks (`Bag Attributes`, blank lines), indentation
    /// and trailing spaces on body lines, and the key as PKCS#8 or PKCS#1.
    #[test]
    fn combined_pem_layouts_load_the_same_certificate_and_key() {
        let baseline = load_private_key(KEY).unwrap().sign_sha1(b"signed info").unwrap();
        let pkcs1 = pkcs1_key();
        let indented: String = KEY.lines().map(|l| format!("  {l} \n")).collect();
        let layouts = [
            format!("{CERT}{KEY}"),
            format!("{KEY}\n\n{CERT}"),
            format!("{CERT}{KEY}").replace('\n', "\r\n"),
            format!("Bag Attributes\n    localKeyID: 01\n{CERT}Bag Attributes\n{KEY}"),
            format!("{CERT}{indented}"),
            format!("{CERT}{pkcs1}"),
            format!("{pkcs1}{CERT}").replace('\n', "\r\n"),
        ];
        for (i, pem) in layouts.iter().enumerate() {
            let certificates = load_certificates(pem).unwrap();
            let key = load_private_key(pem).unwrap();
            assert_eq!(certificates.len(), 1, "layout {i}");
            assert!(*key.public_key() == certificates[0].public_key, "layout {i}");
            assert_eq!(key.sign_sha1(b"signed info").unwrap(), baseline, "layout {i}");
        }
    }

    #[test]
    fn what_is_not_normalized_is_an_error() {
        let encrypted = "-----BEGIN ENCRYPTED PRIVATE KEY-----\nQUJD\n-----END ENCRYPTED PRIVATE KEY-----\n";
        assert!(matches!(load_private_key(&format!("{CERT}{encrypted}")), Err(SignError::InvalidPrivateKey(_))));
        let legacy = KEY.replacen("-----\n", "-----\nProc-Type: 4,ENCRYPTED\nDEK-Info: AES-128-CBC,00\n\n", 1);
        assert!(matches!(load_private_key(&legacy), Err(SignError::InvalidPrivateKey(_))));
        assert!(matches!(load_private_key(CERT), Err(SignError::InvalidPrivateKey(_))));
        assert!(matches!(load_certificates(KEY), Err(SignError::InvalidCertificate(_))));
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
