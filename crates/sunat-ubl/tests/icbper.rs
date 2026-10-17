//! ICBPER, tax 7152 (F03): the five valid vectors of the contract (`fixtures/icbper/v1` to `v5`) are
//! built, signed with the test certificate and verified by this workspace's verifier and by the
//! independent one (`sunat-xmldsig/tests/verify_independent.py`); the three invalid ones (`n1` to
//! `n3`) are rejected before building with the SUNAT code the contract expects.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use sunat_model::{Document, ModelError};

const VALID: &[&str] = &["v1-invoice-bags", "v2-receipt-bags", "v3-receipt-free-bags", "v4-credit-note-bags", "v5-rc-icbper"];
const DECLARATION: &[u8] = b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n";

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(path: PathBuf) -> String {
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn parsed(case: &str) -> Result<Document, ModelError> {
    Document::from_json(&read(repository().join("fixtures/icbper").join(format!("{case}.json"))))
}

fn built(case: &str) -> Vec<u8> {
    let document = parsed(case).unwrap_or_else(|e| panic!("{case}: {e}"));
    kembec_sunat_ubl::build(&document).unwrap_or_else(|e| panic!("{case}: {e}"))
}

fn signed(case: &str) -> Vec<u8> {
    let cert = read(repository().join("tests/vectors/test-cert/test-cert.pem"));
    let key = read(repository().join("tests/vectors/test-cert/test-key.pem"));
    sunat_xmldsig::sign(&built(case), &cert, &key).unwrap_or_else(|e| panic!("{case}: {e}"))
}

fn text(case: &str) -> String {
    String::from_utf8(built(case)).unwrap()
}

#[test]
fn valid_vectors_build_canonical_and_sign_and_verify() {
    for case in VALID {
        let xml = built(case);
        let body = xml.strip_prefix(DECLARATION).unwrap_or_else(|| panic!("{case}: declaration"));
        assert!(body == sunat_xmldsig::canonicalize(&xml).unwrap().as_slice(), "{case}: not canonical");
        sunat_xmldsig::verify(&signed(case)).unwrap_or_else(|e| panic!("{case}: {e}"));
    }
}

fn independent_verifier() -> PathBuf {
    ["sunat-xmldsig", "kembec-sunat-xmldsig"]
        .iter()
        .map(|folder| repository().join("crates").join(folder).join("tests/verify_independent.py"))
        .find(|path| path.is_file())
        .expect("sunat-xmldsig's tests/verify_independent.py")
}

#[test]
fn independent_verifier_accepts_the_signed_vectors() {
    let explicit = std::env::var("SUNAT_XMLDSIG_PYTHON").ok();
    let python = explicit.clone().unwrap_or_else(|| "python3".into());
    let probe = Command::new(&python).args(["-I", "-c", "import lxml, cryptography"]).status();
    if !matches!(probe, Ok(s) if s.success()) {
        assert!(explicit.is_none(), "SUNAT_XMLDSIG_PYTHON={python} lacks lxml or cryptography");
        eprintln!("SKIPPED independent verifier: set SUNAT_XMLDSIG_PYTHON to a Python with lxml and cryptography");
        return;
    }
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("sunat-ubl-icbper");
    fs::create_dir_all(&dir).unwrap();
    let files: Vec<PathBuf> = VALID
        .iter()
        .map(|case| {
            let path = dir.join(format!("{case}.signed.xml"));
            fs::write(&path, signed(case)).unwrap();
            path
        })
        .collect();
    let output = Command::new(&python).arg("-I").arg(independent_verifier()).args(&files).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    eprintln!("{stdout}{}", String::from_utf8_lossy(&output.stderr));
    assert!(output.status.success(), "independent verifier rejected: {stdout}");
    assert_eq!(stdout.lines().filter(|l| l.starts_with("OK ")).count(), files.len());
}

#[test]
fn invalid_vectors_fail_with_the_contract_codes() {
    let code = |case: &str| parsed(case).err().and_then(|e| e.sunat_code()).unwrap_or_else(|| panic!("{case}: accepted"));
    assert_eq!(code("n1-amount-not-unit-times-quantity"), "4318");
    assert_eq!(code("n2-total-not-sum-of-lines"), "3306");
    assert_eq!(code("n3-stale-rate"), "4237");
}

const SCHEME: &str = "<cac:TaxScheme><cbc:ID>7152</cbc:ID><cbc:Name>ICBPER</cbc:Name><cbc:TaxTypeCode>OTH</cbc:TaxTypeCode></cac:TaxScheme>";

fn line_subtotal(amount: &str, bags: &str) -> String {
    format!(
        "<cac:TaxSubtotal><cbc:TaxAmount currencyID=\"PEN\">{amount}</cbc:TaxAmount><cbc:BaseUnitMeasure unitCode=\"NIU\">{bags}</cbc:BaseUnitMeasure><cac:TaxCategory><cbc:PerUnitAmount currencyID=\"PEN\">0.50</cbc:PerUnitAmount>{SCHEME}</cac:TaxCategory></cac:TaxSubtotal></cac:TaxTotal>"
    )
}

fn total_subtotal(amount: &str) -> String {
    format!("<cac:TaxSubtotal><cbc:TaxAmount currencyID=\"PEN\">{amount}</cbc:TaxAmount><cac:TaxCategory>{SCHEME}</cac:TaxCategory></cac:TaxSubtotal></cac:TaxTotal>")
}

#[test]
fn the_bag_line_carries_7152_after_its_igv_subtotal() {
    let xml = text(VALID[0]);
    assert!(xml.contains("<cac:TaxTotal><cbc:TaxAmount currencyID=\"PEN\">2.04</cbc:TaxAmount><cac:TaxSubtotal><cbc:TaxableAmount currencyID=\"PEN\">0.20</cbc:TaxableAmount>"), "{xml}");
    assert!(xml.contains(&line_subtotal("2.00", "4")), "{xml}");
    assert_eq!(xml.matches("<cbc:ID>7152</cbc:ID>").count(), 2, "one line subtotal and one document subtotal");
    assert!(text(VALID[3]).contains(&line_subtotal("1.00", "2")), "credit note line");
    assert!(text(VALID[2]).contains(&line_subtotal("1.50", "3")), "free line");
}

#[test]
fn the_document_total_has_7152_without_a_base_and_the_amounts_include_it() {
    let xml = text(VALID[0]);
    assert!(xml.contains(&total_subtotal("2.00")), "{xml}");
    assert!(xml.contains("<cac:TaxTotal><cbc:TaxAmount currencyID=\"PEN\">20.04</cbc:TaxAmount>"));
    assert!(xml.contains("<cbc:LineExtensionAmount currencyID=\"PEN\">100.20</cbc:LineExtensionAmount><cbc:TaxInclusiveAmount currencyID=\"PEN\">120.24</cbc:TaxInclusiveAmount><cbc:PayableAmount currencyID=\"PEN\">120.24</cbc:PayableAmount>"));
    assert!(text(VALID[3]).contains("<cbc:PayableAmount currencyID=\"PEN\">1.12</cbc:PayableAmount>"));
}

#[test]
fn the_summary_line_has_its_own_7152_tax_total_after_the_igv() {
    let xml = text(VALID[4]);
    let igv_then_icbper = format!("</cac:TaxTotal><cac:TaxTotal><cbc:TaxAmount currencyID=\"PEN\">1.50</cbc:TaxAmount>{}</sac:SummaryDocumentsLine>", total_subtotal("1.50"));
    assert!(xml.contains(&igv_then_icbper), "{xml}");
}

#[test]
fn per_unit_amount_keeps_at_least_two_decimals() {
    let mut value: serde_json::Value = serde_json::from_str(&read(repository().join("fixtures/icbper/v1-invoice-bags.json"))).unwrap();
    value["lines"][1]["icbper_factor"] = serde_json::json!("0.5");
    let xml = String::from_utf8(kembec_sunat_ubl::build(&Document::from_json(&value.to_string()).unwrap()).unwrap()).unwrap();
    assert!(xml.contains("<cbc:PerUnitAmount currencyID=\"PEN\">0.50</cbc:PerUnitAmount>"));
}
