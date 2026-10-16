//! Document-level sections: payment terms («Forma de pago»), the tax total with one subtotal per
//! scheme («Total de tributos») and the monetary totals («Totales del comprobante»).

use sunat_model::{Decimal, IgvRetention, PaymentTerms, Totals};

use crate::amount::two_decimals;
use crate::lines::{factor_text, tax_scheme};
use crate::tax::{TaxScheme, EXEMPT, EXPORT, FREE, IGV, UNAFFECTED};
use crate::writer::XmlWriter;

const PAYMENT_TERMS_ID: &str = "FormaPago";

pub(crate) fn payment_terms(w: &mut XmlWriter, terms: &PaymentTerms, currency: &str) {
    match terms {
        PaymentTerms::Cash => {
            w.open("cac:PaymentTerms", &[]);
            w.text("cbc:ID", PAYMENT_TERMS_ID);
            w.text("cbc:PaymentMeansID", "Contado");
            w.close();
        }
        PaymentTerms::Credit { amount, currency: own, installments } => {
            let currency = own.as_deref().unwrap_or(currency);
            w.open("cac:PaymentTerms", &[]);
            w.text("cbc:ID", PAYMENT_TERMS_ID);
            w.text("cbc:PaymentMeansID", "Credito");
            w.leaf("cbc:Amount", &[("currencyID", currency)], &two_decimals(amount));
            w.close();
            for (index, installment) in installments.iter().enumerate() {
                let currency = installment.currency.as_deref().unwrap_or(currency);
                w.open("cac:PaymentTerms", &[]);
                w.text("cbc:ID", PAYMENT_TERMS_ID);
                w.text("cbc:PaymentMeansID", &format!("Cuota{:03}", index + 1));
                w.leaf("cbc:Amount", &[("currencyID", currency)], &two_decimals(&installment.amount));
                w.text("cbc:PaymentDueDate", &installment.due_date.to_lima().date_string());
                w.close();
            }
        }
    }
}

/// The IGV retention as a global allowance with catalog 53 code `62`, right before the tax total,
/// where UBL places `cac:AllowanceCharge` in invoices, receipts and both notes.
pub(crate) fn igv_retention(w: &mut XmlWriter, retention: &IgvRetention, currency: &str) {
    let money = [("currencyID", currency)];
    w.open("cac:AllowanceCharge", &[]);
    w.text("cbc:ChargeIndicator", "false");
    w.text("cbc:AllowanceChargeReasonCode", "62");
    w.text("cbc:MultiplierFactorNumeric", &factor_text(&retention.factor));
    w.leaf("cbc:Amount", &money, &two_decimals(&retention.amount));
    w.leaf("cbc:BaseAmount", &money, &two_decimals(&retention.base_amount));
    w.close();
}

/// `cac:TaxTotal`: the total of taxes, then one subtotal per scheme with a non-zero base, in
/// descending catalog 05 order (IGV 1000, INA 9998, EXO 9997, GRA 9996, EXP 9995).
pub(crate) fn tax_total(w: &mut XmlWriter, totals: &Totals, currency: &str) {
    let zero: Decimal = "0".parse().expect("literal decimal");
    let subtotals: [(TaxScheme, Option<&Decimal>, Option<&Decimal>); 5] = [
        (IGV, totals.taxed.as_ref(), totals.igv.as_ref()),
        (UNAFFECTED, totals.unaffected.as_ref(), None),
        (EXEMPT, totals.exempt.as_ref(), None),
        (FREE, totals.free.as_ref(), totals.free_igv.as_ref()),
        (EXPORT, totals.export.as_ref(), None),
    ];
    let money = [("currencyID", currency)];
    w.open("cac:TaxTotal", &[]);
    w.leaf("cbc:TaxAmount", &money, &two_decimals(totals.total_taxes.as_ref().unwrap_or(&zero)));
    for (scheme, base, tax) in subtotals {
        let Some(base) = base.filter(|b| !b.is_zero()) else { continue };
        w.open("cac:TaxSubtotal", &[]);
        w.leaf("cbc:TaxableAmount", &money, &two_decimals(base));
        w.leaf("cbc:TaxAmount", &money, &two_decimals(tax.unwrap_or(&zero)));
        w.open("cac:TaxCategory", &[]);
        tax_scheme(w, scheme);
        w.close();
        w.close();
    }
    w.close();
}

/// Which monetary amounts a document type declares.
pub(crate) enum MonetaryTotal {
    /// Invoice and receipt (`LegalMonetaryTotal`) and debit note (`RequestedMonetaryTotal`):
    /// sale value, total with taxes, rounding and payable amount.
    Full(&'static str),
    /// Credit note: rounding and payable amount.
    PayableOnly,
}

pub(crate) fn monetary_total(w: &mut XmlWriter, kind: MonetaryTotal, totals: &Totals, currency: &str) {
    let money = [("currencyID", currency)];
    let name = match kind {
        MonetaryTotal::Full(name) => name,
        MonetaryTotal::PayableOnly => "cac:LegalMonetaryTotal",
    };
    w.open(name, &[]);
    if let MonetaryTotal::Full(_) = kind {
        amount_opt(w, "cbc:LineExtensionAmount", totals.sale_value.as_ref(), &money);
        amount_opt(w, "cbc:TaxInclusiveAmount", totals.subtotal.as_ref(), &money);
    }
    if let Some(rounding) = totals.rounding.as_ref().filter(|r| !r.is_zero()) {
        w.leaf("cbc:PayableRoundingAmount", &money, &two_decimals(rounding));
    }
    amount_opt(w, "cbc:PayableAmount", totals.payable.as_ref(), &money);
    w.close();
}

fn amount_opt(w: &mut XmlWriter, name: &'static str, value: Option<&Decimal>, money: &[(&str, &str)]) {
    if let Some(value) = value {
        w.leaf(name, money, &two_decimals(value));
    }
}
