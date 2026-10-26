//! Detraction with line data (1002 hydrobiological resources, 1004 freight transport): the vectors of
//! `fixtures/detraction/` are built, signed with the test certificate and verified by this workspace's
//! verifier and by the independent one (`sunat-xmldsig/tests/verify_independent.py`); the catch
//! (catalog 55, 3001 to 3006) closes `cac:Item` and the trip goes in the line's `cac:Delivery`, in UBL order.
//! They live outside `fixtures/` so the bindings' vector count stays the same.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use sunat_model::Document;

const VALID: &[&str] = &[
    "v1-invoice-detraction-1002",
    "v2-invoice-detraction-1004",
    "v3-receipt-detraction-1004",
];
const DECLARATION: &[u8] = b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n";

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(path: PathBuf) -> String {
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn built(case: &str) -> Vec<u8> {
    let json = read(repository().join("fixtures/detraction").join(format!("{case}.json")));
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

fn position(xml: &str, needle: &str) -> usize {
    xml.find(needle).unwrap_or_else(|| panic!("missing {needle}"))
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
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("sunat-ubl-detraction");
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
fn fishing_lines_carry_the_six_catch_properties_after_the_item_codes() {
    let xml = text("v1-invoice-detraction-1002");
    assert!(xml.contains("<cbc:InvoiceTypeCode listID=\"1002\">01</cbc:InvoiceTypeCode>"));
    assert_eq!(xml.matches("<cac:AdditionalItemProperty>").count(), 6);
    let first = "</cac:CommodityClassification><cac:AdditionalItemProperty><cbc:Name>Detracciones: Recursos Hidrobiológicos-Matrícula de la embarcación</cbc:Name><cbc:NameCode>3001</cbc:NameCode><cbc:Value>CO-12345-PM</cbc:Value></cac:AdditionalItemProperty>";
    assert!(xml.contains(first), "missing {first}");
    assert!(xml.contains("<cbc:NameCode>3005</cbc:NameCode><cac:UsabilityPeriod><cbc:StartDate>2026-10-20</cbc:StartDate></cac:UsabilityPeriod>"));
    let last = "<cbc:NameCode>3006</cbc:NameCode><cbc:ValueQuantity unitCode=\"TNE\">12.50</cbc:ValueQuantity></cac:AdditionalItemProperty></cac:Item>";
    assert!(xml.contains(last), "missing {last}");
    assert!(!xml.contains("<cac:DeliveryTerms>"));
}

#[test]
fn freight_lines_carry_the_trip_in_delivery_between_pricing_and_taxes() {
    for case in ["v2-invoice-detraction-1004", "v3-receipt-detraction-1004"] {
        let xml = text(case);
        let delivery = "<cac:Delivery><cac:DeliveryLocation><cac:Address><cbc:ID>150101</cbc:ID><cac:AddressLine><cbc:Line>AV. ARGENTINA 123, LIMA</cbc:Line></cac:AddressLine></cac:Address></cac:DeliveryLocation><cac:Despatch><cbc:Instructions>AREQUIPA - LIMA, CARGA GENERAL</cbc:Instructions><cac:DespatchAddress><cbc:ID>040101</cbc:ID><cac:AddressLine><cbc:Line>CALLE MERCADERES 45, AREQUIPA</cbc:Line></cac:AddressLine></cac:DespatchAddress></cac:Despatch>";
        assert!(xml.contains(delivery), "{case}: missing {delivery}");
        let terms = "<cac:DeliveryTerms><cbc:ID>01</cbc:ID><cbc:Amount currencyID=\"PEN\">1500.00</cbc:Amount></cac:DeliveryTerms><cac:DeliveryTerms><cbc:ID>02</cbc:ID><cbc:Amount currencyID=\"PEN\">1400.00</cbc:Amount></cac:DeliveryTerms><cac:DeliveryTerms><cbc:ID>03</cbc:ID><cbc:Amount currencyID=\"PEN\">1600.00</cbc:Amount></cac:DeliveryTerms></cac:Delivery>";
        assert!(xml.contains(terms), "{case}: missing {terms}");
        let line = position(&xml, "<cac:InvoiceLine>");
        let line_delivery = line + position(&xml[line..], "<cac:Delivery>");
        assert!(position(&xml[line..], "</cac:PricingReference>") + line < line_delivery, "{case}: Delivery before PricingReference");
        assert!(line_delivery < line + position(&xml[line..], "<cac:TaxTotal>"), "{case}: Delivery after TaxTotal");
        assert!(!xml.contains("<cbc:NameCode>300"), "{case}: catch properties on a freight line");
    }
}
