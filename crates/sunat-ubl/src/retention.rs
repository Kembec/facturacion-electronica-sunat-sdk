//! The retention voucher (20): UBL 2.0 `Retention` with SUNAT's customization 1.0, in the element
//! order of the template SUNAT's systems have accepted for years (Greenter's `retention.xml.twig`)
//! and the content of the sheet «Retenciones1_0» of SUNAT's validation rules.
//!
//! Two departures from that template, both from the sheet: the agent's `cbc:CityName` is the
//! province and `cbc:CountrySubentity` the department (data 14 and 15; the template swaps them), and
//! `cac:Payment/cbc:ID` is the payment number of the related document, not the line's position.
//! Every amount of the voucher itself (retained, paid, rounding) is in PEN.

use sunat_model::{Address, Retention, RetentionLine};

use crate::document::root;
use crate::parties::{signature_reference, RUC_SCHEME};
use crate::writer::XmlWriter;
use crate::Options;

const NAMESPACE: &str = "urn:sunat:names:specification:ubl:peru:schema:xsd:Retention-1";

/// SUNAT's customization of UBL 2.0 for the retention voucher.
const CUSTOMIZATION_ID: &str = "1.0";

const PEN: &[(&str, &str)] = &[("currencyID", "PEN")];

pub(crate) fn body(w: &mut XmlWriter, r: &Retention, options: &Options) {
    root(w, "Retention", NAMESPACE, true);
    let issued = r.issue_date.to_lima();
    w.text("cbc:UBLVersionID", &r.ubl_version);
    w.text("cbc:CustomizationID", CUSTOMIZATION_ID);
    signature_reference(w, &r.issuer, options);
    w.text("cbc:ID", &r.identifier());
    w.text("cbc:IssueDate", &issued.date_string());
    w.text("cbc:IssueTime", &issued.time_string());
    agent(w, r);
    receiver(w, r);
    w.text("sac:SUNATRetentionSystemCode", &r.regime);
    w.text("sac:SUNATRetentionPercent", r.rate.as_str());
    w.text_opt("cbc:Note", r.note.as_deref());
    w.leaf("cbc:TotalInvoiceAmount", PEN, r.total_retained.as_str());
    w.leaf("sac:SUNATTotalPaid", PEN, r.total_paid.as_str());
    if let Some(rounding) = &r.rounding_amount {
        w.leaf("cbc:PayableRoundingAmount", PEN, rounding.as_str());
    }
    for (index, line) in r.lines.iter().enumerate() {
        document_reference(w, line, r.payment_number(index));
    }
    w.close();
}

fn agent(w: &mut XmlWriter, r: &Retention) {
    let issuer = &r.issuer;
    w.open("cac:AgentParty", &[]);
    party_identification(w, RUC_SCHEME, &issuer.ruc);
    if let Some(trade_name) = filled(issuer.trade_name.as_deref()) {
        w.open("cac:PartyName", &[]);
        w.text("cbc:Name", trade_name);
        w.close();
    }
    postal_address(w, &issuer.address);
    w.open("cac:PartyLegalEntity", &[]);
    w.text("cbc:RegistrationName", &issuer.legal_name);
    w.close();
    w.close();
}

/// Ubigeo, street, urbanization, province, department, district and country (sheet data 11 to 17).
/// An urbanization of `-`, which some systems write for «none», is left out.
fn postal_address(w: &mut XmlWriter, a: &Address) {
    w.open("cac:PostalAddress", &[]);
    w.text_opt("cbc:ID", filled(a.ubigeo.as_deref()));
    w.text_opt("cbc:StreetName", filled(a.street.as_deref()));
    w.text_opt("cbc:CitySubdivisionName", filled(a.urbanization.as_deref()).filter(|u| *u != "-"));
    w.text_opt("cbc:CityName", filled(a.province.as_deref()));
    w.text_opt("cbc:CountrySubentity", filled(a.department.as_deref()));
    w.text_opt("cbc:District", filled(a.district.as_deref()));
    if let Some(country) = filled(a.country_code.as_deref()) {
        w.open("cac:Country", &[]);
        w.text("cbc:IdentificationCode", country);
        w.close();
    }
    w.close();
}

fn receiver(w: &mut XmlWriter, r: &Retention) {
    let s = &r.supplier;
    w.open("cac:ReceiverParty", &[]);
    party_identification(w, &s.document_type, &s.document_number);
    w.open("cac:PartyLegalEntity", &[]);
    w.text("cbc:RegistrationName", &s.legal_name);
    w.close();
    w.close();
}

fn document_reference(w: &mut XmlWriter, line: &RetentionLine, payment_number: u64) {
    let d = &line.related_document;
    let currency: &[(&str, &str)] = &[("currencyID", d.currency.as_str())];
    w.open("sac:SUNATRetentionDocumentReference", &[]);
    w.leaf("cbc:ID", &[("schemeID", d.document_type.as_str())], &d.number);
    w.text("cbc:IssueDate", &d.issue_date.date_string());
    w.leaf("cbc:TotalInvoiceAmount", currency, d.total.as_str());
    w.open("cac:Payment", &[]);
    w.text("cbc:ID", &payment_number.to_string());
    w.leaf("cbc:PaidAmount", currency, line.payment.amount.as_str());
    w.text("cbc:PaidDate", &line.payment.date.date_string());
    w.close();
    let amounts = &line.retention;
    w.open("sac:SUNATRetentionInformation", &[]);
    w.leaf("sac:SUNATRetentionAmount", PEN, amounts.amount.as_str());
    w.text("sac:SUNATRetentionDate", &line.retention_date().date_string());
    w.leaf("sac:SUNATNetTotalPaid", PEN, amounts.net_paid.as_str());
    if let Some(x) = &line.exchange_rate {
        w.open("cac:ExchangeRate", &[]);
        w.text_opt("cbc:SourceCurrencyCode", x.source_currency.as_deref());
        w.text_opt("cbc:TargetCurrencyCode", x.target_currency.as_deref());
        w.text_opt("cbc:CalculationRate", x.rate.as_ref().map(|rate| rate.as_str()));
        w.text_opt("cbc:Date", x.date.map(|date| date.date_string()).as_deref());
        w.close();
    }
    w.close();
    w.close();
}

fn party_identification(w: &mut XmlWriter, scheme: &str, number: &str) {
    w.open("cac:PartyIdentification", &[]);
    w.leaf("cbc:ID", &[("schemeID", scheme)], number);
    w.close();
}

fn filled(value: Option<&str>) -> Option<&str> {
    value.filter(|v| !v.trim().is_empty())
}
