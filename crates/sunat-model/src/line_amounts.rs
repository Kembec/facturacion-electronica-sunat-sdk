//! The unit price and the IGV of each line of an invoice or receipt, in the order of SUNAT's sheets
//! «Factura2_0» and «Boleta2_0»:
//!
//! - unit price (row 183 of the invoice, error 3270; row 176 of the receipt, observation 4287):
//!   `unit_price` = (`line_value` + `total_taxes` − line discounts `01` + line charges `48`) ÷
//!   `quantity`, within 1, on a line that is not free;
//! - line IGV (row 213 of the invoice, row 206 of the receipt, error 3103 in both): with affectation
//!   10 to 17, `igv` = `igv_rate` × `igv_base`, within 1.
//!
//! An observed receipt is rejected like an invoice error (see `allowances`). Amounts are compared
//! exactly, never as floating point; the IGV product is rounded half up to the cent before the
//! tolerance, and the unit price, which takes up to 10 decimals, is compared as a fraction.

use crate::allowances::{half_up_cents, show_cents, within_one_cents};
use crate::retention_lines::{units, SCALE};
use crate::{AllowanceCharge, Decimal, DocumentType, Line, ModelError, Voucher};

/// Catalog 07 affectations whose line carries IGV or IVAP.
const TAXED: &[&str] = &["10", "11", "12", "13", "14", "15", "16", "17"];

pub(crate) fn check(voucher: &Voucher) -> Result<(), ModelError> {
    let price_code = if voucher.document_type == DocumentType::Receipt { "4287" } else { "3270" };
    for (index, line) in voucher.lines.iter().enumerate() {
        check_unit_price(index, line, price_code)?;
        check_igv(index, line)?;
    }
    Ok(())
}

fn check_unit_price(index: usize, line: &Line, code: &'static str) -> Result<(), ModelError> {
    if line.free_unit_value.is_some() {
        return Ok(());
    }
    let Some(price) = line.unit_price.as_ref().and_then(fraction) else { return Ok(()) };
    let Some((quantity, quantity_scale)) = fraction(&line.quantity).filter(|(q, _)| *q > 0) else { return Ok(()) };
    let sum = |entries: &[AllowanceCharge], code: &str| -> Option<i128> {
        entries.iter().filter(|e| e.code == code).map(|e| units(&e.amount)).sum()
    };
    let (Some(value), Some(taxes), Some(d01), Some(c48)) =
        (units(&line.line_value), units(&line.total_taxes), sum(&line.discounts, "01"), sum(&line.charges, "48"))
    else {
        return Ok(());
    };
    let gross = value + taxes - d01 + c48;
    let (price, price_scale) = price;
    // |price − gross ÷ quantity| ≤ 1, multiplied through by quantity and every scale.
    let within = || -> Option<bool> {
        let lhs = price.checked_mul(quantity)?.checked_mul(SCALE)?;
        let rhs = gross.checked_mul(pow10(price_scale + quantity_scale)?)?;
        let tolerance = quantity.checked_mul(pow10(price_scale)?)?.checked_mul(SCALE)?;
        Some(lhs.checked_sub(rhs)?.abs() <= tolerance)
    };
    if within().unwrap_or(true) {
        return Ok(());
    }
    let reason = "unit_price must be (line_value + total_taxes − discounts 01 + charges 48) ÷ quantity, within 1";
    Err(ModelError::rule(code, format!("$.lines[{index}].unit_price"), reason))
}

fn check_igv(index: usize, line: &Line) -> Result<(), ModelError> {
    if !TAXED.contains(&line.affectation.as_str()) {
        return Ok(());
    }
    let (Some(igv), Some(base), Some(rate)) = (units(&line.igv), units(&line.igv_base), units(&line.igv_rate)) else {
        return Ok(());
    };
    let expected = half_up_cents(base * rate, 100 * SCALE * SCALE);
    if within_one_cents(crate::allowances::cents(igv), expected) {
        return Ok(());
    }
    let reason = format!("igv must be igv_base × igv_rate ({}), within 1", show_cents(expected));
    Err(ModelError::rule("3103", format!("$.lines[{index}].igv"), reason))
}

/// A decimal as an exact fraction: (digits, decimals), so `1.25` is `(125, 2)`. Trailing zeros are
/// dropped to keep the products small; `None` past 18 significant digits.
fn fraction(value: &Decimal) -> Option<(i128, u32)> {
    let text = value.as_str();
    let (negative, unsigned) = match text.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, text),
    };
    let (int, frac) = unsigned.split_once('.').unwrap_or((unsigned, ""));
    let frac = frac.trim_end_matches('0');
    let digits = format!("{}{frac}", int.trim_start_matches('0'));
    if digits.len() > 18 {
        return None;
    }
    let mantissa = if digits.is_empty() { 0 } else { digits.parse::<i128>().ok()? };
    Some((if negative { -mantissa } else { mantissa }, u32::try_from(frac.len()).ok()?))
}

fn pow10(exponent: u32) -> Option<i128> {
    10i128.checked_pow(exponent)
}
