//! Export (0200 to 0208): the valid vectors of `fixtures/export/` are built, signed with the test certificate
//! and verified by this workspace's verifier and by the independent one
//! (`sunat-xmldsig/tests/verify_independent.py`); the negative ones are rejected before building with the
//! SUNAT code of the contract, and the country of use and the guest data come out in UBL order.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use sunat_model::{Document, ModelError};

const VALID: &[&str] = &[
    "v1-invoice-export-goods-0200",
    "v2-invoice-export-saas-hosting-0201",
    "v3-invoice-export-consulting-0208",
    "v4-invoice-export-lodging-0202",
];
const DECLARATION: &[u8] = b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n";

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(path: PathBuf) -> String {
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn parsed(case: &str) -> Result<Document, ModelError> {
    Document::from_json(&read(repository().join("fixtures/export").join(format!("{case}.json"))))
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
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("sunat-ubl-export");
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
    assert_eq!(code("n1-0208-without-country"), "3098");
    assert_eq!(code("n2-0200-customer-with-ruc"), "2800");
    assert_eq!(code("n3-0202-without-stay-days"), "3145");
}

#[test]
fn services_carry_only_the_country_of_use_between_customer_and_payment_terms() {
    let xml = text("v2-invoice-export-saas-hosting-0201");
    assert!(xml.contains("<cbc:InvoiceTypeCode listID=\"0201\">01</cbc:InvoiceTypeCode>"));
    let delivery = "</cac:AccountingCustomerParty><cac:Delivery><cac:DeliveryLocation><cac:Address><cac:Country><cbc:IdentificationCode>DE</cbc:IdentificationCode></cac:Country></cac:Address></cac:DeliveryLocation></cac:Delivery><cac:PaymentTerms>";
    assert!(xml.contains(delivery), "missing {delivery}");
    assert!(text("v3-invoice-export-consulting-0208").contains("<cbc:IdentificationCode>CL</cbc:IdentificationCode>"));
    assert!(!text("v1-invoice-export-goods-0200").contains("cac:Delivery"));
}

#[test]
fn export_totals_are_one_9995_subtotal_and_the_export_amount() {
    let xml = text("v2-invoice-export-saas-hosting-0201");
    assert!(xml.contains("<cac:TaxTotal><cbc:TaxAmount currencyID=\"USD\">0.00</cbc:TaxAmount><cac:TaxSubtotal><cbc:TaxableAmount currencyID=\"USD\">1550.25</cbc:TaxableAmount><cbc:TaxAmount currencyID=\"USD\">0.00</cbc:TaxAmount><cac:TaxCategory><cac:TaxScheme><cbc:ID>9995</cbc:ID>"));
    assert!(xml.contains("<cbc:PayableAmount currencyID=\"USD\">1550.25</cbc:PayableAmount>"));
    assert!(!xml.contains("<cbc:ID>1000</cbc:ID>"));
}

#[test]
fn lodging_lines_carry_the_ten_guest_properties_after_the_item_codes() {
    let xml = text("v4-invoice-export-lodging-0202");
    assert_eq!(xml.matches("<cac:AdditionalItemProperty>").count(), 20);
    let first = "<cac:SellersItemIdentification><cbc:ID>HAB-DBL</cbc:ID></cac:SellersItemIdentification><cac:AdditionalItemProperty><cbc:Name>Beneficio Hospedajes-Paquete turístico: Código de país de emisión del pasaporte</cbc:Name><cbc:NameCode>4000</cbc:NameCode><cbc:Value>US</cbc:Value></cac:AdditionalItemProperty>";
    assert!(xml.contains(first), "missing {first}");
    assert!(xml.contains("<cbc:NameCode>4003</cbc:NameCode><cac:UsabilityPeriod><cbc:StartDate>2026-10-14</cbc:StartDate></cac:UsabilityPeriod>"));
    assert!(xml.contains("<cbc:NameCode>4005</cbc:NameCode><cac:UsabilityPeriod><cbc:DurationMeasure unitCode=\"DAY\">3</cbc:DurationMeasure></cac:UsabilityPeriod>"));
    let last = "<cbc:NameCode>4009</cbc:NameCode><cbc:Value>X12345678</cbc:Value></cac:AdditionalItemProperty></cac:Item>";
    assert!(xml.contains(last), "missing {last}");
}
