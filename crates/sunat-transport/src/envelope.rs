//! SOAP 1.1 envelopes for billService.sendBill and billConsultService.getStatusCdr, with the
//! WS-Security UsernameToken header (PasswordText) of the SUNAT programmer's manual (SEE v2.1).

use base64::engine::general_purpose::STANDARD;
use base64::Engine;

use crate::archive::{is_alnum, is_digits};
use crate::TransportError;

pub const SOAP_ACTION_SEND_BILL: &str = "urn:sendBill";
pub const SOAP_ACTION_GET_STATUS_CDR: &str = "urn:getStatusCdr";

const ENVELOPE_OPEN: &str = concat!(
    r#"<soapenv:Envelope xmlns:soapenv="http://schemas.xmlsoap.org/soap/envelope/""#,
    r#" xmlns:ser="http://service.sunat.gob.pe""#,
    r#" xmlns:wsse="http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-wssecurity-secext-1.0.xsd">"#,
);
const PASSWORD_TYPE: &str =
    "http://docs.oasis-open.org/wss/2004/01/oasis-200401-wss-username-token-profile-1.0#PasswordText";

/// sendBill envelope. `sol_user_full` is the RUC followed by the SOL user (e.g. `20100066603MODDATOS`);
/// `zip_name` is the zip file name (`RUC-TIPO-SERIE-NUMERO.zip`); `zip` goes base64-encoded.
pub fn send_bill_envelope(sol_user_full: &str, sol_password: &str, zip_name: &str, zip: &[u8]) -> String {
    let body = format!(
        "<ser:sendBill><fileName>{}</fileName><contentFile>{}</contentFile></ser:sendBill>",
        escape(zip_name),
        STANDARD.encode(zip),
    );
    envelope(sol_user_full, sol_password, &body)
}

/// getStatusCdr envelope for the document `ruc`-`document_type`-`series`-`number`.
pub fn get_status_cdr_envelope(
    sol_user_full: &str,
    sol_password: &str,
    ruc: &str,
    document_type: &str,
    series: &str,
    number: u32,
) -> Result<String, TransportError> {
    if !is_digits(ruc, 11, 11) {
        return Err(TransportError::InvalidArgument("ruc must have 11 digits"));
    }
    if !is_alnum(document_type, 2, 2) {
        return Err(TransportError::InvalidArgument("document_type must have 2 characters"));
    }
    if !is_alnum(series, 4, 4) {
        return Err(TransportError::InvalidArgument("series must have 4 characters"));
    }
    let body = format!(
        "<ser:getStatusCdr><rucComprobante>{ruc}</rucComprobante><tipoComprobante>{document_type}</tipoComprobante><serieComprobante>{series}</serieComprobante><numeroComprobante>{number}</numeroComprobante></ser:getStatusCdr>",
    );
    Ok(envelope(sol_user_full, sol_password, &body))
}

fn envelope(user: &str, password: &str, body: &str) -> String {
    format!(
        "{ENVELOPE_OPEN}<soapenv:Header><wsse:Security><wsse:UsernameToken><wsse:Username>{}</wsse:Username><wsse:Password Type=\"{PASSWORD_TYPE}\">{}</wsse:Password></wsse:UsernameToken></wsse:Security></soapenv:Header><soapenv:Body>{body}</soapenv:Body></soapenv:Envelope>",
        escape(user),
        escape(password),
    )
}

fn escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for c in value.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(c),
        }
    }
    out
}
