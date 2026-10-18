//! `totals.total_taxes` against the global taxes the document writes, in invoices, receipts and notes:
//! SUNAT 3294 (invoice, or a note on one: error) and 4301 (receipt, or a note on anything else:
//! observation, rejected here like the errors), within 1 (one currency unit). Notes: NotaCredito2_0
//! rows 286-287 and NotaDebito2_0 rows 269-270 of the 2026-04-24 sheet. One rule: ICBPER documents reach it from `icbper.rs`, in the sheet's order; export
//! invoices are held to zero earlier by `export.rs` (3294) and never get here with a different total.
//! The IGV of free sales (9996) stays out of the sum, as in the ICBPER check.

use crate::allowances::{cents, show_cents, within_one_cents};
use crate::retention_lines::units;
use crate::{Decimal, DocumentType, ModelError, Voucher};

/// A document without ICBPER; one with ICBPER is checked by `icbper::check`. `modifies` is the
/// document type a note modifies; `None` for an invoice or a receipt.
pub(crate) fn check(voucher: &Voucher, modifies: Option<&str>) -> Result<(), ModelError> {
    if voucher.lines.iter().any(|l| l.icbper.is_some() || l.icbper_factor.is_some()) {
        return Ok(());
    }
    let invoice = match modifies {
        Some(affected) => affected == "01",
        None => voucher.document_type == DocumentType::Invoice,
    };
    check_sum(voucher, if invoice { "3294" } else { "4301" })
}

/// `total_taxes` = IGV + IVAP + ICBPER, within 1; `code` is the one SUNAT answers for the document.
pub(crate) fn check_sum(voucher: &Voucher, code: &'static str) -> Result<(), ModelError> {
    let totals = &voucher.totals;
    let Some(given) = &totals.total_taxes else { return Ok(()) };
    let expected = cents(opt(&totals.igv) + opt(&totals.ivap) + opt(&totals.icbper));
    if within_one_cents(cents(amount(given)), expected) {
        return Ok(());
    }
    let reason = format!("must be igv + ivap + icbper ({}), within 1", show_cents(expected));
    Err(ModelError::rule(code, "$.totals.total_taxes", reason))
}

fn amount(value: &Decimal) -> i128 {
    units(value).unwrap_or(0)
}

fn opt(value: &Option<Decimal>) -> i128 {
    value.as_ref().map_or(0, amount)
}
