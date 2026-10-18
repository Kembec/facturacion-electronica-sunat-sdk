//! `cac:OrderReference` and `cac:DespatchDocumentReference`, which UBL 2.1 places after the
//! currency (and, in a note, around `cac:BillingReference`) and before `cac:Signature`. The guide's
//! optional `listAgencyName`, `listName` and `listURI` are left out: SUNAT only checks them when present.

use sunat_model::{Advance, Voucher};

use crate::parties::RUC_SCHEME;
use crate::writer::XmlWriter;

pub(crate) fn order(w: &mut XmlWriter, voucher: &Voucher) {
    let Some(order) = &voucher.order_reference else { return };
    w.open("cac:OrderReference", &[]);
    w.text("cbc:ID", order);
    w.close();
}

pub(crate) fn despatches(w: &mut XmlWriter, voucher: &Voucher) {
    for reference in &voucher.despatch_references {
        w.open("cac:DespatchDocumentReference", &[]);
        w.text("cbc:ID", &reference.number);
        w.text("cbc:DocumentTypeCode", &reference.document_type);
        w.close();
    }
}

/// One `cac:AdditionalDocumentReference` per advance, after the despatch references: the document that
/// received the payment, the payment's number (`DocumentStatusCode`, the same as its `PrepaidPayment`
/// `ID`) and who issued it.
pub(crate) fn advances(w: &mut XmlWriter, voucher: &Voucher, advances: &[Advance]) {
    for (index, advance) in advances.iter().enumerate() {
        w.open("cac:AdditionalDocumentReference", &[]);
        w.text("cbc:ID", &advance.document_number);
        w.text("cbc:DocumentTypeCode", &advance.document_type);
        w.text("cbc:DocumentStatusCode", &(index + 1).to_string());
        w.open("cac:IssuerParty", &[]);
        w.open("cac:PartyIdentification", &[]);
        w.leaf("cbc:ID", &[("schemeID", RUC_SCHEME)], advance.issuer_ruc.as_deref().unwrap_or(&voucher.issuer.ruc));
        w.close();
        w.close();
        w.close();
    }
}
