//! The despatch advices (`fixtures/09-*.json`), built and signed, validate against the OASIS
//! UBL 2.1 `DespatchAdvice` schema (`tests/xsd/ubl-2.1`). SUNAT has no test environment for despatch
//! advices, so this schema check, the signature and the model's rules are what is verified before
//! sending.
//!
//! The schema is checked with lxml: Python comes from `SUNAT_XMLDSIG_PYTHON`; without it, `python3`
//! is tried and the test is skipped with a notice when that interpreter lacks lxml.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use sunat_model::Document;

const CASES: &[&str] = &["09-despatch-sale", "09-despatch-between-establishments"];

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(path: PathBuf) -> String {
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn signed(case: &str) -> Vec<u8> {
    let json = read(repository().join("fixtures").join(format!("{case}.json")));
    let document = Document::from_json(&json).unwrap_or_else(|e| panic!("{case}: {e}"));
    let xml = kembec_sunat_ubl::build(&document).unwrap_or_else(|e| panic!("{case}: {e}"));
    let cert = read(repository().join("tests/vectors/test-cert/test-cert.pem"));
    let key = read(repository().join("tests/vectors/test-cert/test-key.pem"));
    sunat_xmldsig::sign(&xml, &cert, &key).unwrap_or_else(|e| panic!("{case}: {e}"))
}

#[test]
fn signed_despatch_advices_are_valid_ubl() {
    let explicit = std::env::var("SUNAT_XMLDSIG_PYTHON").ok();
    let python = explicit.clone().unwrap_or_else(|| "python3".into());
    let probe = Command::new(&python).args(["-I", "-c", "import lxml"]).status();
    if !matches!(probe, Ok(s) if s.success()) {
        assert!(explicit.is_none(), "SUNAT_XMLDSIG_PYTHON={python} lacks lxml");
        eprintln!("SKIPPED UBL schema check: set SUNAT_XMLDSIG_PYTHON to a Python with lxml");
        return;
    }
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("sunat-ubl-xsd");
    fs::create_dir_all(&dir).unwrap();
    let files: Vec<PathBuf> = CASES
        .iter()
        .map(|case| {
            let path = dir.join(format!("{case}.signed.xml"));
            fs::write(&path, signed(case)).unwrap();
            path
        })
        .collect();
    let schema = repository().join("tests/xsd/ubl-2.1/maindoc/UBL-DespatchAdvice-2.1.xsd");
    let output = Command::new(&python)
        .arg("-I")
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/validate_xsd.py"))
        .arg(&schema)
        .args(&files)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    eprintln!("{stdout}{}", String::from_utf8_lossy(&output.stderr));
    assert!(output.status.success(), "UBL 2.1 schema rejected: {stdout}");
    assert_eq!(stdout.lines().filter(|l| l.starts_with("OK ")).count(), files.len());
}

#[test]
fn despatch_advice_carries_sunat_shipment_and_lines() {
    let xml = String::from_utf8(signed(CASES[0])).unwrap();
    for expected in [
        "<cbc:DespatchAdviceTypeCode>09</cbc:DespatchAdviceTypeCode>",
        "<cbc:ID>SUNAT_Envio</cbc:ID><cbc:HandlingCode>01</cbc:HandlingCode>",
        "<cbc:GrossWeightMeasure unitCode=\"KGM\">125.500</cbc:GrossWeightMeasure>",
        "<cbc:TransportModeCode>02</cbc:TransportModeCode><cac:TransitPeriod><cbc:StartDate>2026-10-15</cbc:StartDate>",
        "<cbc:JobTitle>Principal</cbc:JobTitle><cac:IdentityDocumentReference><cbc:ID>Q45678912</cbc:ID>",
        "<cbc:AddressTypeCode listID=\"20123456789\">0000</cbc:AddressTypeCode>",
        "<cbc:ID>ABC123</cbc:ID><cac:AttachedTransportEquipment><cbc:ID>XYZ9876</cbc:ID>",
        "<cbc:DeliveredQuantity unitCode=\"NIU\">10</cbc:DeliveredQuantity><cac:OrderLineReference><cbc:LineID>1</cbc:LineID>",
        "Papel bond reciclado &amp; cartón",
    ] {
        assert!(xml.contains(expected), "missing {expected}");
    }
    let between = String::from_utf8(signed(CASES[1])).unwrap();
    assert!(between.contains("<cbc:CompanyID>MTC1508123CNG</cbc:CompanyID>"));
    assert!(!between.contains("cac:DriverPerson") && !between.contains("cac:TransportHandlingUnit"));
}
