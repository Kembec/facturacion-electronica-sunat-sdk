//! Input types of `docs/input-model.md`. An absent optional field is `None`, and the renderer
//! leaves out the element it would fill.

use serde::Deserialize;

use crate::{DateTime, Decimal};

/// Catalog 01 document types of phase 1.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DocumentType {
    Invoice,
    Receipt,
    CreditNote,
    DebitNote,
}

impl DocumentType {
    pub fn code(self) -> &'static str {
        match self {
            Self::Invoice => "01",
            Self::Receipt => "03",
            Self::CreditNote => "07",
            Self::DebitNote => "08",
        }
    }

    pub fn from_code(code: &str) -> Option<Self> {
        match code {
            "01" => Some(Self::Invoice),
            "03" => Some(Self::Receipt),
            "07" => Some(Self::CreditNote),
            "08" => Some(Self::DebitNote),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Address {
    pub ubigeo: Option<String>,
    pub country_code: Option<String>,
    pub department: Option<String>,
    pub province: Option<String>,
    pub district: Option<String>,
    pub urbanization: Option<String>,
    pub street: Option<String>,
    pub establishment_code: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Company {
    pub ruc: String,
    pub legal_name: String,
    pub trade_name: Option<String>,
    pub email: Option<String>,
    pub telephone: Option<String>,
    pub address: Address,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Client {
    pub document_type: String,
    pub document_number: String,
    pub legal_name: String,
    pub address: Option<Address>,
    pub email: Option<String>,
    pub telephone: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Totals {
    pub taxed: Option<Decimal>,
    pub unaffected: Option<Decimal>,
    pub exempt: Option<Decimal>,
    pub export: Option<Decimal>,
    pub free: Option<Decimal>,
    pub free_igv: Option<Decimal>,
    pub igv: Option<Decimal>,
    pub total_taxes: Option<Decimal>,
    pub sale_value: Option<Decimal>,
    pub subtotal: Option<Decimal>,
    pub rounding: Option<Decimal>,
    pub payable: Option<Decimal>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Line {
    pub unit: String,
    pub quantity: Decimal,
    pub product_code: Option<String>,
    pub sunat_product_code: Option<String>,
    pub description: String,
    pub unit_value: Decimal,
    pub igv_base: Decimal,
    pub igv_rate: Decimal,
    pub igv: Decimal,
    /// Catalog 07.
    pub affectation: String,
    pub total_taxes: Decimal,
    pub unit_price: Option<Decimal>,
    pub line_value: Decimal,
    pub free_unit_value: Option<Decimal>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Legend {
    pub code: String,
    pub value: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(tag = "type", deny_unknown_fields)]
pub enum PaymentTerms {
    #[serde(rename = "Contado")]
    Cash,
    #[serde(rename = "Credito")]
    Credit {
        amount: Decimal,
        #[serde(default)]
        currency: Option<String>,
        #[serde(default)]
        installments: Vec<Installment>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Installment {
    pub amount: Decimal,
    pub currency: Option<String>,
    pub due_date: DateTime,
}

/// The document a credit or debit note modifies, and why.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NoteReference {
    /// Catalog 09 (credit) or 10 (debit).
    pub reason_code: String,
    pub reason: String,
    pub affected_document_type: String,
    /// `SERIES-NUMBER`, e.g. `F001-123`.
    pub affected_document_number: String,
}

/// Fields every phase 1 document carries.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Voucher {
    pub ubl_version: String,
    pub document_type: DocumentType,
    pub series: String,
    pub number: String,
    pub issue_date: DateTime,
    pub currency: String,
    pub payment_terms: Option<PaymentTerms>,
    pub issuer: Company,
    pub customer: Client,
    pub totals: Totals,
    pub lines: Vec<Line>,
    pub legends: Vec<Legend>,
}

/// Invoice (01) or receipt (03).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Invoice {
    pub voucher: Voucher,
    /// Catalog 51.
    pub operation_type: String,
    pub due_date: Option<DateTime>,
}

/// Credit (07) or debit (08) note.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Note {
    pub voucher: Voucher,
    pub reference: NoteReference,
}
