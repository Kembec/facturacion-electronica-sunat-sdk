//! Daily summary (RC) and void communication (RA): UBL 2.0 documents with SUNAT's aggregates
//! (`sac:`), in the element order of the compatibility vectors `tests/vectors/rc-*` and `ra-*`.

use sunat_model::{Company, Date, Decimal, Summary, SummaryLine, Voided, VoidedLine};

use crate::amount::two_decimals;
use crate::document::root;
use crate::lines::tax_scheme;
use crate::parties::{signature_reference, RUC_SCHEME};
use crate::tax::{FREE, ICBPER, IGV};
use crate::writer::XmlWriter;
use crate::Options;

const UBL_VERSION: &str = "2.0";
const SUMMARY_CUSTOMIZATION: &str = "1.1";
const VOIDED_CUSTOMIZATION: &str = "1.0";

pub(crate) fn summary_body(w: &mut XmlWriter, summary: &Summary, options: &Options) {
    root(w, "SummaryDocuments", "urn:sunat:names:specification:ubl:peru:schema:xsd:SummaryDocuments-1", true);
    header(w, SUMMARY_CUSTOMIZATION, &summary.identifier(), &summary.reference_date, &summary.generation_date);
    signature_reference(w, &summary.issuer, options);
    supplier(w, &summary.issuer);
    for (index, line) in summary.lines.iter().enumerate() {
        summary_line(w, index + 1, line, &summary.currency);
    }
    w.close();
}

pub(crate) fn voided_body(w: &mut XmlWriter, voided: &Voided, options: &Options) {
    root(w, "VoidedDocuments", "urn:sunat:names:specification:ubl:peru:schema:xsd:VoidedDocuments-1", true);
    header(w, VOIDED_CUSTOMIZATION, &voided.identifier(), &voided.generation_date, &voided.communication_date);
    signature_reference(w, &voided.issuer, options);
    supplier(w, &voided.issuer);
    for (index, line) in voided.lines.iter().enumerate() {
        voided_line(w, index + 1, line);
    }
    w.close();
}

fn header(w: &mut XmlWriter, customization: &str, id: &str, reference_date: &Date, issue_date: &Date) {
    w.text("cbc:UBLVersionID", UBL_VERSION);
    w.text("cbc:CustomizationID", customization);
    w.text("cbc:ID", id);
    w.text("cbc:ReferenceDate", &reference_date.date_string());
    w.text("cbc:IssueDate", &issue_date.date_string());
}

/// The issuer in the UBL 2.0 shape: RUC and its catalog 06 code, then the legal name.
fn supplier(w: &mut XmlWriter, issuer: &Company) {
    w.open("cac:AccountingSupplierParty", &[]);
    w.text("cbc:CustomerAssignedAccountID", &issuer.ruc);
    w.text("cbc:AdditionalAccountID", RUC_SCHEME);
    w.open("cac:Party", &[]);
    w.open("cac:PartyLegalEntity", &[]);
    w.text("cbc:RegistrationName", &issuer.legal_name);
    w.close();
    w.close();
    w.close();
}

fn summary_line(w: &mut XmlWriter, line_id: usize, line: &SummaryLine, currency: &str) {
    let money = [("currencyID", currency)];
    w.open("sac:SummaryDocumentsLine", &[]);
    w.text("cbc:LineID", &line_id.to_string());
    w.text("cbc:DocumentTypeCode", &line.document_type);
    w.text("cbc:ID", &line.document_number);
    w.open("cac:AccountingCustomerParty", &[]);
    w.text("cbc:CustomerAssignedAccountID", &line.customer_document_number);
    w.text("cbc:AdditionalAccountID", &line.customer_document_type);
    if let Some(name) = &line.customer_name {
        w.open("cac:Party", &[]);
        w.open("cac:PartyLegalEntity", &[]);
        w.text("cbc:RegistrationName", name);
        w.close();
        w.close();
    }
    w.close();
    if let Some(reference) = &line.reference_document {
        w.open("cac:BillingReference", &[]);
        w.open("cac:InvoiceDocumentReference", &[]);
        w.text("cbc:ID", &reference.document_number);
        w.text("cbc:DocumentTypeCode", &reference.document_type);
        w.close();
        w.close();
    }
    w.open("cac:Status", &[]);
    w.text("cbc:ConditionCode", &line.status);
    w.close();
    w.leaf("sac:TotalAmount", &money, &two_decimals(&line.total));
    billing_payments(w, line, &money);
    if let Some(charges) = non_zero(&line.other_charges) {
        w.open("cac:AllowanceCharge", &[]);
        w.text("cbc:ChargeIndicator", "true");
        w.leaf("cbc:Amount", &money, &two_decimals(charges));
        w.close();
    }
    igv_total(w, line.igv.as_ref(), line.igv_rate.as_ref(), &money);
    if let Some(free_igv) = non_zero(&line.free_igv) {
        free_igv_total(w, free_igv, &money);
    }
    if let Some(icbper) = &line.icbper {
        icbper_total(w, icbper, &money);
    }
    w.close();
}

/// IGV of free sales (field 25): its own `cac:TaxTotal` with tax 9996, after the IGV one. The model only lets
/// it through from 2027-01-01.
fn free_igv_total(w: &mut XmlWriter, amount: &Decimal, money: &[(&str, &str)]) {
    let amount = two_decimals(amount);
    w.open("cac:TaxTotal", &[]);
    w.leaf("cbc:TaxAmount", money, &amount);
    w.open("cac:TaxSubtotal", &[]);
    w.leaf("cbc:TaxAmount", money, &amount);
    w.open("cac:TaxCategory", &[]);
    tax_scheme(w, FREE);
    w.close();
    w.close();
    w.close();
}

/// ICBPER as its own `cac:TaxTotal` after the IGV one, one per tax code (SUNAT 2355), with the same
/// amount in its subtotal (2344).
fn icbper_total(w: &mut XmlWriter, icbper: &Decimal, money: &[(&str, &str)]) {
    let amount = two_decimals(icbper);
    w.open("cac:TaxTotal", &[]);
    w.leaf("cbc:TaxAmount", money, &amount);
    w.open("cac:TaxSubtotal", &[]);
    w.leaf("cbc:TaxAmount", money, &amount);
    w.open("cac:TaxCategory", &[]);
    tax_scheme(w, ICBPER);
    w.close();
    w.close();
    w.close();
}

/// One `sac:BillingPayment` per sale type (catalog 11). Taxed (01) is rendered whenever given, even
/// zero; exempt (02), unaffected (03), export (04), free (05) and the free ones by type (06 to 09, from
/// 2027-01-01) only when non-zero, as the vectors do.
fn billing_payments(w: &mut XmlWriter, line: &SummaryLine, money: &[(&str, &str)]) {
    let payments = [
        ("01", line.taxed.as_ref()),
        ("02", non_zero(&line.exempt)),
        ("03", non_zero(&line.unaffected)),
        ("04", non_zero(&line.export)),
        ("05", non_zero(&line.free)),
        ("06", non_zero(&line.free_taxed)),
        ("07", non_zero(&line.free_exempt)),
        ("08", non_zero(&line.free_unaffected)),
        ("09", non_zero(&line.free_export)),
    ];
    for (instruction, amount) in payments {
        let Some(amount) = amount else { continue };
        w.open("sac:BillingPayment", &[]);
        w.leaf("cbc:PaidAmount", money, &two_decimals(amount));
        w.text("cbc:InstructionID", instruction);
        w.close();
    }
}

/// The IGV total is always present: `0.00` when the line gives none. Its rate (`cbc:Percent`, field 21,
/// SUNAT errors 2992, 3504 and 3102) only when the line gives one.
fn igv_total(w: &mut XmlWriter, igv: Option<&Decimal>, rate: Option<&Decimal>, money: &[(&str, &str)]) {
    let amount = igv.map_or_else(|| "0.00".to_owned(), two_decimals);
    w.open("cac:TaxTotal", &[]);
    w.leaf("cbc:TaxAmount", money, &amount);
    w.open("cac:TaxSubtotal", &[]);
    w.leaf("cbc:TaxAmount", money, &amount);
    w.open("cac:TaxCategory", &[]);
    if let Some(rate) = rate {
        w.text("cbc:Percent", &two_decimals(rate));
    }
    tax_scheme(w, IGV);
    w.close();
    w.close();
    w.close();
}

fn voided_line(w: &mut XmlWriter, line_id: usize, line: &VoidedLine) {
    w.open("sac:VoidedDocumentsLine", &[]);
    w.text("cbc:LineID", &line_id.to_string());
    w.text("cbc:DocumentTypeCode", &line.document_type);
    w.text("sac:DocumentSerialID", &line.series);
    w.text("sac:DocumentNumberID", &line.number);
    w.text("sac:VoidReasonDescription", &line.reason);
    w.close();
}

fn non_zero(amount: &Option<Decimal>) -> Option<&Decimal> {
    amount.as_ref().filter(|a| !a.is_zero())
}
