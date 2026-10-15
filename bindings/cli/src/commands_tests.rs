use std::cell::RefCell;

use super::*;

const CDR_REJECTED_ZIP: &[u8] = include_bytes!("../../../crates/sunat-transport/tests/data/R-20150147718-01-FT01-982.zip");

fn conn() -> Connection {
    Connection { endpoint: "https://sunat.test/billService".into(), user: "20100066603MODDATOS".into(), password: "pw-secret".into() }
}

#[test]
fn zip_returns_file_name_and_content() {
    let out = zip(&json!({ "name": "20100066603-01-F001-1", "xml": "<Invoice/>" })).unwrap();
    assert_eq!(out["file_name"], "20100066603-01-F001-1.zip");
    let bytes = STANDARD.decode(out["zip_base64"].as_str().unwrap()).unwrap();
    assert_eq!(sunat_sdk::unzip_xml(&bytes).unwrap(), b"<Invoice/>");
    assert!(zip(&json!({ "name": "bad", "xml": "<a/>" })).is_err());
}

#[test]
fn send_posts_the_envelope_and_reads_the_cdr() {
    let seen = RefCell::new(String::new());
    let post = |endpoint: &str, action: &str, envelope: &str| -> Result<Vec<u8>, String> {
        assert_eq!((endpoint, action), ("https://sunat.test/billService", "urn:sendBill"));
        *seen.borrow_mut() = envelope.to_string();
        Ok(format!(
            "<S:Envelope xmlns:S=\"s\"><S:Body><ns2:sendBillResponse xmlns:ns2=\"n\"><applicationResponse>{}</applicationResponse></ns2:sendBillResponse></S:Body></S:Envelope>",
            STANDARD.encode(CDR_REJECTED_ZIP)
        )
        .into_bytes())
    };
    let input = json!({ "name": "20100066603-01-F001-2", "xml": "<Invoice/>" });
    let out = send(&input, &conn(), &post).unwrap();
    assert!(seen.borrow().contains("<wsse:Username>20100066603MODDATOS</wsse:Username>"));
    assert!(seen.borrow().contains("<fileName>20100066603-01-F001-2.zip</fileName>"));
    assert_eq!(out["outcome"], "cdr");
    assert_eq!(out["cdr"]["response_code"], "2047");
    assert_eq!(out["cdr"]["class"], "rejected");
    assert!(!out.to_string().contains("pw-secret"));
}

#[test]
fn send_reports_a_fault_with_its_class() {
    let post = |_: &str, _: &str, _: &str| -> Result<Vec<u8>, String> {
        Ok(b"<e:Envelope xmlns:e=\"s\"><e:Body><e:Fault><faultcode>e:Client.1033</faultcode><faultstring>Registrado previamente</faultstring></e:Fault></e:Body></e:Envelope>".to_vec())
    };
    let out = send(&json!({ "name": "20100066603-01-F001-1", "xml": "<a/>" }), &conn(), &post).unwrap();
    assert_eq!(out, json!({ "outcome": "fault", "code": "1033", "message": "Registrado previamente", "class": "already_registered" }));
}

#[test]
fn status_builds_get_status_cdr_and_attaches_the_cdr() {
    let post = |_: &str, action: &str, envelope: &str| -> Result<Vec<u8>, String> {
        assert_eq!(action, "urn:getStatusCdr");
        assert!(envelope.contains("<numeroComprobante>7782</numeroComprobante>"));
        Ok(format!(
            "<S:Envelope xmlns:S=\"s\"><S:Body><n:getStatusCdrResponse xmlns:n=\"n\"><statusCdr><statusCode>0004</statusCode><content>{}</content><statusMessage>La constancia existe</statusMessage></statusCdr></n:getStatusCdrResponse></S:Body></S:Envelope>",
            STANDARD.encode(CDR_REJECTED_ZIP)
        )
        .into_bytes())
    };
    let input = json!({ "ruc": "20100066603", "document_type": "01", "series": "F022", "number": "7782" });
    let out = status(&input, &conn(), &post).unwrap();
    assert_eq!(out["status_code"], "0004");
    assert_eq!(out["cdr"]["reference_id"], "FT01-982");
    assert!(status(&json!({ "ruc": "1", "document_type": "01", "series": "F022", "number": 1 }), &conn(), &post).is_err());
}

#[test]
fn cdr_reads_fields_and_class() {
    let out = cdr(&json!({ "cdr_base64": STANDARD.encode(CDR_REJECTED_ZIP) })).unwrap();
    assert_eq!(
        out,
        json!({ "reference_id": "FT01-982", "response_code": "2047", "description": "Es obligatorio al menos un AdditionalMonetaryTotal con codigo 1001, 1002 o 1003", "notes": [], "class": "rejected" })
    );
}
