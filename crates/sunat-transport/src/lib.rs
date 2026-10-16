//! sunat-transport: sans-IO zip, SOAP envelopes (WS-Security UsernameToken), and parsing of
//! SUNAT responses and CDRs. No network or file I/O happens here; see docs/architecture.md.

mod archive;
mod cdr;
mod envelope;
mod error;
mod response;
mod xml;

pub use archive::{unzip_xml, zip_document, zip_file_name};
pub use cdr::{read_cdr, Cdr};
pub use envelope::{
    get_status_cdr_envelope, get_status_envelope, send_bill_envelope, send_summary_envelope,
    SOAP_ACTION_GET_STATUS, SOAP_ACTION_GET_STATUS_CDR, SOAP_ACTION_SEND_BILL, SOAP_ACTION_SEND_SUMMARY,
};
pub use error::TransportError;
pub use response::{
    parse_get_status_cdr_response, parse_get_status_response, parse_send_bill_response,
    parse_send_summary_response, SendBillOutcome, SendSummaryOutcome, StatusCdr, StatusCdrOutcome,
    TicketStatus, TicketStatusOutcome,
};
