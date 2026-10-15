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
pub use envelope::{get_status_cdr_envelope, send_bill_envelope, SOAP_ACTION_GET_STATUS_CDR, SOAP_ACTION_SEND_BILL};
pub use error::TransportError;
pub use response::{
    parse_get_status_cdr_response, parse_send_bill_response, SendBillOutcome, StatusCdr,
    StatusCdrOutcome,
};
