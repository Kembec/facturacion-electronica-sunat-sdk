//! Summary-line rules the vectors do not reach: which zero amounts are rendered, other charges, and
//! the IGV total when the line gives none.

use sunat_model::Document;

const SUMMARY: &str = include_str!("../../../fixtures/rc-receipts-added.json");

fn first_line(edit: impl FnOnce(&mut serde_json::Value)) -> String {
    let mut value: serde_json::Value = serde_json::from_str(SUMMARY).unwrap();
    edit(&mut value["lines"][0]);
    let document = Document::from_json(&value.to_string()).unwrap();
    let xml = String::from_utf8(kembec_sunat_ubl::build(&document).unwrap()).unwrap();
    let start = xml.find("<sac:SummaryDocumentsLine>").unwrap();
    let end = xml[start..].find("</sac:SummaryDocumentsLine>").unwrap();
    xml[start..start + end].to_owned()
}

fn payment(amount: &str, instruction: &str) -> String {
    format!(
        "<sac:BillingPayment><cbc:PaidAmount currencyID=\"PEN\">{amount}</cbc:PaidAmount>\
         <cbc:InstructionID>{instruction}</cbc:InstructionID></sac:BillingPayment>"
    )
}

#[test]
fn taxed_zero_is_rendered_and_other_zero_amounts_are_not() {
    let line = first_line(|l| {
        l["taxed"] = serde_json::json!("0.00");
        for key in ["exempt", "unaffected", "export", "free", "other_charges"] {
            l[key] = serde_json::json!("0.00");
        }
    });
    assert!(line.contains(&payment("0.00", "01")), "{line}");
    for instruction in ["02", "03", "04", "05"] {
        assert!(!line.contains(&format!("<cbc:InstructionID>{instruction}<")), "{instruction}: {line}");
    }
    assert!(!line.contains("cac:AllowanceCharge"), "{line}");
}

#[test]
fn export_free_and_other_charges_follow_the_payments() {
    let line = first_line(|l| {
        l["export"] = serde_json::json!("10");
        l["free"] = serde_json::json!("5.5");
        l["other_charges"] = serde_json::json!("1.255");
    });
    let charge = "<cac:AllowanceCharge><cbc:ChargeIndicator>true</cbc:ChargeIndicator>\
                  <cbc:Amount currencyID=\"PEN\">1.26</cbc:Amount></cac:AllowanceCharge>";
    let expected = [payment("93.21", "01"), payment("10.00", "04"), payment("5.50", "05"), charge.to_owned()].concat();
    assert!(line.contains(&format!("{expected}<cac:TaxTotal>")), "{line}");
}

#[test]
fn absent_igv_is_rendered_as_zero() {
    let line = first_line(|l| {
        l.as_object_mut().unwrap().remove("igv");
    });
    let zero = "<cbc:TaxAmount currencyID=\"PEN\">0.00</cbc:TaxAmount>";
    assert!(line.contains(&format!("<cac:TaxTotal>{zero}<cac:TaxSubtotal>{zero}")), "{line}");
}
