//! ICBPER (tax 7152, plastic bags) in invoices, receipts and notes, checked only when a line carries
//! it: a document without it keeps the caller's totals as given, as before. Order of SUNAT's sheet:
//! line 2033, 2892, 3238, 2949, 4237, 4318, then 3292 (formats before arithmetic); document 2048, 3294, 3306, 3279, 3280.
//!
//! The bags are `quantity` of their own line, so `BaseUnitMeasure` equals it by construction (3236,
//! 3237). Observations 4318 and 4237 are rejected here like the errors: a document SUNAT accepts
//! with a note is still wrong. A receipt, and a note on anything but an invoice, answers with the
//! observation codes the sheet gives them (4293, 4301, 4310, 4312, 4321).

use crate::allowances::{cents, half_up_cents, show_cents, within_one_cents};
use crate::icbper_rates::{rate_on, ICBPER_RATES_VERSION};
use crate::retention_lines::{units, SCALE};
use crate::validate::{check_format, fits};
use crate::{Decimal, DocumentType, Line, ModelError, SummaryLine, Voucher};

const PEN: &str = "PEN";
const MAX_BAGS: i128 = 99_999;

/// The code SUNAT answers for each total, by what the document is or modifies.
struct Codes {
    line_taxes: &'static str,
    total_taxes: &'static str,
    icbper_total: &'static str,
    subtotal: &'static str,
    payable: &'static str,
}

const INVOICE: Codes = Codes { line_taxes: "3292", total_taxes: "3294", icbper_total: "3306", subtotal: "3279", payable: "3280" };
const OTHER: Codes = Codes { line_taxes: "4293", total_taxes: "4301", icbper_total: "4321", subtotal: "4310", payable: "4312" };

/// `modifies` is the document type a note modifies; `None` for an invoice or a receipt.
pub(crate) fn check(voucher: &Voucher, modifies: Option<&str>) -> Result<(), ModelError> {
    let carries = voucher.lines.iter().any(|l| l.icbper.is_some() || l.icbper_factor.is_some());
    if !carries {
        if voucher.totals.icbper.is_some() {
            return Err(ModelError::invalid("$.totals.icbper", "applies only when a line carries icbper"));
        }
        return Ok(());
    }
    if voucher.currency != PEN {
        return Err(ModelError::OutOfScope { path: "$.currency".into(), feature: "ICBPER in a currency other than PEN" });
    }
    let invoice = match modifies {
        Some(affected) => affected == "01",
        None => voucher.document_type == DocumentType::Invoice,
    };
    let codes = if invoice { &INVOICE } else { &OTHER };
    let date = voucher.issue_date.to_lima().date_string();
    for (index, line) in voucher.lines.iter().enumerate() {
        check_line(index, line, &date, codes)?;
    }
    check_totals(voucher, codes, modifies.is_some())
}

fn check_line(index: usize, line: &Line, date: &str, codes: &Codes) -> Result<(), ModelError> {
    let at = |field: &str| format!("$.lines[{index}].{field}");
    let icbper = match (&line.icbper, &line.icbper_factor) {
        (None, None) => return Ok(()),
        (None, Some(_)) => return Err(ModelError::invalid(at("icbper_factor"), "applies only with icbper")),
        (Some(icbper), _) => icbper,
    };
    positive(&at("icbper"), icbper, 12, 2, "2033")?;
    let bags = bags(line).ok_or_else(|| {
        let reason = format!("expected a whole number of bags from 1 to {MAX_BAGS} on a line with icbper (SUNAT 2892), got {:?}", line.quantity.as_str());
        ModelError::rule("2892", at("quantity"), reason)
    })?;
    let factor = match &line.icbper_factor {
        Some(factor) if !factor.is_zero() => factor,
        _ => return Err(ModelError::rule("3238", at("icbper_factor"), "the ICBPER per bag is required and not zero")),
    };
    positive(&at("icbper_factor"), factor, 3, 5, "2892")?;
    let Some(rate) = rate_on(date) else {
        return Err(ModelError::rule("2949", "$.issue_date", "ICBPER is in force from 2019-08-01"));
    };
    if units(factor) != units(&rate.value.parse().expect("rate table")) {
        let reason = format!("must be {} per bag, the rate in force on {date} (rate table {ICBPER_RATES_VERSION}, {})", rate.value, rate.state);
        return Err(ModelError::rule("4237", at("icbper_factor"), reason));
    }
    let expected = half_up_cents(units(factor).unwrap_or(0) * bags, SCALE);
    if cents(amount(icbper)) != expected {
        let reason = format!("must be icbper_factor × quantity ({})", show_cents(expected));
        return Err(ModelError::rule("4318", at("icbper"), reason));
    }
    // A free line's IGV goes under 9996, which the sheet leaves out of the line's taxes; whether
    // SUNAT expects it there (`03-receipt-free-line.json` has it) is for its beta to say.
    if line.free_unit_value.is_some() {
        return Ok(());
    }
    let expected = amount(&line.igv) + amount(icbper);
    within_one(codes.line_taxes, &at("total_taxes"), &line.total_taxes, expected, "igv + icbper")
}

fn check_totals(voucher: &Voucher, codes: &Codes, note: bool) -> Result<(), ModelError> {
    let totals = &voucher.totals;
    let Some(total) = &totals.icbper else { return Err(ModelError::missing("$.totals.icbper")) };
    positive("$.totals.icbper", total, 12, 2, "2048")?;
    if !note {
        crate::total_taxes::check_sum(voucher, codes.total_taxes)?;
    } else {
        let taxes = opt(&totals.igv) + opt(&totals.ivap) + amount(total);
        within_one(codes.total_taxes, "$.totals.total_taxes", totals.total_taxes.as_ref().expect("checked"), taxes, "igv + ivap + icbper")?;
    }
    let lines: i128 = voucher.lines.iter().filter_map(|l| l.icbper.as_ref()).map(amount).sum();
    if cents(amount(total)) != cents(lines) {
        let reason = format!("must be Σ lines[].icbper ({})", show_cents(cents(lines)));
        return Err(ModelError::rule(codes.icbper_total, "$.totals.icbper", reason));
    }
    let taxes = opt(&totals.total_taxes);
    if !note {
        if let Some(subtotal) = &totals.subtotal {
            let expected = opt(&totals.sale_value) + taxes;
            within_one(codes.subtotal, "$.totals.subtotal", subtotal, expected, "sale_value + total_taxes (ICBPER included)")?;
        }
        let expected = opt(&totals.subtotal) + opt(&totals.charge_total) - opt(&totals.allowance_total) + opt(&totals.rounding);
        let formula = "subtotal + charge_total − allowance_total + rounding";
        return within_one(codes.payable, "$.totals.payable", totals.payable.as_ref().expect("checked"), expected, formula);
    }
    let bases = [&totals.taxed, &totals.unaffected, &totals.exempt, &totals.export, &totals.ivap_base];
    let expected = bases.iter().map(|b| opt(b)).sum::<i128>() + taxes + opt(&totals.rounding);
    let formula = "the sale values + total_taxes (ICBPER included) + rounding";
    within_one(codes.payable, "$.totals.payable", totals.payable.as_ref().expect("checked"), expected, formula)
}

/// A summary line's ICBPER: a positive amount in PEN (2048).
pub(crate) fn check_summary_line(index: usize, line: &SummaryLine, currency: &str) -> Result<(), ModelError> {
    let Some(icbper) = &line.icbper else { return Ok(()) };
    if currency != PEN {
        return Err(ModelError::OutOfScope { path: "$.currency".into(), feature: "ICBPER in a currency other than PEN" });
    }
    positive(&format!("$.lines[{index}].icbper"), icbper, 12, 2, "2048")
}

/// The bags of the line: `quantity` as a whole number from 1 to 99999.
fn bags(line: &Line) -> Option<i128> {
    let quantity = units(&line.quantity)?;
    let whole = quantity % SCALE == 0 && (1..=MAX_BAGS).contains(&(quantity / SCALE));
    whole.then_some(quantity / SCALE)
}

fn positive(path: &str, value: &Decimal, int_max: usize, frac_max: usize, code: &str) -> Result<(), ModelError> {
    let ok = !value.is_negative() && !value.is_zero() && fits(value, int_max, frac_max);
    let expected = format!("a positive amount with up to {int_max} integer digits and {frac_max} decimals (SUNAT {code})");
    check_format(path, value.as_str(), &expected, |_| ok)
}

fn within_one(code: &'static str, path: &str, given: &Decimal, expected: i128, formula: &str) -> Result<(), ModelError> {
    let expected = cents(expected);
    if within_one_cents(cents(amount(given)), expected) {
        return Ok(());
    }
    Err(ModelError::rule(code, path, format!("must be {formula} ({}), within 1", show_cents(expected))))
}

/// In millionths; an amount too long to read exactly counts as zero.
fn amount(value: &Decimal) -> i128 {
    units(value).unwrap_or(0)
}

fn opt(value: &Option<Decimal>) -> i128 {
    value.as_ref().map_or(0, amount)
}
