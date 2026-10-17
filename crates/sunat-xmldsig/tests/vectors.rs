//! Signing and verification against the compatibility vectors (`tests/vectors/` at the workspace
//! root): signing each `<case>.unsigned.xml` with `tests/vectors/test-cert/` gives the
//! `DigestValue` and `SignatureValue` of `<case>.signed.xml`.

use std::path::{Path, PathBuf};
use std::process::Command;

use kembec_sunat_xmldsig::{sign, sign_with_id, verify, SignError, DEFAULT_SIGNATURE_ID};

const VECTOR_SIGNATURE_ID: &str = "signatureKG";

fn vectors_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/vectors")
}

fn read(path: &Path) -> Vec<u8> {
    std::fs::read(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn cert_pem() -> String {
    String::from_utf8(read(&vectors_dir().join("test-cert/test-cert.pem"))).unwrap()
}

fn key_pem() -> String {
    String::from_utf8(read(&vectors_dir().join("test-cert/test-key.pem"))).unwrap()
}

/// Every vector case name (`<case>.unsigned.xml` with its `<case>.signed.xml`), sorted.
fn cases() -> Vec<String> {
    let mut cases: Vec<String> = std::fs::read_dir(vectors_dir())
        .unwrap()
        .filter_map(|entry| {
            let name = entry.unwrap().file_name().into_string().unwrap();
            name.strip_suffix(".unsigned.xml").map(str::to_owned)
        })
        .collect();
    cases.sort();
    assert!(!cases.is_empty(), "no vectors in {}", vectors_dir().display());
    cases
}

fn unsigned(case: &str) -> Vec<u8> {
    read(&vectors_dir().join(format!("{case}.unsigned.xml")))
}

fn expected(case: &str) -> Vec<u8> {
    read(&vectors_dir().join(format!("{case}.signed.xml")))
}

fn sign_case(case: &str) -> Vec<u8> {
    sign_with_id(&unsigned(case), &cert_pem(), &key_pem(), VECTOR_SIGNATURE_ID).unwrap()
}

fn element_text<'a>(xml: &'a str, element: &str) -> &'a str {
    let open = format!("<{element}>");
    let start = xml.find(&open).unwrap_or_else(|| panic!("no {element}")) + open.len();
    &xml[start..start + xml[start..].find('<').unwrap()]
}

#[test]
fn signing_each_vector_gives_its_digest_and_signature_values() {
    for case in cases() {
        let ours = String::from_utf8(sign_case(&case)).unwrap();
        let theirs = String::from_utf8(expected(&case)).unwrap();
        for element in ["ds:DigestValue", "ds:SignatureValue", "ds:X509Certificate"] {
            assert_eq!(
                element_text(&ours, element),
                element_text(&theirs, element),
                "{case}: {element}"
            );
        }
    }
}

#[test]
fn every_vector_and_our_signature_of_it_verify() {
    for case in cases() {
        for (label, xml) in [("vector", expected(&case)), ("ours", sign_case(&case))] {
            let result = verify(&xml).unwrap();
            assert!(result.is_valid(), "{case} ({label}): {result:?}");
            assert_eq!(result.signature_id.as_deref(), Some(VECTOR_SIGNATURE_ID));
            assert!(!result.certificate_der.is_empty());
        }
    }
}

#[test]
fn default_id_is_the_vectors_id() {
    assert_eq!(DEFAULT_SIGNATURE_ID, VECTOR_SIGNATURE_ID);
    let case = &cases()[0];
    let ours = sign(&unsigned(case), &cert_pem(), &key_pem()).unwrap();
    assert_eq!(ours, sign_case(case));
}

#[test]
fn a_changed_document_does_not_verify() {
    for case in cases() {
        let signed = String::from_utf8(expected(&case)).unwrap();
        let tampered = signed.replacen("</cbc:ID>", "9</cbc:ID>", 1);
        assert_ne!(tampered, signed, "{case}");
        let result = verify(tampered.as_bytes()).unwrap();
        assert!(!result.digest_matches, "{case}");
        assert!(result.signature_matches, "{case}: SignedInfo was not touched");
        assert!(!result.is_valid(), "{case}");
    }
}

#[test]
fn a_changed_digest_or_signature_value_does_not_verify() {
    let signed = String::from_utf8(expected(&cases()[0])).unwrap();
    let digest = element_text(&signed, "ds:DigestValue").to_string();
    let forged = signed.replacen(&digest, "AAAAAAAAAAAAAAAAAAAAAAAAAAA=", 1);
    let result = verify(forged.as_bytes()).unwrap();
    assert!(!result.digest_matches && !result.signature_matches);

    let value = element_text(&signed, "ds:SignatureValue").to_string();
    let flipped = if value.starts_with('A') { "B" } else { "A" };
    let forged = signed.replacen(&value, &format!("{flipped}{}", &value[1..]), 1);
    let result = verify(forged.as_bytes()).unwrap();
    assert!(result.digest_matches && !result.signature_matches);
}

#[test]
fn unsigned_documents_foreign_profiles_and_bad_ids_are_errors() {
    let case = &cases()[0];
    assert_eq!(verify(&unsigned(case)), Err(SignError::SignatureNotFound));

    let signed = String::from_utf8(expected(case)).unwrap();
    let sha256 = signed.replace("xmldsig#rsa-sha1", "xmldsig-more#rsa-sha256");
    assert!(matches!(
        verify(sha256.as_bytes()),
        Err(SignError::UnsupportedAlgorithm(_))
    ));

    for id in ["", "a\"b", "a&b", "1abc"] {
        let err = sign_with_id(&unsigned(case), &cert_pem(), &key_pem(), id).unwrap_err();
        assert!(matches!(err, SignError::MalformedSignature(_)), "{id:?}: {err:?}");
    }
}

#[test]
fn signature_goes_into_the_first_blank_extension_content() {
    let xml = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<r xmlns=\"urn:x\" \
        xmlns:ext=\"urn:oasis:names:specification:ubl:schema:xsd:CommonExtensionComponents-2\">\
        <ext:UBLExtensions><ext:UBLExtension><ext:ExtensionContent>data</ext:ExtensionContent>\
        </ext:UBLExtension><ext:UBLExtension><ext:ExtensionContent></ext:ExtensionContent>\
        </ext:UBLExtension></ext:UBLExtensions><a>1</a></r>\n";
    let signed = String::from_utf8(sign(xml.as_bytes(), &cert_pem(), &key_pem()).unwrap()).unwrap();
    let first_content_end = signed.find("</ext:ExtensionContent>").unwrap();
    let signature_start = signed.find("<ds:Signature").unwrap();
    assert!(signature_start > first_content_end, "the first ExtensionContent has text");
    assert!(signed.contains(
        "<ds:Signature xmlns:ds=\"http://www.w3.org/2000/09/xmldsig#\" Id=\"signatureKG\">"
    ));
    assert!(verify(signed.as_bytes()).unwrap().is_valid());
}

#[test]
fn without_extension_content_the_signature_goes_last_in_the_root() {
    let xml = b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<r xmlns=\"urn:x\"><a>1</a></r>\n";
    let signed = String::from_utf8(sign(xml, &cert_pem(), &key_pem()).unwrap()).unwrap();
    assert!(signed.ends_with("</ds:Signature></r>\n"));
    assert!(verify(signed.as_bytes()).unwrap().is_valid());
}

#[test]
fn a_key_from_another_certificate_is_refused_without_leaking_it() {
    let case = &cases()[0];
    let other_pem = String::from_utf8(read(&vectors_dir().join("test-cert/other-key.pem"))).unwrap();
    let err = sign(&unsigned(case), &cert_pem(), &other_pem).unwrap_err();
    assert_eq!(err, SignError::KeyCertificateMismatch);

    let broken = key_pem().replace("-----BEGIN PRIVATE KEY-----\n", "-----BEGIN PRIVATE KEY-----\n!");
    let err = sign(&unsigned(case), &cert_pem(), &broken).unwrap_err();
    let shown = format!("{err} {err:?}");
    let key_line = broken.lines().nth(1).unwrap();
    assert!(matches!(err, SignError::InvalidPrivateKey(_)));
    assert!(!shown.contains(&key_line[1..]), "error output must not carry key material");
}

/// Runs `tests/verify_independent.py` (lxml + cryptography) on the signatures this crate makes of
/// every vector. Python comes from `SUNAT_XMLDSIG_PYTHON`; without it, `python3` is tried and the
/// test is skipped with a notice when that interpreter lacks lxml or cryptography.
#[test]
fn independent_verifier_accepts_our_signatures() {
    let explicit = std::env::var("SUNAT_XMLDSIG_PYTHON").ok();
    let python = explicit.clone().unwrap_or_else(|| "python3".into());
    let probe = Command::new(&python)
        .args(["-I", "-c", "import lxml, cryptography"])
        .status();
    if !matches!(probe, Ok(s) if s.success()) {
        assert!(explicit.is_none(), "SUNAT_XMLDSIG_PYTHON={python} lacks lxml or cryptography");
        eprintln!("SKIPPED independent verifier: set SUNAT_XMLDSIG_PYTHON to a Python with lxml and cryptography");
        return;
    }
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("sunat-xmldsig");
    std::fs::create_dir_all(&dir).unwrap();
    let files: Vec<PathBuf> = cases()
        .iter()
        .map(|case| {
            let path = dir.join(format!("{case}.signed.xml"));
            std::fs::write(&path, sign_case(case)).unwrap();
            path
        })
        .collect();
    let output = Command::new(&python)
        .arg("-I")
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/verify_independent.py"))
        .args(&files)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    eprintln!("{stdout}");
    assert!(output.status.success(), "independent verifier rejected: {stdout}");
    assert_eq!(stdout.lines().filter(|l| l.starts_with("OK ")).count(), files.len());
}
