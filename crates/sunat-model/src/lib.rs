//! sunat-model: the input (`docs/input-model.md`), its JSON form and its scope
//! checks.

mod advances;
mod allowance_totals;
mod allowances;
mod countries;
mod daily;
mod datetime;
mod decimal;
mod despatch;
mod despatch_rules;
mod despatch_transport;
mod error;
mod export;
mod export_lodging;
mod icbper;
mod icbper_rates;
mod ivap;
mod line_base;
mod raw;
mod references;
mod retention;
mod retention_lines;
mod retention_rules;
mod free_sales;
mod penalty_notes;
mod rule_dates;
mod scope;
mod self_billed;
mod self_billed_lines;
mod self_billed_rules;
mod reversal_rules;
mod total_taxes;
mod types;
mod validate;

pub use daily::{DocumentReference, Summary, SummaryLine, Voided, VoidedLine};
pub use datetime::{Date, DateTime, LimaDateTime};
pub use decimal::Decimal;
pub use despatch::{Carrier, Despatch, DespatchLine, DespatchParty, Driver, Place, RelatedDocument, Shipment, Vehicle};
pub use error::ModelError;
pub use retention::{ExchangeRate, RetainedDocument, Retention, RetentionAmounts, RetentionLine, RetentionPayment, RetentionSupplier};
pub use scope::is_xml_text;
pub use self_billed::{IncomeTaxRetention, ItemProperty, OperationPlace, SelfBilled, SelfBilledLine, SelfBilledSeller, SelfBilledTotals, SellerAddress};
pub use types::{
    Address, Advance, AllowanceCharge, Client, Company, DespatchReference, Detraction, DocumentType, IgvRetention, Installment, Invoice, Legend, Line, LineDiscount, Lodging, Note, NoteReference, PaymentTerms,
    Totals, Voucher,
};

/// Documents: invoice or receipt (`Invoice`), credit or debit note (`Note`). Summaries: daily summary
/// (`Summary`, RC) and void communication (`Voided`, RA). Despatch: sender's despatch advice (`Despatch`, 09).
/// Retention: retention voucher (`Retention`, 20). Purchase settlement (`SelfBilled`, 04) and its reversal
/// summary (`Reversal`, RR), which has the shape of a void communication.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Document {
    Invoice(Invoice),
    Note(Note),
    Summary(Summary),
    Voided(Voided),
    Despatch(Despatch),
    Retention(Retention),
    SelfBilled(SelfBilled),
    Reversal(Voided),
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
            Document::Summary(_) | Document::Voided(_) | Document::Despatch(_) | Document::Retention(_) | Document::SelfBilled(_) | Document::Reversal(_) => None,
        }
    }

    /// The name SUNAT expects for the XML and its zip: `RUC-TYPE-SERIES-NUMBER`, or
    /// `RUC-RC-YYYYMMDD-N` and `RUC-RA-YYYYMMDD-N`; a despatch advice is `RUC-09-SERIES-NUMBER` and a
    /// retention voucher `RUC-20-SERIES-NUMBER`; a purchase settlement, `RUC-04-SERIES-NUMBER`, and its reversal,
    /// `RUC-RR-YYYYMMDD-N`.
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
            Document::Retention(retention) => format!("{}-20-{}", retention.issuer.ruc, retention.identifier()),
            Document::SelfBilled(settlement) => format!("{}-04-{}", settlement.issuer.ruc, settlement.identifier()),
            Document::Reversal(reversal) => format!("{}-{}", reversal.issuer.ruc, reversal.reversal_identifier()),
        }
    }
}

// The README examples compile against this crate as doctests, so the page on crates.io cannot drift
// from the API.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;
