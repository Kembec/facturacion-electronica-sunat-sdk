//! The totals that catalog 53 discounts and charges change, checked only when the document has any:
//! a document without them keeps the caller's totals as given, as before. Order of SUNAT's sheet:
//! 3300, 3301, 3280, 3277, 3291, 3278, 3279 (receipt: its observation codes).
//!
//! Base: global `02` and `49` change the IGV base; `03`, `45` (FISE, as the sheet's 3301 has it),
//! `46` (recargo al consumo, art. 14 of the IGV law) and `50` do not, and go to the charge and
//! allowance totals with line `01` and `48`.

use crate::allowances::{cents, half_up_cents, show_cents, within_one_cents, Codes};
use crate::retention_lines::{units, SCALE};
use crate::validate::{check_format, fits};
use crate::{AllowanceCharge, Decimal, Invoice, Line, ModelError};

/// Catalog 07 affectations whose scheme is IGV (1000), and those that count in the sale value:
/// IGV, IVAP (1016), export (9995), exempt (9997) and unaffected (9998).
const IGV_AFFECTATION: &str = "10";
const SALE_VALUE_AFFECTATIONS: &[&str] = &["10", "17", "20", "30", "40"];

pub(crate) fn check(invoice: &Invoice, codes: &Codes) -> Result<(), ModelError> {
    let voucher = &invoice.voucher;
    let totals = &voucher.totals;
    let globals = &invoice.global_adjustments;
    let lines = &voucher.lines;
    optional_format("allowance_total", totals.allowance_total.as_ref(), "2065")?;
    optional_format("charge_total", totals.charge_total.as_ref(), "2064")?;
    let line_sum = |code: &str| lines.iter().map(|l| sum(&l.discounts, code) + sum(&l.charges, code)).sum::<i128>();
    let global = |code: &str| sum(globals, code);
    let base_change = global("49") - global("02");
    let allowances = line_sum("01") + global("03");
    let charges = line_sum("48") + global("45") + global("46") + global("50");

    compare(codes.allowance_total, "allowance_total", totals.allowance_total.as_ref(), allowances, "line discounts 01 + global discounts 03")?;
    compare(codes.charge_total, "charge_total", totals.charge_total.as_ref(), charges, "line charges 48 + global charges 45, 46 and 50")?;
    let payable = amount(totals.subtotal.as_ref()) + charges - allowances + amount(totals.rounding.as_ref());
    compare(codes.payable, "payable", totals.payable.as_ref(), payable, "subtotal + charge_total − allowance_total + rounding")?;

    let taxed_lines: Vec<&Line> = lines.iter().filter(|l| l.affectation == IGV_AFFECTATION && positive(&l.igv_base)).collect();
    let taxed: i128 = taxed_lines.iter().map(|l| amount(Some(&l.line_value))).sum::<i128>() + base_change;
    compare(codes.taxed, "taxed", totals.taxed.as_ref(), taxed, "Σ line_value of IGV lines − global 02 + global 49")?;
    check_igv(codes.igv, totals.igv.as_ref(), &taxed_lines, base_change)?;

    let sold = lines
        .iter()
        .filter(|l| SALE_VALUE_AFFECTATIONS.contains(&l.affectation.as_str()) && positive(&l.igv_base))
        .map(|l| amount(Some(&l.line_value)))
        .sum::<i128>()
        + base_change;
    compare(codes.sale_value, "sale_value", totals.sale_value.as_ref(), sold, "Σ line_value − global 02 + global 49")?;
    let subtotal = amount(totals.sale_value.as_ref()) + amount(totals.total_taxes.as_ref());
    compare(codes.subtotal, "subtotal", totals.subtotal.as_ref(), subtotal, "sale_value + total_taxes")
}

/// IGV over the net base: the rate of the lines × (Σ base − 02 + 49), within 1. With two IGV rates
/// (18 and 10.5) the sheet does not say how a global 02 or 49 splits between them, so the SDK does
/// not guess: out of scope until SUNAT or its beta says.
fn check_igv(code: &'static str, igv: Option<&Decimal>, lines: &[&Line], base_change: i128) -> Result<(), ModelError> {
    let mut rates: Vec<i128> = lines.iter().filter_map(|l| units(&l.igv_rate)).collect();
    rates.sort_unstable();
    rates.dedup();
    if rates.len() > 1 && base_change != 0 {
        let feature = "global discounts 02 or charges 49 with lines at two IGV rates";
        return Err(ModelError::OutOfScope { path: "$.global_adjustments".into(), feature });
    }
    let mut expected: i128 = 0;
    for rate in &rates {
        let base: i128 = lines.iter().filter(|l| units(&l.igv_rate) == Some(*rate)).map(|l| amount(Some(&l.igv_base))).sum();
        let base = if rates.len() == 1 { base + base_change } else { base };
        expected += half_up_cents(base * rate, SCALE * SCALE * 100) * SCALE / 100;
    }
    compare(code, "igv", igv, expected, "the IGV rate × (Σ igv_base − global 02 + global 49)")
}

fn compare(code: &'static str, field: &str, given: Option<&Decimal>, expected: i128, formula: &str) -> Result<(), ModelError> {
    let expected = cents(expected);
    let actual = given.and_then(units).map(cents).unwrap_or(0);
    if within_one_cents(actual, expected) {
        return Ok(());
    }
    let reason = format!("must be {formula} ({}), within 1", show_cents(expected));
    Err(ModelError::rule(code, format!("$.totals.{field}"), reason))
}

fn optional_format(field: &str, value: Option<&Decimal>, code: &str) -> Result<(), ModelError> {
    let Some(value) = value else { return Ok(()) };
    let ok = !value.is_negative() && fits(value, 12, 2);
    let expected = format!("an amount with up to 12 integer digits and 2 decimals (SUNAT {code})");
    check_format(&format!("$.totals.{field}"), value.as_str(), &expected, |_| ok)
}

fn sum(entries: &[AllowanceCharge], code: &str) -> i128 {
    entries.iter().filter(|e| e.code == code).map(|e| amount(Some(&e.amount))).sum()
}

/// In millionths; an absent amount, or one too long to read exactly, counts as zero.
fn amount(value: Option<&Decimal>) -> i128 {
    value.and_then(units).unwrap_or(0)
}

fn positive(value: &Decimal) -> bool {
    !value.is_negative() && !value.is_zero()
}
