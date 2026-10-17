//! Input types of `docs/input-model.md`. An absent optional field is `None`, and the renderer
//! leaves out the element it would fill.

use serde::Deserialize;

use crate::{DateTime, Decimal};

/// Catalog 01 document types the SDK renders.
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
    /// Base of the lines with affectation 17, sale of rice under the IVAP (`TaxSubtotal` 1016).
    pub ivap_base: Option<Decimal>,
    /// IVAP of those lines, 4 % of `ivap_base`.
    pub ivap: Option<Decimal>,
    /// ICBPER (tax 7152): Σ `lines[].icbper`. Outside the IGV base, inside `total_taxes`, `subtotal`
    /// and `payable`.
    pub icbper: Option<Decimal>,
    pub total_taxes: Option<Decimal>,
    pub sale_value: Option<Decimal>,
    pub subtotal: Option<Decimal>,
    /// `cbc:AllowanceTotalAmount`: line discounts `01` plus global discounts `03`, the ones that do not
    /// reduce the IGV base (SUNAT 3300).
    pub allowance_total: Option<Decimal>,
    /// `cbc:ChargeTotalAmount`: line charges `48` plus global charges `45`, `46` and `50`, the ones that
    /// do not raise the IGV base (SUNAT 3301).
    pub charge_total: Option<Decimal>,
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
    /// ICBPER (tax 7152) of a line of plastic bags, in PEN: `quantity` (the bags) × `icbper_factor`.
    /// Included in `total_taxes` of an onerous line.
    pub icbper: Option<Decimal>,
    /// ICBPER per bag: the rate in force on the year of `issue_date` (SUNAT 4237).
    pub icbper_factor: Option<Decimal>,
    /// Line discounts, invoice and receipt only: catalog 53 code `00` reduces the IGV base and
    /// `line_value` already has it subtracted (SUNAT 3271); `01` does not.
    #[serde(default)]
    pub discounts: Vec<AllowanceCharge>,
    /// Line charges, invoice and receipt only: catalog 53 code `47` raises the IGV base and
    /// `line_value` already has it added (SUNAT 3271); `48` does not.
    #[serde(default)]
    pub charges: Vec<AllowanceCharge>,
}

/// One `cac:AllowanceCharge`, of a line or of the document. `ChargeIndicator` is not an input: it
/// follows from the code (`AllowanceCharge::is_charge`), so SUNAT's 3114 cannot be built. SUNAT
/// checks `amount` against `base_amount` × `factor`, within 1 (3290 line, 3307 global).
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AllowanceCharge {
    /// Catalog 53: `00`, `01` (line discount), `47`, `48` (line charge), `02`, `03` (global discount),
    /// `45`, `46`, `49`, `50` (global charge).
    pub code: String,
    /// Positive, up to 3 integer digits and 5 decimals (`"0.15"`).
    pub factor: Decimal,
    pub amount: Decimal,
    pub base_amount: Decimal,
}

/// The name the line discount had when code `00` was the only one: same type.
pub type LineDiscount = AllowanceCharge;

impl AllowanceCharge {
    /// `ChargeIndicator`: true for the charges of catalog 53 (45 to 50), false for the discounts.
    pub fn is_charge(&self) -> bool {
        matches!(self.code.as_str(), "45" | "46" | "47" | "48" | "49" | "50")
    }
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

/// A despatch guide the document cites (one `cac:DespatchDocumentReference`).
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DespatchReference {
    /// Catalog 01: `"09"` (sender's guide) or `"31"` (carrier's guide).
    pub document_type: String,
    /// `SERIES-NUMBER`, e.g. `T001-45`.
    pub number: String,
}

/// Fields every document (01, 03, 07, 08) carries.
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
    /// Purchase order or service order number (`cac:OrderReference`).
    pub order_reference: Option<String>,
    pub despatch_references: Vec<DespatchReference>,
    pub igv_retention: Option<IgvRetention>,
}

/// IGV the customer withholds as a retention agent, informed in the document as a global
/// `cac:AllowanceCharge` with catalog 53 code `62` (`ChargeIndicator` false). It does not change
/// any total: the customer pays `payable` minus `amount` and deposits `amount` with SUNAT.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IgvRetention {
    /// The retention rate as a factor (`"0.03"` for 3 %).
    pub factor: Decimal,
    /// `base_amount` × `factor`: SUNAT checks it within 1 (3263).
    pub amount: Decimal,
    /// The amount of the operation, IGV included: SUNAT rejects it above `payable` (3264).
    pub base_amount: Decimal,
}

/// Payment to the supplier's Banco de la Nación account that the customer withholds («detracción»,
/// operation types 1001 and 1003). The caller decides whether it applies and computes it; SUNAT checks
/// the codes and that `amount` is in soles.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Detraction {
    /// Catalog 54: goods or service subject to detraction (`"037"`).
    pub code: String,
    /// Catalog 54 rate, as a percentage (`"12.00"`).
    pub percent: Decimal,
    /// In PEN whatever the document currency: SUNAT rejects any other (3208).
    pub amount: Decimal,
    /// The supplier's detraction account at Banco de la Nación.
    pub account: String,
    /// Catalog 59: means of payment (`"001"` deposit in account).
    pub payment_means_code: String,
}

/// Invoice (01) or receipt (03).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Invoice {
    pub voucher: Voucher,
    /// Catalog 51.
    pub operation_type: String,
    pub due_date: Option<DateTime>,
    pub detraction: Option<Detraction>,
    /// Global discounts and charges (catalog 53 `02`, `03`, `45`, `46`, `49`, `50`). `02` and `49`
    /// change the IGV base and the totals already carry them; see `docs/input-model.md`.
    pub global_adjustments: Vec<AllowanceCharge>,
}

/// Credit (07) or debit (08) note.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Note {
    pub voucher: Voucher,
    pub reference: NoteReference,
}
