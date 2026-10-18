//! Export (catalog 51, operation types 0200 to 0208) in invoices and receipts, in the order of the
//! contract's table: operation type for the document (3206), country of use (3098, 3099), customer
//! (2800 to 2802), lines (2642, 3110, 3102, guest data in `export_lodging`) and totals (3107, 3003,
//! 3273, 3294, 3278 to 3280). The SDK does not decide whether a sale is an export: domicile, place of
//! use, the exporters' register and the 60-day limits stay with the caller.

use crate::allowances::{cents, show_cents, within_one_cents};
use crate::retention_lines::units;
use crate::{Decimal, DocumentType, Invoice, Line, ModelError};

const EXPORT_OPERATIONS: &[&str] = &["0200", "0201", "0202", "0203", "0204", "0205", "0206", "0207", "0208"];
/// Services used abroad: they carry the country of use (`cac:Delivery`).
const SERVICE_OPERATIONS: &[&str] = &["0201", "0208"];
/// Lodging and tour package: invoice only (catalog 51) and guest data on every line.
const GUEST_OPERATIONS: &[&str] = &["0202", "0205"];
/// A customer with RUC cannot meet these types (sheet rows 97, 311 and 319 together).
const NO_RUC_OPERATIONS: &[&str] = &["0200", "0201", "0204"];
/// Catalog 06 identity documents, plus `-` (SUNAT 2800).
pub(crate) const IDENTITY_DOCUMENTS: &[&str] = &["0", "1", "4", "6", "7", "A", "B", "C", "D", "E", "F", "G", "H", "-"];
/// Foreign identity documents: up to 15 characters without spaces (SUNAT 2802).
const FOREIGN_DOCUMENTS: &[&str] = &["0", "4", "7", "A", "B", "C", "D", "E", "G"];
const EXPORT_AFFECTATION: &str = "40";

pub(crate) fn is_export(operation: &str) -> bool {
    EXPORT_OPERATIONS.contains(&operation)
}

pub(crate) fn check(invoice: &Invoice) -> Result<(), ModelError> {
    let operation = invoice.operation_type.as_str();
    if !is_export(operation) {
        return check_not_export(invoice);
    }
    let receipt = invoice.voucher.document_type == DocumentType::Receipt;
    if receipt && GUEST_OPERATIONS.contains(&operation) {
        let reason = format!("operation type {operation} is invoice only (catalog 51)");
        return Err(ModelError::rule("3206", "$.operation_type", reason));
    }
    check_country(invoice, operation)?;
    check_customer(invoice, operation, receipt)?;
    check_out_of_scope(invoice)?;
    for (index, line) in invoice.voucher.lines.iter().enumerate() {
        check_line(index, line)?;
        crate::export_lodging::check(index, line, operation)?;
    }
    check_totals(invoice)
}

/// Outside export, the two export fields would be dropped in silence (or observed, 4041): rejected.
fn check_not_export(invoice: &Invoice) -> Result<(), ModelError> {
    if invoice.service_use_country.is_some() {
        return Err(ModelError::invalid("$.service_use_country", "applies only with operation_type 0201 or 0208"));
    }
    crate::export_lodging::reject_any(&invoice.voucher.lines, "applies only with operation_type 0202 or 0205")
}

fn check_country(invoice: &Invoice, operation: &str) -> Result<(), ModelError> {
    let path = "$.service_use_country";
    let country = invoice.service_use_country.as_deref();
    if !SERVICE_OPERATIONS.contains(&operation) {
        return match country {
            Some(_) => Err(ModelError::invalid(path, "applies only with operation_type 0201 or 0208")),
            None => Ok(()),
        };
    }
    let Some(country) = country else {
        let reason = format!("operation type {operation} requires the country where the service is used");
        return Err(ModelError::rule("3098", path, reason));
    };
    if !is_country(country) || country == "PE" {
        let reason = format!("expected a catalog 04 country other than \"PE\", got {country:?}");
        return Err(ModelError::rule("3099", path, reason));
    }
    Ok(())
}

/// Catalog 04 is ISO 3166-1 alpha-2: two uppercase letters.
pub(crate) fn is_country(code: &str) -> bool {
    code.len() == 2 && code.bytes().all(|b| b.is_ascii_uppercase())
}

fn check_customer(invoice: &Invoice, operation: &str, receipt: bool) -> Result<(), ModelError> {
    let customer = &invoice.voucher.customer;
    let (kind, number) = (customer.document_type.as_str(), customer.document_number.as_str());
    if !IDENTITY_DOCUMENTS.contains(&kind) {
        let reason = format!("expected a catalog 06 code or \"-\", got {kind:?}");
        return Err(ModelError::rule("2800", "$.customer.document_type", reason));
    }
    if kind == "6" && (receipt || NO_RUC_OPERATIONS.contains(&operation)) {
        let reason = format!("a customer with RUC cannot take operation type {operation} in this document");
        return Err(ModelError::rule("2800", "$.customer.document_type", reason));
    }
    let path = "$.customer.document_number";
    if kind == "1" && !(number.len() == 8 && number.bytes().all(|b| b.is_ascii_digit())) {
        return Err(ModelError::rule("2801", path, format!("a DNI is 8 digits, got {number:?}")));
    }
    let foreign_ok = (1..=15).contains(&number.chars().count()) && !number.chars().any(char::is_whitespace);
    if FOREIGN_DOCUMENTS.contains(&kind) && !foreign_ok {
        return Err(ModelError::rule("2802", path, format!("expected up to 15 characters without spaces, got {number:?}")));
    }
    Ok(())
}

/// Left out of the first cut of export, with a named error instead of a SUNAT rejection.
fn check_out_of_scope(invoice: &Invoice) -> Result<(), ModelError> {
    let out = |path: String, feature| Err(ModelError::OutOfScope { path, feature });
    if !invoice.advances.is_empty() {
        return out("$.advances".into(), "advances in an export (catalog 53 has no export advance)");
    }
    if invoice.voucher.igv_retention.is_some() {
        return out("$.igv_retention".into(), "IGV retention in an export");
    }
    for (index, line) in invoice.voucher.lines.iter().enumerate() {
        if line.free_unit_value.is_some() {
            return out(format!("$.lines[{index}].free_unit_value"), "free lines in an export");
        }
        if line.icbper.is_some() {
            return out(format!("$.lines[{index}].icbper"), "ICBPER in an export");
        }
    }
    Ok(())
}

fn check_line(index: usize, line: &Line) -> Result<(), ModelError> {
    let path = |field: &str| format!("$.lines[{index}].{field}");
    if line.affectation != EXPORT_AFFECTATION {
        return Err(ModelError::rule("2642", path("affectation"), "every line of an export takes affectation \"40\""));
    }
    if !line.igv.is_zero() {
        return Err(ModelError::rule("3110", path("igv"), "an export line carries no IGV: \"0.00\""));
    }
    if line.igv_rate.is_zero() || line.igv_rate.is_negative() {
        return Err(ModelError::rule("3102", path("igv_rate"), "the IGV rate of the period, not zero"));
    }
    Ok(())
}

fn check_totals(invoice: &Invoice) -> Result<(), ModelError> {
    let totals = &invoice.voucher.totals;
    for (field, value) in [("taxed", &totals.taxed), ("exempt", &totals.exempt), ("unaffected", &totals.unaffected), ("ivap_base", &totals.ivap_base), ("igv", &totals.igv)] {
        if value.as_ref().is_some_and(|v| !v.is_zero()) {
            return Err(ModelError::rule("3107", format!("$.totals.{field}"), "an export carries only the export total: absent or \"0.00\""));
        }
    }
    let Some(export) = &totals.export else {
        return Err(ModelError::rule("3003", "$.totals.export", "an export requires the export total"));
    };
    let lines: i128 = invoice.voucher.lines.iter().map(|l| amount(Some(&l.line_value))).sum();
    compare("3273", "export", Some(export), lines, "Σ line_value")?;
    compare("3294", "total_taxes", totals.total_taxes.as_ref(), 0, "zero")?;
    compare("3278", "sale_value", totals.sale_value.as_ref(), amount(Some(export)), "export")?;
    compare("3279", "subtotal", totals.subtotal.as_ref(), amount(totals.sale_value.as_ref()), "sale_value")?;
    let payable = amount(totals.subtotal.as_ref()) + amount(totals.charge_total.as_ref()) - amount(totals.allowance_total.as_ref());
    compare("3280", "payable", totals.payable.as_ref(), payable, "subtotal + charge_total − allowance_total")
}

/// An absent total is left to the required-field checks that follow.
fn compare(code: &'static str, field: &str, given: Option<&Decimal>, expected: i128, formula: &str) -> Result<(), ModelError> {
    let Some(given) = given else { return Ok(()) };
    let (actual, expected) = (cents(amount(Some(given))), cents(expected));
    if within_one_cents(actual, expected) {
        return Ok(());
    }
    let reason = format!("must be {formula} ({}), within 1", show_cents(expected));
    Err(ModelError::rule(code, format!("$.totals.{field}"), reason))
}

fn amount(value: Option<&Decimal>) -> i128 {
    value.and_then(units).unwrap_or(0)
}
