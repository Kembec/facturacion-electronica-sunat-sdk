//! Parsing of sendBill, sendSummary, getStatus and getStatusCdr responses, including SOAP faults.

use base64::engine::general_purpose::STANDARD;
use base64::Engine;

use crate::xml::{decode, has_suffix, nodes, text_by_suffix, Node};
use crate::TransportError;

/// Result of sendBill: the CDR zip, or the exception SUNAT raised as a SOAP fault.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SendBillOutcome {
    Cdr(Vec<u8>),
    /// `code` holds only the digits of `faultcode` (or of `faultstring` when the code has none),
    /// so `soap-env:Client.1033` gives `1033`; `message` is the text of `detail` when it has one (an
    /// OSE writes the code in `faultstring` and the description there), else `faultstring`.
    Fault { code: String, message: String },
}

/// Body of a getStatusCdr answer. `cdr` is the CDR zip when SUNAT attaches one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusCdr {
    pub status_code: String,
    pub status_message: String,
    pub cdr: Option<Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatusCdrOutcome {
    Status(StatusCdr),
    Fault { code: String, message: String },
}

/// Result of sendSummary: the ticket to ask getStatus with, or the exception SUNAT raised.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SendSummaryOutcome {
    Ticket(String),
    Fault { code: String, message: String },
}

/// Body of a getStatus answer. `status_code` is `0` (processed; the CDR says how), `98` (still
/// in process: ask again with the same ticket) or `99` (processed with errors; the CDR says
/// which). `cdr` is the CDR zip when SUNAT attaches one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TicketStatus {
    pub status_code: String,
    pub cdr: Option<Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TicketStatusOutcome {
    Status(TicketStatus),
    Fault { code: String, message: String },
}

/// Parses the HTTP body of a sendSummary call.
pub fn parse_send_summary_response(body: &[u8]) -> Result<SendSummaryOutcome, TransportError> {
    let nodes = nodes(&decode(body)?)?;
    if let Some((code, message)) = fault(&nodes) {
        return Ok(SendSummaryOutcome::Fault { code, message });
    }
    let ticket = text_by_suffix(&nodes, &["sendSummaryResponse", "ticket"])
        .ok_or(TransportError::MissingElement("ticket"))?;
    Ok(SendSummaryOutcome::Ticket(ticket))
}

/// Parses the HTTP body of a getStatus call.
pub fn parse_get_status_response(body: &[u8]) -> Result<TicketStatusOutcome, TransportError> {
    let nodes = nodes(&decode(body)?)?;
    if let Some((code, message)) = fault(&nodes) {
        return Ok(TicketStatusOutcome::Fault { code, message });
    }
    let status_code = text_by_suffix(&nodes, &["getStatusResponse", "status", "statusCode"])
        .ok_or(TransportError::MissingElement("statusCode"))?;
    let cdr = text_by_suffix(&nodes, &["getStatusResponse", "status", "content"])
        .map(|c| decode_base64(&c))
        .transpose()?;
    Ok(TicketStatusOutcome::Status(TicketStatus { status_code, cdr }))
}

/// Parses the HTTP body of a sendBill call (a 200 answer or a 500 SOAP fault).
pub fn parse_send_bill_response(body: &[u8]) -> Result<SendBillOutcome, TransportError> {
    let nodes = nodes(&decode(body)?)?;
    if let Some((code, message)) = fault(&nodes) {
        return Ok(SendBillOutcome::Fault { code, message });
    }
    let content = text_by_suffix(&nodes, &["sendBillResponse", "applicationResponse"])
        .ok_or(TransportError::MissingElement("applicationResponse"))?;
    Ok(SendBillOutcome::Cdr(decode_base64(&content)?))
}

/// Parses the HTTP body of a getStatusCdr call.
pub fn parse_get_status_cdr_response(body: &[u8]) -> Result<StatusCdrOutcome, TransportError> {
    let nodes = nodes(&decode(body)?)?;
    if let Some((code, message)) = fault(&nodes) {
        return Ok(StatusCdrOutcome::Fault { code, message });
    }
    if !has_suffix(&nodes, &["getStatusCdrResponse", "statusCdr"]) {
        return Err(TransportError::MissingElement("statusCdr"));
    }
    let status_code = text_by_suffix(&nodes, &["statusCdr", "statusCode"])
        .ok_or(TransportError::MissingElement("statusCode"))?;
    let status_message = text_by_suffix(&nodes, &["statusCdr", "statusMessage"]).unwrap_or_default();
    let cdr = text_by_suffix(&nodes, &["statusCdr", "content"])
        .map(|c| decode_base64(&c))
        .transpose()?;
    Ok(StatusCdrOutcome::Status(StatusCdr { status_code, status_message, cdr }))
}

fn fault(nodes: &[Node]) -> Option<(String, String)> {
    if !has_suffix(nodes, &["Body", "Fault"]) {
        return None;
    }
    let raw_code = text_by_suffix(nodes, &["Fault", "faultcode"]).unwrap_or_default();
    let fault_string = text_by_suffix(nodes, &["Fault", "faultstring"]).unwrap_or_default();
    let mut code = digits(&raw_code);
    if code.is_empty() {
        code = digits(&fault_string);
    }
    let message = text_by_suffix(nodes, &["Fault", "detail"]).unwrap_or(fault_string);
    Some((code, message))
}

fn digits(value: &str) -> String {
    value.chars().filter(char::is_ascii_digit).collect()
}

fn decode_base64(value: &str) -> Result<Vec<u8>, TransportError> {
    let compact: String = value.chars().filter(|c| !c.is_whitespace()).collect();
    STANDARD.decode(compact).map_err(|e| TransportError::Base64(e.to_string()))
}
