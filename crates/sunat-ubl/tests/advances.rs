//! Advances (F04): the valid vectors of `fixtures/advances/` are built, signed with the test certificate
//! and verified by this workspace's verifier and by the independent one
//! (`sunat-xmldsig/tests/verify_independent.py`); the two negative ones (`n1`, `n2`) are rejected before
//! building with the SUNAT code the contract expects, and the nodes come out in UBL order.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use sunat_model::{Document, ModelError};

const VALID: &[&str] = &[
    "v1-invoice-advance",
    "v1b-invoice-advance-second",
    "v1c-receipt-advance",
    "v2-invoice-final-one-advance",
    "v3-invoice-final-two-advances",
    "v4-receipt-final-advance",
    "s3-discount-with-factor",
];
const DECLARATION: &[u8] = b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n";

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(path: PathBuf) -> String {
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn parsed(case: &str) -> Result<Document, ModelError> {
    Document::from_json(&read(repository().join("fixtures/advances").join(format!("{case}.json"))))
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
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("sunat-ubl-advances");
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
fn negative_vectors_fail_with_the_contract_codes() {
    let code = |case: &str| parsed(case).err().and_then(|e| e.sunat_code()).unwrap_or_else(|| panic!("{case}: accepted"));
    assert_eq!(code("n1-prepaid-not-sum-of-advances"), "2509");
    assert_eq!(code("n2-igv-on-gross-base"), "3291");
    parsed("n3-advance-not-found").expect("only the SUNAT beta can tell the advance does not exist");
}

const REFERENCE: &str = "<cac:AdditionalDocumentReference><cbc:ID>F001-8101</cbc:ID><cbc:DocumentTypeCode>02</cbc:DocumentTypeCode><cbc:DocumentStatusCode>1</cbc:DocumentStatusCode><cac:IssuerParty><cac:PartyIdentification><cbc:ID schemeID=\"6\">20000000001</cbc:ID></cac:PartyIdentification></cac:IssuerParty></cac:AdditionalDocumentReference><cac:Signature>";
const PREPAID: &str = "<cac:PrepaidPayment><cbc:ID>1</cbc:ID><cbc:PaidAmount currencyID=\"PEN\">590.00</cbc:PaidAmount></cac:PrepaidPayment><cac:AllowanceCharge><cbc:ChargeIndicator>false</cbc:ChargeIndicator><cbc:AllowanceChargeReasonCode>04</cbc:AllowanceChargeReasonCode><cbc:Amount currencyID=\"PEN\">500.00</cbc:Amount></cac:AllowanceCharge><cac:TaxTotal>";

#[test]
fn the_final_invoice_cites_the_advance_pays_it_and_discounts_it() {
    let xml = text("v2-invoice-final-one-advance");
    assert!(xml.contains(REFERENCE), "missing {REFERENCE}");
    assert!(xml.contains(PREPAID), "missing {PREPAID}");
    assert!(xml.contains("<cbc:TaxInclusiveAmount currencyID=\"PEN\">2655.00</cbc:TaxInclusiveAmount><cbc:PrepaidAmount currencyID=\"PEN\">590.00</cbc:PrepaidAmount><cbc:PayableAmount currencyID=\"PEN\">2065.00</cbc:PayableAmount>"));
    assert!(!xml.contains("cbc:AllowanceTotalAmount"), "the advance is not in the allowance total");
}

#[test]
fn two_advances_are_numbered_by_position() {
    let xml = text("v3-invoice-final-two-advances");
    for number in ["1", "2"] {
        assert!(xml.contains(&format!("<cbc:DocumentStatusCode>{number}</cbc:DocumentStatusCode>")));
        assert!(xml.contains(&format!("<cac:PrepaidPayment><cbc:ID>{number}</cbc:ID>")));
    }
    assert_eq!(xml.matches("<cbc:AllowanceChargeReasonCode>04</cbc:AllowanceChargeReasonCode>").count(), 2);
}

#[test]
fn the_discount_carries_factor_and_base_when_given() {
    let xml = text("s3-discount-with-factor");
    assert!(xml.contains("<cbc:AllowanceChargeReasonCode>04</cbc:AllowanceChargeReasonCode><cbc:MultiplierFactorNumeric>0.22222</cbc:MultiplierFactorNumeric><cbc:Amount currencyID=\"PEN\">500.00</cbc:Amount><cbc:BaseAmount currencyID=\"PEN\">2250.00</cbc:BaseAmount>"));
}

#[test]
fn the_receipt_final_discounts_its_advance() {
    let xml = text("v4-receipt-final-advance");
    assert!(xml.contains("<cac:AdditionalDocumentReference><cbc:ID>B001-8100</cbc:ID><cbc:DocumentTypeCode>03</cbc:DocumentTypeCode>"));
    assert!(xml.contains("<cbc:PrepaidAmount currencyID=\"PEN\">118.00</cbc:PrepaidAmount><cbc:PayableAmount currencyID=\"PEN\">236.00</cbc:PayableAmount>"));
}

#[test]
fn a_sale_covered_by_its_advance_keeps_the_igv_subtotal_at_zero() {
    let xml = text("s2-payable-zero");
    assert!(xml.contains("<cac:TaxTotal><cbc:TaxAmount currencyID=\"PEN\">0.00</cbc:TaxAmount><cac:TaxSubtotal><cbc:TaxableAmount currencyID=\"PEN\">0.00</cbc:TaxableAmount><cbc:TaxAmount currencyID=\"PEN\">0.00</cbc:TaxAmount>"));
    assert!(xml.contains("<cbc:PrepaidAmount currencyID=\"PEN\">590.00</cbc:PrepaidAmount><cbc:PayableAmount currencyID=\"PEN\">0.00</cbc:PayableAmount>"));
}
