//! sunat-sdk: facade over the SDK crates and classification of SUNAT response codes.
//! See docs/architecture.md.

mod classify;

pub use classify::{classify, classify_cdr, classify_code, ResponseClass};
pub use sunat_transport as transport;
pub use sunat_transport::{
    get_status_cdr_envelope, parse_get_status_cdr_response, parse_send_bill_response, read_cdr,
    send_bill_envelope, unzip_xml, zip_document, zip_file_name, Cdr, SendBillOutcome, StatusCdr,
    StatusCdrOutcome, TransportError,
};
