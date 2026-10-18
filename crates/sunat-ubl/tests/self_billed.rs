//! The purchase settlement (04) and its reversal (RR): the valid vectors of its contract (`fixtures/self-billed/v1`,
//! `v2`, `v3`)
//! are built, signed with the test certificate, verified by this workspace's verifier and by the
//! independent one (`sunat-xmldsig/tests/verify_independent.py`), and carry the nodes of the contract
//! (§3). The SUNAT beta accepted both (`fixtures/self-billed/beta/`, 18/10/2026).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use sunat_model::Document;

const VALID: &[&str] = &["v1-taxed-purchase", "v2-gold-retention-4", "v3-reversal"];
/// The ones the beta received with `sendBill`; the reversal goes with `sendSummary`.
const SENT: &[&str] = &["v1-taxed-purchase", "v2-gold-retention-4"];
const DECLARATION: &[u8] = b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n";

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(path: PathBuf) -> String {
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn document(case: &str) -> Document {
    let json = read(repository().join("fixtures/self-billed").join(format!("{case}.json")));
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
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("sunat-ubl-self-billed");
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
fn roles_are_reversed_and_the_place_is_in_delivery_terms() {
    let xml = text("v1-taxed-purchase");
    for expected in [
        "<SelfBilledInvoice xmlns=\"urn:oasis:names:specification:ubl:schema:xsd:SelfBilledInvoice-2\"",
        "<cbc:UBLVersionID>2.1</cbc:UBLVersionID><cbc:CustomizationID>2.0</cbc:CustomizationID><cbc:ID>L001-1</cbc:ID>",
        "<cbc:InvoiceTypeCode listID=\"0501\">04</cbc:InvoiceTypeCode>",
        "<cac:AccountingCustomerParty><cac:Party><cac:PartyIdentification><cbc:ID schemeID=\"6\">20000000001</cbc:ID>",
        "<cac:AccountingSupplierParty><cac:Party><cac:PartyIdentification><cbc:ID schemeID=\"1\">40123456</cbc:ID>",
        "<cac:RegistrationAddress><cbc:ID>040101</cbc:ID><cbc:AddressTypeCode>03</cbc:AddressTypeCode>",
        "<cac:DeliveryTerms><cac:DeliveryLocation><cbc:LocationTypeCode>01</cbc:LocationTypeCode><cac:Address><cbc:ID>150101</cbc:ID>",
    ] {
        assert!(xml.contains(expected), "missing {expected}");
    }
    let customer = xml.find("<cac:AccountingCustomerParty>").unwrap();
    assert!(customer < xml.find("<cac:AccountingSupplierParty>").unwrap());
}

/// The document's `TaxAmount` is the IGV; the retention is a subtotal of tax 3000 it does not add.
#[test]
fn tax_total_carries_the_retention_apart_and_the_totals() {
    let xml = text("v1-taxed-purchase");
    for expected in [
        "<cac:TaxTotal><cbc:TaxAmount currencyID=\"PEN\">214.20</cbc:TaxAmount>",
        "<cbc:TaxableAmount currencyID=\"PEN\">1190.00</cbc:TaxableAmount><cbc:TaxAmount currencyID=\"PEN\">17.85</cbc:TaxAmount><cac:TaxCategory><cac:TaxScheme><cbc:ID>3000</cbc:ID><cbc:Name>IR</cbc:Name><cbc:TaxTypeCode>TOX</cbc:TaxTypeCode>",
        "<cac:LegalMonetaryTotal><cbc:LineExtensionAmount currencyID=\"PEN\">1190.00</cbc:LineExtensionAmount><cbc:TaxInclusiveAmount currencyID=\"PEN\">1404.20</cbc:TaxInclusiveAmount><cbc:PayableAmount currencyID=\"PEN\">1172.15</cbc:PayableAmount>",
        "<cbc:TaxAmount currencyID=\"PEN\">10.50</cbc:TaxAmount><cac:TaxCategory><cbc:Percent>1.5</cbc:Percent><cac:TaxScheme><cbc:ID>3000</cbc:ID>",
    ] {
        assert!(xml.contains(expected), "missing {expected}");
    }
}

#[test]
fn gold_lines_carry_the_four_properties() {
    let xml = text("v2-gold-retention-4");
    assert!(xml.contains("<cbc:InvoiceTypeCode listID=\"0503\">04</cbc:InvoiceTypeCode>"));
    for code in ["6000", "6004", "6005", "6006"] {
        assert!(xml.contains(&format!("<cbc:NameCode>{code}</cbc:NameCode>")), "missing {code}");
    }
    assert!(xml.contains("<cbc:NameCode>6000</cbc:NameCode><cbc:Value>010012718</cbc:Value>"));
}

/// What the beta received is what the SDK builds today, apart from the signature.
#[test]
fn the_xml_sent_to_the_beta_is_the_xml_built() {
    for case in SENT {
        let sent = read(repository().join("fixtures/self-billed/beta").join(format!("{case}.sent.xml")));
        let unsigned = String::from_utf8(built(case)).unwrap();
        let strip = |xml: &str| xml[xml.find("</ext:UBLExtensions>").unwrap()..].to_owned();
        assert_eq!(strip(&sent), strip(&unsigned), "{case}");
    }
}

#[test]
fn file_name_is_ruc_04_series_number() {
    assert_eq!(document("v2-gold-retention-4").file_name(), "20000000001-04-L001-2");
}

/// The reversal is the void communication's `VoidedDocuments` with its own identifier and type 04 lines.
#[test]
fn reversal_is_voided_documents_named_rr_with_04_lines() {
    let xml = text("v3-reversal");
    for expected in [
        "<VoidedDocuments xmlns=\"urn:sunat:names:specification:ubl:peru:schema:xsd:VoidedDocuments-1\"",
        "<cbc:UBLVersionID>2.0</cbc:UBLVersionID><cbc:CustomizationID>1.0</cbc:CustomizationID><cbc:ID>RR-20261018-1</cbc:ID><cbc:ReferenceDate>2026-10-18</cbc:ReferenceDate><cbc:IssueDate>2026-10-18</cbc:IssueDate>",
        "<cac:AccountingSupplierParty><cbc:CustomerAssignedAccountID>20000000001</cbc:CustomerAssignedAccountID><cbc:AdditionalAccountID>6</cbc:AdditionalAccountID>",
        "<sac:VoidedDocumentsLine><cbc:LineID>1</cbc:LineID><cbc:DocumentTypeCode>04</cbc:DocumentTypeCode><sac:DocumentSerialID>L001</sac:DocumentSerialID><sac:DocumentNumberID>1</sac:DocumentNumberID><sac:VoidReasonDescription>ERROR EN LA CANTIDAD ACOPIADA</sac:VoidReasonDescription></sac:VoidedDocumentsLine>",
    ] {
        assert!(xml.contains(expected), "missing {expected}");
    }
    assert_eq!(document("v3-reversal").file_name(), "20000000001-RR-20261018-1");
}
