//! Purchase settlement input (`docs/input-model.md`, «Purchase settlement (04)»): the liquidación de
//! compra electrónica, UBL 2.1 `SelfBilledInvoice` with SUNAT's customization 2.0. The buyer issues it
//! for a purchase from a natural person without RUC; the roles of the invoice are reversed.
//!
//! The caller gives every amount and the SDK reconciles them against SUNAT's validation rules (sheet
//! «LiquidacionCompra2_0»), in `self_billed_rules` and `self_billed_lines`. Whether a settlement may be
//! issued at all (the seller's condition, the goods, the 75 UIT ceiling: SUNAT 2451) and the rules that
//! query SUNAT's registries (the seller's DNI 2450 to 2462, the buyer's RUC 2010, 2011, 2463) are not.

use serde::Deserialize;
use serde_json::Value;

use crate::scope::check_text;
use crate::{Company, DateTime, Decimal, DespatchReference, Document, Legend, ModelError};

/// Purchase settlement (04).
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SelfBilled {
    /// `"2.1"`.
    pub ubl_version: String,
    /// Catalog 51: `"0501"` purchase, `"0503"` purchase of gold.
    pub operation_type: String,
    /// Catalog 01: `"04"`.
    pub document_type: String,
    /// `L` and 3 uppercase letters or digits (SUNAT 1001).
    pub series: String,
    /// 1 to 8 digits, as issued.
    pub number: String,
    pub issue_date: DateTime,
    pub currency: String,
    /// The buyer, who issues the settlement (`cac:AccountingCustomerParty`).
    pub issuer: Company,
    /// The natural person who sells (`cac:AccountingSupplierParty`).
    pub seller: SelfBilledSeller,
    /// Where the purchase took place (`cac:DeliveryTerms`); required (SUNAT 2454).
    pub operation_place: Option<OperationPlace>,
    pub lines: Vec<SelfBilledLine>,
    pub totals: SelfBilledTotals,
    #[serde(default)]
    pub legends: Vec<Legend>,
    #[serde(default)]
    pub despatch_references: Vec<DespatchReference>,
}

/// The seller: a DNI, a carné de extranjería or a passport, never a RUC (SUNAT 2800).
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SelfBilledSeller {
    /// Catalog 06: `"1"`, `"4"` or `"7"`.
    pub document_type: String,
    pub document_number: String,
    pub legal_name: String,
    pub address: SellerAddress,
}

/// The seller's address. `street` and `ubigeo` are required (SUNAT 2452, 2453).
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SellerAddress {
    pub ubigeo: Option<String>,
    pub country_code: Option<String>,
    pub department: Option<String>,
    pub province: Option<String>,
    pub district: Option<String>,
    pub urbanization: Option<String>,
    pub street: Option<String>,
    /// Catalog 60: point of sale, production, extraction or none (SUNAT 2456).
    pub address_type: Option<String>,
}

/// Where the purchase took place. `street`, `ubigeo` and `location_type` are required (SUNAT 2454,
/// 2455, 2458).
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperationPlace {
    pub ubigeo: Option<String>,
    pub country_code: Option<String>,
    pub department: Option<String>,
    pub province: Option<String>,
    pub district: Option<String>,
    pub urbanization: Option<String>,
    pub street: Option<String>,
    /// Catalog 60.
    pub location_type: Option<String>,
}

/// One purchased item, with the IGV the buyer pays for the seller (tax 1000, 9997 or 9998) and the
/// income tax the buyer withholds (tax 3000).
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SelfBilledLine {
    pub unit: String,
    pub quantity: Decimal,
    pub product_code: Option<String>,
    /// Catalog 25, 8 digits; required (SUNAT 3506).
    pub sunat_product_code: Option<String>,
    pub description: String,
    pub unit_value: Decimal,
    /// Unit price with IGV (`PricingReference`, code 01); required (SUNAT 2028).
    pub unit_price: Option<Decimal>,
    pub igv_base: Decimal,
    pub igv_rate: Decimal,
    pub igv: Decimal,
    /// Catalog 07: `"10"` taxed, `"20"` exempt, `"30"` unaffected.
    pub affectation: String,
    /// The line's IGV: the income tax retention is not a tax of the line total (SUNAT 4293).
    pub total_taxes: Decimal,
    pub line_value: Decimal,
    /// Required on every line (SUNAT 2464 rejects a line without it).
    pub income_tax_retention: Option<IncomeTaxRetention>,
    /// Operation type 0503 only: catalog 55 codes 6000, 6004, 6005 and 6006.
    #[serde(default)]
    pub item_properties: Vec<ItemProperty>,
}

/// The income tax the buyer withholds from the line (tax 3000): `amount` = `base` × `rate` / 100.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IncomeTaxRetention {
    /// The line value (SUNAT 4294).
    pub base: Decimal,
    /// Percent: `"1.50"`, or `"4.00"` for gold.
    pub rate: Decimal,
    pub amount: Decimal,
}

/// `cac:AdditionalItemProperty`: a catalog 55 code and its value.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemProperty {
    pub code: String,
    pub value: String,
}

/// Totals of the settlement (`docs/input-model.md` gives each formula).
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SelfBilledTotals {
    pub taxed: Option<Decimal>,
    pub exempt: Option<Decimal>,
    pub unaffected: Option<Decimal>,
    pub igv: Option<Decimal>,
    pub income_tax_retention_base: Option<Decimal>,
    pub income_tax_retention: Option<Decimal>,
    /// The IGV: the retention is not added (SUNAT 4301).
    pub total_taxes: Option<Decimal>,
    pub sale_value: Option<Decimal>,
    /// `sale_value` + IGV (SUNAT 4310).
    pub subtotal: Option<Decimal>,
    pub rounding: Option<Decimal>,
    /// Advances of a settlement: out of scope.
    pub prepaid: Option<Decimal>,
    /// What the seller receives: `subtotal` − IGV − retention + `rounding` (SUNAT 4312).
    pub payable: Option<Decimal>,
}

impl SelfBilled {
    /// `SERIES-NUMBER`, e.g. `L001-1`.
    pub fn identifier(&self) -> String {
        format!("{}-{}", self.series, self.number)
    }

    /// SUNAT 2108: the settlement reaches SUNAT on its issue date or the next calendar day, in Lima
    /// (RS 097-2012 num. 12.1 b, as amended by RS 003-2023). Not part of `validate`, which has no clock.
    pub fn check_send_deadline(&self, now: DateTime) -> Result<(), ModelError> {
        let lima_day = |at: DateTime| (at.unix_seconds() - 5 * 3600).div_euclid(86_400);
        let elapsed_days = lima_day(now) - lima_day(self.issue_date);
        if elapsed_days < 0 {
            return Err(ModelError::rule("2329", "$.issue_date", "the issue date is after the sending date"));
        }
        if elapsed_days > 1 {
            let reason = format!("sent {elapsed_days} days after the issue date; the term is 1");
            return Err(ModelError::rule("2108", "$.issue_date", reason));
        }
        Ok(())
    }
}

/// `kind` is `"self_billed"`; the caller has already read it.
pub(crate) fn parse(mut value: Value) -> Result<Document, ModelError> {
    check_text(&value, "$")?;
    if let Value::Object(map) = &mut value {
        map.remove("kind");
    }
    serde_json::from_value(value).map(Document::SelfBilled).map_err(|e| ModelError::Json(e.to_string()))
}
