use kembec_sunat_model::{Document, ModelError};

const BASE: &str = include_str!("../../../fixtures/01-invoice-taxed-cash.json");

fn with(edit: impl FnOnce(&mut serde_json::Value)) -> Result<Document, ModelError> {
    let mut value: serde_json::Value = serde_json::from_str(BASE).unwrap();
    edit(&mut value);
    Document::from_json(&value.to_string())
}

#[test]
fn file_name_is_ruc_type_series_number() {
    assert_eq!(Document::from_json(BASE).unwrap().file_name(), "20123456789-01-F001-1");
}

#[test]
fn out_of_scope_features_get_named_errors() {
    let cases: [(&str, fn(&mut serde_json::Value)); 5] = [
        ("advances", |v| v["advances"] = serde_json::json!([])),
        ("perception", |v| v["perception"] = serde_json::json!({})),
        ("global discounts", |v| v["discounts"] = serde_json::json!([])),
        ("other taxes", |v| v["lines"][0]["other_tax"] = serde_json::json!("0.50")),
        ("ISC", |v| v["totals"]["isc"] = serde_json::json!("1.00")),
    ];
    for (feature, edit) in cases {
        match with(edit) {
            Err(ModelError::OutOfScope { feature: f, .. }) => assert_eq!(f, feature),
            other => panic!("{feature}: {other:?}"),
        }
    }
}

#[test]
fn detraction_goes_with_its_operation_types() {
    const DETRACTION: &str = include_str!("../../../fixtures/01-invoice-detraction.json");
    let invoice = match Document::from_json(DETRACTION).unwrap() {
        Document::Invoice(invoice) => invoice,
        other => panic!("{other:?}"),
    };
    let detraction = invoice.detraction.unwrap();
    assert_eq!((detraction.code.as_str(), detraction.amount.as_str()), ("037", "141.60"));
    let receipt = edited(DETRACTION, |v| {
        v["document_type"] = serde_json::json!("03");
        v["series"] = serde_json::json!("B001");
    });
    assert!(receipt.is_ok(), "receipts take detraction too (catalog 51 code 1001): {receipt:?}");

    let missing = edited(DETRACTION, |v| { v.as_object_mut().unwrap().remove("detraction"); });
    assert!(matches!(missing, Err(ModelError::Missing { .. })), "1001 without detraction (3127): {missing:?}");
    let wrong_type = edited(DETRACTION, |v| v["operation_type"] = serde_json::json!("0101"));
    assert!(matches!(wrong_type, Err(ModelError::Invalid { .. })), "detraction with 0101 (3128): {wrong_type:?}");
    for (operation, feature) in [
        ("1002", "detraction on hydrobiological resources (operation type 1002)"),
        ("1004", "detraction on freight transport (operation type 1004)"),
    ] {
        match edited(DETRACTION, |v| v["operation_type"] = serde_json::json!(operation)) {
            Err(ModelError::OutOfScope { feature: f, .. }) => assert_eq!(f, feature),
            other => panic!("{operation}: {other:?}"),
        }
    }
    let rejected: [fn(&mut serde_json::Value); 7] = [
        |v| v["operation_type"] = serde_json::json!("1003"),
        |v| v["detraction"]["code"] = serde_json::json!("37"),
        |v| v["detraction"]["payment_means_code"] = serde_json::json!("1"),
        |v| v["detraction"]["account"] = serde_json::json!(""),
        |v| v["detraction"]["amount"] = serde_json::json!("0.00"),
        |v| v["detraction"]["amount"] = serde_json::json!("141.605"),
        |v| v["detraction"]["percent"] = serde_json::json!("-12"),
    ];
    for edit in rejected {
        assert!(matches!(edited(DETRACTION, edit), Err(ModelError::Invalid { .. })));
    }
    let passenger = edited(DETRACTION, |v| {
        v["operation_type"] = serde_json::json!("1003");
        v["detraction"]["code"] = serde_json::json!("028");
    });
    assert!(passenger.is_ok(), "1003 with code 028: {passenger:?}");
    assert!(edited(DETRACTION, |v| v["detraction"]["currency"] = serde_json::json!("USD")).is_err(), "unknown fields");
    match edited(include_str!("../../../fixtures/07-credit-note-void.json"), |v| {
        v["detraction"] = serde_json::from_str(DETRACTION).map(|d: serde_json::Value| d["detraction"].clone()).unwrap();
    }) {
        Err(ModelError::Invalid { path, .. }) => assert_eq!(path, "$.detraction"),
        other => panic!("note with detraction: {other:?}"),
    }
}

#[test]
fn line_discounts_take_codes_00_and_01_with_sunat_formats() {
    const RECEIPT: &str = include_str!("../../../fixtures/03-receipt-line-discount.json");
    let discount = |edit: fn(&mut serde_json::Value)| edited(RECEIPT, |v| edit(&mut v["lines"][0]["discounts"][0]));
    let receipt = Document::from_json(RECEIPT).unwrap();
    let line = &receipt.voucher().unwrap().lines[0];
    assert_eq!((line.discounts[0].code.as_str(), line.discounts[0].factor.as_str()), ("00", "0.10"));
    assert!(discount(|d| d["factor"] = serde_json::json!("0.100000")).is_ok(), "trailing zeros do not count");

    // 01 does not reduce the line's base: the receipt has to carry it in allowance_total (4307).
    assert_eq!(discount(|d| d["code"] = serde_json::json!("01")).unwrap_err().sunat_code(), Some("4307"));
    assert_eq!(discount(|d| d["code"] = serde_json::json!("02")).unwrap_err().sunat_code(), Some("4268"));
    let rejected: [fn(&mut serde_json::Value); 6] = [
        |d| d["factor"] = serde_json::json!("0.123456"),
        |d| d["factor"] = serde_json::json!("1000"),
        |d| d["factor"] = serde_json::json!("0"),
        |d| d["amount"] = serde_json::json!("5.095"),
        |d| d["amount"] = serde_json::json!("-5.09"),
        |d| d["base_amount"] = serde_json::json!("0.00"),
    ];
    for edit in rejected {
        assert!(matches!(discount(edit), Err(ModelError::Invalid { .. })));
    }
    assert!(discount(|d| d["reason"] = serde_json::json!("x")).is_err(), "unknown fields");
    match edited(include_str!("../../../fixtures/07-credit-note-void.json"), |v| {
        v["lines"][0]["discounts"] = serde_json::json!([{ "code": "00", "factor": "0.1", "amount": "1", "base_amount": "10" }])
    }) {
        Err(ModelError::OutOfScope { feature, .. }) => assert_eq!(feature, "line discounts in credit and debit notes"),
        other => panic!("note: {other:?}"),
    }
}

#[test]
fn kind_and_type_must_agree() {
    assert!(with(|v| v["document_type"] = serde_json::json!("07")).is_err());
    assert!(with(|v| v["kind"] = serde_json::json!("note")).is_err());
    assert!(with(|v| v["unknown"] = serde_json::json!(1)).is_err());
    assert!(with(|v| v["totals"]["payable"] = serde_json::json!(218.0)).is_err());
}

const SUMMARY: &str = include_str!("../../../fixtures/rc-credit-note-reference.json");
const VOIDED: &str = include_str!("../../../fixtures/ra-invoice.json");

fn edited(base: &str, edit: impl FnOnce(&mut serde_json::Value)) -> Result<Document, ModelError> {
    let mut value: serde_json::Value = serde_json::from_str(base).unwrap();
    edit(&mut value);
    Document::from_json(&value.to_string())
}

#[test]
fn summary_and_voided_file_names_carry_their_dates() {
    assert_eq!(Document::from_json(SUMMARY).unwrap().file_name(), "20123456789-RC-20261015-2");
    assert_eq!(Document::from_json(VOIDED).unwrap().file_name(), "20123456789-RA-20261015-1");
    assert!(Document::from_json(SUMMARY).unwrap().voucher().is_none());
}

#[test]
fn summary_lines_are_checked() {
    let invalid: [(&str, fn(&mut serde_json::Value)); 8] = [
        ("$.lines[0].reference_document", |v| v["lines"][0]["document_type"] = serde_json::json!("03")),
        ("$.lines[0].reference_document", |v| {
            v["lines"][0].as_object_mut().unwrap().remove("reference_document");
        }),
        ("$.lines[0].document_type", |v| v["lines"][0]["document_type"] = serde_json::json!("01")),
        ("$.lines[0].status", |v| v["lines"][0]["status"] = serde_json::json!("4")),
        ("$.lines[0].document_number", |v| v["lines"][0]["document_number"] = serde_json::json!("B0013")),
        ("$.number", |v| v["number"] = serde_json::json!("123456")),
        ("$.reference_date", |v| v["reference_date"] = serde_json::json!("2026-10-16")),
        ("$.lines", |v| v["lines"] = serde_json::json!([])),
    ];
    for (path, edit) in invalid {
        match edited(SUMMARY, edit) {
            Err(ModelError::Invalid { path: p, .. } | ModelError::Missing { path: p }) => assert_eq!(p, path),
            other => panic!("{path}: {other:?}"),
        }
    }
    assert!(edited(SUMMARY, |v| v["lines"][0]["unknown"] = serde_json::json!(1)).is_err());
    assert!(edited(SUMMARY, |v| v["generation_date"] = serde_json::json!("2026-10-15T00:00:00-05:00")).is_err());
}

#[test]
fn summary_line_taxes_out_of_scope_get_named_errors() {
    for (key, feature) in [("isc", "ISC"), ("perception", "perception"), ("ivap", "IVAP")] {
        match edited(SUMMARY, |v| v["lines"][0][key] = serde_json::json!("1.00")) {
            Err(ModelError::OutOfScope { feature: f, .. }) => assert_eq!(f, feature),
            other => panic!("{key}: {other:?}"),
        }
    }
}

#[test]
fn void_communication_is_checked() {
    let invalid: [(&str, fn(&mut serde_json::Value)); 4] = [
        ("$.lines[0].document_type", |v| v["lines"][0]["document_type"] = serde_json::json!("03")),
        ("$.lines[0].reason", |v| v["lines"][0]["reason"] = serde_json::json!("")),
        ("$.lines[0].series", |v| v["lines"][0]["series"] = serde_json::json!("F01")),
        ("$.generation_date", |v| v["generation_date"] = serde_json::json!("2026-10-16")),
    ];
    for (path, edit) in invalid {
        match edited(VOIDED, edit) {
            Err(ModelError::Invalid { path: p, .. } | ModelError::Missing { path: p }) => assert_eq!(p, path),
            other => panic!("{path}: {other:?}"),
        }
    }
}

const SUMMARY_RECEIPTS: &str = include_str!("../../../fixtures/rc-receipts-added.json");

/// The IGV rate (errors 3102, 3504) and the customer's name (2022) SUNAT checks on a summary line.
#[test]
fn summary_line_rate_and_customer_name_are_checked() {
    let set = |base: &str, key: &str, value: &str| edited(base, |v| v["lines"][0][key] = serde_json::json!(value));
    for rate in ["18.00", "10.50", "10.5", "18"] {
        assert!(set(SUMMARY_RECEIPTS, "igv_rate", rate).is_ok(), "{rate}");
    }
    assert!(set(SUMMARY, "igv_rate", "10.00").is_ok(), "a note may modify a receipt taxed at 10 %");
    for (base, rate) in [(SUMMARY_RECEIPTS, "10.00"), (SUMMARY_RECEIPTS, "8"), (SUMMARY_RECEIPTS, "-18"), (SUMMARY, "1234.5")] {
        match set(base, "igv_rate", rate) {
            Err(ModelError::Invalid { path, .. }) => assert_eq!(path, "$.lines[0].igv_rate", "{rate}"),
            other => panic!("{rate}: {other:?}"),
        }
    }
    assert!(set(SUMMARY_RECEIPTS, "customer_name", "JUAN PEREZ").is_ok());
    let long = "A".repeat(251);
    for name in ["AB", "   ", "JUAN\nPEREZ", long.as_str()] {
        match set(SUMMARY_RECEIPTS, "customer_name", name) {
            Err(ModelError::Invalid { path, .. }) => assert_eq!(path, "$.lines[0].customer_name", "{name:?}"),
            other => panic!("{name:?}: {other:?}"),
        }
    }
}

const SUMMARY_VOID: &str = include_str!("../../../fixtures/rc-receipt-void.json");
const SUMMARY_EXEMPT: &str = include_str!("../../../fixtures/rc-exempt-and-unaffected.json");

/// SUNAT 2992 (Resumen Diario sheet: tax code 1000 without `cbc:Percent`): the writer puts tax 1000 on every line,
/// so every line needs `igv_rate`, whatever its status or its taxed amounts.
#[test]
fn every_summary_line_needs_its_igv_rate() {
    for base in [SUMMARY_RECEIPTS, SUMMARY, SUMMARY_VOID, SUMMARY_EXEMPT] {
        assert!(Document::from_json(base).is_ok());
        let lines = serde_json::from_str::<serde_json::Value>(base).unwrap()["lines"].as_array().unwrap().len();
        for index in 0..lines {
            let result = edited(base, |v| {
                v["lines"][index].as_object_mut().unwrap().remove("igv_rate");
            });
            match result {
                Err(error @ ModelError::Rule { .. }) => {
                    assert_eq!(error.sunat_code(), Some("2992"));
                    assert!(error.to_string().contains(&format!("$.lines[{index}].igv_rate")), "{error}");
                }
                other => panic!("line {index}: {other:?}"),
            }
        }
    }
}

const REFERENCES: &str = include_str!("../../../fixtures/01-invoice-references.json");

/// SUNAT beta, 16/10/2026: what it observed (4005, 4006, 4233) or rejected (2364) on a cited guide or order.
#[test]
fn references_are_checked_as_beta_checks_them() {
    let document = Document::from_json(REFERENCES).unwrap();
    let voucher = document.voucher().unwrap();
    assert_eq!(voucher.order_reference.as_deref(), Some("OC-2026-0457"));
    assert_eq!(voucher.despatch_references.len(), 3);

    let guide = |number: &str, kind: &str| edited(REFERENCES, |v| v["despatch_references"] = serde_json::json!([{ "document_type": kind, "number": number }]));
    for number in ["T001-12", "TAB1-1", "V001-3", "G001-9", "0001-123", "EG01-5", "EG04-1", "EG07-9", "T001-00000012"] {
        assert!(guide(number, "09").is_ok(), "{number}");
    }
    assert!(guide("T001-1", "31").is_ok(), "type and series are not crossed");
    for number in ["XX-5", "E1-5", "EG05-1", "EG08-1", "t001-1", "T001-123456789", "T001-", "T001", "G00A-1", "00A1-1"] {
        match guide(number, "09") {
            Err(ModelError::Invalid { path, .. }) => assert_eq!(path, "$.despatch_references[0].number", "{number}"),
            other => panic!("{number}: {other:?}"),
        }
    }
    match guide("T001-12", "99") {
        Err(ModelError::Invalid { path, .. }) => assert_eq!(path, "$.despatch_references[0].document_type"),
        other => panic!("type 99: {other:?}"),
    }
    let twice = edited(REFERENCES, |v| {
        v["despatch_references"] = serde_json::json!([
            { "document_type": "09", "number": "T001-12" },
            { "document_type": "31", "number": "T001-12" },
            { "document_type": "09", "number": "T001-12" },
        ])
    });
    match twice {
        Err(ModelError::Invalid { path, .. }) => assert_eq!(path, "$.despatch_references[2].number"),
        other => panic!("repeated: {other:?}"),
    }
    assert!(edited(REFERENCES, |v| v["despatch_references"][0]["unknown"] = serde_json::json!(1)).is_err());
}

#[test]
fn purchase_order_takes_20_characters_without_whitespace() {
    let order = |value: &str| edited(REFERENCES, |v| v["order_reference"] = serde_json::json!(value));
    for value in ["12345678901234567890", "ÑANDÚ/9.5_x", "A\u{a0}B", "X"] {
        assert!(order(value).is_ok(), "{value:?}");
    }
    for value in ["", "OC 1", "  X", "OC\t1", "OC\n1", "123456789012345678901"] {
        match order(value) {
            Err(ModelError::Invalid { path, .. }) => assert_eq!(path, "$.order_reference", "{value:?}"),
            other => panic!("{value:?}: {other:?}"),
        }
    }
}

#[test]
fn igv_retention_goes_in_every_voucher_with_a_ruc_customer() {
    const RETENTION: &str = include_str!("../../../fixtures/01-invoice-igv-retention.json");
    let invoice = Document::from_json(RETENTION).unwrap();
    let retention = invoice.voucher().unwrap().igv_retention.as_ref().unwrap();
    assert_eq!((retention.factor.as_str(), retention.amount.as_str(), retention.base_amount.as_str()), ("0.03", "4.44", "148.00"));
    for note in [
        include_str!("../../../fixtures/07-credit-note-igv-retention.json"),
        include_str!("../../../fixtures/08-debit-note-igv-retention.json"),
    ] {
        assert!(Document::from_json(note).unwrap().voucher().unwrap().igv_retention.is_some());
    }

    let dni = edited(RETENTION, |v| {
        v["customer"]["document_type"] = serde_json::json!("1");
        v["customer"]["document_number"] = serde_json::json!("12345678");
    });
    assert!(matches!(dni, Err(ModelError::Invalid { .. })), "retention needs a RUC customer (3262): {dni:?}");
    let rejected: [fn(&mut serde_json::Value); 5] = [
        |v| v["igv_retention"]["factor"] = serde_json::json!("0"),
        |v| v["igv_retention"]["factor"] = serde_json::json!("0.000001"),
        |v| v["igv_retention"]["amount"] = serde_json::json!("-4.44"),
        |v| v["igv_retention"]["amount"] = serde_json::json!("4.444"),
        |v| v["igv_retention"]["base_amount"] = serde_json::json!("0.00"),
    ];
    for edit in rejected {
        let result = edited(RETENTION, edit);
        assert!(matches!(result, Err(ModelError::Invalid { .. })), "{result:?}");
    }
    let unknown = edited(RETENTION, |v| v["igv_retention"]["code"] = serde_json::json!("62"));
    assert!(unknown.is_err(), "the code is always 62: {unknown:?}");
}
