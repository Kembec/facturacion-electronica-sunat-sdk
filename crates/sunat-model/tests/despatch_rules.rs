//! One test per SUNAT validation rule of the sender's despatch advice: each edits a valid despatch
//! advice so that it breaks exactly that rule and expects SUNAT's error code.

use serde_json::{json, Value};
use sunat_model::{Document, ModelError};

/// Reason 01 (sale), private transport, two vehicles and a driver.
const SALE: &str = include_str!("../../../fixtures/09-despatch-sale.json");
/// Reason 04 (between establishments of the same company), public transport.
const BETWEEN: &str = include_str!("../../../fixtures/09-despatch-between-establishments.json");

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

/// Reason 02 (purchase): the sender receives the goods from a supplier.
fn purchase(v: &mut Value) {
    v["shipment"]["reason_code"] = json!("02");
    v["shipment"]["origin"]["establishment_code"] = Value::Null;
    v["shipment"]["origin"]["establishment_ruc"] = Value::Null;
    v["seller"] = json!({"document_type": "6", "document_number": "20444555666", "legal_name": "PROVEEDOR DEMO S.A."});
}

/// Reason 03 (sale with delivery to a third party): the buyer is not the recipient.
fn third_party(v: &mut Value) {
    v["shipment"]["reason_code"] = json!("03");
    v["buyer"] = json!({"document_type": "6", "document_number": "20111222333", "legal_name": "COMPRADOR DEMO S.A."});
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

#[test]
fn both_fixtures_are_valid() {
    let sale = Document::from_json(SALE).unwrap();
    assert_eq!(sale.file_name(), "20123456789-09-T001-1");
    sale.validate().unwrap();
    Document::from_json(BETWEEN).unwrap().validate().unwrap();
}

#[test]
fn m1_or_l_vehicles_need_no_driver_and_no_carrier() {
    let private = with(SALE, |v| {
        v["shipment"]["m1_or_l_vehicle"] = json!(true);
        v["shipment"]["vehicles"] = json!([{"plate": "ABC123"}]);
        v["shipment"]["drivers"] = json!([]);
    });
    private.unwrap().validate().unwrap();
    let public = with(BETWEEN, |v| {
        v["shipment"]["m1_or_l_vehicle"] = json!(true);
        v["shipment"]["carrier"] = Value::Null;
    });
    public.unwrap().validate().unwrap();
}

#[test]
fn imports_exports_and_itinerant_issuers_are_out_of_scope() {
    for reason in ["08", "09", "18"] {
        let result = with(SALE, |v| v["shipment"]["reason_code"] = json!(reason));
        assert!(matches!(result, Err(ModelError::OutOfScope { .. })), "{reason}: {result:?}");
    }
}

#[test]
fn purchase_and_third_party_sale_are_valid() {
    with(BETWEEN, purchase).unwrap().validate().unwrap();
    with(SALE, third_party).unwrap().validate().unwrap();
}

// Header, parties, related documents and lines (`despatch_rules`).
rejects! {
    ubl_version_2110: "2110", SALE, |v| v["ubl_version"] = json!("2.0");
    document_type_1051: "1051", SALE, |v| v["document_type"] = json!("01");
    series_1001: "1001", SALE, |v| v["series"] = json!("F001");
    number_1001: "1001", SALE, |v| v["number"] = json!("123456789");
    issuer_name_1037: "1037", SALE, |v| v["issuer"]["legal_name"] = json!(" ");
    recipient_type_required_2759: "2759", SALE, |v| v["recipient"]["document_type"] = json!("");
    recipient_type_catalog_2760: "2760", SALE, |v| v["recipient"]["document_type"] = json!("Z");
    recipient_ruc_for_reason_06_3417: "3417", SALE, |v| {
        v["shipment"]["reason_code"] = json!("06");
        v["recipient"] = json!({"document_type": "1", "document_number": "45678912", "legal_name": "PERSONA DEMO"});
    };
    recipient_number_required_2757: "2757", SALE, |v| v["recipient"]["document_number"] = json!("");
    recipient_number_format_2758: "2758", SALE, |v| v["recipient"]["document_number"] = json!("2098765432");
    recipient_name_2761: "2761", SALE, |v| v["recipient"]["legal_name"] = json!("");
    recipient_not_sender_2555: "2555", SALE, |v| v["recipient"]["document_number"] = json!("20123456789");
    recipient_is_sender_2554: "2554", BETWEEN, |v| v["recipient"]["document_number"] = json!("20987654321");
    seller_type_required_2552: "2552", BETWEEN, |v| { purchase(v); v["seller"]["document_type"] = json!(""); };
    seller_type_for_reason_3447: "3447", BETWEEN, |v| { purchase(v); v["seller"]["document_type"] = json!("0"); };
    seller_number_required_2723: "2723", BETWEEN, |v| { purchase(v); v["seller"]["document_number"] = json!(""); };
    seller_number_format_2724: "2724", BETWEEN, |v| { purchase(v); v["seller"]["document_number"] = json!("204445556"); };
    seller_name_3449: "3449", BETWEEN, |v| { purchase(v); v["seller"]["legal_name"] = json!(" "); };
    seller_not_sender_3448: "3448", BETWEEN, |v| { purchase(v); v["seller"]["document_number"] = json!("20123456789"); };
    buyer_type_required_3331: "3331", SALE, |v| { third_party(v); v["buyer"]["document_type"] = json!(""); };
    buyer_type_catalog_3332: "3332", SALE, |v| { third_party(v); v["buyer"]["document_type"] = json!("Z"); };
    buyer_number_required_3333: "3333", SALE, |v| { third_party(v); v["buyer"]["document_number"] = json!(""); };
    buyer_number_format_3337: "3337", SALE, |v| { third_party(v); v["buyer"]["document_number"] = json!("2011122233"); };
    buyer_name_3339: "3339", SALE, |v| { third_party(v); v["buyer"]["legal_name"] = json!(""); };
    buyer_not_sender_3334: "3334", SALE, |v| { third_party(v); v["buyer"]["document_number"] = json!("20123456789"); };
    buyer_not_recipient_3335: "3335", SALE, |v| { third_party(v); v["buyer"]["document_number"] = json!("20987654321"); };
    related_type_required_3376: "3376", SALE, |v| v["related_documents"][0]["document_type"] = json!("");
    related_type_catalog_2692: "2692", SALE, |v| v["related_documents"][0]["document_type"] = json!("07");
    related_number_required_3403: "3403", SALE, |v| v["related_documents"][0]["document_number"] = json!("");
    related_number_format_3441: "3441", SALE, |v| v["related_documents"][0]["document_number"] = json!("B001-123");
    related_customs_declaration_3445: "3445", SALE, |v| {
        v["related_documents"][0] = json!({"document_type": "50", "document_number": "118-2026-10-123456", "description": "DAM"});
    };
    related_tariff_heading_3352: "3352", SALE, |v| {
        v["related_documents"][0] = json!({"document_type": "49", "document_number": "123456", "description": "Constancia"});
    };
    related_duplicated_3340: "3340", SALE, |v| {
        let first = v["related_documents"][0].clone();
        v["related_documents"] = json!([first.clone(), first]);
    };
    related_issuer_required_3380: "3380", SALE, |v| v["related_documents"][0]["issuer_ruc"] = Value::Null;
    related_issuer_format_3409: "3409", SALE, |v| v["related_documents"][0]["issuer_ruc"] = json!("2012345678X");
    related_issuer_is_sender_3381: "3381", SALE, |v| v["related_documents"][0]["issuer_ruc"] = json!("20987654321");
    lines_at_most_9999_2023: "2023", SALE, |v| {
        let line = v["lines"][0].clone();
        v["lines"] = Value::Array(vec![line; 10_000]);
    };
    line_quantity_required_2580: "2580", SALE, |v| v["lines"][0]["quantity"] = Value::Null;
    line_quantity_format_2780: "2780", SALE, |v| v["lines"][0]["quantity"] = json!("0");
    line_unit_2883: "2883", SALE, |v| v["lines"][0]["unit"] = json!("");
    line_description_2781: "2781", SALE, |v| v["lines"][0]["description"] = json!(" ");
    line_sunat_product_code_3002: "3002", SALE, |v| v["lines"][0]["sunat_product_code"] = json!("00000000");
}

// Shipment (`despatch_transport`).
rejects! {
    reason_required_3404: "3404", SALE, |v| v["shipment"]["reason_code"] = json!("");
    reason_catalog_3405: "3405", SALE, |v| v["shipment"]["reason_code"] = json!("99");
    reason_13_description_3457: "3457", SALE, |v| {
        v["shipment"]["reason_code"] = json!("13");
        v["shipment"]["reason_description"] = Value::Null;
    };
    gross_weight_required_2880: "2880", SALE, |v| v["shipment"]["gross_weight"] = Value::Null;
    gross_weight_format_2523: "2523", SALE, |v| v["shipment"]["gross_weight"] = json!("1.2345");
    weight_unit_catalog_2523: "2523", SALE, |v| v["shipment"]["weight_unit"] = json!("LBR");
    weight_unit_required_2881: "2881", SALE, |v| v["shipment"]["weight_unit"] = Value::Null;
    transport_mode_required_2532: "2532", SALE, |v| v["shipment"]["transport_mode"] = json!("");
    transport_mode_catalog_2773: "2773", SALE, |v| v["shipment"]["transport_mode"] = json!("03");
    start_date_required_3406: "3406", SALE, |v| v["shipment"]["start_date"] = Value::Null;
    start_date_not_before_issue_3343: "3343", SALE, |v| v["shipment"]["start_date"] = json!("2026-10-14");
    private_transport_has_no_carrier_3347: "3347", SALE, |v| {
        v["shipment"]["carrier"] = json!({"ruc": "20555666777", "legal_name": "TRANSPORTES DEMO S.R.L."});
    };
    carrier_required_2558: "2558", BETWEEN, |v| v["shipment"]["carrier"] = Value::Null;
    carrier_not_sender_2560: "2560", BETWEEN, |v| v["shipment"]["carrier"]["ruc"] = json!("20123456789");
    carrier_name_2563: "2563", BETWEEN, |v| v["shipment"]["carrier"]["legal_name"] = json!(" ");
    main_vehicle_required_2566: "2566", SALE, |v| v["shipment"]["vehicles"] = json!([]);
    no_secondary_vehicles_with_m1_or_l_3453: "3453", SALE, |v| {
        v["shipment"]["m1_or_l_vehicle"] = json!(true);
        v["shipment"]["drivers"] = json!([]);
    };
    no_vehicles_in_public_transport_3354: "3354", BETWEEN, |v| v["shipment"]["vehicles"] = json!([{"plate": "ABC123"}]);
    plate_format_2567: "2567", SALE, |v| v["shipment"]["vehicles"][1]["plate"] = json!("XYZ-987");
    no_drivers_in_public_transport_3455: "3455", BETWEEN, |v| {
        v["shipment"]["drivers"] = json!([{"document_type": "1", "document_number": "45678912", "first_name": "JUAN", "family_name": "PÉREZ", "license": "Q45678912"}]);
    };
    main_driver_required_3357: "3357", SALE, |v| v["shipment"]["drivers"] = json!([]);
    driver_document_type_required_2570: "2570", SALE, |v| v["shipment"]["drivers"][0]["document_type"] = json!("");
    driver_document_type_catalog_2571: "2571", SALE, |v| v["shipment"]["drivers"][0]["document_type"] = json!("6");
    driver_document_number_required_2568: "2568", SALE, |v| v["shipment"]["drivers"][0]["document_number"] = json!("");
    driver_document_number_format_2569: "2569", SALE, |v| v["shipment"]["drivers"][0]["document_number"] = json!("4567891");
    driver_first_name_3360: "3360", SALE, |v| v["shipment"]["drivers"][0]["first_name"] = json!("");
    driver_family_name_3361: "3361", SALE, |v| v["shipment"]["drivers"][0]["family_name"] = json!(" ");
    driver_license_required_2572: "2572", SALE, |v| v["shipment"]["drivers"][0]["license"] = json!("");
    driver_license_format_2573: "2573", SALE, |v| v["shipment"]["drivers"][0]["license"] = json!("Q4567");
    ubigeo_required_2775: "2775", SALE, |v| v["shipment"]["destination"]["ubigeo"] = json!("");
    ubigeo_format_2776: "2776", SALE, |v| v["shipment"]["origin"]["ubigeo"] = json!("15010A");
    origin_address_2577: "2577", SALE, |v| v["shipment"]["origin"]["address"] = json!(" ");
    destination_address_2574: "2574", SALE, |v| v["shipment"]["destination"]["address"] = json!("");
    origin_establishment_for_reason_04_3365: "3365", BETWEEN, |v| v["shipment"]["origin"]["establishment_code"] = Value::Null;
    destination_establishment_for_reason_04_3369: "3369", BETWEEN, |v| v["shipment"]["destination"]["establishment_code"] = Value::Null;
    establishment_ruc_required_3410: "3410", SALE, |v| v["shipment"]["destination"]["establishment_code"] = json!("0001");
    establishment_ruc_format_3409: "3409", SALE, |v| v["shipment"]["origin"]["establishment_ruc"] = json!("2012345678");
    destination_not_sender_for_sale_3411: "3411", SALE, |v| {
        v["shipment"]["destination"]["establishment_code"] = json!("0001");
        v["shipment"]["destination"]["establishment_ruc"] = json!("20123456789");
    };
    origin_not_sender_for_purchase_3411: "3411", BETWEEN, |v| {
        purchase(v);
        v["shipment"]["origin"]["establishment_code"] = json!("0001");
        v["shipment"]["origin"]["establishment_ruc"] = json!("20123456789");
    };
    establishment_is_sender_for_reason_04_3414: "3414", BETWEEN, |v| v["shipment"]["destination"]["establishment_ruc"] = json!("20987654321");
}
