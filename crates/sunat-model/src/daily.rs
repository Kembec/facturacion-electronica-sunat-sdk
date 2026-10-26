//! Summary input (`docs/input-model.md`, «Daily summary (RC) and void communication (RA)»): the daily summary (RC) of receipts and the
//! notes that modify them, and the void communication (RA). Both go to SUNAT with `sendSummary`.

use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::Value;

use crate::scope::check_text;
use crate::{Company, Date, Decimal, Document, ModelError};

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
    /// Field 10.3: the customer's name or legal name, 3 to 250 characters (SUNAT error 2022).
    pub customer_name: Option<String>,
    /// Rate of the IGV (`"18.00"`, `"10.50"`), rendered as `cbc:Percent` of tax 1000 (field 21).
    pub igv_rate: Option<Decimal>,
    /// The receipt a note modifies; notes only.
    pub reference_document: Option<DocumentReference>,
    pub total: Decimal,
    pub taxed: Option<Decimal>,
    pub exempt: Option<Decimal>,
    pub unaffected: Option<Decimal>,
    pub export: Option<Decimal>,
    pub free: Option<Decimal>,
    /// Free sales by catalog 11 code, from 2027-01-01 (`reference_date`): 06 taxed, 07 exempt, 08 unaffected,
    /// 09 export. Before it only `free` (05) exists.
    pub free_taxed: Option<Decimal>,
    pub free_exempt: Option<Decimal>,
    pub free_unaffected: Option<Decimal>,
    pub free_export: Option<Decimal>,
    /// IGV of free sales (tax 9996, field 25), from 2027-01-01, as its own `cac:TaxTotal` after the IGV one.
    pub free_igv: Option<Decimal>,
    pub other_charges: Option<Decimal>,
    pub igv: Option<Decimal>,
    /// ICBPER (tax 7152) of the reported document, in PEN; `total` includes it.
    pub icbper: Option<Decimal>,
    /// IVAP (tax 1016) of a receipt or note of rice under affectation 17, in place of the IGV: excludes `igv` and
    /// `igv_rate` (2355), and the line then needs no `igv_rate`.
    pub ivap: Option<Decimal>,
    /// ISC (tax 2000).
    pub isc: Option<Decimal>,
    /// Other taxes (tax 9999).
    pub other_taxes: Option<Decimal>,
    /// IGV perception charged on a receipt (03) that is added or voided, never modified (2986).
    pub perception: Option<SummaryPerception>,
}

/// `sac:SUNATPerceptionSummaryDocumentReference` of a summary line, in PEN. The rate is not input: the SDK takes it
/// from the regime's code in catalog 22.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SummaryPerception {
    /// Catalog 22: `01` (2 %), `02` (1 %), `03` (0.5 %).
    pub system_code: String,
    /// The amount the perception applies to (`cbc:TaxableAmount`).
    pub base: Decimal,
    /// The perception (`cbc:TotalInvoiceAmount`): `base` × the regime's rate.
    pub amount: Decimal,
    /// What the customer pays with the perception (`sac:SUNATTotalCashed`): the line's `total` + `amount`.
    pub total_cashed: Decimal,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocumentReference {
    pub document_type: String,
    /// `SERIES-NUMBER`.
    pub document_number: String,
}

/// Void communication (RA) and, as `Document::Reversal`, the reversal summary (RR) of a purchase
/// settlement (04): the same document with another identifier.
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

/// One invoice (01) or note (07, 08) to void, or one purchase settlement (04) to reverse.
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

    /// `RR-YYYYMMDD-N`, with the communication date: the identifier when it reverses a purchase settlement.
    pub fn reversal_identifier(&self) -> String {
        format!("RR-{}-{}", self.communication_date.compact(), self.number)
    }
}

/// `kind` is `"summary"`, `"voided"` or `"reversal"`; the caller has already read it.
pub(crate) fn parse(kind: &str, mut value: Value) -> Result<Document, ModelError> {
    check_text(&value, "$")?;
    if let Value::Object(map) = &mut value {
        map.remove("kind");
    }
    if kind == "voided" {
        return typed(value).map(Document::Voided);
    }
    if kind == "reversal" {
        return typed(value).map(Document::Reversal);
    }
    typed(value).map(Document::Summary)
}

fn typed<T: DeserializeOwned>(value: Value) -> Result<T, ModelError> {
    serde_json::from_value(value).map_err(|e| ModelError::Json(e.to_string()))
}
