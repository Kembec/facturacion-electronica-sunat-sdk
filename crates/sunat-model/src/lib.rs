//! sunat-model: the input of phase 1 (`docs/input-model.md`), its JSON form and its scope checks.

mod datetime;
mod decimal;
mod error;
mod raw;
mod scope;
mod types;
mod validate;

pub use datetime::{DateTime, LimaDateTime};
pub use decimal::Decimal;
pub use error::ModelError;
pub use scope::is_xml_text;
pub use types::{
    Address, Client, Company, DocumentType, Installment, Invoice, Legend, Line, Note, NoteReference, PaymentTerms,
    Totals, Voucher,
};

/// A phase 1 document: invoice or receipt (`Invoice`), credit or debit note (`Note`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Document {
    Invoice(Invoice),
    Note(Note),
}

impl Document {
    /// Parses one JSON document and checks its scope.
    pub fn from_json(json: &str) -> Result<Self, ModelError> {
        let document = raw::parse(json)?;
        document.validate()?;
        Ok(document)
    }

    pub fn voucher(&self) -> &Voucher {
        match self {
            Document::Invoice(invoice) => &invoice.voucher,
            Document::Note(note) => &note.voucher,
        }
    }

    /// `RUC-TYPE-SERIES-NUMBER`, the name SUNAT expects for the XML and its zip.
    pub fn file_name(&self) -> String {
        let voucher = self.voucher();
        format!(
            "{}-{}-{}-{}",
            voucher.issuer.ruc,
            voucher.document_type.code(),
            voucher.series,
            voucher.number
        )
    }
}
