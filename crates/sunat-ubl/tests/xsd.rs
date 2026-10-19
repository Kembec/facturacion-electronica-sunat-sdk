//! The documents the SDK builds from `fixtures/`, signed, validate against the OASIS UBL 2.1 schemas
//! (`tests/xsd/ubl-2.1`): the despatch advices (`09-*.json`) against `DespatchAdvice`, and every invoice
//! and receipt (01 and 03, in any folder of `fixtures/`, that the model accepts) against `Invoice`.
//! SUNAT has no test environment for despatch advices, so this schema check, the signature and the
//! model's rules are what is verified before sending; an invoice or receipt is also sent to beta.
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
    sign(case, &document)
}

fn sign(case: &str, document: &Document) -> Vec<u8> {
    let xml = kembec_sunat_ubl::build(document).unwrap_or_else(|e| panic!("{case}: {e}"));
    let cert = read(repository().join("tests/vectors/test-cert/test-cert.pem"));
    let key = read(repository().join("tests/vectors/test-cert/test-key.pem"));
    sunat_xmldsig::sign(&xml, &cert, &key).unwrap_or_else(|e| panic!("{case}: {e}"))
}

/// The Python with lxml, or `None` (the test is skipped) when none was asked for and `python3` lacks it.
fn python() -> Option<String> {
    let explicit = std::env::var("SUNAT_XMLDSIG_PYTHON").ok();
    let python = explicit.clone().unwrap_or_else(|| "python3".into());
    let probe = Command::new(&python).args(["-I", "-c", "import lxml"]).status();
    if matches!(probe, Ok(s) if s.success()) {
        return Some(python);
    }
    assert!(explicit.is_none(), "SUNAT_XMLDSIG_PYTHON={python} lacks lxml");
    eprintln!("SKIPPED UBL schema check: set SUNAT_XMLDSIG_PYTHON to a Python with lxml");
    None
}

/// Writes each signed document and validates them all against `maindoc/<schema>`.
fn assert_valid(python: &str, schema: &str, signed: Vec<(String, Vec<u8>)>) {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("sunat-ubl-xsd");
    fs::create_dir_all(&dir).unwrap();
    let files: Vec<PathBuf> = signed
        .into_iter()
        .map(|(case, xml)| {
            let path = dir.join(format!("{}.signed.xml", case.replace('/', "--")));
            fs::write(&path, xml).unwrap();
            path
        })
        .collect();
    let schema = repository().join("tests/xsd/ubl-2.1/maindoc").join(schema);
    let output = Command::new(python)
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
fn signed_despatch_advices_are_valid_ubl() {
    let Some(python) = python() else { return };
    let signed = CASES.iter().map(|case| (case.to_string(), signed(case))).collect();
    assert_valid(&python, "UBL-DespatchAdvice-2.1.xsd", signed);
}

/// Every JSON under `fixtures/`, by its path relative to it, without the extension.
fn fixture_cases(dir: &Path, prefix: &str, cases: &mut Vec<String>) {
    let mut entries: Vec<_> = fs::read_dir(dir).unwrap().map(|e| e.unwrap().path()).collect();
    entries.sort();
    for path in entries {
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        if path.is_dir() {
            fixture_cases(&path, &format!("{prefix}{name}/"), cases);
        } else if let Some(stem) = name.strip_suffix(".json") {
            cases.push(format!("{prefix}{stem}"));
        }
    }
}

/// Invoices and receipts the model accepts; the vectors it rejects are not built.
fn invoices_and_receipts() -> Vec<(String, Document)> {
    let mut cases = Vec::new();
    fixture_cases(&repository().join("fixtures"), "", &mut cases);
    cases
        .into_iter()
        .filter_map(|case| {
            let json = read(repository().join("fixtures").join(format!("{case}.json")));
            let value: serde_json::Value = serde_json::from_str(&json).ok()?;
            let kind = (value["kind"].as_str()?, value["document_type"].as_str()?);
            if !matches!(kind, ("invoice", "01" | "03")) {
                return None;
            }
            Some((case, Document::from_json(&json).ok()?))
        })
        .collect()
}

#[test]
fn signed_invoices_and_receipts_are_valid_ubl() {
    let documents = invoices_and_receipts();
    let names: Vec<&str> = documents.iter().map(|(case, _)| case.as_str()).collect();
    let mut top_level = Vec::new();
    fixture_cases(&repository().join("fixtures"), "", &mut top_level);
    for case in top_level.iter().filter(|c| !c.contains('/') && (c.starts_with("01-") || c.starts_with("03-"))) {
        assert!(names.contains(&case.as_str()), "{case} is not built");
    }
    let Some(python) = python() else { return };
    let signed = documents.iter().map(|(case, document)| (case.clone(), sign(case, document))).collect();
    assert_valid(&python, "UBL-Invoice-2.1.xsd", signed);
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
    assert!(between.contains(
        "</cac:CarrierParty><cac:LoadingTransportEvent><cbc:OccurrenceDate>2026-10-15</cbc:OccurrenceDate></cac:LoadingTransportEvent></cac:ShipmentStage>"
    ));
    assert!(!xml.contains("cac:LoadingTransportEvent"), "private transport has no handover date");
    assert!(!between.contains("cac:DriverPerson") && !between.contains("cac:TransportHandlingUnit"));
}
