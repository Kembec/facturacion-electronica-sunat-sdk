//! The retention voucher (20): the three valid vectors of its contract (`fixtures/retention/v1` to
//! `v3`) are built, signed with the test certificate, verified by this workspace's verifier and by
//! the independent one (`sunat-xmldsig/tests/verify_independent.py`, lxml and cryptography), and
//! carry the nodes of the contract (§5). There is no SUNAT schema for `Retention` in the repository:
//! the structure is checked node by node.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use sunat_model::Document;

const VALID: &[&str] = &["v1-pen-simple", "v2-several-documents", "v3-usd-correct"];
const DECLARATION: &[u8] = b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n";

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(path: PathBuf) -> String {
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn document(case: &str) -> Document {
    let json = read(repository().join("fixtures/retention").join(format!("{case}.json")));
    Document::from_json(&json).unwrap_or_else(|e| panic!("{case}: {e}"))
}

fn built(case: &str) -> Vec<u8> {
    kembec_sunat_ubl::build(&document(case)).unwrap_or_else(|e| panic!("{case}: {e}"))
}

fn signed(case: &str) -> Vec<u8> {
    let cert = read(repository().join("tests/vectors/test-cert/test-cert.pem"));
    let key = read(repository().join("tests/vectors/test-cert/test-key.pem"));
    sunat_xmldsig::sign(&built(case), &cert, &key).unwrap_or_else(|e| panic!("{case}: {e}"))
}

fn text(case: &str) -> String {
    String::from_utf8(signed(case)).unwrap()
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

/// `sunat-xmldsig`'s verifier: `crates/sunat-xmldsig` in the repository, `crates/kembec-sunat-xmldsig`
/// in the battery of the packaged crates (`crates/scripts/test-package.sh`).
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
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("sunat-ubl-retention");
    fs::create_dir_all(&dir).unwrap();
    let files: Vec<PathBuf> = VALID
        .iter()
        .map(|case| {
            let path = dir.join(format!("{case}.signed.xml"));
            fs::write(&path, signed(case)).unwrap();
            path
        })
        .collect();
    let output = Command::new(&python)
        .arg("-I")
        .arg(independent_verifier())
        .args(&files)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    eprintln!("{stdout}{}", String::from_utf8_lossy(&output.stderr));
    assert!(output.status.success(), "independent verifier rejected: {stdout}");
    assert_eq!(stdout.lines().filter(|l| l.starts_with("OK ")).count(), files.len());
}

#[test]
fn header_parties_regime_and_totals() {
    let xml = text("v1-pen-simple");
    for expected in [
        "<Retention xmlns=\"urn:sunat:names:specification:ubl:peru:schema:xsd:Retention-1\"",
        "<cbc:UBLVersionID>2.0</cbc:UBLVersionID><cbc:CustomizationID>1.0</cbc:CustomizationID>",
        "<cbc:ID>R001-1</cbc:ID>",
        "<cac:AgentParty><cac:PartyIdentification><cbc:ID schemeID=\"6\">20000000001</cbc:ID>",
        "<cac:ReceiverParty><cac:PartyIdentification><cbc:ID schemeID=\"6\">",
        "<sac:SUNATRetentionSystemCode>01</sac:SUNATRetentionSystemCode><sac:SUNATRetentionPercent>3.00</sac:SUNATRetentionPercent>",
        "<cbc:TotalInvoiceAmount currencyID=\"PEN\">35.40</cbc:TotalInvoiceAmount><sac:SUNATTotalPaid currencyID=\"PEN\">1144.60</sac:SUNATTotalPaid>",
        "<sac:SUNATRetentionAmount currencyID=\"PEN\">35.40</sac:SUNATRetentionAmount>",
    ] {
        assert!(xml.contains(expected), "missing {expected}");
    }
    assert!(!xml.contains("cac:ExchangeRate"), "no exchange rate in PEN without one given");
}

/// The sheet's data 14 and 15: `CityName` is the province and `CountrySubentity` the department.
#[test]
fn agent_address_follows_the_sheet_not_greenter() {
    let mut json: serde_json::Value =
        serde_json::from_str(&read(repository().join("fixtures/retention/v1-pen-simple.json"))).unwrap();
    json["issuer"]["address"]["province"] = serde_json::json!("HUAURA");
    json["issuer"]["address"]["district"] = serde_json::json!("HUACHO");
    let document = Document::from_json(&json.to_string()).unwrap();
    let xml = String::from_utf8(kembec_sunat_ubl::build(&document).unwrap()).unwrap();
    let expected = "<cbc:CityName>HUAURA</cbc:CityName><cbc:CountrySubentity>LIMA</cbc:CountrySubentity><cbc:District>HUACHO</cbc:District>";
    assert!(xml.contains(expected), "missing {expected}");
    assert!(xml.contains("<cac:Country><cbc:IdentificationCode>PE</cbc:IdentificationCode></cac:Country>"));
}

/// `cac:Payment/cbc:ID` is the payment number of the related document, not the line's position.
#[test]
fn payment_ids_are_the_payment_numbers() {
    let xml = text("v2-several-documents");
    let ids: Vec<&str> = xml
        .match_indices("<cac:Payment><cbc:ID>")
        .map(|(at, open)| {
            let start = at + open.len();
            &xml[start..start + xml[start..].find('<').unwrap()]
        })
        .collect();
    assert_eq!(ids, ["1", "2", "1", "1"]);
    assert!(xml.contains("<cbc:ID schemeID=\"08\">FD01-12</cbc:ID>"));
}

#[test]
fn foreign_currency_lines_carry_the_exchange_rate() {
    let xml = text("v3-usd-correct");
    for expected in [
        "<cbc:PaidAmount currencyID=\"USD\">",
        "<cac:ExchangeRate><cbc:SourceCurrencyCode>USD</cbc:SourceCurrencyCode><cbc:TargetCurrencyCode>PEN</cbc:TargetCurrencyCode><cbc:CalculationRate>3.750000</cbc:CalculationRate>",
        "<sac:SUNATRetentionAmount currencyID=\"PEN\">112.50</sac:SUNATRetentionAmount>",
    ] {
        assert!(xml.contains(expected), "missing {expected}");
    }
}

#[test]
fn file_name_is_ruc_20_series_number() {
    assert_eq!(document("v1-pen-simple").file_name(), "20000000001-20-R001-1");
}
