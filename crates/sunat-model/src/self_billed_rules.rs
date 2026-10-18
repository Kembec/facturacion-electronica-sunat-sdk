//! SUNAT's validation rules for the purchase settlement (04) that can be checked on the document itself
//! (sheet «LiquidacionCompra2_0» of SUNAT's validation rules, 24/04/2026): header, parties, the place
//! of the operation and the totals. The lines are in `self_billed_lines`. Each rejection carries the
//! code SUNAT would answer; the totals' observations (43xx) are enforced too, so a settlement never
//! reaches SUNAT with a note on it.

use crate::despatch_rules::{alnum_upper, blank, digits, digits_between, rule};
use crate::retention_lines::{units, SCALE};
use crate::self_billed_lines::check_lines;
use crate::{Decimal, ModelError, SelfBilled};

/// Catalog 51 operation types of the settlement the SDK renders; 0502 (advances) is out of scope.
const OPERATIONS: &[&str] = &["0501", "0503"];

/// Catalog 06 identity documents a seller may have: DNI, carné de extranjería, passport (SUNAT 2800).
const SELLER_IDENTITIES: &[&str] = &["1", "4", "7"];

pub(crate) fn check(s: &SelfBilled) -> Result<(), ModelError> {
    check_header(s)?;
    check_issuer(s)?;
    check_seller(s)?;
    check_operation_place(s)?;
    if s.lines.is_empty() {
        return Err(ModelError::missing("$.lines"));
    }
    check_lines(s)?;
    crate::references::check(None, &s.despatch_references)?;
    check_totals(s)
}

fn check_header(s: &SelfBilled) -> Result<(), ModelError> {
    rule(s.ubl_version == "2.1", "2074", "$.ubl_version", "must be \"2.1\"")?;
    rule(s.document_type == "04", "1003", "$.document_type", "must be \"04\"")?;
    if s.operation_type == "0502" {
        let feature = "advances of a purchase settlement (operation type 0502)";
        return Err(ModelError::OutOfScope { path: "$.operation_type".into(), feature });
    }
    rule(OPERATIONS.contains(&s.operation_type.as_str()), "3206", "$.operation_type", "must be \"0501\" or \"0503\"")?;
    rule(series(&s.series), "1001", "$.series", "must be L and 3 uppercase letters or digits")?;
    rule(digits_between(&s.number, 1, 8), "1001", "$.number", "must be 1 to 8 digits")?;
    let currency = s.currency.len() == 3 && s.currency.bytes().all(|b| b.is_ascii_uppercase());
    rule(currency, "3088", "$.currency", "must be an ISO 4217 code (catalog 02)")
}

fn check_issuer(s: &SelfBilled) -> Result<(), ModelError> {
    let ruc = &s.issuer.ruc;
    if !(ruc.len() == 11 && digits(ruc)) {
        return Err(ModelError::invalid("$.issuer.ruc", format!("expected 11 digits, got {ruc:?}")));
    }
    rule(!blank(&s.issuer.legal_name), "1037", "$.issuer.legal_name", "required")?;
    let country = s.issuer.address.country_code.as_deref();
    rule(country.is_none_or(|c| c == "PE"), "4041", "$.issuer.address.country_code", "must be \"PE\"")
}

fn check_seller(s: &SelfBilled) -> Result<(), ModelError> {
    let seller = &s.seller;
    let at = |field: &str| format!("$.seller.{field}");
    let kind = seller.document_type.as_str();
    rule(SELLER_IDENTITIES.contains(&kind), "2800", at("document_type"), "must be 1 (DNI), 4 (carné de extranjería) or 7 (passport)")?;
    rule(!seller.document_number.is_empty(), "2014", at("document_number"), "required")?;
    let number_ok = crate::despatch_rules::identity_number(kind, &seller.document_number);
    rule(number_ok, "2802", at("document_number"), crate::despatch_rules::identity_format(kind))?;
    rule(!blank(&seller.legal_name), "2021", at("legal_name"), "required")?;
    rule(seller.legal_name.chars().count() <= 1500, "2022", at("legal_name"), "up to 1500 characters")?;
    let address = &seller.address;
    check_street(address.street.as_deref(), ("2452", "2593"), &at("address.street"))?;
    rule(present(&address.ubigeo), "2453", at("address.ubigeo"), "required")?;
    let country = address.country_code.as_deref();
    rule(country.is_none_or(|c| c == "PE"), "4041", at("address.country_code"), "must be \"PE\"")?;
    let kind = address.address_type.as_deref();
    rule(present(&address.address_type), "2456", at("address.address_type"), "required")?;
    rule(kind.is_some_and(catalog_60), "2457", at("address.address_type"), "must be a catalog 60 code of 2 digits")
}

fn check_operation_place(s: &SelfBilled) -> Result<(), ModelError> {
    let Some(place) = &s.operation_place else {
        return Err(ModelError::rule("2454", "$.operation_place", "required: the address where the purchase took place"));
    };
    check_street(place.street.as_deref(), ("2454", "2778"), "$.operation_place.street")?;
    rule(present(&place.ubigeo), "2455", "$.operation_place.ubigeo", "required")?;
    let ubigeo_ok = place.ubigeo.as_deref().is_some_and(|u| u.len() == 6 && digits(u));
    rule(ubigeo_ok, "2917", "$.operation_place.ubigeo", "must be a catalog 13 code of 6 digits")?;
    let country = place.country_code.as_deref();
    rule(country.is_none_or(|c| c == "PE"), "4041", "$.operation_place.country_code", "must be \"PE\"")?;
    rule(present(&place.location_type), "2458", "$.operation_place.location_type", "required")?;
    let kind = place.location_type.as_deref();
    rule(kind.is_some_and(catalog_60), "2459", "$.operation_place.location_type", "must be a catalog 60 code of 2 digits")
}

/// 3 to 250 characters, not starting with a space or a tab: `missing` when absent or blank,
/// `format` otherwise.
fn check_street(street: Option<&str>, (missing, format): (&'static str, &'static str), path: &str) -> Result<(), ModelError> {
    let Some(street) = street.filter(|s| !blank(s)) else {
        return Err(ModelError::rule(missing, path, "required"));
    };
    rule((3..=250).contains(&street.chars().count()), format, path, "3 to 250 characters")?;
    rule(!street.starts_with([' ', '\t']), format, path, "must not start with a space or a tab")
}

/// The totals against the lines, with the ±1 the sheet allows (4309, 4290, 4301, 4345, 4346, 4310,
/// 4312) and the rounding limit (4314).
fn check_totals(s: &SelfBilled) -> Result<(), ModelError> {
    let t = &s.totals;
    if t.prepaid.is_some() {
        return Err(ModelError::OutOfScope { path: "$.totals.prepaid".into(), feature: "advances of a purchase settlement" });
    }
    let sum = |pick: fn(&crate::SelfBilledLine) -> Option<i128>| -> i128 { s.lines.iter().filter_map(pick).sum() };
    let value = sum(|l| units(&l.line_value));
    let igv = sum(|l| (l.affectation == "10").then(|| units(&l.igv)).flatten());
    let base = sum(|l| l.income_tax_retention.as_ref().and_then(|r| units(&r.base)));
    let retained = sum(|l| l.income_tax_retention.as_ref().and_then(|r| units(&r.amount)));
    let sale_value = required(&t.sale_value, "$.totals.sale_value")?;
    near(sale_value, value, "4309", "$.totals.sale_value", "must be the sum of the lines' line_value, ±1")?;
    near(amount(&t.igv), igv, "4290", "$.totals.igv", "must be the sum of the taxed lines' igv, ±1")?;
    let total_taxes = required(&t.total_taxes, "$.totals.total_taxes")?;
    near(total_taxes, igv, "4301", "$.totals.total_taxes", "must be the IGV, without the income tax retention, ±1")?;
    let retention_base = required(&t.income_tax_retention_base, "$.totals.income_tax_retention_base")?;
    near(retention_base, base, "4345", "$.totals.income_tax_retention_base", "must be the sum of the lines' retention base, ±1")?;
    let retention = required(&t.income_tax_retention, "$.totals.income_tax_retention")?;
    near(retention, retained, "4346", "$.totals.income_tax_retention", "must be the sum of the lines' retention amount, ±1")?;
    let subtotal = required(&t.subtotal, "$.totals.subtotal")?;
    near(subtotal, sale_value + total_taxes, "4310", "$.totals.subtotal", "must be sale_value + total_taxes, ±1")?;
    let rounding = amount(&t.rounding);
    rule(rounding.abs() <= SCALE, "4314", "$.totals.rounding", "absolute value up to 1")?;
    let payable = required(&t.payable, "$.totals.payable")?;
    let expected = subtotal - igv - retained + rounding;
    near(payable, expected, "4312", "$.totals.payable", "must be subtotal − IGV − income tax retention + rounding, ±1")
}

fn required(value: &Option<Decimal>, path: &str) -> Result<i128, ModelError> {
    let value = value.as_ref().ok_or_else(|| ModelError::missing(path))?;
    units(value).ok_or_else(|| ModelError::invalid(path, "up to 6 decimals"))
}

fn amount(value: &Option<Decimal>) -> i128 {
    value.as_ref().and_then(units).unwrap_or(0)
}

pub(crate) fn near(given: i128, expected: i128, code: &'static str, path: &str, reason: &str) -> Result<(), ModelError> {
    rule((given - expected).abs() <= SCALE, code, path, reason)
}

fn present(value: &Option<String>) -> bool {
    value.as_deref().is_some_and(|v| !blank(v))
}

/// Catalog 60 is not in the sources the SDK was written from: two digits, the form of its codes.
fn catalog_60(code: &str) -> bool {
    code.len() == 2 && digits(code)
}

fn series(text: &str) -> bool {
    text.len() == 4 && text.starts_with('L') && alnum_upper(&text[1..])
}
