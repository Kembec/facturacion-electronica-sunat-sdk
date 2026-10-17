//! Catalog 53 discounts and charges (F02): the six valid vectors of the contract
//! (`fixtures/allowances/inv-01` to `inv-06`) are built, signed with the test certificate and
//! verified by this workspace's verifier and by the independent one
//! (`sunat-xmldsig/tests/verify_independent.py`); the three invalid ones (`inv-R1` to `inv-R3`) are
//! rejected before building with the SUNAT code the contract expects.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use sunat_model::{Document, ModelError};

const VALID: &[&str] = &[
    "inv-01-descuento-global-02-afecta-base",
    "inv-02-descuento-global-03-no-afecta-base",
    "inv-03-cargo-global-49-afecta-base",
    "inv-04-recargo-consumo-46-fuera-de-base",
    "inv-05-fise-45",
    "inv-06-cargo-linea-47-afecta-base",
];
const DECLARATION: &[u8] = b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n";

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(path: PathBuf) -> String {
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn parsed(case: &str) -> Result<Document, ModelError> {
    Document::from_json(&read(repository().join("fixtures/allowances").join(format!("{case}.json"))))
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
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("sunat-ubl-allowances");
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
    let r1 = code("inv-R1-igv-sobre-base-bruta");
    assert!(["3277", "3278", "3279", "3280", "3291"].contains(&r1), "R1: {r1}");
    assert_eq!(code("inv-R2-descuento-global-factor-no-coincide"), "3307");
    assert_eq!(code("inv-R3-cargo-linea-47-factor-no-coincide"), "3290");
}

const GLOBAL_02: &str = "<cac:AllowanceCharge><cbc:ChargeIndicator>false</cbc:ChargeIndicator><cbc:AllowanceChargeReasonCode>02</cbc:AllowanceChargeReasonCode><cbc:MultiplierFactorNumeric>0.1</cbc:MultiplierFactorNumeric><cbc:Amount currencyID=\"PEN\">25.00</cbc:Amount><cbc:BaseAmount currencyID=\"PEN\">250.00</cbc:BaseAmount></cac:AllowanceCharge><cac:TaxTotal>";

#[test]
fn global_discount_02_goes_before_the_tax_total_over_the_net_base() {
    let xml = text(VALID[0]);
    assert!(xml.contains(GLOBAL_02), "missing {GLOBAL_02}");
    assert!(xml.contains("<cac:TaxTotal><cbc:TaxAmount currencyID=\"PEN\">40.50</cbc:TaxAmount><cac:TaxSubtotal><cbc:TaxableAmount currencyID=\"PEN\">225.00</cbc:TaxableAmount>"));
    assert!(!xml.contains("cbc:AllowanceTotalAmount"), "02 is not in the allowance total");
}

#[test]
fn totals_51_and_52_follow_the_tax_inclusive_amount() {
    let xml = text(VALID[1]);
    assert!(xml.contains("<cbc:TaxInclusiveAmount currencyID=\"PEN\">295.00</cbc:TaxInclusiveAmount><cbc:AllowanceTotalAmount currencyID=\"PEN\">12.50</cbc:AllowanceTotalAmount><cbc:PayableAmount currencyID=\"PEN\">282.50</cbc:PayableAmount>"));
    for (case, code) in [(VALID[2], "49"), (VALID[3], "46"), (VALID[4], "45")] {
        let xml = text(case);
        let expected = format!("<cbc:ChargeIndicator>true</cbc:ChargeIndicator><cbc:AllowanceChargeReasonCode>{code}</cbc:AllowanceChargeReasonCode>");
        assert!(xml.contains(&expected), "{case}: missing {expected}");
    }
    assert!(text(VALID[3]).contains("<cbc:ChargeTotalAmount currencyID=\"PEN\">25.00</cbc:ChargeTotalAmount><cbc:PayableAmount currencyID=\"PEN\">320.00</cbc:PayableAmount>"));
}

#[test]
fn line_charge_47_is_a_charge_of_the_line() {
    let xml = text(VALID[5]);
    let expected = "<cac:AllowanceCharge><cbc:ChargeIndicator>true</cbc:ChargeIndicator><cbc:AllowanceChargeReasonCode>47</cbc:AllowanceChargeReasonCode><cbc:MultiplierFactorNumeric>0.05</cbc:MultiplierFactorNumeric><cbc:Amount currencyID=\"PEN\">10</cbc:Amount><cbc:BaseAmount currencyID=\"PEN\">200</cbc:BaseAmount></cac:AllowanceCharge><cac:TaxTotal>";
    assert!(xml.contains(expected), "missing {expected}");
    assert!(xml.contains("<cbc:LineExtensionAmount currencyID=\"PEN\">210.00</cbc:LineExtensionAmount>"));
}
