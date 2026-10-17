//! Catalog 53 discounts and charges of invoices and receipts, line and global: codes, formats and
//! `amount` = `base_amount` × `factor`, in the order of SUNAT's sheet. The totals they change are
//! checked in `allowance_totals`. A receipt answers with SUNAT's observation codes where the invoice
//! has an error: the SDK rejects both, because an observed receipt is still wrong.
//!
//! Amounts are compared exactly in millionths (`units`), never as floating point; a product is
//! rounded half up to the cent before the tolerance of plus or minus 1 the sheet allows.

use crate::retention_lines::{units, SCALE};
use crate::validate::{check_format, fits};
use crate::{AllowanceCharge, Decimal, DocumentType, Invoice, Line, ModelError};

const LINE_DISCOUNTS: &[&str] = &["00", "01"];
const LINE_CHARGES: &[&str] = &["47", "48"];
const GLOBAL_CODES: &[&str] = &["02", "03", "45", "46", "49", "50"];
/// Catalog 53 codes the SDK does not render: advances (04, 05, 06, 20), the DU 010-2004 factors (07,
/// 54), the ISC and other taxes (51 to 53) and perception and retention codes (61 to 64; the IGV
/// retention 62 has its own field).
const OUT_OF_SCOPE: &[&str] = &["04", "05", "06", "07", "20", "51", "52", "53", "54", "61", "62", "63", "64"];
/// FISE (Ley 29852), a global charge outside the IGV base.
const FISE: &str = "45";

/// The code SUNAT answers for each rule, by document type: invoice error, receipt observation.
pub(crate) struct Codes {
    pub(crate) global_line_code: &'static str,
    pub(crate) global_factor: &'static str,
    pub(crate) line_factor: &'static str,
    pub(crate) allowance_total: &'static str,
    pub(crate) charge_total: &'static str,
    pub(crate) payable: &'static str,
    pub(crate) taxed: &'static str,
    pub(crate) igv: &'static str,
    pub(crate) sale_value: &'static str,
    pub(crate) subtotal: &'static str,
}

const INVOICE: Codes = Codes {
    global_line_code: "3071",
    global_factor: "3307",
    line_factor: "3290",
    allowance_total: "3300",
    charge_total: "3301",
    payable: "3280",
    taxed: "3277",
    igv: "3291",
    sale_value: "3278",
    subtotal: "3279",
};

const RECEIPT: Codes = Codes {
    global_line_code: "4291",
    global_factor: "4322",
    line_factor: "4289",
    allowance_total: "4307",
    charge_total: "4308",
    payable: "4312",
    taxed: "4299",
    igv: "4290",
    sale_value: "4309",
    subtotal: "4310",
};

pub(crate) fn check(invoice: &Invoice) -> Result<(), ModelError> {
    let voucher = &invoice.voucher;
    let codes = if voucher.document_type == DocumentType::Receipt { &RECEIPT } else { &INVOICE };
    let receipt = voucher.document_type == DocumentType::Receipt;
    for (index, line) in voucher.lines.iter().enumerate() {
        check_line(index, line, codes, receipt)?;
    }
    for (n, adjustment) in invoice.global_adjustments.iter().enumerate() {
        check_global(&format!("$.global_adjustments[{n}]"), adjustment, codes)?;
    }
    let adjusted = !invoice.global_adjustments.is_empty()
        || voucher.lines.iter().any(|line| !line.discounts.is_empty() || !line.charges.is_empty());
    if !adjusted {
        return Ok(());
    }
    crate::allowance_totals::check(invoice, codes)
}

fn check_line(index: usize, line: &Line, codes: &Codes, receipt: bool) -> Result<(), ModelError> {
    for (n, discount) in line.discounts.iter().enumerate() {
        let path = format!("$.lines[{index}].discounts[{n}]");
        check_line_code(&path, &discount.code, LINE_DISCOUNTS, "charges")?;
        check_entry(&path, discount, codes.line_factor, LINE_FORMATS)?;
    }
    for (n, charge) in line.charges.iter().enumerate() {
        let path = format!("$.lines[{index}].charges[{n}]");
        check_line_code(&path, &charge.code, LINE_CHARGES, "discounts")?;
        check_entry(&path, charge, codes.line_factor, LINE_FORMATS)?;
    }
    if receipt {
        return Ok(());
    }
    check_line_value(index, line)
}

/// `00` and `01` go in `discounts`, `47` and `48` in `charges`: the list decides nothing the code
/// does not already say, but a code in the wrong list is a caller's mistake worth naming.
fn check_line_code(path: &str, code: &str, allowed: &[&str], other_list: &str) -> Result<(), ModelError> {
    let at = format!("{path}.code");
    if code.is_empty() {
        return Err(ModelError::rule("3073", at, "the catalog 53 code is required"));
    }
    if allowed.contains(&code) {
        return Ok(());
    }
    if LINE_DISCOUNTS.contains(&code) || LINE_CHARGES.contains(&code) {
        return Err(ModelError::invalid(at, format!("code {code:?} goes in the line's {other_list}")));
    }
    if GLOBAL_CODES.contains(&code) || OUT_OF_SCOPE.contains(&code) {
        return Err(ModelError::rule("4268", at, "not a line code of catalog 53 (00, 01, 47, 48)"));
    }
    Err(ModelError::rule("2954", at, "not a catalog 53 code"))
}

fn check_global(path: &str, adjustment: &AllowanceCharge, codes: &Codes) -> Result<(), ModelError> {
    let at = format!("{path}.code");
    let code = adjustment.code.as_str();
    if code.is_empty() {
        return Err(ModelError::rule("3072", at, "the catalog 53 code is required"));
    }
    if code == "62" {
        return Err(ModelError::invalid(at, "the IGV retention (62) goes in igv_retention"));
    }
    if OUT_OF_SCOPE.contains(&code) {
        let feature = "global discounts and charges other than catalog 53 codes 02, 03, 45, 46, 49 and 50";
        return Err(ModelError::OutOfScope { path: at, feature });
    }
    if LINE_DISCOUNTS.contains(&code) || LINE_CHARGES.contains(&code) {
        return Err(ModelError::rule(codes.global_line_code, at, "a line code of catalog 53 at document level"));
    }
    if !GLOBAL_CODES.contains(&code) {
        return Err(ModelError::rule("3071", at, "not a global code of catalog 53"));
    }
    if code == FISE {
        positive_rule(path, "base_amount", &adjustment.base_amount, "3092")?;
        positive_rule(path, "amount", &adjustment.amount, "3074")?;
    }
    check_entry(path, adjustment, codes.global_factor, GLOBAL_FORMATS)
}

fn positive_rule(path: &str, field: &str, value: &Decimal, code: &'static str) -> Result<(), ModelError> {
    if value.is_negative() || value.is_zero() {
        return Err(ModelError::rule(code, format!("{path}.{field}"), "must be greater than zero for the FISE (45)"));
    }
    Ok(())
}

/// SUNAT's format codes for factor, amount and base: (line) 3052, 2955, 3053; (global) 3025, 2968, 3016.
struct Formats {
    factor: &'static str,
    amount: &'static str,
    base: &'static str,
}

const LINE_FORMATS: Formats = Formats { factor: "3052", amount: "2955", base: "3053" };
const GLOBAL_FORMATS: Formats = Formats { factor: "3025", amount: "2968", base: "3016" };

fn check_entry(path: &str, entry: &AllowanceCharge, factor_code: &'static str, formats: Formats) -> Result<(), ModelError> {
    let positive = |field: &str, value: &Decimal, int_max: usize, frac_max: usize, code: &str| {
        let ok = !value.is_negative() && !value.is_zero() && fits(value, int_max, frac_max);
        let expected = format!("a positive amount with up to {int_max} integer digits and {frac_max} decimals (SUNAT {code})");
        check_format(&format!("{path}.{field}"), value.as_str(), &expected, |_| ok)
    };
    positive("factor", &entry.factor, 3, 5, formats.factor)?;
    positive("amount", &entry.amount, 12, 2, formats.amount)?;
    positive("base_amount", &entry.base_amount, 12, 2, formats.base)?;
    let (Some(amount), Some(base), Some(factor)) = (units(&entry.amount), units(&entry.base_amount), units(&entry.factor)) else {
        return Ok(());
    };
    let expected = half_up_cents(base * factor, SCALE * SCALE);
    if within_one_cents(cents(amount), expected) {
        return Ok(());
    }
    let reason = format!("amount must be base_amount × factor ({}), within 1", show_cents(expected));
    Err(ModelError::rule(factor_code, format!("{path}.amount"), reason))
}

/// SUNAT 3271, invoice only: `line_value` = `unit_value` × `quantity` − line discounts `00` + line
/// charges `47`, within 1. Free lines (with `free_unit_value`) follow other rules and are not checked.
fn check_line_value(index: usize, line: &Line) -> Result<(), ModelError> {
    if line.free_unit_value.is_some() || (line.discounts.is_empty() && line.charges.is_empty()) {
        return Ok(());
    }
    let sum = |entries: &[AllowanceCharge], code: &str| -> Option<i128> {
        entries.iter().filter(|e| e.code == code).map(|e| units(&e.amount)).sum()
    };
    let (Some(unit), Some(quantity), Some(value), Some(d00), Some(c47)) = (
        units(&line.unit_value),
        units(&line.quantity),
        units(&line.line_value),
        sum(&line.discounts, "00"),
        sum(&line.charges, "47"),
    ) else {
        return Ok(());
    };
    let expected = unit * quantity + (c47 - d00) * SCALE;
    if (value * SCALE - expected).abs() <= SCALE * SCALE {
        return Ok(());
    }
    let reason = "line_value must be unit_value × quantity − discounts 00 + charges 47, within 1";
    Err(ModelError::rule("3271", format!("$.lines[{index}].line_value"), reason))
}

/// `units` (millionths) to cents, half up.
pub(crate) fn cents(units: i128) -> i128 {
    half_up_cents(units, SCALE)
}

/// `numerator / denominator` currency units, rounded half up to cents (away from zero on .5).
pub(crate) fn half_up_cents(numerator: i128, denominator: i128) -> i128 {
    let scaled = numerator * 100;
    let rounded = (scaled.abs() * 2 + denominator) / (denominator * 2);
    if scaled < 0 {
        -rounded
    } else {
        rounded
    }
}

/// The sheet's tolerance: plus or minus 1 currency unit.
pub(crate) fn within_one_cents(actual: i128, expected: i128) -> bool {
    (actual - expected).abs() <= 100
}

pub(crate) fn show_cents(cents: i128) -> String {
    let sign = if cents < 0 { "-" } else { "" };
    format!("{sign}{}.{:02}", cents.abs() / 100, cents.abs() % 100)
}
