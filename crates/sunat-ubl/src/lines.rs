//! Document lines (`InvoiceLine`, `CreditNoteLine`, `DebitNoteLine`), per the «Datos por línea»
//! sections of SUNAT's UBL 2.1 guides.

use sunat_model::{Decimal, Line};

use crate::amount::{shortest, two_decimals};
use crate::tax::{scheme_for_affectation, TaxScheme, ICBPER};
use crate::writer::XmlWriter;

/// Catalog 16: unit price including taxes (onerous) or referential unit value (free transfer).
const PRICE_WITH_TAXES: &str = "01";
const REFERENTIAL_VALUE: &str = "02";

/// Element names of a line, which change with the document type.
pub(crate) struct LineNames {
    pub(crate) line: &'static str,
    pub(crate) quantity: &'static str,
}

pub(crate) const INVOICE_LINE: LineNames = LineNames { line: "cac:InvoiceLine", quantity: "cbc:InvoicedQuantity" };
pub(crate) const CREDIT_NOTE_LINE: LineNames =
    LineNames { line: "cac:CreditNoteLine", quantity: "cbc:CreditedQuantity" };
pub(crate) const DEBIT_NOTE_LINE: LineNames = LineNames { line: "cac:DebitNoteLine", quantity: "cbc:DebitedQuantity" };

pub(crate) fn lines(w: &mut XmlWriter, names: &LineNames, lines: &[Line], currency: &str) {
    for (index, line) in lines.iter().enumerate() {
        one_line(w, names, index + 1, line, currency);
    }
}

fn one_line(w: &mut XmlWriter, names: &LineNames, number: usize, line: &Line, currency: &str) {
    let money = [("currencyID", currency)];
    w.open(names.line, &[]);
    w.text("cbc:ID", &number.to_string());
    w.leaf(names.quantity, &[("unitCode", line.unit.as_str())], &shortest(&line.quantity));
    w.leaf("cbc:LineExtensionAmount", &money, &two_decimals(&line.line_value));
    pricing_reference(w, line, currency);
    discounts(w, line, currency);
    line_tax_total(w, line, currency);
    item(w, line);
    w.open("cac:Price", &[]);
    w.leaf("cbc:PriceAmount", &money, &shortest(&line.unit_value));
    w.close();
    w.close();
}

/// A free transfer declares its referential value (code 02); any other line, its unit price with
/// taxes (code 01) when given.
fn pricing_reference(w: &mut XmlWriter, line: &Line, currency: &str) {
    let (amount, code) = match (&line.free_unit_value, &line.unit_price) {
        (Some(free), _) => (free, REFERENTIAL_VALUE),
        (None, Some(price)) => (price, PRICE_WITH_TAXES),
        (None, None) => return,
    };
    w.open("cac:PricingReference", &[]);
    w.open("cac:AlternativeConditionPrice", &[]);
    w.leaf("cbc:PriceAmount", &[("currencyID", currency)], &shortest(amount));
    w.text("cbc:PriceTypeCode", code);
    w.close();
    w.close();
}

/// Line discounts (catalog 53 00, 01), then line charges (47, 48); `ChargeIndicator` follows from
/// the code. Factor, amount and base print in the shortest form, as the compatibility vectors carry
/// them (`0.10` -> `0.1`, `10.00` -> `10`).
fn discounts(w: &mut XmlWriter, line: &Line, currency: &str) {
    let money = [("currencyID", currency)];
    for entry in line.discounts.iter().chain(&line.charges) {
        w.open("cac:AllowanceCharge", &[]);
        w.text("cbc:ChargeIndicator", if entry.is_charge() { "true" } else { "false" });
        w.text("cbc:AllowanceChargeReasonCode", &entry.code);
        w.text("cbc:MultiplierFactorNumeric", &factor_text(&entry.factor));
        w.leaf("cbc:Amount", &money, &shortest(&entry.amount));
        w.leaf("cbc:BaseAmount", &money, &shortest(&entry.base_amount));
        w.close();
    }
}

/// The factor never in exponent form, which SUNAT rejects: below 1e-4 it prints with five decimals.
pub(crate) fn factor_text(factor: &Decimal) -> String {
    let text = shortest(factor);
    if text.contains('E') {
        format!("{:.5}", factor.to_f64())
    } else {
        text
    }
}

fn line_tax_total(w: &mut XmlWriter, line: &Line, currency: &str) {
    let money = [("currencyID", currency)];
    w.open("cac:TaxTotal", &[]);
    w.leaf("cbc:TaxAmount", &money, &two_decimals(&line.total_taxes));
    w.open("cac:TaxSubtotal", &[]);
    w.leaf("cbc:TaxableAmount", &money, &two_decimals(&line.igv_base));
    w.leaf("cbc:TaxAmount", &money, &two_decimals(&line.igv));
    w.open("cac:TaxCategory", &[]);
    w.text("cbc:Percent", &shortest(&line.igv_rate));
    w.text("cbc:TaxExemptionReasonCode", &line.affectation);
    tax_scheme(w, scheme_for_affectation(&line.affectation));
    w.close();
    w.close();
    if let (Some(icbper), Some(factor)) = (&line.icbper, &line.icbper_factor) {
        icbper_subtotal(w, line, icbper, factor, currency);
    }
    w.close();
}

/// The bags are the line's `quantity` (`BaseUnitMeasure`, always `NIU`: SUNAT 4320 observes any
/// other), and the rate per bag prints with at least two decimals, as SUNAT's example does.
fn icbper_subtotal(w: &mut XmlWriter, line: &Line, icbper: &Decimal, factor: &Decimal, currency: &str) {
    let money = [("currencyID", currency)];
    w.open("cac:TaxSubtotal", &[]);
    w.leaf("cbc:TaxAmount", &money, &two_decimals(icbper));
    w.leaf("cbc:BaseUnitMeasure", &[("unitCode", "NIU")], &shortest(&line.quantity));
    w.open("cac:TaxCategory", &[]);
    w.leaf("cbc:PerUnitAmount", &money, &at_least_two_decimals(factor));
    tax_scheme(w, ICBPER);
    w.close();
    w.close();
}

/// `0.5` -> `0.50`, `0.12345` stays: never fewer than two decimals, never a digit dropped.
pub(crate) fn at_least_two_decimals(value: &Decimal) -> String {
    let text = value.as_str();
    let decimals = text.split_once('.').map_or(0, |(_, frac)| frac.trim_end_matches('0').len());
    if decimals <= 2 {
        return two_decimals(value);
    }
    let (int, frac) = text.split_once('.').expect("has decimals");
    format!("{int}.{}", frac.trim_end_matches('0'))
}

fn item(w: &mut XmlWriter, line: &Line) {
    w.open("cac:Item", &[]);
    w.text("cbc:Description", &line.description);
    if let Some(code) = line.product_code.as_deref().filter(|c| !c.is_empty()) {
        w.open("cac:SellersItemIdentification", &[]);
        w.text("cbc:ID", code);
        w.close();
    }
    if let Some(code) = line.sunat_product_code.as_deref().filter(|c| !c.is_empty()) {
        w.open("cac:CommodityClassification", &[]);
        w.text("cbc:ItemClassificationCode", code);
        w.close();
    }
    w.close();
}

pub(crate) fn tax_scheme(w: &mut XmlWriter, scheme: TaxScheme) {
    w.open("cac:TaxScheme", &[]);
    w.text("cbc:ID", scheme.id);
    w.text("cbc:Name", scheme.name);
    w.text("cbc:TaxTypeCode", scheme.type_code);
    w.close();
}
