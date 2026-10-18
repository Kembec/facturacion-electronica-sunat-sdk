//! SUNAT's rules for each line of the purchase settlement (sheet «LiquidacionCompra2_0», rows 113 to
//! 233): quantity, item, prices, the IGV (tax 1000, 9997 or 9998), the income tax retention (tax 3000)
//! and the four properties of a purchase of gold. Amounts are compared exactly, in millionths, with the
//! ±1 the sheet allows.

use crate::despatch_rules::{blank, digits, positive_fits, rule};
use crate::retention_lines::{units, SCALE};
use crate::self_billed_rules::near;
use crate::{Decimal, IncomeTaxRetention, ModelError, SelfBilled, SelfBilledLine};

/// Catalog 07 affectations of a settlement line the SDK renders: taxed, exempt, unaffected.
const AFFECTATIONS: &[&str] = &["10", "20", "30"];

/// Income tax retention rates in force, in percent (RS 234-2005 as amended by RS 028-2013): 1.5 %,
/// and 4 % for gold. A rate SUNAT observes (4237).
const RETENTION_RATES: &[i128] = &[1_500_000, 4_000_000];

/// Catalog 55 codes a purchase of gold (0503) requires on each line, and the error for each one.
const GOLD_PROPERTIES: &[(&str, &str)] = &[("6000", "2466"), ("6004", "2467"), ("6005", "2468"), ("6006", "2469")];

pub(crate) fn check_lines(s: &SelfBilled) -> Result<(), ModelError> {
    s.lines.iter().enumerate().try_for_each(|(index, line)| {
        let at = |field: &str| format!("$.lines[{index}].{field}");
        check_item(line, &at)?;
        check_igv(line, &at)?;
        check_retention(line, &at)?;
        check_properties(&s.operation_type, line, &at)
    })
}

fn check_item(line: &SelfBilledLine, at: &dyn Fn(&str) -> String) -> Result<(), ModelError> {
    rule(!line.unit.is_empty(), "2883", at("unit"), "required")?;
    rule(!line.quantity.is_zero(), "2024", at("quantity"), "required and not zero")?;
    rule(positive_fits(&line.quantity, 12, 10), "2025", at("quantity"), "must be positive, up to 12 integer digits and 10 decimals")?;
    rule(!blank(&line.description), "2026", at("description"), "required")?;
    rule(line.description.chars().count() <= 500, "2027", at("description"), "up to 500 characters")?;
    let code = line.sunat_product_code.as_deref().filter(|c| !c.is_empty());
    rule(code.is_some(), "3506", at("sunat_product_code"), "required on every line of a purchase settlement")?;
    let code_ok = code.is_some_and(|c| c.len() == 8 && digits(c) && c != "00000000");
    rule(code_ok, "3496", at("sunat_product_code"), "must be a catalog 25 code of 8 digits")?;
    rule(positive_fits(&line.unit_value, 12, 10), "2369", at("unit_value"), "must be positive, up to 12 integer digits and 10 decimals")?;
    let Some(price) = &line.unit_price else {
        return Err(ModelError::rule("2028", at("unit_price"), "required"));
    };
    rule(positive_fits(price, 12, 10), "2367", at("unit_price"), "must be positive, up to 12 integer digits and 10 decimals")?;
    rule(positive_fits(&line.line_value, 12, 2), "2370", at("line_value"), "must be positive, up to 12 integer digits and 2 decimals")
}

fn check_igv(line: &SelfBilledLine, at: &dyn Fn(&str) -> String) -> Result<(), ModelError> {
    let reason = "must be \"10\" (taxed), \"20\" (exempt) or \"30\" (unaffected)";
    rule(AFFECTATIONS.contains(&line.affectation.as_str()), "2040", at("affectation"), reason)?;
    let rate_ok = !line.igv_rate.is_negative() && crate::validate::fits(&line.igv_rate, 3, 5);
    rule(rate_ok, "3102", at("igv_rate"), "must be positive, up to 3 integer digits and 5 decimals")?;
    let value = exact(&line.line_value, &at("line_value"))?;
    let base = exact(&line.igv_base, &at("igv_base"))?;
    near(base, value, "4294", &at("igv_base"), "must be the line_value")?;
    let igv = exact(&line.igv, &at("igv"))?;
    if line.affectation == "10" {
        let expected = base * exact(&line.igv_rate, &at("igv_rate"))? / (100 * SCALE);
        near(igv, expected, "3103", &at("igv"), "must be igv_base × igv_rate, ±1")?;
    } else {
        rule(igv == 0, "3110", at("igv"), "must be 0 on an exempt or unaffected line")?;
    }
    let total = exact(&line.total_taxes, &at("total_taxes"))?;
    near(total, igv, "4293", &at("total_taxes"), "must be the line's IGV, ±1")
}

fn check_retention(line: &SelfBilledLine, at: &dyn Fn(&str) -> String) -> Result<(), ModelError> {
    let Some(IncomeTaxRetention { base, rate, amount }) = &line.income_tax_retention else {
        return Err(ModelError::rule("2464", at("income_tax_retention"), "required on every line: the buyer withholds income tax (tax 3000)"));
    };
    let at = |field: &str| at(&format!("income_tax_retention.{field}"));
    let base = exact(base, &at("base"))?;
    near(base, exact(&line.line_value, &at("base"))?, "4294", &at("base"), "must be the line_value")?;
    let rate_ok = !rate.is_negative() && crate::validate::fits(rate, 3, 5);
    rule(rate_ok, "3102", at("rate"), "must be positive, up to 3 integer digits and 5 decimals")?;
    let rate = exact(rate, &at("rate"))?;
    rule(RETENTION_RATES.contains(&rate), "4237", at("rate"), "must be a rate in force: 1.50, or 4.00 for gold")?;
    let amount_ok = !amount.is_negative() && crate::validate::fits(amount, 12, 2);
    rule(amount_ok, "2033", at("amount"), "must be positive, up to 12 integer digits and 2 decimals")?;
    let amount = exact(amount, &at("amount"))?;
    near(amount, base * rate / (100 * SCALE), "2464", &at("amount"), "must be base × rate, ±1")
}

fn check_properties(operation: &str, line: &SelfBilledLine, at: &dyn Fn(&str) -> String) -> Result<(), ModelError> {
    if operation != "0503" {
        let reason = "applies to operation type 0503 (purchase of gold) only";
        return rule(line.item_properties.is_empty(), "3064", at("item_properties"), reason);
    }
    for (index, property) in line.item_properties.iter().enumerate() {
        let path = at(&format!("item_properties[{index}]"));
        if !GOLD_PROPERTIES.iter().any(|(code, _)| *code == property.code) {
            return Err(ModelError::invalid(format!("{path}.code"), "expected \"6000\", \"6004\", \"6005\" or \"6006\""));
        }
        rule(!blank(&property.value), "3064", format!("{path}.value"), "required")?;
    }
    for (code, error) in GOLD_PROPERTIES {
        let present = line.item_properties.iter().any(|p| p.code == *code);
        rule(present, error, at("item_properties"), "a purchase of gold carries properties 6000, 6004, 6005 and 6006")?;
    }
    Ok(())
}

fn exact(value: &Decimal, path: &str) -> Result<i128, ModelError> {
    units(value).ok_or_else(|| ModelError::invalid(path, "up to 6 decimals"))
}
