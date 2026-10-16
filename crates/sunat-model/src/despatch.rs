//! Despatch advice input (`docs/input-model.md`, «Despatch advice (09)»): the sender's despatch advice
//! (guía de remisión remitente, 09), UBL 2.1 `DespatchAdvice` with SUNAT's customization 2.0.
//!
//! SUNAT has no test environment for despatch advices. What SUNAT's validation rules check on the XML
//! itself (sheet «Guía-Remitente2_0», type XSL) is checked here, each with its SUNAT code, in
//! `despatch_rules` and `despatch_transport`; the rules that query SUNAT's registries (type REST:
//! taxpayer status, RENIEC, MTC, issued documents) and the reception date (2108, 2329) are not.

use serde::Deserialize;
use serde_json::Value;

use crate::scope::check_text;
use crate::{Company, Date, DateTime, Decimal, Document, ModelError};

/// Sender's despatch advice (09).
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Despatch {
    pub ubl_version: String,
    /// Catalog 01: `"09"`.
    pub document_type: String,
    /// `T` and 3 uppercase letters or digits (SUNAT 1001).
    pub series: String,
    pub number: String,
    pub issue_date: DateTime,
    /// `cbc:Note`, up to 250 characters.
    pub note: Option<String>,
    /// The sender (remitente): always the issuer, identified by its RUC.
    pub issuer: Company,
    /// The recipient (destinatario).
    pub recipient: DespatchParty,
    /// The supplier (proveedor), for reasons 02, 07 and 13.
    pub seller: Option<DespatchParty>,
    /// The buyer (comprador), for reasons 03 and 13.
    pub buyer: Option<DespatchParty>,
    #[serde(default)]
    pub related_documents: Vec<RelatedDocument>,
    pub shipment: Shipment,
    pub lines: Vec<DespatchLine>,
}

/// A party identified by a catalog 06 document.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DespatchParty {
    pub document_type: String,
    pub document_number: String,
    pub legal_name: String,
}

/// `cac:AdditionalDocumentReference`: a catalog 61 document the transfer relates to.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RelatedDocument {
    pub document_type: String,
    pub document_number: String,
    /// The document type's name, `cbc:DocumentType` (`"Factura"`).
    pub description: String,
    /// RUC of the document's issuer: required for 01, 03, 04, 09, 12 and 48 (SUNAT 3380).
    pub issuer_ruc: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Shipment {
    /// Catalog 20.
    pub reason_code: String,
    /// Required for reason 13 (SUNAT 3457).
    pub reason_description: Option<String>,
    /// Catalog 18: `"01"` public (a carrier moves the goods), `"02"` private (the sender does).
    pub transport_mode: String,
    /// Start of the transfer, or the day the goods are handed to the carrier.
    pub start_date: Option<Date>,
    /// Gross weight of the load, up to 12 integer digits and 3 decimals.
    pub gross_weight: Option<Decimal>,
    /// `"KGM"` or `"TNE"`.
    pub weight_unit: Option<String>,
    /// Number of packages or pallets.
    pub packages: Option<String>,
    /// Transfer in M1 or L category vehicles (`SUNAT_Envio_IndicadorTrasladoVehiculoM1L`): no
    /// vehicle, driver or carrier is given.
    #[serde(default)]
    pub m1_or_l_vehicle: bool,
    /// Public transport only.
    pub carrier: Option<Carrier>,
    /// Plates: the first is the main vehicle, the rest are secondary.
    #[serde(default)]
    pub vehicles: Vec<Vehicle>,
    /// The first is the main driver (`Principal`), the rest are secondary.
    #[serde(default)]
    pub drivers: Vec<Driver>,
    pub origin: Place,
    pub destination: Place,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Carrier {
    pub ruc: String,
    pub legal_name: String,
    /// Registration number at the MTC.
    pub mtc_registration: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Vehicle {
    pub plate: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Driver {
    pub document_type: String,
    pub document_number: String,
    pub first_name: String,
    pub family_name: String,
    /// Driving licence.
    pub license: String,
}

/// Where the transfer starts or ends.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Place {
    /// Catalog 13, 6 digits.
    pub ubigeo: String,
    pub address: String,
    /// Annex establishment code (4 digits) registered at SUNAT; required for reason 04.
    pub establishment_code: Option<String>,
    /// RUC the establishment belongs to; goes with `establishment_code`.
    pub establishment_ruc: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DespatchLine {
    pub quantity: Option<Decimal>,
    /// Catalog 03.
    pub unit: String,
    pub description: String,
    pub product_code: Option<String>,
    /// Catalog 25, up to 8 digits.
    pub sunat_product_code: Option<String>,
}

impl Despatch {
    /// `SERIES-NUMBER`, e.g. `T001-1`.
    pub fn identifier(&self) -> String {
        format!("{}-{}", self.series, self.number)
    }
}

/// `kind` is `"despatch"`; the caller has already read it.
pub(crate) fn parse(mut value: Value) -> Result<Document, ModelError> {
    check_text(&value, "$")?;
    if let Value::Object(map) = &mut value {
        map.remove("kind");
    }
    serde_json::from_value(value).map(Document::Despatch).map_err(|e| ModelError::Json(e.to_string()))
}
