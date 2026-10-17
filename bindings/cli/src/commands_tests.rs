use std::cell::RefCell;

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use serde_json::json;

use super::*;

const CDR_REJECTED_ZIP: &[u8] = include_bytes!("../../../crates/sunat-transport/tests/data/R-20150147718-01-FT01-982.zip");

fn conn() -> Connection {
    Connection {
        endpoint: "https://sunat.test/billService".into(),
        user: "20100066603MODDATOS".into(),
        password: "pw-secret".into(),
        timeout: std::time::Duration::from_secs(90),
    }
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
    let post = |endpoint: &str, action: &str, envelope: &str| -> Result<Vec<u8>, PostError> {
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
    let post = |_: &str, _: &str, _: &str| -> Result<Vec<u8>, PostError> {
        Ok(b"<e:Envelope xmlns:e=\"s\"><e:Body><e:Fault><faultcode>e:Client.1033</faultcode><faultstring>Registrado previamente</faultstring></e:Fault></e:Body></e:Envelope>".to_vec())
    };
    let out = send(&json!({ "name": "20100066603-01-F001-1", "xml": "<a/>" }), &conn(), &post).unwrap();
    let diagnosis = json!([{
        "code": "1033",
        "detail": null,
        "fields": [{ "input_path": "series", "xpath": "/Invoice/cbc:ID", "certainty": "fuente" }]
    }]);
    assert_eq!(
        out,
        json!({ "outcome": "fault", "code": "1033", "message": "Registrado previamente", "class": "already_registered", "diagnosis": diagnosis })
    );
}

#[test]
fn send_diagnoses_the_beta_2800_on_the_customer_document_type() {
    let faultstring = include_str!("../../../crates/sunat-sdk/tests/data/error-fields/faultstring-2800.txt").trim().replace('&', "&amp;").replace('<', "&lt;");
    let body = format!("<e:Envelope xmlns:e=\"s\"><e:Body><e:Fault><faultcode>e:Client.2800</faultcode><faultstring>{faultstring}</faultstring></e:Fault></e:Body></e:Envelope>");
    let post = |_: &str, _: &str, _: &str| -> Result<Vec<u8>, PostError> { Ok(body.clone().into_bytes()) };
    let out = send(&json!({ "name": "20100066603-03-B001-1", "xml": "<a/>" }), &conn(), &post).unwrap();
    assert_eq!(out["class"], "rejected");
    assert_eq!(out["diagnosis"][0]["detail"], json!({ "node": "cbc:ID/schemeID", "value": "1" }));
    assert_eq!(out["diagnosis"][0]["fields"][0]["input_path"], "customer.document_type");
    assert_eq!(out["diagnosis"].as_array().unwrap().len(), 1);
}

#[test]
fn status_diagnoses_each_note_of_a_recorded_cdr() {
    // Annex C.1 of the programmer's manual: accepted with notes 4031 and 4001, codes of the UBL 2.0
    // sheet that the 2026 rules no longer list, so they carry no candidate field.
    let cdr_xml = include_bytes!("../../../crates/sunat-transport/tests/data/R-20150147718-01-FA01-981.xml");
    let post = |_: &str, _: &str, _: &str| -> Result<Vec<u8>, PostError> {
        Ok(format!(
            "<S:Envelope xmlns:S=\"s\"><S:Body><n:getStatusCdrResponse xmlns:n=\"n\"><statusCdr><statusCode>0004</statusCode><content>{}</content><statusMessage>La constancia existe</statusMessage></statusCdr></n:getStatusCdrResponse></S:Body></S:Envelope>",
            STANDARD.encode(cdr_xml)
        )
        .into_bytes())
    };
    let input = json!({ "ruc": "20150147718", "document_type": "01", "series": "FA01", "number": 981 });
    let out = status(&input, &conn(), &post).unwrap();
    assert_eq!(out["cdr"]["class"], "observed");
    assert_eq!(
        out["diagnosis"],
        json!([{ "code": "4031", "detail": null, "fields": [] }, { "code": "4001", "detail": null, "fields": [] }])
    );
}

#[test]
fn status_builds_get_status_cdr_and_attaches_the_cdr() {
    let post = |_: &str, action: &str, envelope: &str| -> Result<Vec<u8>, PostError> {
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

#[test]
fn cdr_with_code_0_and_an_observation_note_is_observed() {
    let xml = concat!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>",
        "<ar:ApplicationResponse xmlns:ar=\"urn:oasis:names:specification:ubl:schema:xsd:ApplicationResponse-2\" ",
        "xmlns:cac=\"urn:oasis:names:specification:ubl:schema:xsd:CommonAggregateComponents-2\" ",
        "xmlns:cbc=\"urn:oasis:names:specification:ubl:schema:xsd:CommonBasicComponents-2\">",
        "<cbc:Note>4287 - El precio unitario difiere del calculado</cbc:Note>",
        "<cac:DocumentResponse><cac:Response><cbc:ReferenceID>F001-7</cbc:ReferenceID>",
        "<cbc:ResponseCode>0</cbc:ResponseCode><cbc:Description>La Factura numero F001-7, ha sido aceptada</cbc:Description>",
        "</cac:Response></cac:DocumentResponse></ar:ApplicationResponse>"
    );
    let out = cdr(&json!({ "cdr_base64": STANDARD.encode(xml) })).unwrap();
    assert_eq!(out["response_code"], "0");
    assert_eq!(out["notes"], json!(["4287 - El precio unitario difiere del calculado"]));
    assert_eq!(out["class"], "observed");
}

/// The `contentFile` of a sendBill envelope, unzipped.
fn sent_xml(envelope: &str) -> Vec<u8> {
    let start = envelope.find("<contentFile>").unwrap() + "<contentFile>".len();
    let end = envelope.find("</contentFile>").unwrap();
    sunat_sdk::unzip_xml(&STANDARD.decode(&envelope[start..end]).unwrap()).unwrap()
}

#[test]
fn send_resends_a_stored_signed_xml_byte_for_byte() {
    // A stored signed vector with its declaration, line breaks and signature as they were kept;
    // CRLF added so any normalization by the SDK would show.
    let stored = include_str!("../../../tests/vectors/03-receipt-consumer-dni.signed.xml").replace('\n', "\r\n");
    let envelopes = RefCell::new(Vec::new());
    let post = |_: &str, _: &str, envelope: &str| -> Result<Vec<u8>, PostError> {
        envelopes.borrow_mut().push(envelope.to_string());
        Err(PostError::Timeout(1))
    };
    let name = "20000000001-03-B001-1";
    let inputs = [json!({ "name": name, "xml": stored }), json!({ "name": name, "xml_base64": STANDARD.encode(stored.as_bytes()) })];
    for input in &inputs {
        assert!(send(input, &conn(), &post).unwrap_err().contains("timed out"));
        assert!(send(input, &conn(), &post).unwrap_err().contains("timed out"), "the retry");
    }
    let envelopes = envelopes.borrow();
    assert_eq!(envelopes.len(), 4);
    for envelope in envelopes.iter() {
        assert_eq!(sent_xml(envelope), stored.as_bytes(), "the XML is sent as stored, never rebuilt or re-signed");
        assert!(envelope.contains(&format!("<fileName>{name}.zip</fileName>")), "same series and number");
    }
}

#[test]
fn summary_sends_with_send_summary_and_returns_the_ticket() {
    let calls = RefCell::new(Vec::new());
    let post = |endpoint: &str, action: &str, envelope: &str| -> Result<Vec<u8>, PostError> {
        calls.borrow_mut().push((endpoint.to_string(), action.to_string(), envelope.to_string()));
        Ok(br#"<S:Envelope xmlns:S="http://schemas.xmlsoap.org/soap/envelope/"><S:Body><ns2:sendSummaryResponse xmlns:ns2="http://service.sunat.gob.pe"><ticket>1760620000123</ticket></ns2:sendSummaryResponse></S:Body></S:Envelope>"#.to_vec())
    };
    let input = json!({ "name": "20123456789-RC-20261016-1", "xml": "<SummaryDocuments/>" });
    assert_eq!(summary(&input, &conn(), &post).unwrap(), json!({ "outcome": "ticket", "ticket": "1760620000123" }));
    let calls = calls.borrow();
    assert_eq!(calls[0].1, "urn:sendSummary");
    assert!(calls[0].2.contains("<fileName>20123456789-RC-20261016-1.zip</fileName>"));
}

#[test]
fn ticket_reports_in_process_and_the_cdr_once_processed() {
    let answers = RefCell::new(vec![
        format!(r#"<S:Envelope xmlns:S="http://schemas.xmlsoap.org/soap/envelope/"><S:Body><ns2:getStatusResponse xmlns:ns2="http://service.sunat.gob.pe"><status><content>{}</content><statusCode>99</statusCode></status></ns2:getStatusResponse></S:Body></S:Envelope>"#, STANDARD.encode(CDR_REJECTED_ZIP)),
        r#"<S:Envelope xmlns:S="http://schemas.xmlsoap.org/soap/envelope/"><S:Body><ns2:getStatusResponse xmlns:ns2="http://service.sunat.gob.pe"><status><statusCode>98</statusCode></status></ns2:getStatusResponse></S:Body></S:Envelope>"#.to_string(),
    ]);
    let post = |_: &str, action: &str, envelope: &str| -> Result<Vec<u8>, PostError> {
        assert_eq!(action, "urn:getStatus");
        assert!(envelope.contains("<ticket>1760620000123</ticket>"));
        Ok(answers.borrow_mut().pop().unwrap().into_bytes())
    };
    let input = json!({ "ticket": "1760620000123" });
    assert_eq!(ticket(&input, &conn(), &post).unwrap(), json!({ "outcome": "ticket_status", "status_code": "98", "state": "in_process" }));
    let done = ticket(&input, &conn(), &post).unwrap();
    assert_eq!(done["state"], "processed_with_errors");
    assert_eq!(done["cdr"]["response_code"], "2047");
    assert_eq!(done["cdr"]["class"], "rejected");
    assert!(ticket(&json!({ "ticket": "17-6" }), &conn(), &post).is_err());
}

#[test]
fn queries_without_an_answer_are_no_verdict_and_sends_stay_errors() {
    let unauthorized = |_: &str, _: &str, _: &str| -> Result<Vec<u8>, PostError> { Err(PostError::Status(401)) };
    let timed_out = |_: &str, _: &str, _: &str| -> Result<Vec<u8>, PostError> { Err(PostError::Timeout(45)) };
    let broken = |_: &str, _: &str, _: &str| -> Result<Vec<u8>, PostError> { Err(PostError::Other("transport error: Dns".into())) };
    let by_ticket = json!({ "ticket": "1760620000123" });
    let by_number = json!({ "ruc": "20100066603", "document_type": "03", "series": "B001", "number": 7 });

    let out = ticket(&by_ticket, &conn(), &unauthorized).unwrap();
    assert_eq!(out["outcome"], "no_verdict");
    assert_eq!((out["reason"].as_str(), out["http_status"].as_u64(), out["ticket"].as_str()), (Some("http_status"), Some(401), Some("1760620000123")));
    let out = status(&by_number, &conn(), &timed_out).unwrap();
    assert_eq!((out["outcome"].as_str(), out["reason"].as_str()), (Some("no_verdict"), Some("timeout")));
    assert!(out["http_status"].is_null() && out.get("ticket").is_none());

    assert!(ticket(&by_ticket, &conn(), &broken).is_err());
    assert!(status(&by_number, &conn(), &broken).is_err());
    let doc = json!({ "name": "20100066603-01-F001-1", "xml": "<a/>" });
    assert_eq!(send(&doc, &conn(), &unauthorized).unwrap_err(), "SUNAT answered HTTP 401");
    assert!(summary(&json!({ "name": "20123456789-RC-20261016-1", "xml": "<a/>" }), &conn(), &timed_out).unwrap_err().contains("outcome is unknown"));
}
