use base64::Engine;
use sunat_transport::*;

/// Synthetic CDRs shaped after the programmer's manual examples C.1 (accepted with notes) and
/// C.2 (rejected); see `data/README.md`.
const CDR_XML: &[u8] = include_bytes!("data/R-20150147718-01-FA01-981.xml");
const CDR_REJECTED_ZIP: &[u8] = include_bytes!("data/R-20150147718-01-FT01-982.zip");

fn compact(reference: &str) -> String {
    reference.lines().map(str::trim).collect()
}

fn b64(bytes: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

#[test]
fn zip_holds_one_xml_named_after_the_document() {
    let xml = b"<Invoice>firmado</Invoice>";
    let zip = zip_document("20100066603-01-F001-1", xml).unwrap();
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(&zip)).unwrap();
    assert_eq!(archive.len(), 1);
    assert_eq!(archive.by_index(0).unwrap().name(), "20100066603-01-F001-1.xml");
    assert_eq!(unzip_xml(&zip).unwrap(), xml);
    assert_eq!(zip_file_name("20100066603-01-F001-1"), "20100066603-01-F001-1.zip");
    assert_eq!(zip, zip_document("20100066603-01-F001-1", xml).unwrap(), "deterministic");
}

#[test]
fn zip_rejects_names_out_of_ruc_type_series_number() {
    for name in ["2010006660-01-F001-1", "20100066603-01-F01-1", "20100066603-01-F001-123456789",
        "20100066603-01-F001", "../20100066603-01-F001-1", "20100066603-01-F001-1.xml"] {
        assert_eq!(zip_document(name, b"x"), Err(TransportError::InvalidName(name.into())), "{name}");
    }
}

#[test]
fn send_bill_envelope_matches_reference() {
    let reference = compact(include_str!("data/send_bill_envelope.xml"));
    let envelope = send_bill_envelope("20100066603MODDATOS", "moddatos", "20100066603-01-F001-1.zip", b"PK-test");
    assert_eq!(envelope, reference);
}

#[test]
fn get_status_cdr_envelope_matches_reference() {
    let reference = compact(include_str!("data/get_status_cdr_envelope.xml"));
    let envelope = get_status_cdr_envelope("20520485750MODDATOS", "moddatos", "20520485750", "01", "F022", 7782).unwrap();
    assert_eq!(envelope, reference);
    assert!(get_status_cdr_envelope("u", "p", "205", "01", "F022", 1).is_err());
}

#[test]
fn envelope_escapes_credentials() {
    let envelope = send_bill_envelope("20100066603A&B", "p<w>\"'", "f.zip", b"");
    assert!(envelope.contains("<wsse:Username>20100066603A&amp;B</wsse:Username>"));
    assert!(envelope.contains(">p&lt;w&gt;&quot;&apos;</wsse:Password>"));
}

#[test]
fn send_bill_response_with_cdr() {
    let cdr_zip = zip_document("20150147718-01-FA01-981", CDR_XML).unwrap();
    let body = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?><soap-env:Envelope xmlns:soap-env="http://schemas.xmlsoap.org/soap/envelope/"><soap-env:Header/><soap-env:Body><br:sendBillResponse xmlns:br="http://service.sunat.gob.pe"><applicationResponse>{}</applicationResponse></br:sendBillResponse></soap-env:Body></soap-env:Envelope>"#,
        b64(&cdr_zip)
    );
    let SendBillOutcome::Cdr(zip) = parse_send_bill_response(body.as_bytes()).unwrap() else {
        panic!("expected cdr");
    };
    assert_eq!(zip, cdr_zip);
    assert_eq!(read_cdr(&zip).unwrap().reference_id, "FA01-981");
}

#[test]
fn send_bill_response_with_fault_code() {
    let body = br#"<soap-env:Envelope xmlns:soap-env="http://schemas.xmlsoap.org/soap/envelope/"><soap-env:Header/><soap-env:Body><soap-env:Fault><faultcode>soap-env:Client.1033</faultcode><faultstring>El comprobante fue registrado previamente con otros datos - Detalle: F001-1</faultstring></soap-env:Fault></soap-env:Body></soap-env:Envelope>"#;
    assert_eq!(
        parse_send_bill_response(body).unwrap(),
        SendBillOutcome::Fault {
            code: "1033".into(),
            message: "El comprobante fue registrado previamente con otros datos - Detalle: F001-1".into(),
        }
    );
}

#[test]
fn fault_without_digits_in_code_takes_them_from_the_message() {
    let body = br#"<S:Envelope xmlns:S="http://schemas.xmlsoap.org/soap/envelope/"><S:Body><S:Fault><faultcode>S:Server</faultcode><faultstring>0111</faultstring></S:Fault></S:Body></S:Envelope>"#;
    let SendBillOutcome::Fault { code, .. } = parse_send_bill_response(body).unwrap() else { panic!() };
    assert_eq!(code, "0111");
}

#[test]
fn send_bill_response_without_cdr_is_an_error() {
    let body = br#"<S:Envelope xmlns:S="http://schemas.xmlsoap.org/soap/envelope/"><S:Body><ns2:sendBillResponse xmlns:ns2="http://service.sunat.gob.pe"><applicationResponse/></ns2:sendBillResponse></S:Body></S:Envelope>"#;
    assert_eq!(parse_send_bill_response(body), Err(TransportError::MissingElement("applicationResponse")));
}

#[test]
fn get_status_cdr_response_with_content() {
    let body = format!(
        r#"<S:Envelope xmlns:S="http://schemas.xmlsoap.org/soap/envelope/"><S:Body><ns0:getStatusCdrResponse xmlns:ns0="http://service.sunat.gob.pe"><statusCdr><statusCode>0004</statusCode><content>{}</content><statusMessage>La constancia existe</statusMessage></statusCdr></ns0:getStatusCdrResponse></S:Body></S:Envelope>"#,
        b64(CDR_REJECTED_ZIP)
    );
    let StatusCdrOutcome::Status(status) = parse_get_status_cdr_response(body.as_bytes()).unwrap() else {
        panic!("expected status");
    };
    assert_eq!(status.status_code, "0004");
    assert_eq!(status.status_message, "La constancia existe");
    assert_eq!(read_cdr(&status.cdr.unwrap()).unwrap().response_code, "2047");
}

#[test]
fn get_status_cdr_response_without_content_and_fault() {
    let body = br#"<S:Envelope xmlns:S="http://schemas.xmlsoap.org/soap/envelope/"><S:Body><ns0:getStatusCdrResponse xmlns:ns0="http://service.sunat.gob.pe"><statusCdr><statusCode>0127</statusCode><statusMessage>El ticket no le pertenece al usuario</statusMessage></statusCdr></ns0:getStatusCdrResponse></S:Body></S:Envelope>"#;
    let StatusCdrOutcome::Status(status) = parse_get_status_cdr_response(body).unwrap() else { panic!() };
    assert_eq!((status.status_code.as_str(), status.cdr), ("0127", None));

    let fault = br#"<S:Envelope xmlns:S="http://schemas.xmlsoap.org/soap/envelope/"><S:Body><S:Fault><faultcode>S:Client.0102</faultcode><faultstring>Usuario o contrasena incorrectos</faultstring></S:Fault></S:Body></S:Envelope>"#;
    assert_eq!(
        parse_get_status_cdr_response(fault).unwrap(),
        StatusCdrOutcome::Fault { code: "0102".into(), message: "Usuario o contrasena incorrectos".into() }
    );
}

#[test]
fn read_cdr_accepted_with_notes() {
    let cdr = read_cdr(CDR_XML).unwrap();
    assert_eq!(cdr.reference_id, "FA01-981");
    assert_eq!(cdr.response_code, "0");
    assert_eq!(cdr.description, "La Factura numero FA01-981, ha sido aceptada");
    assert_eq!(
        cdr.notes,
        vec!["4031 - Debe indicar el nombre comercial", "4001 - El numero de RUC del receptor no existe."]
    );
    assert_eq!(cdr.response_code_number(), Some(0));
}

#[test]
fn read_cdr_from_zip_skips_dummy_entry() {
    let cdr = read_cdr(CDR_REJECTED_ZIP).unwrap();
    assert_eq!(cdr.reference_id, "FT01-982");
    assert_eq!(cdr.response_code, "2047");
    assert_eq!(
        cdr.description,
        "Es obligatorio al menos un AdditionalMonetaryTotal con codigo 1001, 1002 o 1003"
    );
    assert!(cdr.notes.is_empty());
}

#[test]
fn read_cdr_decodes_latin1() {
    let xml = b"<?xml version=\"1.0\" encoding=\"ISO-8859-1\"?><ar:ApplicationResponse xmlns:ar=\"a\" xmlns:cac=\"c\" xmlns:cbc=\"b\"><cbc:Note>4000 - Direcci\xF3n</cbc:Note><cac:DocumentResponse><cac:Response><cbc:ReferenceID>B001-9</cbc:ReferenceID><cbc:ResponseCode>2800</cbc:ResponseCode><cbc:Description>N\xFAmero rechazado</cbc:Description></cac:Response></cac:DocumentResponse></ar:ApplicationResponse>";
    let cdr = read_cdr(xml).unwrap();
    assert_eq!(cdr.description, "N\u{fa}mero rechazado");
    assert_eq!(cdr.notes, vec!["4000 - Direcci\u{f3}n"]);
    assert_eq!(read_cdr(b"<a><b/></a>"), Err(TransportError::MissingElement("ResponseCode")));
}

#[test]
fn summaries_and_void_communications_zip_under_their_own_names() {
    for name in ["20123456789-RC-20261016-1", "20123456789-RA-20261016-12345"] {
        let zip = zip_document(name, b"<SummaryDocuments/>").unwrap();
        assert_eq!(unzip_xml(&zip).unwrap(), b"<SummaryDocuments/>", "{name}");
    }
    for name in ["20123456789-RC-2026101-1", "20123456789-RA-20261016-123456", "20123456789-RC-2026101A-1"] {
        assert_eq!(zip_document(name, b"x"), Err(TransportError::InvalidName(name.into())), "{name}");
    }
}

#[test]
fn send_summary_and_get_status_envelopes() {
    let envelope = send_summary_envelope("20123456789MODDATOS", "moddatos", "20123456789-RC-20261016-1.zip", b"PK-test");
    let expected_body = format!(
        "<soapenv:Body><ser:sendSummary><fileName>20123456789-RC-20261016-1.zip</fileName><contentFile>{}</contentFile></ser:sendSummary></soapenv:Body>",
        b64(b"PK-test")
    );
    assert!(envelope.contains(&expected_body), "{envelope}");
    assert!(envelope.contains("<wsse:Username>20123456789MODDATOS</wsse:Username>"));

    let envelope = get_status_envelope("20123456789MODDATOS", "moddatos", "1760620000123").unwrap();
    assert!(envelope.contains("<soapenv:Body><ser:getStatus><ticket>1760620000123</ticket></ser:getStatus></soapenv:Body>"));
    for bad in ["", "17606-20000", "<x>"] {
        assert!(get_status_envelope("u", "p", bad).is_err(), "{bad}");
    }
}

#[test]
fn send_summary_response_gives_the_ticket_or_the_fault() {
    let body = br#"<S:Envelope xmlns:S="http://schemas.xmlsoap.org/soap/envelope/"><S:Body><ns2:sendSummaryResponse xmlns:ns2="http://service.sunat.gob.pe"><ticket>1760620000123</ticket></ns2:sendSummaryResponse></S:Body></S:Envelope>"#;
    assert_eq!(parse_send_summary_response(body).unwrap(), SendSummaryOutcome::Ticket("1760620000123".into()));

    let fault = br#"<S:Envelope xmlns:S="http://schemas.xmlsoap.org/soap/envelope/"><S:Body><S:Fault><faultcode>S:Client.2324</faultcode><faultstring>El identificador de la comunicacion ya fue presentado</faultstring></S:Fault></S:Body></S:Envelope>"#;
    let SendSummaryOutcome::Fault { code, .. } = parse_send_summary_response(fault).unwrap() else { panic!() };
    assert_eq!(code, "2324");

    let empty = br#"<S:Envelope xmlns:S="http://schemas.xmlsoap.org/soap/envelope/"><S:Body><ns2:sendSummaryResponse xmlns:ns2="http://service.sunat.gob.pe"/></S:Body></S:Envelope>"#;
    assert_eq!(parse_send_summary_response(empty), Err(TransportError::MissingElement("ticket")));
}

#[test]
fn get_status_response_in_process_and_processed() {
    let in_process = br#"<S:Envelope xmlns:S="http://schemas.xmlsoap.org/soap/envelope/"><S:Body><ns2:getStatusResponse xmlns:ns2="http://service.sunat.gob.pe"><status><statusCode>98</statusCode></status></ns2:getStatusResponse></S:Body></S:Envelope>"#;
    assert_eq!(
        parse_get_status_response(in_process).unwrap(),
        TicketStatusOutcome::Status(TicketStatus { status_code: "98".into(), cdr: None })
    );

    let processed = format!(
        r#"<S:Envelope xmlns:S="http://schemas.xmlsoap.org/soap/envelope/"><S:Body><ns2:getStatusResponse xmlns:ns2="http://service.sunat.gob.pe"><status><content>{}</content><statusCode>99</statusCode></status></ns2:getStatusResponse></S:Body></S:Envelope>"#,
        b64(CDR_REJECTED_ZIP)
    );
    let TicketStatusOutcome::Status(status) = parse_get_status_response(processed.as_bytes()).unwrap() else { panic!() };
    assert_eq!(status.status_code, "99");
    assert_eq!(read_cdr(&status.cdr.unwrap()).unwrap().response_code, "2047");
}
