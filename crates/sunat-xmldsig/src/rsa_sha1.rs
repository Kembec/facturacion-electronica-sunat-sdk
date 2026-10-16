//! RSASSA-PKCS1-v1_5 with SHA-1 (RFC 8017 §8.2), the only RSA scheme of the SUNAT profile, on
//! crypto-bigint's constant-time Montgomery arithmetic.
//!
//! It replaces the `rsa` crate: its private-key exponentiation leaks timing (RUSTSEC-2023-0071)
//! and no release is patched (`patched = []` in the advisory, 0.9.10 and 0.10.0-rc.19 included).
//! Here the private exponentiation always runs over the full precision of the modulus with
//! constant-time window lookups, so its time depends on the key size only. PKCS#1 v1.5 signing
//! is deterministic: no randomness is drawn, and the output equals any other conforming signer's.

use crypto_bigint::modular::{BoxedMontyForm, BoxedMontyParams};
use crypto_bigint::{BoxedUint, Odd};
use pkcs1::der::Decode;
use sha1::{Digest, Sha1};
use zeroize::Zeroizing;

use crate::SignError;

/// DER prefix of the SHA-1 `DigestInfo` (RFC 8017 §9.2, note 1).
const SHA1_DIGEST_INFO: [u8; 15] = [
    0x30, 0x21, 0x30, 0x09, 0x06, 0x05, 0x2b, 0x0e, 0x03, 0x02, 0x1a, 0x05, 0x00, 0x04, 0x14,
];
/// `rsaEncryption` (RFC 8017 appendix C). pkcs1 exports it only behind its `pkcs8` feature.
pub(crate) const RSA_ENCRYPTION_OID: pkcs8::ObjectIdentifier =
    pkcs8::ObjectIdentifier::new_unwrap("1.2.840.113549.1.1.1");
/// Same bounds the `rsa` crate applied, so the keys accepted before are accepted now.
const MAX_MODULUS_BITS: usize = 4096;
const MAX_PUBLIC_EXPONENT: u64 = (1 << 33) - 1;

/// An RSA public key `(n, e)`. Equality compares both, as the key/certificate check needs.
#[derive(Clone)]
pub(crate) struct PublicKey {
    /// Big-endian modulus without leading zeros; its length is `k` of RFC 8017.
    modulus: Vec<u8>,
    exponent: Vec<u8>,
    params: BoxedMontyParams,
}

impl PartialEq for PublicKey {
    fn eq(&self, other: &Self) -> bool {
        self.modulus == other.modulus && self.exponent == other.exponent
    }
}

impl PublicKey {
    /// From the DER `RSAPublicKey` (PKCS#1) inside a `SubjectPublicKeyInfo`.
    pub(crate) fn from_pkcs1_der(der: &[u8]) -> Option<Self> {
        let key = pkcs1::RsaPublicKey::from_der(der).ok()?;
        Self::new(key.modulus.as_bytes(), key.public_exponent.as_bytes())
    }

    fn new(modulus: &[u8], exponent: &[u8]) -> Option<Self> {
        let modulus = trim(modulus);
        let exponent = trim(exponent);
        let k = modulus.len();
        if k < SHA1_DIGEST_INFO.len() + 20 + 11 || k * 8 > MAX_MODULUS_BITS + 7 || exponent.len() > 8 {
            return None;
        }
        let e = exponent.iter().fold(0u64, |acc, b| (acc << 8) | u64::from(*b));
        if !(2..=MAX_PUBLIC_EXPONENT).contains(&e) {
            return None;
        }
        let n = BoxedUint::from_be_slice(modulus, precision(k)).ok()?;
        if n.bits_vartime() as usize > MAX_MODULUS_BITS {
            return None;
        }
        let params = BoxedMontyParams::new_vartime(Odd::new(n).into_option()?);
        Some(Self { modulus: modulus.to_vec(), exponent: exponent.to_vec(), params })
    }

    fn len(&self) -> usize {
        self.modulus.len()
    }

    fn precision(&self) -> u32 {
        precision(self.len())
    }

    /// `s^e mod n` as `k` big-endian bytes; `None` if `s` is not below `n`.
    fn public_op(&self, signature: &[u8]) -> Option<Vec<u8>> {
        if signature.len() != self.len() || signature >= self.modulus.as_slice() {
            return None;
        }
        let s = BoxedUint::from_be_slice(signature, self.precision()).ok()?;
        let e = BoxedUint::from_be_slice(&self.exponent, self.precision()).ok()?;
        // The public exponent is public: bounding the loop to its bits leaks nothing.
        let m = BoxedMontyForm::new(s, &self.params).pow_bounded_exp(&e, e.bits_vartime()).retrieve();
        Some(to_k_bytes(&m, self.len()))
    }

    /// Whether `signature` is a valid RSASSA-PKCS1-v1_5 SHA-1 signature of `message`.
    pub(crate) fn verify_sha1(&self, message: &[u8], signature: &[u8]) -> bool {
        match (self.public_op(signature), encode_sha1(message, self.len())) {
            (Some(recovered), Some(expected)) => recovered == expected,
            _ => false,
        }
    }
}

/// An RSA private key: its public half and the private exponent, zeroized on drop.
pub(crate) struct PrivateKey {
    public: PublicKey,
    exponent: Zeroizing<BoxedUint>,
}

impl PrivateKey {
    /// From a DER `RSAPrivateKey` (PKCS#1, `RSA PRIVATE KEY`).
    pub(crate) fn from_pkcs1_der(der: &[u8]) -> Option<Self> {
        let key = pkcs1::RsaPrivateKey::from_der(der).ok()?;
        let public = PublicKey::new(key.modulus.as_bytes(), key.public_exponent.as_bytes())?;
        let d = trim(key.private_exponent.as_bytes());
        if d.is_empty() || d.len() > public.len() {
            return None;
        }
        let exponent = Zeroizing::new(BoxedUint::from_be_slice(d, public.precision()).ok()?);
        Some(Self { public, exponent })
    }

    /// From a DER `PrivateKeyInfo` (PKCS#8, `PRIVATE KEY`) whose algorithm is rsaEncryption.
    pub(crate) fn from_pkcs8_der(der: &[u8]) -> Option<Self> {
        let info = pkcs8::PrivateKeyInfo::try_from(der).ok()?;
        if info.algorithm.oid != RSA_ENCRYPTION_OID {
            return None;
        }
        Self::from_pkcs1_der(info.private_key)
    }

    pub(crate) fn public_key(&self) -> &PublicKey {
        &self.public
    }

    /// RSASSA-PKCS1-v1_5 SHA-1 signature of `message`, `k` bytes. The result is checked with the
    /// public key before it is returned, so a key whose private exponent does not match its
    /// modulus fails here instead of producing a signature SUNAT would reject.
    pub(crate) fn sign_sha1(&self, message: &[u8]) -> Result<Vec<u8>, SignError> {
        let k = self.public.len();
        let encoded = encode_sha1(message, k).ok_or(SignError::Crypto("RSA key too small for SHA-1"))?;
        let m = BoxedUint::from_be_slice(&encoded, self.public.precision())
            .map_err(|_| SignError::Crypto("RSA-SHA1 signing failed"))?;
        // `pow` walks every bit of the exponent's precision (that of the modulus), never d's length.
        let s = BoxedMontyForm::new(m, &self.public.params).pow(&self.exponent).retrieve();
        let signature = to_k_bytes(&s, k);
        if self.public.public_op(&signature).as_deref() != Some(encoded.as_slice()) {
            return Err(SignError::Crypto("RSA-SHA1 signing failed"));
        }
        Ok(signature)
    }
}

/// EMSA-PKCS1-v1_5 encoding of SHA-1(`message`) into `k` bytes (RFC 8017 §9.2).
fn encode_sha1(message: &[u8], k: usize) -> Option<Vec<u8>> {
    let t_len = SHA1_DIGEST_INFO.len() + 20;
    if k < t_len + 11 {
        return None;
    }
    let mut em = Vec::with_capacity(k);
    em.extend_from_slice(&[0x00, 0x01]);
    em.resize(k - t_len - 1, 0xff);
    em.push(0x00);
    em.extend_from_slice(&SHA1_DIGEST_INFO);
    em.extend_from_slice(&Sha1::digest(message));
    Some(em)
}

fn trim(bytes: &[u8]) -> &[u8] {
    let start = bytes.iter().position(|b| *b != 0).unwrap_or(bytes.len());
    &bytes[start..]
}

fn precision(k: usize) -> u32 {
    u32::try_from(k * 8).unwrap_or(u32::MAX)
}

/// The `k` least significant big-endian bytes of `value` (which is below `n`, so nothing is lost).
fn to_k_bytes(value: &BoxedUint, k: usize) -> Vec<u8> {
    let bytes = value.to_be_bytes();
    let mut out = vec![0u8; k];
    let take = bytes.len().min(k);
    out[k - take..].copy_from_slice(&bytes[bytes.len() - take..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encoding_is_the_rfc_8017_layout() {
        let em = encode_sha1(b"abc", 128).unwrap();
        assert_eq!(em.len(), 128);
        assert_eq!(&em[..2], &[0x00, 0x01]);
        assert!(em[2..128 - 36].iter().all(|b| *b == 0xff));
        assert_eq!(em[128 - 36], 0x00);
        assert_eq!(&em[128 - 35..128 - 20], &SHA1_DIGEST_INFO);
        // SHA-1("abc"), FIPS 180-2 appendix A.
        assert_eq!(em[128 - 20..][..4], [0xa9, 0x99, 0x3e, 0x36]);
        assert!(encode_sha1(b"abc", 45).is_none());
    }

    #[test]
    fn out_of_range_public_keys_are_rejected() {
        assert!(PublicKey::new(&[0xff; 64], &[0x01, 0x00, 0x01]).is_some());
        assert!(PublicKey::new(&[0xfe; 64], &[0x01, 0x00, 0x01]).is_none(), "even modulus");
        assert!(PublicKey::new(&[0xff; 64], &[0x01]).is_none(), "e = 1");
        assert!(PublicKey::new(&[0xff; 513], &[0x01, 0x00, 0x01]).is_none(), "over 4096 bits");
        assert!(PublicKey::new(&[0xff; 40], &[0x03]).is_none(), "below RFC 8017 minimum");
    }
}
