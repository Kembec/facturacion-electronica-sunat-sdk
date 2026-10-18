//! One test per rule of the export contract (operation types 0200 to 0208): each edits a valid vector of
//! `fixtures/export/` and expects the SUNAT code of the sheet, or a local error where SUNAT has none.

use kembec_sunat_model::{Document, ModelError};
use serde_json::{json, Value};

const V1: &str = include_str!("../../../fixtures/export/v1-invoice-export-goods-0200.json");
const V2: &str = include_str!("../../../fixtures/export/v2-invoice-export-saas-hosting-0201.json");
const V3: &str = include_str!("../../../fixtures/export/v3-invoice-export-consulting-0208.json");
const V4: &str = include_str!("../../../fixtures/export/v4-invoice-export-lodging-0202.json");
const NEW_TYPES: &[&str] = &[
    include_str!("../../../fixtures/export/v5-invoice-export-shipping-0203.json"),
    include_str!("../../../fixtures/export/v6-invoice-export-foreign-vessel-0204.json"),
    include_str!("../../../fixtures/export/v7-invoice-export-tour-package-0205.json"),
    include_str!("../../../fixtures/export/v8-invoice-export-cargo-services-0206.json"),
    include_str!("../../../fixtures/export/v9-invoice-export-electricity-zed-0207.json"),
];
const N1: &str = include_str!("../../../fixtures/export/n1-0208-without-country.json");
const N2: &str = include_str!("../../../fixtures/export/n2-0200-customer-with-ruc.json");
const N3: &str = include_str!("../../../fixtures/export/n3-0202-without-stay-days.json");
const OLD_EXPORT: &str = include_str!("../../../fixtures/01-invoice-export.json");

fn edited(base: &str, edit: impl FnOnce(&mut Value)) -> Result<Document, ModelError> {
    let mut value: Value = serde_json::from_str(base).unwrap();
    edit(&mut value);
    Document::from_json(&value.to_string())
}

fn code(base: &str, edit: impl FnOnce(&mut Value)) -> (&'static str, String) {
    match edited(base, edit) {
        Err(ModelError::Rule { code, path, .. }) => (code, path),
        other => panic!("expected a SUNAT rule, got {other:?}"),
    }
}

fn invalid_path(result: Result<Document, ModelError>) -> String {
    match result {
        Err(ModelError::Invalid { path, .. }) => path,
        other => panic!("expected Invalid, got {other:?}"),
    }
}

fn out_of_scope(result: Result<Document, ModelError>) -> String {
    match result {
        Err(ModelError::OutOfScope { path, .. }) => path,
        other => panic!("expected OutOfScope, got {other:?}"),
    }
}

fn as_receipt(v: &mut Value) {
    v["document_type"] = json!("03");
    v["series"] = json!("B001");
}

#[test]
fn every_valid_vector_and_the_old_export_fixture_parse() {
    for base in [V1, V2, V3, V4, OLD_EXPORT] {
        Document::from_json(base).unwrap();
    }
    for (index, base) in NEW_TYPES.iter().enumerate() {
        Document::from_json(base).unwrap_or_else(|e| panic!("v{}: {e}", index + 5));
    }
}

#[test]
fn the_negative_vectors_fail_with_their_code() {
    assert_eq!(Document::from_json(N1).unwrap_err().sunat_code(), Some("3098"));
    assert_eq!(Document::from_json(N2).unwrap_err().sunat_code(), Some("2800"));
    assert_eq!(Document::from_json(N3).unwrap_err().sunat_code(), Some("3145"));
}

#[test]
fn all_nine_operation_types_are_accepted_on_an_invoice() {
    for op in ["0200", "0203", "0204", "0206", "0207"] {
        edited(V1, |v| v["operation_type"] = json!(op)).unwrap_or_else(|e| panic!("{op}: {e}"));
    }
    edited(V3, |v| v["operation_type"] = json!("0201")).unwrap();
    edited(V4, |v| {
        v["operation_type"] = json!("0205");
        for line in v["lines"].as_array_mut().unwrap() {
            let lodging = line["lodging"].as_object_mut().unwrap();
            for field in ["residence_country", "country_entry_date", "check_in_date", "check_out_date", "stay_days", "consumption_date"] {
                lodging.remove(field);
            }
        }
    })
    .unwrap();
}

#[test]
fn receipt_takes_the_export_types_but_lodging_and_tour_package_3206() {
    edited(V1, as_receipt).unwrap();
    edited(V3, as_receipt).unwrap();
    assert_eq!(code(V4, as_receipt), ("3206", "$.operation_type".into()));
}

#[test]
fn service_use_country_missing_3098() {
    assert_eq!(code(V2, |v| { v.as_object_mut().unwrap().remove("service_use_country"); }), ("3098", "$.service_use_country".into()));
}

#[test]
fn service_use_country_peru_or_not_iso_3099() {
    assert_eq!(code(V3, |v| v["service_use_country"] = json!("PE")).0, "3099");
    assert_eq!(code(V3, |v| v["service_use_country"] = json!("chl")).0, "3099");
    assert_eq!(code(V3, |v| v["service_use_country"] = json!("ZZ")), ("3099", "$.service_use_country".into()));
}

/// Catalog 04 is ISO 3166-1 in force (`crates/sunat-model/data/catalog-04.json`): `AN` and `CS`, withdrawn
/// but on the page the SUNAT sheet cites, give 3099 like the beta did with `AN`.
#[test]
fn service_use_country_in_iso_in_force_passes_and_withdrawn_codes_3099() {
    for country in ["RS", "ME", "SS", "CW", "CL"] {
        edited(V3, |v| v["service_use_country"] = json!(country)).unwrap_or_else(|e| panic!("{country}: {e}"));
    }
    for country in ["AN", "CS"] {
        assert_eq!(code(V3, |v| v["service_use_country"] = json!(country)), ("3099", "$.service_use_country".into()));
        assert_eq!(code(V2, |v| v["service_use_country"] = json!(country)).0, "3099", "0201 {country}");
    }
}

/// 4000 and 4001 (guest) take the same catalog 04, `PE` included.
#[test]
fn guest_countries_take_catalog_04_with_peru() {
    for field in ["passport_country", "residence_country"] {
        for country in ["ZZ", "AN", "CS"] {
            let (got, path) = code(V4, |v| v["lines"][0]["lodging"][field] = json!(country));
            assert_eq!((got, path), ("4280", format!("$.lines[0].lodging.{field}")), "{field} {country}");
        }
        for country in ["PE", "RS"] {
            edited(V4, |v| v["lines"][0]["lodging"][field] = json!(country)).unwrap_or_else(|e| panic!("{field} {country}: {e}"));
        }
    }
}

/// 3272 in every operation type: the base of an export line is its value.
#[test]
fn export_line_base_other_than_its_value_3272() {
    assert_eq!(code(V1, |v| v["lines"][0]["igv_base"] = json!("2400.00")), ("3272", "$.lines[0].igv_base".into()));
}

#[test]
fn service_use_country_outside_0201_and_0208_is_rejected_locally() {
    assert_eq!(invalid_path(edited(V1, |v| v["service_use_country"] = json!("US"))), "$.service_use_country");
    let domestic = include_str!("../../../fixtures/01-invoice-taxed-cash.json");
    assert_eq!(invalid_path(edited(domestic, |v| v["service_use_country"] = json!("US"))), "$.service_use_country");
}

#[test]
fn customer_document_outside_catalog_06_2800() {
    assert_eq!(code(V1, |v| v["customer"]["document_type"] = json!("X")), ("2800", "$.customer.document_type".into()));
    edited(V1, |v| v["customer"]["document_type"] = json!("-")).unwrap();
}

#[test]
fn customer_with_ruc_2800_where_the_sheet_forbids_it() {
    let ruc = |v: &mut Value| {
        v["customer"]["document_type"] = json!("6");
        v["customer"]["document_number"] = json!("20100066603");
    };
    for op in ["0200", "0201", "0204"] {
        let (got, _) = code(V3, |v| {
            ruc(v);
            v["operation_type"] = json!(op);
            if op != "0201" {
                v.as_object_mut().unwrap().remove("service_use_country");
            }
        });
        assert_eq!(got, "2800", "{op}");
    }
    edited(V3, ruc).expect("0208 takes a customer with RUC on an invoice");
    assert_eq!(code(V3, |v| { ruc(v); as_receipt(v); }).0, "2800");
}

#[test]
fn dni_not_8_digits_2801_and_foreign_document_over_15_or_with_spaces_2802() {
    let doc = |kind: &'static str, number: &'static str| move |v: &mut Value| {
        v["customer"]["document_type"] = json!(kind);
        v["customer"]["document_number"] = json!(number);
    };
    assert_eq!(code(V1, doc("1", "1234567")).0, "2801");
    edited(V1, doc("1", "12345678")).unwrap();
    assert_eq!(code(V1, doc("7", "X1234567890123456")).0, "2802");
    assert_eq!(code(V1, doc("0", "US 998877")).0, "2802");
}

#[test]
fn a_line_not_40_2642() {
    assert_eq!(code(V2, |v| v["lines"][1]["affectation"] = json!("30")), ("2642", "$.lines[1].affectation".into()));
}

#[test]
fn a_line_with_igv_3110() {
    assert_eq!(code(V1, |v| v["lines"][0]["igv"] = json!("450.00")).0, "3110");
}

#[test]
fn igv_rate_zero_3102() {
    assert_eq!(code(V1, |v| v["lines"][0]["igv_rate"] = json!("0.00")), ("3102", "$.lines[0].igv_rate".into()));
}

#[test]
fn lodging_0202_requires_the_ten_guest_fields_in_code_order() {
    let fields = [
        ("guest_document_number", "3136"),
        ("guest_document_type", "3137"),
        ("passport_country", "3138"),
        ("guest_name", "3139"),
        ("residence_country", "3140"),
        ("country_entry_date", "3141"),
        ("check_in_date", "3142"),
        ("check_out_date", "3143"),
        ("consumption_date", "3144"),
        ("stay_days", "3145"),
    ];
    for (field, want) in fields {
        let (got, path) = code(V4, |v| { v["lines"][1]["lodging"].as_object_mut().unwrap().remove(field); });
        assert_eq!((got, path.as_str()), (want, format!("$.lines[1].lodging.{field}").as_str()));
    }
    assert_eq!(code(V4, |v| { v["lines"][0].as_object_mut().unwrap().remove("lodging"); }).0, "3136");
}

#[test]
fn tour_package_0205_requires_its_four_fields_and_takes_no_other() {
    let tour = |v: &mut Value| {
        v["operation_type"] = json!("0205");
        for line in v["lines"].as_array_mut().unwrap() {
            let lodging = line["lodging"].as_object_mut().unwrap();
            for field in ["residence_country", "country_entry_date", "check_in_date", "check_out_date", "stay_days", "consumption_date"] {
                lodging.remove(field);
            }
        }
    };
    for (field, want) in [("passport_country", "3138"), ("guest_name", "3139"), ("guest_document_type", "3137"), ("guest_document_number", "3136")] {
        assert_eq!(code(V4, |v| { tour(v); v["lines"][0]["lodging"].as_object_mut().unwrap().remove(field); }).0, want);
    }
    let extra = edited(V4, |v| { tour(v); v["lines"][0]["lodging"]["stay_days"] = json!("3"); });
    assert_eq!(invalid_path(extra), "$.lines[0].lodging.stay_days");
}

#[test]
fn lodging_outside_0202_and_0205_is_rejected_locally() {
    let lodging: Value = serde_json::from_str::<Value>(V4).unwrap()["lines"][0]["lodging"].clone();
    assert_eq!(invalid_path(edited(V1, |v| v["lines"][0]["lodging"] = lodging.clone())), "$.lines[0].lodging");
    let domestic = include_str!("../../../fixtures/01-invoice-taxed-cash.json");
    assert_eq!(invalid_path(edited(domestic, |v| v["lines"][0]["lodging"] = lodging)), "$.lines[0].lodging");
}

#[test]
fn guest_value_formats_4280() {
    for (field, value) in [("guest_document_type", "X"), ("passport_country", "USA"), ("residence_country", "us"), ("guest_name", "JO"), ("guest_document_number", "X1")] {
        let (got, path) = code(V4, |v| v["lines"][0]["lodging"][field] = json!(value));
        assert_eq!((got, path), ("4280", format!("$.lines[0].lodging.{field}")), "{field}");
    }
}

#[test]
fn check_out_before_check_in_4282() {
    assert_eq!(code(V4, |v| v["lines"][0]["lodging"]["check_out_date"] = json!("2026-10-13")).0, "4282");
    assert_eq!(invalid_path(edited(V4, |v| v["lines"][0]["lodging"]["check_in_date"] = json!("14/10/2026"))), "$.lines[0].lodging.check_in_date");
}

#[test]
fn stay_days_over_4_digits_or_not_numeric_4281() {
    assert_eq!(code(V4, |v| v["lines"][0]["lodging"]["stay_days"] = json!("12345")).0, "4281");
    assert_eq!(code(V4, |v| v["lines"][0]["lodging"]["stay_days"] = json!("tres")).0, "4281");
}

/// The SDK does not decide what is an export (contract §1.5): over 60 days passes, and `docs/input-model.md`
/// says that part is invoiced apart.
#[test]
fn stay_days_over_60_passes() {
    edited(V4, |v| {
        for line in v["lines"].as_array_mut().unwrap() {
            line["lodging"]["stay_days"] = json!("75");
        }
    })
    .unwrap();
}

#[test]
fn other_totals_or_igv_3107() {
    for field in ["taxed", "exempt", "unaffected", "igv"] {
        assert_eq!(code(V1, |v| v["totals"][field] = json!("10.00")), ("3107", format!("$.totals.{field}")), "{field}");
    }
    edited(V1, |v| v["totals"]["taxed"] = json!("0.00")).unwrap();
    // `ivap_base` without lines of affectation 17 is already rejected by the IVAP check, before this one.
    assert_eq!(invalid_path(edited(V1, |v| v["totals"]["ivap_base"] = json!("10.00"))), "$.totals.ivap_base");
}

#[test]
fn export_total_missing_3003() {
    assert_eq!(code(V1, |v| { v["totals"].as_object_mut().unwrap().remove("export"); }).0, "3003");
}

#[test]
fn export_total_not_the_sum_of_lines_3273() {
    assert_eq!(code(V2, |v| v["totals"]["export"] = json!("1552.00")).0, "3273");
    edited(V2, |v| v["totals"]["export"] = json!("1551.00")).expect("within 1");
}

#[test]
fn total_taxes_not_zero_3294() {
    assert_eq!(code(V1, |v| v["totals"]["total_taxes"] = json!("5.00")).0, "3294");
}

#[test]
fn sale_value_subtotal_and_payable_3278_3279_3280() {
    assert_eq!(code(V1, |v| v["totals"]["sale_value"] = json!("2400.00")).0, "3278");
    assert_eq!(code(V1, |v| v["totals"]["subtotal"] = json!("2400.00")).0, "3279");
    assert_eq!(code(V1, |v| v["totals"]["payable"] = json!("2400.00")).0, "3280");
}

#[test]
fn advances_free_lines_and_icbper_in_an_export_are_out_of_scope() {
    let advance = json!([{"document_type": "02", "document_number": "F001-1", "paid_amount": "100.00", "affectation": "40", "value": "100.00"}]);
    assert_eq!(out_of_scope(edited(V1, |v| v["advances"] = advance)), "$.advances");
    assert_eq!(out_of_scope(edited(V1, |v| v["lines"][0]["free_unit_value"] = json!("250.00"))), "$.lines[0].free_unit_value");
}

#[test]
fn notes_take_neither_country_nor_lodging() {
    let note = include_str!("../../../fixtures/07-credit-note-void.json");
    assert_eq!(invalid_path(edited(note, |v| v["service_use_country"] = json!("US"))), "$.service_use_country");
    let lodging: Value = serde_json::from_str::<Value>(V4).unwrap()["lines"][0]["lodging"].clone();
    assert_eq!(invalid_path(edited(note, |v| v["lines"][0]["lodging"] = lodging)), "$.lines[0].lodging");
}
