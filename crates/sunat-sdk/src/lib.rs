//! sunat-sdk: facade over the SDK crates and classification of SUNAT response codes.
//! See docs/architecture.md.

mod classify;
pub mod diagnosis;
pub mod exchange;
pub mod gre;
pub mod validity;
mod validity_input;

pub use classify::{classify, classify_cdr, classify_code, ResponseClass};
pub use diagnosis::{diagnose, Diagnosis};
pub use sunat_transport as transport;
pub use sunat_transport::{
    get_status_cdr_envelope, get_status_envelope, parse_get_status_cdr_response,
    parse_get_status_response, parse_send_bill_response, parse_send_summary_response, read_cdr,
    send_bill_envelope, send_summary_envelope, unzip_xml, zip_document, zip_file_name, Cdr,
    SendBillOutcome, SendSummaryOutcome, StatusCdr, StatusCdrOutcome, TicketStatus,
    TicketStatusOutcome, TransportError,
};

// The README examples compile against this crate as doctests, so the page on crates.io cannot drift
// from the API.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;
