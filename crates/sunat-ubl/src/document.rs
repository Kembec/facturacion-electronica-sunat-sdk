//! Root element and header of each document type, and the order of its sections, per SUNAT's
//! UBL 2.1 guides for invoice and receipt (01, 03), credit note (07) and debit note (08). The daily
//! summary and the void communication are in `daily`; the despatch advice (09), in `despatch`; the
//! retention voucher (20), in `retention`; the purchase settlement (04), in `self_billed`.

use sunat_model::{Document, DocumentType, Invoice, Note, Voucher};

use crate::daily;
use crate::despatch;
use crate::lines::{self, CREDIT_NOTE_LINE, DEBIT_NOTE_LINE, INVOICE_LINE};
use crate::parties;
use crate::references;
use crate::retention;
use crate::self_billed;
use crate::totals::{self, MonetaryTotal};
use crate::writer::XmlWriter;
use crate::Options;

const NS_CAC: &str = "urn:oasis:names:specification:ubl:schema:xsd:CommonAggregateComponents-2";
const NS_CBC: &str = "urn:oasis:names:specification:ubl:schema:xsd:CommonBasicComponents-2";
const NS_DS: &str = "http://www.w3.org/2000/09/xmldsig#";
const NS_EXT: &str = "urn:oasis:names:specification:ubl:schema:xsd:CommonExtensionComponents-2";
const NS_SAC: &str = "urn:sunat:names:specification:ubl:peru:schema:xsd:SunatAggregateComponents-1";

/// SUNAT's customization of UBL 2.1 for these documents.
const CUSTOMIZATION_ID: &str = "2.0";

pub(crate) fn render(document: &Document, options: &Options) -> Vec<u8> {
    let mut w = XmlWriter::new();
    match document {
        Document::Invoice(invoice) => invoice_body(&mut w, invoice, options),
        Document::Note(note) => note_body(&mut w, note, options),
        Document::Summary(summary) => daily::summary_body(&mut w, summary, options),
        Document::Voided(voided) => daily::voided_body(&mut w, voided, options),
        Document::Despatch(despatch) => despatch::body(&mut w, despatch, options),
        Document::Retention(r) => retention::body(&mut w, r, options),
        Document::SelfBilled(s) => self_billed::body(&mut w, s, options),
    }
    w.finish()
}

fn invoice_body(w: &mut XmlWriter, invoice: &Invoice, options: &Options) {
    let v = &invoice.voucher;
    root(w, "Invoice", "urn:oasis:names:specification:ubl:schema:xsd:Invoice-2", false);
    header(w, v);
    if let Some(due) = &invoice.due_date {
        w.text("cbc:DueDate", &due.to_lima().date_string());
    }
    w.leaf("cbc:InvoiceTypeCode", &[("listID", invoice.operation_type.as_str())], v.document_type.code());
    legends_and_currency(w, v);
    references::order(w, v);
    references::despatches(w, v);
    references::advances(w, v, &invoice.advances);
    signature_and_parties(w, v, options);
    if let Some(detraction) = &invoice.detraction {
        totals::detraction(w, detraction);
    }
    if let Some(terms) = &v.payment_terms {
        totals::payment_terms(w, terms, &v.currency);
    }
    totals::prepaid_payments(w, &invoice.advances, &v.currency);
    totals::global_adjustments(w, &invoice.global_adjustments, &v.currency);
    totals::advance_discounts(w, &invoice.advances, &v.currency);
    if let Some(retention) = &v.igv_retention {
        totals::igv_retention(w, retention, &v.currency);
    }
    totals::tax_total(w, &v.totals, &v.currency, invoice.advances.iter().any(|a| a.discount_code() == Some("04")));
    totals::monetary_total(w, MonetaryTotal::Full("cac:LegalMonetaryTotal"), &v.totals, &v.currency);
    lines::lines(w, &INVOICE_LINE, &v.lines, &v.currency);
    w.close();
}

fn note_body(w: &mut XmlWriter, note: &Note, options: &Options) {
    let v = &note.voucher;
    let credit = v.document_type == DocumentType::CreditNote;
    if credit {
        root(w, "CreditNote", "urn:oasis:names:specification:ubl:schema:xsd:CreditNote-2", false);
    } else {
        root(w, "DebitNote", "urn:oasis:names:specification:ubl:schema:xsd:DebitNote-2", false);
    }
    header(w, v);
    legends_and_currency(w, v);
    discrepancy_response(w, note);
    references::order(w, v);
    billing_reference(w, note);
    references::despatches(w, v);
    signature_and_parties(w, v, options);
    // Payment terms apply to credit notes (reason 13 adjusts them); debit notes carry none.
    if let (true, Some(terms)) = (credit, &v.payment_terms) {
        totals::payment_terms(w, terms, &v.currency);
    }
    if let Some(retention) = &v.igv_retention {
        totals::igv_retention(w, retention, &v.currency);
    }
    totals::tax_total(w, &v.totals, &v.currency, false);
    if credit {
        totals::monetary_total(w, MonetaryTotal::PayableOnly, &v.totals, &v.currency);
        lines::lines(w, &CREDIT_NOTE_LINE, &v.lines, &v.currency);
    } else {
        totals::monetary_total(w, MonetaryTotal::Full("cac:RequestedMonetaryTotal"), &v.totals, &v.currency);
        lines::lines(w, &DEBIT_NOTE_LINE, &v.lines, &v.currency);
    }
    w.close();
}

/// Root with its namespaces in canonical order (default first, then by prefix; `sac` last, for the
/// documents that use SUNAT's aggregates) and the empty extension the signature goes into.
pub(crate) fn root(w: &mut XmlWriter, name: &'static str, namespace: &str, sunat_aggregates: bool) {
    let mut namespaces =
        vec![("xmlns", namespace), ("xmlns:cac", NS_CAC), ("xmlns:cbc", NS_CBC), ("xmlns:ds", NS_DS), ("xmlns:ext", NS_EXT)];
    if sunat_aggregates {
        namespaces.push(("xmlns:sac", NS_SAC));
    }
    w.open(name, &namespaces);
    w.open("ext:UBLExtensions", &[]);
    w.open("ext:UBLExtension", &[]);
    w.open("ext:ExtensionContent", &[]);
    w.close();
    w.close();
    w.close();
}

fn header(w: &mut XmlWriter, v: &Voucher) {
    let issued = v.issue_date.to_lima();
    w.text("cbc:UBLVersionID", &v.ubl_version);
    w.text("cbc:CustomizationID", CUSTOMIZATION_ID);
    w.text("cbc:ID", &format!("{}-{}", v.series, v.number));
    w.text("cbc:IssueDate", &issued.date_string());
    w.text("cbc:IssueTime", &issued.time_string());
}

fn legends_and_currency(w: &mut XmlWriter, v: &Voucher) {
    for legend in &v.legends {
        w.leaf("cbc:Note", &[("languageLocaleID", legend.code.as_str())], &legend.value);
    }
    w.text("cbc:DocumentCurrencyCode", &v.currency);
}

fn discrepancy_response(w: &mut XmlWriter, note: &Note) {
    let reference = &note.reference;
    w.open("cac:DiscrepancyResponse", &[]);
    w.text("cbc:ReferenceID", &reference.affected_document_number);
    w.text("cbc:ResponseCode", &reference.reason_code);
    w.text("cbc:Description", &reference.reason);
    w.close();
}

fn billing_reference(w: &mut XmlWriter, note: &Note) {
    let reference = &note.reference;
    w.open("cac:BillingReference", &[]);
    w.open("cac:InvoiceDocumentReference", &[]);
    w.text("cbc:ID", &reference.affected_document_number);
    w.text("cbc:DocumentTypeCode", &reference.affected_document_type);
    w.close();
    w.close();
}

fn signature_and_parties(w: &mut XmlWriter, v: &Voucher, options: &Options) {
    parties::signature_reference(w, &v.issuer, options);
    parties::supplier(w, &v.issuer);
    parties::customer(w, &v.customer);
}
