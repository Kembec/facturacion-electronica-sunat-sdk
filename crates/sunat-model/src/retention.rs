//! Retention voucher input (`docs/input-model.md`, «Retention voucher (20)»): the comprobante de
//! retención electrónico, UBL 2.0 `Retention` with SUNAT's customization 1.0.
//!
//! The caller gives every amount and the SDK reconciles them: what SUNAT's validation rules (sheet
//! «Retenciones1_0») check on the document itself is checked in `retention_rules` and
//! `retention_lines`, each with its SUNAT code. The rules that query SUNAT's registries (agent status
//! 2617, supplier RUC 2621, numbering already used 1033, supplier condition 4091) are not.

use serde::Deserialize;
use serde_json::Value;

use crate::scope::check_text;
use crate::{Company, Date, DateTime, Decimal, Document, ModelError};

/// Retention voucher (20).
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Retention {
    /// `"2.0"`.
    pub ubl_version: String,
    /// Catalog 01: `"20"`.
    pub document_type: String,
    /// `R` and 3 uppercase letters or digits (SUNAT 1001).
    pub series: String,
    /// 1 to 8 digits, as issued (zero padding is kept).
    pub number: String,
    pub issue_date: DateTime,
    /// The retention agent.
    pub issuer: Company,
    /// The supplier whose payments are retained (proveedor).
    pub supplier: RetentionSupplier,
    /// Catalog 23: `"01"` or `"02"`.
    pub regime: String,
    /// Percentage of the regime, `"3.00"` for 01.
    pub rate: Decimal,
    /// `cbc:Note`, up to 250 characters.
    pub note: Option<String>,
    /// PEN: the sum of the lines' retained amounts (SUNAT 2628).
    pub total_retained: Decimal,
    /// PEN: the sum of the lines' net paid amounts plus `rounding_amount` (SUNAT 2629).
    pub total_paid: Decimal,
    /// PEN, absolute value up to 1 (`cbc:PayableRoundingAmount`).
    pub rounding_amount: Option<Decimal>,
    /// One per payment of a related document.
    pub lines: Vec<RetentionLine>,
}

/// The supplier, identified by its RUC.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RetentionSupplier {
    /// Catalog 06: `"6"`.
    pub document_type: String,
    pub document_number: String,
    pub legal_name: String,
}

/// One payment of a related document and what was retained from it.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RetentionLine {
    pub related_document: RetainedDocument,
    pub payment: RetentionPayment,
    pub retention: RetentionAmounts,
    /// Required when the related document is not in PEN.
    pub exchange_rate: Option<ExchangeRate>,
}

/// `sac:SUNATRetentionDocumentReference`: the document paid.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RetainedDocument {
    /// Catalog 01: `"01"`, `"08"` or `"12"`.
    pub document_type: String,
    /// `SERIES-NUMBER`.
    pub number: String,
    pub issue_date: Date,
    /// ISO 4217.
    pub currency: String,
    pub total: Decimal,
}

/// `cac:Payment`, in the currency of the related document.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RetentionPayment {
    /// Up to 9 digits, unique for the same related document within the voucher (SUNAT 2626). By
    /// default 1, 2, … in the order the document appears.
    pub number: Option<u64>,
    pub date: Date,
    pub amount: Decimal,
}

/// `sac:SUNATRetentionInformation`, in PEN.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RetentionAmounts {
    /// By default the payment date: the retention is made when paying.
    pub date: Option<Date>,
    pub amount: Decimal,
    pub net_paid: Decimal,
}

/// `cac:ExchangeRate`: from the related document's currency to PEN.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExchangeRate {
    pub source_currency: Option<String>,
    pub target_currency: Option<String>,
    /// Up to 4 integer digits and 6 decimals.
    pub rate: Option<Decimal>,
    pub date: Option<Date>,
}

impl Retention {
    /// `SERIES-NUMBER`, e.g. `R001-1`.
    pub fn identifier(&self) -> String {
        format!("{}-{}", self.series, self.number)
    }

    /// The payment number of line `index`: the one given, or how many lines up to this one pay the
    /// same related document.
    pub fn payment_number(&self, index: usize) -> u64 {
        let line = &self.lines[index];
        if let Some(number) = line.payment.number {
            return number;
        }
        let same = |other: &&RetentionLine| other.related_document.number == line.related_document.number;
        self.lines[..=index].iter().filter(same).count() as u64
    }

    /// SUNAT 2600: the voucher reaches SUNAT within `max_days` calendar days counted from the day
    /// after its issue date, in Lima. The term is a parameter, not a constant: SUNAT's rules defer it
    /// to «the regulation in force» (7 days by RS 274-2015 art. 41). Not part of `validate`, which has
    /// no clock.
    pub fn check_send_deadline(&self, now: DateTime, max_days: u32) -> Result<(), ModelError> {
        // Peru keeps UTC-05:00 all year (see `datetime`): the Lima calendar day of an instant.
        let lima_day = |at: DateTime| (at.unix_seconds() - 5 * 3600).div_euclid(86_400);
        let elapsed_days = lima_day(now) - lima_day(self.issue_date);
        if elapsed_days > i64::from(max_days) {
            let reason = format!("sent {elapsed_days} days after the issue date; the term is {max_days}");
            return Err(ModelError::rule("2600", "$.issue_date", reason));
        }
        Ok(())
    }
}

impl RetentionLine {
    /// The retention date: the one given, or the payment date.
    pub fn retention_date(&self) -> Date {
        self.retention.date.unwrap_or(self.payment.date)
    }
}

/// `kind` is `"retention"`; the caller has already read it.
pub(crate) fn parse(mut value: Value) -> Result<Document, ModelError> {
    check_text(&value, "$")?;
    if let Value::Object(map) = &mut value {
        map.remove("kind");
    }
    serde_json::from_value(value).map(Document::Retention).map_err(|e| ModelError::Json(e.to_string()))
}
