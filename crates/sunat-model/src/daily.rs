//! Phase 2 input (`docs/input-model.md`, «Phase 2»): the daily summary (RC) of receipts and the
//! notes that modify them, and the void communication (RA). Both go to SUNAT with `sendSummary`.

use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::Value;

use crate::scope::{check_text, reject_keys};
use crate::{Company, Date, Decimal, Document, ModelError};

/// Taxes and regimes a summary line may carry in SUNAT's format but this SDK does not render.
const SUMMARY_LINE_KEYS: &[(&str, &str)] = &[
    ("perception", "perception"),
    ("isc", "ISC"),
    ("icbper", "ICBPER"),
    ("ivap", "IVAP"),
    ("other_taxes", "other taxes"),
];

/// Daily summary (RC).
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Summary {
    /// Correlative of the day: 1 to 5 digits.
    pub number: String,
    /// The day the summary is generated: its `IssueDate` and the date in its identifier.
    pub generation_date: Date,
    /// The day the reported documents were issued: its `ReferenceDate`.
    pub reference_date: Date,
    pub currency: String,
    pub issuer: Company,
    pub lines: Vec<SummaryLine>,
}

/// One receipt (03) or note (07, 08) in a daily summary. Amounts as the caller gives them.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SummaryLine {
    pub document_type: String,
    /// `SERIES-NUMBER`, e.g. `B001-1`.
    pub document_number: String,
    /// Catalog 19: `1` add, `2` modify, `3` void.
    pub status: String,
    /// Catalog 06.
    pub customer_document_type: String,
    pub customer_document_number: String,
    /// The receipt a note modifies; notes only.
    pub reference_document: Option<DocumentReference>,
    pub total: Decimal,
    pub taxed: Option<Decimal>,
    pub exempt: Option<Decimal>,
    pub unaffected: Option<Decimal>,
    pub export: Option<Decimal>,
    pub free: Option<Decimal>,
    pub other_charges: Option<Decimal>,
    pub igv: Option<Decimal>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocumentReference {
    pub document_type: String,
    /// `SERIES-NUMBER`.
    pub document_number: String,
}

/// Void communication (RA).
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Voided {
    /// Correlative of the day: 1 to 5 digits.
    pub number: String,
    /// The day the voided documents were issued: its `ReferenceDate`.
    pub generation_date: Date,
    /// The day the communication is sent: its `IssueDate` and the date in its identifier.
    pub communication_date: Date,
    pub issuer: Company,
    pub lines: Vec<VoidedLine>,
}

/// One invoice (01) or note (07, 08) to void.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VoidedLine {
    pub document_type: String,
    pub series: String,
    pub number: String,
    pub reason: String,
}

impl Summary {
    /// `RC-YYYYMMDD-N`, with the generation date.
    pub fn identifier(&self) -> String {
        format!("RC-{}-{}", self.generation_date.compact(), self.number)
    }
}

impl Voided {
    /// `RA-YYYYMMDD-N`, with the communication date.
    pub fn identifier(&self) -> String {
        format!("RA-{}-{}", self.communication_date.compact(), self.number)
    }
}

/// `kind` is `"summary"` or `"voided"`; the caller has already read it.
pub(crate) fn parse(kind: &str, mut value: Value) -> Result<Document, ModelError> {
    check_text(&value, "$")?;
    if let Value::Object(map) = &mut value {
        map.remove("kind");
    }
    if kind == "voided" {
        return typed(value).map(Document::Voided);
    }
    if let Some(Value::Array(lines)) = value.get("lines") {
        for (index, line) in lines.iter().enumerate() {
            reject_keys(line, &format!("$.lines[{index}]"), SUMMARY_LINE_KEYS)?;
        }
    }
    typed(value).map(Document::Summary)
}

fn typed<T: DeserializeOwned>(value: Value) -> Result<T, ModelError> {
    serde_json::from_value(value).map_err(|e| ModelError::Json(e.to_string()))
}
