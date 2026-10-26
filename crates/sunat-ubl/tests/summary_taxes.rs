//! IVAP (1016), ISC (2000), other taxes (9999) and the IGV perception on a daily summary line: the vectors of
//! `fixtures/summary-taxes/` are built, signed with the test certificate and verified by this workspace's
//! verifier and by the independent one (`sunat-xmldsig/tests/verify_independent.py`); IVAP takes the place of
//! the IGV total, ISC and 9999 follow it, and the perception sits between the customer and `cac:Status`
//! (Resumen Diario sheet, rows 58-75 and 116-148). They live outside `fixtures/` so the bindings' vector
//! count stays the same.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use sunat_model::Document;

const VALID: &[&str] = &[
    "v1-rc-receipt-ivap",
    "v2-rc-receipt-isc",
    "v3-rc-receipt-other-taxes",
    "v4-rc-receipt-perception",
];
const DECLARATION: &[u8] = b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n";

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(path: PathBuf) -> String {
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn built(case: &str) -> Vec<u8> {
    let json = read(repository().join("fixtures/summary-taxes").join(format!("{case}.json")));
    let document = Document::from_json(&json).unwrap_or_else(|e| panic!("{case}: {e}"));
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

fn tax_total(amount: &str, percent: Option<&str>, id: &str, name: &str, type_code: &str) -> String {
    let money = format!("<cbc:TaxAmount currencyID=\"PEN\">{amount}</cbc:TaxAmount>");
    let percent = percent.map(|p| format!("<cbc:Percent>{p}</cbc:Percent>")).unwrap_or_default();
    format!(
        "<cac:TaxTotal>{money}<cac:TaxSubtotal>{money}<cac:TaxCategory>{percent}<cac:TaxScheme><cbc:ID>{id}</cbc:ID>\
         <cbc:Name>{name}</cbc:Name><cbc:TaxTypeCode>{type_code}</cbc:TaxTypeCode></cac:TaxScheme></cac:TaxCategory>\
         </cac:TaxSubtotal></cac:TaxTotal>"
    )
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
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("sunat-ubl-summary-taxes");
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
fn ivap_takes_the_place_of_the_igv_total_without_a_percent() {
    let xml = text("v1-rc-receipt-ivap");
    assert!(xml.contains(&tax_total("4.00", None, "1016", "IVAP", "VAT")));
    assert!(!xml.contains("<cbc:ID>1000</cbc:ID>"));
    assert!(!xml.contains("<cbc:Percent>"));
}

#[test]
fn isc_and_other_taxes_follow_the_igv_total() {
    let xml = text("v2-rc-receipt-isc");
    let igv = tax_total("19.80", Some("18.00"), "1000", "IGV", "VAT");
    let isc = tax_total("10.00", None, "2000", "ISC", "EXC");
    assert!(xml.contains(&format!("{igv}{isc}")), "IGV then ISC");
    let xml = text("v3-rc-receipt-other-taxes");
    let igv = tax_total("18.00", Some("18.00"), "1000", "IGV", "VAT");
    let other = tax_total("1.00", None, "9999", "OTROS", "OTH");
    assert!(xml.contains(&format!("{igv}{other}")), "IGV then 9999");
}

#[test]
fn perception_sits_between_the_customer_and_the_status_in_pen() {
    let xml = text("v4-rc-receipt-perception");
    let perception = "</cac:AccountingCustomerParty><sac:SUNATPerceptionSummaryDocumentReference>\
        <sac:SUNATPerceptionSystemCode>01</sac:SUNATPerceptionSystemCode>\
        <sac:SUNATPerceptionPercent>2.00</sac:SUNATPerceptionPercent>\
        <cbc:TotalInvoiceAmount currencyID=\"PEN\">2.36</cbc:TotalInvoiceAmount>\
        <sac:SUNATTotalCashed currencyID=\"PEN\">120.36</sac:SUNATTotalCashed>\
        <cbc:TaxableAmount currencyID=\"PEN\">118.00</cbc:TaxableAmount>\
        </sac:SUNATPerceptionSummaryDocumentReference><cac:Status>";
    assert!(xml.contains(perception), "missing {perception}");
}
