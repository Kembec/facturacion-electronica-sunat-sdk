//! sunat-model: the input of phases 1 and 2 (`docs/input-model.md`), its JSON form and its scope
//! checks.

mod daily;
mod datetime;
mod decimal;
mod error;
mod raw;
mod scope;
mod types;
mod validate;

pub use daily::{DocumentReference, Summary, SummaryLine, Voided, VoidedLine};
pub use datetime::{Date, DateTime, LimaDateTime};
pub use decimal::Decimal;
pub use error::ModelError;
pub use scope::is_xml_text;
pub use types::{
    Address, Client, Company, DocumentType, Installment, Invoice, Legend, Line, LineDiscount, Note, NoteReference, PaymentTerms,
    Totals, Voucher,
};

/// Phase 1: invoice or receipt (`Invoice`), credit or debit note (`Note`). Phase 2: daily summary
/// (`Summary`, RC) and void communication (`Voided`, RA).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Document {
    Invoice(Invoice),
    Note(Note),
    Summary(Summary),
    Voided(Voided),
}

impl Document {
    /// Parses one JSON document and checks its scope.
    pub fn from_json(json: &str) -> Result<Self, ModelError> {
        let document = raw::parse(json)?;
        document.validate()?;
        Ok(document)
    }

    /// The voucher of a phase 1 document; `None` for a summary or a void communication.
    pub fn voucher(&self) -> Option<&Voucher> {
        match self {
            Document::Invoice(invoice) => Some(&invoice.voucher),
            Document::Note(note) => Some(&note.voucher),
            Document::Summary(_) | Document::Voided(_) => None,
        }
    }

    /// The name SUNAT expects for the XML and its zip: `RUC-TYPE-SERIES-NUMBER`, or
    /// `RUC-RC-YYYYMMDD-N` and `RUC-RA-YYYYMMDD-N`.
    pub fn file_name(&self) -> String {
        match self {
            Document::Invoice(Invoice { voucher, .. }) | Document::Note(Note { voucher, .. }) => format!(
                "{}-{}-{}-{}",
                voucher.issuer.ruc,
                voucher.document_type.code(),
                voucher.series,
                voucher.number
            ),
            Document::Summary(summary) => format!("{}-{}", summary.issuer.ruc, summary.identifier()),
            Document::Voided(voided) => format!("{}-{}", voided.issuer.ruc, voided.identifier()),
        }
    }
}
