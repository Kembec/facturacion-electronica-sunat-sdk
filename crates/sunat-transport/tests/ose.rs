//! An OSE's CDRs and faults read like SUNAT's: synthetic answers in `fixtures/ose/` (see its README).

use std::fs;
use std::path::{Path, PathBuf};

use kembec_sunat_transport::*;

fn fixture(name: &str) -> Vec<u8> {
    let path: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/ose").join(name);
    fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn envelope(xml: &[u8]) -> Vec<u8> {
    use base64::Engine;
    let content = base64::engine::general_purpose::STANDARD.encode(xml);
    format!(
        r#"<S:Envelope xmlns:S="http://schemas.xmlsoap.org/soap/envelope/"><S:Body><br:sendBillResponse xmlns:br="http://service.sunat.gob.pe"><applicationResponse>{content}</applicationResponse></br:sendBillResponse></S:Body></S:Envelope>"#
    )
    .into_bytes()
}

fn fault(name: &str) -> (String, String) {
    match parse_send_bill_response(&fixture(name)).unwrap() {
        SendBillOutcome::Fault { code, message } => (code, message),
        other => panic!("{name} is a fault, got {other:?}"),
    }
}

#[test]
fn accepted_invoice_takes_its_number_from_the_document_reference() {
    let cdr = read_cdr(&fixture("ose-cdr-invoice-accepted.xml")).unwrap();
    assert_eq!(cdr.reference_id, "F001-1");
    assert_eq!(cdr.response_code, "0");
    assert!(cdr.notes.is_empty(), "{:?}", cdr.notes);
}

#[test]
fn accepted_summary_takes_its_identifier_from_the_document_reference() {
    let cdr = read_cdr(&fixture("ose-cdr-summary-accepted.xml")).unwrap();
    assert_eq!(cdr.reference_id, "RC-20261017-1");
    assert_eq!(cdr.response_code, "0");
    assert!(cdr.notes.is_empty(), "{:?}", cdr.notes);
}

#[test]
fn an_observation_in_response_status_becomes_a_note_in_sunat_form() {
    let cdr = read_cdr(&fixture("ose-cdr-invoice-observed.xml")).unwrap();
    assert_eq!(cdr.reference_id, "F001-2");
    assert_eq!(cdr.response_code, "0");
    assert_eq!(cdr.notes, ["4252 - El dato ingresado como atributo @listName es incorrecto."]);
}

#[test]
fn a_nonzero_code_is_kept_as_written_so_it_is_not_read_as_accepted() {
    let cdr = read_cdr(&fixture("ose-cdr-nonzero-code.xml")).unwrap();
    assert_eq!(cdr.reference_id, "F001-3");
    assert_eq!(cdr.response_code_number(), Some(1));
    assert!(cdr.notes.is_empty());
}

#[test]
fn an_ose_cdr_inside_send_bill_reads_the_same() {
    let xml = fixture("ose-cdr-invoice-observed.xml");
    let SendBillOutcome::Cdr(cdr) = parse_send_bill_response(&envelope(&xml)).unwrap() else { panic!() };
    assert_eq!(read_cdr(&cdr).unwrap(), read_cdr(&xml).unwrap());
}

#[test]
fn several_statuses_keep_their_order_and_a_lone_code_or_reason_stands_alone() {
    let xml = br#"<ApplicationResponse xmlns:cac="a" xmlns:cbc="b"><cac:DocumentResponse><cac:Response><cbc:ResponseCode>0</cbc:ResponseCode><cac:Status><cbc:StatusReasonCode>4252</cbc:StatusReasonCode><cbc:StatusReason>uno</cbc:StatusReason></cac:Status><cac:Status><cbc:StatusReasonCode>4287</cbc:StatusReasonCode></cac:Status><cac:Status><cbc:StatusReason>sin codigo</cbc:StatusReason></cac:Status><cac:Status/></cac:Response><cac:DocumentReference><cbc:ID>B001-9</cbc:ID></cac:DocumentReference></cac:DocumentResponse></ApplicationResponse>"#;
    let cdr = read_cdr(xml).unwrap();
    assert_eq!(cdr.notes, ["4252 - uno", "4287", "sin codigo"]);
    assert_eq!(cdr.reference_id, "B001-9");
}

#[test]
fn response_reference_id_wins_over_the_document_reference() {
    let xml = br#"<ApplicationResponse xmlns:cac="a" xmlns:cbc="b"><cac:DocumentResponse><cac:Response><cbc:ReferenceID>F001-7</cbc:ReferenceID><cbc:ResponseCode>0</cbc:ResponseCode></cac:Response><cac:DocumentReference><cbc:ID>F001-8</cbc:ID></cac:DocumentReference></cac:DocumentResponse></ApplicationResponse>"#;
    assert_eq!(read_cdr(xml).unwrap().reference_id, "F001-7");
}

#[test]
fn ose_rejection_fault_takes_the_code_from_faultstring_and_the_message_from_detail() {
    assert_eq!(fault("ose-fault-rejected.xml"), ("2108".into(), "Presentacion fuera de fecha".into()));
}

#[test]
fn sunat_fault_with_an_empty_detail_keeps_faultstring_as_the_message() {
    assert_eq!(fault("sunat-fault-contrast.xml"), ("2108".into(), "Presentacion fuera de fecha".into()));
}

/// The OSE's own code (`ERR1`) is not one of SUNAT's: what is left of it (`1`) is below every range
/// the manual defines, so the SDK classifies it as `unknown`, neither accepted nor a known rejection.
#[test]
fn ose_fault_with_its_own_code_keeps_the_description_and_no_sunat_code() {
    let (code, message) = fault("ose-fault-alpha-code.xml");
    assert_eq!(message, "codigo propio del OSE sin digitos de SUNAT");
    let number: u32 = code.parse().unwrap();
    assert!(number < 100, "{code}");
}
