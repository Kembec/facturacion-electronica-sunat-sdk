//! The purchase settlement (04), UBL 2.1 `SelfBilledInvoice`, per SUNAT's sheet
//! «LiquidacionCompra2_0»: the buyer who issues is `AccountingCustomerParty` and the seller
//! `AccountingSupplierParty`, the reverse of an invoice; the place of the purchase goes in
//! `DeliveryTerms`; every line carries the IGV and the income tax retention (tax 3000), and the
//! document's tax total carries the retention as a subtotal that its `TaxAmount` does not add. The
//! order of the elements is UBL 2.1's `SelfBilledInvoice` schema.

use sunat_model::{Decimal, OperationPlace, SelfBilled, SelfBilledLine, SelfBilledSeller, SelfBilledTotals};

use crate::amount::{shortest, two_decimals};
use crate::document::root;
use crate::lines::tax_scheme;
use crate::parties::{issuer_party, non_empty, party_identification, signature_reference};
use crate::tax::{scheme_for_affectation, TaxScheme, EXEMPT, IGV, UNAFFECTED};
use crate::writer::XmlWriter;
use crate::Options;

/// Catalog 05: income tax (retención de renta).
const INCOME_TAX: TaxScheme = TaxScheme { id: "3000", name: "IR", type_code: "TOX" };
/// Catalog 16: unit price including taxes.
const PRICE_WITH_TAXES: &str = "01";
const CUSTOMIZATION_ID: &str = "2.0";

/// Catalog 55 names of the gold properties (sheet rows 152 to 164).
fn property_name(code: &str) -> &'static str {
    match code {
        "6000" => "Codigo unico de concesion minera",
        "6004" => "Ley mineral",
        "6005" => "Naturaleza del mineral",
        _ => "Nombre del derecho minero",
    }
}

pub(crate) fn body(w: &mut XmlWriter, s: &SelfBilled, options: &Options) {
    let issued = s.issue_date.to_lima();
    root(w, "SelfBilledInvoice", "urn:oasis:names:specification:ubl:schema:xsd:SelfBilledInvoice-2", false);
    w.text("cbc:UBLVersionID", &s.ubl_version);
    w.text("cbc:CustomizationID", CUSTOMIZATION_ID);
    w.text("cbc:ID", &s.identifier());
    w.text("cbc:IssueDate", &issued.date_string());
    w.text("cbc:IssueTime", &issued.time_string());
    w.leaf("cbc:InvoiceTypeCode", &[("listID", s.operation_type.as_str())], &s.document_type);
    for legend in &s.legends {
        w.leaf("cbc:Note", &[("languageLocaleID", legend.code.as_str())], &legend.value);
    }
    w.text("cbc:DocumentCurrencyCode", &s.currency);
    for reference in &s.despatch_references {
        w.open("cac:DespatchDocumentReference", &[]);
        w.text("cbc:ID", &reference.number);
        w.text("cbc:DocumentTypeCode", &reference.document_type);
        w.close();
    }
    signature_reference(w, &s.issuer, options);
    issuer_party(w, "cac:AccountingCustomerParty", &s.issuer);
    seller(w, &s.seller);
    if let Some(place) = &s.operation_place {
        delivery_terms(w, place);
    }
    tax_total(w, &s.totals, &s.currency);
    monetary_total(w, &s.totals, &s.currency);
    for (index, line) in s.lines.iter().enumerate() {
        one_line(w, index + 1, line, &s.currency);
    }
    w.close();
}

fn seller(w: &mut XmlWriter, seller: &SelfBilledSeller) {
    let a = &seller.address;
    w.open("cac:AccountingSupplierParty", &[]);
    w.open("cac:Party", &[]);
    party_identification(w, &seller.document_type, &seller.document_number);
    w.open("cac:PartyLegalEntity", &[]);
    w.text("cbc:RegistrationName", &seller.legal_name);
    w.open("cac:RegistrationAddress", &[]);
    let fields = Address { ubigeo: &a.ubigeo, urbanization: &a.urbanization, province: &a.province, department: &a.department, district: &a.district, street: &a.street, country: &a.country_code };
    address(w, &fields, a.address_type.as_deref());
    w.close();
    w.close();
    w.close();
    w.close();
}

fn delivery_terms(w: &mut XmlWriter, p: &OperationPlace) {
    w.open("cac:DeliveryTerms", &[]);
    w.open("cac:DeliveryLocation", &[]);
    w.text_opt("cbc:LocationTypeCode", p.location_type.as_deref());
    w.open("cac:Address", &[]);
    let fields = Address { ubigeo: &p.ubigeo, urbanization: &p.urbanization, province: &p.province, department: &p.department, district: &p.district, street: &p.street, country: &p.country_code };
    address(w, &fields, None);
    w.close();
    w.close();
    w.close();
}

struct Address<'a> {
    ubigeo: &'a Option<String>,
    urbanization: &'a Option<String>,
    province: &'a Option<String>,
    department: &'a Option<String>,
    district: &'a Option<String>,
    street: &'a Option<String>,
    country: &'a Option<String>,
}

/// The children of a UBL `AddressType` in schema order.
fn address(w: &mut XmlWriter, a: &Address, type_code: Option<&str>) {
    w.text_opt("cbc:ID", a.ubigeo.as_deref());
    w.text_opt("cbc:AddressTypeCode", type_code);
    w.text_opt("cbc:CitySubdivisionName", a.urbanization.as_deref());
    w.text_opt("cbc:CityName", a.province.as_deref());
    w.text_opt("cbc:CountrySubentity", a.department.as_deref());
    w.text_opt("cbc:District", a.district.as_deref());
    if let Some(street) = non_empty(a.street) {
        w.open("cac:AddressLine", &[]);
        w.text("cbc:Line", street);
        w.close();
    }
    if let Some(country) = non_empty(a.country) {
        w.open("cac:Country", &[]);
        w.text("cbc:IdentificationCode", country);
        w.close();
    }
}

/// `TaxAmount` is the IGV only (SUNAT 4301); the income tax retention is a subtotal of its own.
fn tax_total(w: &mut XmlWriter, t: &SelfBilledTotals, currency: &str) {
    let zero: Decimal = "0".parse().expect("zero");
    w.open("cac:TaxTotal", &[]);
    w.leaf("cbc:TaxAmount", &[("currencyID", currency)], &two_decimals(t.total_taxes.as_ref().unwrap_or(&zero)));
    if let Some(taxed) = &t.taxed {
        subtotal(w, taxed, t.igv.as_ref().unwrap_or(&zero), IGV, currency);
    }
    if let Some(exempt) = &t.exempt {
        subtotal(w, exempt, &zero, EXEMPT, currency);
    }
    if let Some(unaffected) = &t.unaffected {
        subtotal(w, unaffected, &zero, UNAFFECTED, currency);
    }
    if let (Some(base), Some(amount)) = (&t.income_tax_retention_base, &t.income_tax_retention) {
        subtotal(w, base, amount, INCOME_TAX, currency);
    }
    w.close();
}

fn subtotal(w: &mut XmlWriter, base: &Decimal, amount: &Decimal, scheme: TaxScheme, currency: &str) {
    let money = [("currencyID", currency)];
    w.open("cac:TaxSubtotal", &[]);
    w.leaf("cbc:TaxableAmount", &money, &two_decimals(base));
    w.leaf("cbc:TaxAmount", &money, &two_decimals(amount));
    w.open("cac:TaxCategory", &[]);
    tax_scheme(w, scheme);
    w.close();
    w.close();
}

fn monetary_total(w: &mut XmlWriter, t: &SelfBilledTotals, currency: &str) {
    let money = [("currencyID", currency)];
    w.open("cac:LegalMonetaryTotal", &[]);
    if let Some(value) = &t.sale_value {
        w.leaf("cbc:LineExtensionAmount", &money, &two_decimals(value));
    }
    if let Some(subtotal) = &t.subtotal {
        w.leaf("cbc:TaxInclusiveAmount", &money, &two_decimals(subtotal));
    }
    if let Some(rounding) = &t.rounding {
        w.leaf("cbc:PayableRoundingAmount", &money, &two_decimals(rounding));
    }
    if let Some(payable) = &t.payable {
        w.leaf("cbc:PayableAmount", &money, &two_decimals(payable));
    }
    w.close();
}

fn one_line(w: &mut XmlWriter, number: usize, line: &SelfBilledLine, currency: &str) {
    let money = [("currencyID", currency)];
    w.open("cac:InvoiceLine", &[]);
    w.text("cbc:ID", &number.to_string());
    w.leaf("cbc:InvoicedQuantity", &[("unitCode", line.unit.as_str())], &shortest(&line.quantity));
    w.leaf("cbc:LineExtensionAmount", &money, &two_decimals(&line.line_value));
    if let Some(price) = &line.unit_price {
        w.open("cac:PricingReference", &[]);
        w.open("cac:AlternativeConditionPrice", &[]);
        w.leaf("cbc:PriceAmount", &money, &shortest(price));
        w.text("cbc:PriceTypeCode", PRICE_WITH_TAXES);
        w.close();
        w.close();
    }
    line_tax_total(w, line, currency);
    item(w, line);
    w.open("cac:Price", &[]);
    w.leaf("cbc:PriceAmount", &money, &shortest(&line.unit_value));
    w.close();
    w.close();
}

/// Two subtotals: the IGV with its affectation, and the income tax retention without one (SUNAT 3050).
fn line_tax_total(w: &mut XmlWriter, line: &SelfBilledLine, currency: &str) {
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
    if let Some(retention) = &line.income_tax_retention {
        w.open("cac:TaxSubtotal", &[]);
        w.leaf("cbc:TaxableAmount", &money, &two_decimals(&retention.base));
        w.leaf("cbc:TaxAmount", &money, &two_decimals(&retention.amount));
        w.open("cac:TaxCategory", &[]);
        w.text("cbc:Percent", &shortest(&retention.rate));
        tax_scheme(w, INCOME_TAX);
        w.close();
        w.close();
    }
    w.close();
}

fn item(w: &mut XmlWriter, line: &SelfBilledLine) {
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
    for property in &line.item_properties {
        w.open("cac:AdditionalItemProperty", &[]);
        w.text("cbc:Name", property_name(&property.code));
        w.text("cbc:NameCode", &property.code);
        w.text("cbc:Value", &property.value);
        w.close();
    }
    w.close();
}
