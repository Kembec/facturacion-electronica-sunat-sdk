//! sunat-model: the input (`docs/input-model.md`), its JSON form and its scope
//! checks.

mod daily;
mod datetime;
mod decimal;
mod despatch;
mod despatch_rules;
mod despatch_transport;
mod error;
mod raw;
mod references;
mod scope;
mod types;
mod validate;

pub use daily::{DocumentReference, Summary, SummaryLine, Voided, VoidedLine};
pub use datetime::{Date, DateTime, LimaDateTime};
pub use decimal::Decimal;
pub use despatch::{Carrier, Despatch, DespatchLine, DespatchParty, Driver, Place, RelatedDocument, Shipment, Vehicle};
pub use error::ModelError;
pub use scope::is_xml_text;
pub use types::{
    Address, Client, Company, DespatchReference, Detraction, DocumentType, Installment, Invoice, Legend, Line, LineDiscount, Note, NoteReference, PaymentTerms,
    Totals, Voucher,
};

/// Documents: invoice or receipt (`Invoice`), credit or debit note (`Note`). Summaries: daily summary
/// (`Summary`, RC) and void communication (`Voided`, RA). Despatch: sender's despatch advice (`Despatch`, 09).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Document {
    Invoice(Invoice),
    Note(Note),
    Summary(Summary),
    Voided(Voided),
    Despatch(Despatch),
}

impl Document {
    /// Parses one JSON document and checks its scope.
    pub fn from_json(json: &str) -> Result<Self, ModelError> {
        let document = raw::parse(json)?;
        document.validate()?;
        Ok(document)
    }

    /// The voucher of a document (01, 03, 07, 08); `None` for a summary or a void communication.
    pub fn voucher(&self) -> Option<&Voucher> {
        match self {
            Document::Invoice(invoice) => Some(&invoice.voucher),
            Document::Note(note) => Some(&note.voucher),
            Document::Summary(_) | Document::Voided(_) | Document::Despatch(_) => None,
        }
    }

    /// The name SUNAT expects for the XML and its zip: `RUC-TYPE-SERIES-NUMBER`, or
    /// `RUC-RC-YYYYMMDD-N` and `RUC-RA-YYYYMMDD-N`; a despatch advice is `RUC-09-SERIES-NUMBER`.
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
            Document::Despatch(despatch) => format!("{}-09-{}", despatch.issuer.ruc, despatch.identifier()),
        }
    }
}
