//! `cac:OrderReference` and `cac:DespatchDocumentReference`, which UBL 2.1 places after the
//! currency (and, in a note, around `cac:BillingReference`) and before `cac:Signature`. The guide's
//! optional `listAgencyName`, `listName` and `listURI` are left out: SUNAT only checks them when present.

use sunat_model::Voucher;

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
