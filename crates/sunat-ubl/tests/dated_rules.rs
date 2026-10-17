//! Rules that take effect on 2027-01-01, on both sides of the cut: before it the XML of today and the
//! new codes rejected; from it the new rule. The date is `reference_date` in the daily summary and
//! `issue_date` in notes.

use sunat_model::{Document, ModelError};

const SUMMARY: &str = include_str!("../../../fixtures/rc-receipts-added.json");
const DEBIT: &str = include_str!("../../../fixtures/08-debit-note-value-increase.json");

fn edited(base: &str, edit: impl FnOnce(&mut serde_json::Value)) -> Result<Document, ModelError> {
    let mut value: serde_json::Value = serde_json::from_str(base).unwrap();
    edit(&mut value);
    Document::from_json(&value.to_string())
}

fn xml(document: &Document) -> String {
    String::from_utf8(kembec_sunat_ubl::build(document).unwrap()).unwrap()
}

fn summary(reference: &str, generation: &str, line: serde_json::Value) -> Result<Document, ModelError> {
    edited(SUMMARY, |v| {
        v["reference_date"] = reference.into();
        v["generation_date"] = generation.into();
        for (key, value) in line.as_object().unwrap() {
            v["lines"][0][key] = value.clone();
        }
    })
}

fn payment(amount: &str, code: &str) -> String {
    format!("<cbc:PaidAmount currencyID=\"PEN\">{amount}</cbc:PaidAmount><cbc:InstructionID>{code}</cbc:InstructionID>")
}

#[test]
fn before_the_cut_the_summary_keeps_code_05_and_rejects_06_to_09() {
    let free = summary("2026-12-31", "2027-01-02", serde_json::json!({ "free": "5.50" })).unwrap();
    let xml = xml(&free);
    assert!(xml.contains(&payment("5.50", "05")));
    for (field, code) in [("free_taxed", "06"), ("free_exempt", "07"), ("free_unaffected", "08"), ("free_export", "09")] {
        let error = summary("2026-12-31", "2027-01-02", serde_json::json!({ field: "5.50" })).unwrap_err();
        assert_eq!(error.sunat_code(), Some("2256"), "{code}");
        assert!(error.to_string().contains("2027-01-01") && error.to_string().contains(field), "{error}");
    }
    let igv = summary("2026-12-31", "2027-01-02", serde_json::json!({ "free_igv": "1.00" })).unwrap_err();
    assert!(igv.to_string().contains("free_igv") && igv.to_string().contains("9996"), "{igv}");
}

#[test]
fn from_the_cut_the_summary_splits_free_sales_with_the_igv_of_9996_and_rejects_05() {
    let line = serde_json::json!({
        "free_taxed": "10", "free_exempt": "2", "free_unaffected": "3", "free_export": "4", "free_igv": "1.8", "free": "0.00"
    });
    let xml = xml(&summary("2027-01-01", "2027-01-01", line).unwrap());
    let ordered = [payment("10.00", "06"), payment("2.00", "07"), payment("3.00", "08"), payment("4.00", "09")];
    let mut at = 0;
    for expected in ordered {
        at += xml[at..].find(&expected).unwrap_or_else(|| panic!("{expected}: {xml}"));
    }
    assert!(!xml.contains("<cbc:InstructionID>05<"));
    let free_igv = "<cac:TaxTotal><cbc:TaxAmount currencyID=\"PEN\">1.80</cbc:TaxAmount><cac:TaxSubtotal>\
                    <cbc:TaxAmount currencyID=\"PEN\">1.80</cbc:TaxAmount><cac:TaxCategory><cac:TaxScheme>\
                    <cbc:ID>9996</cbc:ID><cbc:Name>GRA</cbc:Name><cbc:TaxTypeCode>FRE</cbc:TaxTypeCode>";
    let igv_at = xml.find("<cbc:ID>1000</cbc:ID>").unwrap();
    assert!(xml[igv_at..].contains(free_igv), "{xml}");
    let error = summary("2027-01-01", "2027-01-01", serde_json::json!({ "free": "5.50" })).unwrap_err();
    assert!(error.to_string().contains("free_taxed") && error.to_string().contains("2027-01-01"), "{error}");
}

#[test]
fn the_summary_cut_follows_reference_date_not_generation_date() {
    assert!(summary("2026-12-31", "2027-01-05", serde_json::json!({ "free": "1" })).is_ok());
    assert!(summary("2027-01-01", "2027-01-05", serde_json::json!({ "free": "1" })).is_err());
}

fn debit(issue_date: &str, reason: &str, affectation: &str, totals: &[(&str, &str)]) -> Result<Document, ModelError> {
    edited(DEBIT, |v| {
        v["issue_date"] = issue_date.into();
        v["note"]["reason_code"] = reason.into();
        v["lines"][0]["affectation"] = affectation.into();
        // The fixture's invoice-like totals are taxed; a note on unaffected lines zeroes them.
        for (field, amount) in totals {
            v["totals"][*field] = (*amount).into();
        }
    })
}

#[test]
fn before_the_cut_a_debit_note_takes_03_and_rejects_13() {
    assert!(debit("2026-12-31T23:59:59-05:00", "03", "10", &[]).is_ok());
    assert!(debit("2026-12-31T23:59:59-05:00", "02", "10", &[]).is_ok());
    let error = debit("2026-12-31T23:59:59-05:00", "13", "30", &[]).unwrap_err();
    assert!(error.to_string().contains("$.note.reason_code") && error.to_string().contains("2027-01-01"), "{error}");
}

#[test]
fn from_the_cut_a_debit_note_13_takes_unaffected_lines_only_and_03_answers_2524() {
    let ok = debit("2027-01-01T00:00:00-05:00", "13", "30", &[("taxed", "0.00")]).unwrap();
    assert!(xml(&ok).contains("<cbc:ResponseCode>13</cbc:ResponseCode>"));
    for affectation in ["10", "20", "40"] {
        let error = debit("2027-01-01T00:00:00-05:00", "13", affectation, &[("taxed", "0.00")]).unwrap_err();
        assert_eq!(error.sunat_code(), Some("3507"), "{affectation}: {error}");
    }
    let totals = debit("2027-01-01T00:00:00-05:00", "13", "30", &[("taxed", "0.00"), ("export", "10.00")]).unwrap_err();
    assert_eq!((totals.sunat_code(), totals.to_string().contains("$.totals.export")), (Some("3507"), true));
    let old = debit("2027-01-01T00:00:00-05:00", "03", "10", &[]).unwrap_err();
    assert_eq!(old.sunat_code(), Some("2524"));
}

#[test]
fn the_note_cut_is_lima_midnight() {
    // 2027-01-01T04:59:59Z is still 2026-12-31 in Lima.
    assert!(debit("2026-12-31T23:59:59-05:00", "03", "10", &[]).is_ok());
    assert!(debit("2027-01-01T04:59:59+00:00", "03", "10", &[]).is_ok());
    assert!(debit("2027-01-01T05:00:00+00:00", "03", "10", &[]).is_err());
}

#[test]
fn a_credit_note_is_not_touched_by_the_debit_note_cut() {
    let credit = include_str!("../../../fixtures/07-credit-note-partial-return.json");
    for date in ["2026-12-31T10:00:00-05:00", "2027-01-02T10:00:00-05:00"] {
        assert!(edited(credit, |v| v["issue_date"] = date.into()).is_ok(), "{date}");
    }
}
