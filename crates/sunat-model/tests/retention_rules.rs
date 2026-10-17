//! The retention voucher (20): the six vectors of its contract (`fixtures/retention/`) and one test per
//! SUNAT validation rule the SDK checks before sending. Each rule test edits the first vector so that
//! it breaks exactly that rule and expects SUNAT's error code.

use kembec_sunat_model::{DateTime, Document, ModelError};
use serde_json::{json, Value};

/// 3 %, one invoice, one payment, PEN.
const V1: &str = include_str!("../../../fixtures/retention/v1-pen-simple.json");
/// One invoice paid twice, one of the previous month, a debit note.
const V2: &str = include_str!("../../../fixtures/retention/v2-several-documents.json");
/// USD at 3.750000.
const V3: &str = include_str!("../../../fixtures/retention/v3-usd-correct.json");
/// Regime 01 with 6.00, as Syma lets it through.
const V4: &str = include_str!("../../../fixtures/retention/v4-rate-6-with-regime-01.json");
/// USD as Syma computes it: no exchange rate in the amounts, target currency USD.
const V5: &str = include_str!("../../../fixtures/retention/v5-usd-as-syma-sends-it.json");
/// Totals summed before rounding the lines.
const V6: &str = include_str!("../../../fixtures/retention/v6-totals-unrounded.json");

fn with(base: &str, edit: impl FnOnce(&mut Value)) -> Result<Document, ModelError> {
    let mut value: Value = serde_json::from_str(base).unwrap();
    edit(&mut value);
    Document::from_json(&value.to_string())
}

fn expect_code(result: Result<Document, ModelError>, code: &str) {
    match result {
        Err(error) if error.sunat_code() == Some(code) => {
            assert!(error.to_string().ends_with(&format!("(SUNAT ERR-{code})")), "{error}");
        }
        other => panic!("expected SUNAT {code}, got {other:?}"),
    }
}

fn retention(json: &str) -> kembec_sunat_model::Retention {
    match Document::from_json(json).unwrap() {
        Document::Retention(retention) => retention,
        other => panic!("not a retention voucher: {other:?}"),
    }
}

#[test]
fn the_three_valid_vectors_pass() {
    for (json, name) in [(V1, "20000000001-20-R001-1"), (V2, "20000000001-20-R001-2"), (V3, "20000000001-20-R001-3")] {
        let document = Document::from_json(json).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(document.file_name(), name);
        assert!(document.voucher().is_none());
    }
}

#[test]
fn v4_rate_6_with_regime_01_is_2619() {
    expect_code(Document::from_json(V4), "2619");
}

/// V5 breaks 2623 (dato 46) and 2715 (dato 52): the first in the sheet's order is reported.
#[test]
fn v5_usd_as_syma_sends_it_is_2623_then_2715() {
    expect_code(Document::from_json(V5), "2623");
    let fixed = with(V5, |v| {
        v["lines"][0]["retention"] = json!({"date": "2026-10-15", "amount": "112.50", "net_paid": "3637.50"});
        v["total_retained"] = json!("112.50");
        v["total_paid"] = json!("3637.50");
    });
    expect_code(fixed, "2715");
}

/// V6 breaks 2628 and 2629: the first is reported; with the retained total right, 2629.
#[test]
fn v6_totals_unrounded_is_2628_then_2629() {
    expect_code(Document::from_json(V6), "2628");
    expect_code(with(V6, |v| v["total_retained"] = json!("9.06")), "2629");
}

#[test]
fn payment_numbers_default_to_the_order_of_each_document() {
    let mut value: Value = serde_json::from_str(V2).unwrap();
    for line in value["lines"].as_array_mut().unwrap() {
        line["payment"].as_object_mut().unwrap().remove("number");
    }
    let r = retention(&value.to_string());
    let numbers: Vec<u64> = (0..r.lines.len()).map(|i| r.payment_number(i)).collect();
    assert_eq!(numbers, [1, 2, 1, 1]);
}

#[test]
fn retention_date_defaults_to_the_payment_date() {
    let mut value: Value = serde_json::from_str(V1).unwrap();
    value["lines"][0]["retention"].as_object_mut().unwrap().remove("date");
    let r = retention(&value.to_string());
    assert_eq!(r.lines[0].retention_date().date_string(), "2026-10-17");
}

#[test]
fn send_deadline_is_a_parameter() {
    let r = retention(V1);
    let at = |text: &str| text.parse::<DateTime>().unwrap();
    r.check_send_deadline(at("2026-10-24T23:59:59-05:00"), 7).unwrap();
    let late = r.check_send_deadline(at("2026-10-25T00:00:00-05:00"), 7).unwrap_err();
    assert_eq!(late.sunat_code(), Some("2600"));
    r.check_send_deadline(at("2026-10-25T00:00:00-05:00"), 8).unwrap();
}

#[test]
fn related_documents_of_type_07_and_20_are_out_of_scope() {
    for kind in ["07", "20"] {
        let result = with(V1, |v| v["lines"][0]["related_document"]["document_type"] = json!(kind));
        assert!(matches!(result, Err(ModelError::OutOfScope { .. })), "{kind}: {result:?}");
    }
}

#[test]
fn other_versions_types_and_unknown_fields_are_rejected() {
    assert!(matches!(with(V1, |v| v["ubl_version"] = json!("2.1")), Err(ModelError::Invalid { .. })));
    assert!(matches!(with(V1, |v| v["document_type"] = json!("40")), Err(ModelError::Invalid { .. })));
    assert!(matches!(with(V1, |v| v["currency"] = json!("PEN")), Err(ModelError::Json(_))));
    assert!(matches!(with(V1, |v| v["lines"] = json!([])), Err(ModelError::Missing { .. })));
}

macro_rules! rejects {
    ($($name:ident: $code:literal, $base:ident, |$v:ident| $edit:expr;)*) => {
        $(
            #[test]
            fn $name() {
                expect_code(with($base, |$v: &mut Value| { $edit; }), $code);
            }
        )*
    };
}

rejects! {
    // cbc:ID
    series_must_start_with_r_1001: "1001", V1, |v| v["series"] = json!("F001");
    number_up_to_8_digits_1001: "1001", V1, |v| v["number"] = json!("123456789");
    // cac:AgentParty
    issuer_legal_name_required_1037: "1037", V1, |v| v["issuer"]["legal_name"] = json!(" ");
    issuer_legal_name_without_line_breaks_1038: "1038", V1, |v| v["issuer"]["legal_name"] = json!("EMPRESA\nSAC");
    issuer_country_is_pe_2548: "2548", V1, |v| v["issuer"]["address"]["country_code"] = json!("CL");
    // cac:ReceiverParty
    supplier_number_required_2723: "2723", V1, |v| v["supplier"]["document_number"] = json!("");
    supplier_ruc_of_11_digits_2724: "2724", V1, |v| v["supplier"]["document_number"] = json!("1234567");
    supplier_is_not_the_issuer_2620: "2620", V1, |v| v["supplier"]["document_number"] = json!("20000000001");
    supplier_type_required_2516: "2516", V1, |v| v["supplier"]["document_type"] = json!("");
    supplier_type_is_ruc_2511: "2511", V1, |v| v["supplier"]["document_type"] = json!("1");
    supplier_legal_name_required_2134: "2134", V1, |v| v["supplier"]["legal_name"] = json!("");
    supplier_legal_name_without_tabs_2133: "2133", V1, |v| v["supplier"]["legal_name"] = json!("PROVEEDOR\tSAC");
    // sac:SUNATRetentionSystemCode, sac:SUNATRetentionPercent
    regime_in_catalog_23_2618: "2618", V1, |v| v["regime"] = json!("03");
    rate_of_the_regime_2619: "2619", V1, |v| v["rate"] = json!("3.50");
    // cbc:TotalInvoiceAmount
    total_retained_positive_2669: "2669", V1, |v| v["total_retained"] = json!("0.00");
    total_retained_is_the_sum_2628: "2628", V1, |v| v["total_retained"] = json!("35.41");
    // sac:SUNATTotalPaid
    total_paid_positive_2730: "2730", V1, |v| v["total_paid"] = json!("-1144.60");
    total_paid_is_the_sum_2629: "2629", V1, |v| v["total_paid"] = json!("1144.61");
    // cbc:PayableRoundingAmount
    rounding_up_to_1_3303: "3303", V1, |v| { v["rounding_amount"] = json!("1.01"); v["total_paid"] = json!("1145.61"); };
    // sac:SUNATRetentionDocumentReference/cbc:ID
    related_type_required_2691: "2691", V1, |v| v["lines"][0]["related_document"]["document_type"] = json!("");
    related_receipt_is_not_allowed_2692: "2692", V1, |v| v["lines"][0]["related_document"]["document_type"] = json!("03");
    related_number_required_2693: "2693", V1, |v| v["lines"][0]["related_document"]["number"] = json!("");
    related_receipt_series_2694: "2694", V1, |v| v["lines"][0]["related_document"]["number"] = json!("B001-101");
    related_number_all_zeros_2694: "2694", V1, |v| v["lines"][0]["related_document"]["number"] = json!("F001-000");
    related_ticket_format_2694: "2694", V1, |v| {
        v["lines"][0]["related_document"]["document_type"] = json!("12");
        v["lines"][0]["related_document"]["number"] = json!("MAQ-REG");
    };
    // sac:SUNATRetentionDocumentReference/cbc:IssueDate
    regime_02_only_until_february_2014_2985: "2985", V1, |v| {
        v["regime"] = json!("02");
        v["rate"] = json!("6.00");
    };
    // sac:SUNATRetentionDocumentReference/cbc:TotalInvoiceAmount
    related_total_positive_2696: "2696", V1, |v| v["lines"][0]["related_document"]["total"] = json!("0");
    // cac:Payment/cbc:PaidDate
    payments_in_one_month_2661: "2661", V2, |v| {
        v["lines"][3]["payment"]["date"] = json!("2026-09-30");
        v["lines"][3]["related_document"]["issue_date"] = json!("2026-09-05");
    };
    payment_not_after_the_voucher_2625: "2625", V1, |v| v["lines"][0]["payment"]["date"] = json!("2026-10-18");
    payment_not_before_the_document_2625: "2625", V1, |v| v["lines"][0]["payment"]["date"] = json!("2026-10-04");
    payment_from_day_1_for_an_older_document_2625: "2625", V1, |v| {
        v["lines"][0]["related_document"]["issue_date"] = json!("2026-09-20");
        v["lines"][0]["payment"]["date"] = json!("2026-09-25");
    };
    // cac:Payment/cbc:ID
    payment_number_up_to_9_digits_2734: "2734", V1, |v| v["lines"][0]["payment"]["number"] = json!(1_000_000_000u64);
    payment_number_unique_per_document_2626: "2626", V2, |v| v["lines"][1]["payment"]["number"] = json!(1);
    // cac:Payment/cbc:PaidAmount
    payment_amount_positive_2736: "2736", V1, |v| v["lines"][0]["payment"]["amount"] = json!("0.00");
    // sac:SUNATRetentionAmount
    // The totals come first in the sheet: they are kept equal to the sum so that the line is reached.
    retained_positive_2740: "2740", V2, |v| {
        v["lines"][0]["retention"]["amount"] = json!("0");
        v["total_retained"] = json!("42.24");
    };
    retained_is_payment_times_rate_2623: "2623", V1, |v| {
        v["lines"][0]["retention"]["amount"] = json!("36.41");
        v["lines"][0]["retention"]["net_paid"] = json!("1143.59");
        v["total_retained"] = json!("36.41");
        v["total_paid"] = json!("1143.59");
    };
    retained_in_usd_uses_the_exchange_rate_2623: "2623", V3, |v| v["lines"][0]["exchange_rate"]["rate"] = json!("3.800000");
    // sac:SUNATNetTotalPaid
    net_positive_2746: "2746", V2, |v| {
        v["lines"][0]["retention"]["net_paid"] = json!("0.00");
        v["total_paid"] = json!("1365.76");
    };
    net_is_payment_minus_retained_2623: "2623", V1, |v| {
        v["lines"][0]["retention"]["net_paid"] = json!("1146.00");
        v["total_paid"] = json!("1146.00");
    };
    // cac:ExchangeRate
    exchange_rate_required_out_of_pen_2719: "2719", V3, |v| v["lines"][0].as_object_mut().unwrap().remove("exchange_rate");
    exchange_source_is_the_document_currency_2749: "2749", V3, |v| v["lines"][0]["exchange_rate"]["source_currency"] = json!("EUR");
    exchange_target_is_pen_2715: "2715", V3, |v| v["lines"][0]["exchange_rate"]["target_currency"] = json!("USD");
    exchange_rate_required_2721: "2721", V3, |v| v["lines"][0]["exchange_rate"].as_object_mut().unwrap().remove("rate");
    exchange_rate_positive_4_6_2716: "2716", V3, |v| v["lines"][0]["exchange_rate"]["rate"] = json!("3.7500001");
    exchange_date_required_2722: "2722", V3, |v| v["lines"][0]["exchange_rate"].as_object_mut().unwrap().remove("date");
}

#[test]
fn a_usd_rate_off_by_a_cent_is_within_the_tolerance() {
    let result = with(V3, |v| v["lines"][0]["retention"] = json!({"amount": "112.51", "net_paid": "3637.49"}));
    assert!(result.is_err(), "the totals still have no tolerance");
    let result = with(V3, |v| {
        v["lines"][0]["retention"] = json!({"amount": "112.51", "net_paid": "3637.49"});
        v["total_retained"] = json!("112.51");
        v["total_paid"] = json!("3637.49");
    });
    result.unwrap();
}
