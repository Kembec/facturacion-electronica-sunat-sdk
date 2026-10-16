//! The sender's despatch advice (09): UBL 2.1 `DespatchAdvice` with SUNAT's customization 2.0, in
//! the element order of the OASIS schema and the content SUNAT's validation rules for the sheet
//! «Guía-Remitente2_0» expect.
//!
//! SUNAT has no test environment for despatch advices: this output is checked against the OASIS
//! UBL 2.1 schema (`tests/xsd.rs`) and the model applies SUNAT's XSL rules; nothing here has been
//! accepted by SUNAT's REST service.

use sunat_model::{Carrier, Despatch, DespatchLine, DespatchParty, Driver, Place, RelatedDocument, Shipment};

use crate::amount::shortest;
use crate::document::root;
use crate::parties::{signature_reference, RUC_SCHEME};
use crate::writer::XmlWriter;
use crate::Options;

/// SUNAT's customization of UBL 2.1 for the despatch advice.
const CUSTOMIZATION_ID: &str = "2.0";

/// `cbc:ID` of the shipment, fixed by SUNAT.
const SHIPMENT_ID: &str = "SUNAT_Envio";

/// `cbc:SpecialInstructions` of a transfer in M1 or L category vehicles.
const M1_OR_L_INDICATOR: &str = "SUNAT_Envio_IndicadorTrasladoVehiculoM1L";

pub(crate) fn body(w: &mut XmlWriter, d: &Despatch, options: &Options) {
    root(w, "DespatchAdvice", "urn:oasis:names:specification:ubl:schema:xsd:DespatchAdvice-2", false);
    header(w, d);
    for doc in &d.related_documents {
        related_document(w, doc);
    }
    signature_reference(w, &d.issuer, options);
    w.open("cac:DespatchSupplierParty", &[]);
    party(w, RUC_SCHEME, &d.issuer.ruc, &d.issuer.legal_name);
    w.close();
    w.open("cac:DeliveryCustomerParty", &[]);
    despatch_party(w, &d.recipient);
    w.close();
    if let Some(buyer) = &d.buyer {
        w.open("cac:BuyerCustomerParty", &[]);
        despatch_party(w, buyer);
        w.close();
    }
    if let Some(seller) = &d.seller {
        w.open("cac:SellerSupplierParty", &[]);
        despatch_party(w, seller);
        w.close();
    }
    shipment(w, &d.shipment);
    for (index, line) in d.lines.iter().enumerate() {
        despatch_line(w, index + 1, line);
    }
    w.close();
}

fn header(w: &mut XmlWriter, d: &Despatch) {
    let issued = d.issue_date.to_lima();
    w.text("cbc:UBLVersionID", &d.ubl_version);
    w.text("cbc:CustomizationID", CUSTOMIZATION_ID);
    w.text("cbc:ID", &d.identifier());
    w.text("cbc:IssueDate", &issued.date_string());
    w.text("cbc:IssueTime", &issued.time_string());
    w.text("cbc:DespatchAdviceTypeCode", &d.document_type);
    w.text_opt("cbc:Note", d.note.as_deref());
}

fn related_document(w: &mut XmlWriter, doc: &RelatedDocument) {
    w.open("cac:AdditionalDocumentReference", &[]);
    w.text("cbc:ID", &doc.document_number);
    w.text("cbc:DocumentTypeCode", &doc.document_type);
    w.text("cbc:DocumentType", &doc.description);
    if let Some(ruc) = doc.issuer_ruc.as_deref().filter(|r| !r.is_empty()) {
        w.open("cac:IssuerParty", &[]);
        party_identification(w, RUC_SCHEME, ruc);
        w.close();
    }
    w.close();
}

fn despatch_party(w: &mut XmlWriter, p: &DespatchParty) {
    party(w, &p.document_type, &p.document_number, &p.legal_name);
}

/// `cac:Party` with its identity document and legal name.
fn party(w: &mut XmlWriter, scheme: &str, number: &str, legal_name: &str) {
    w.open("cac:Party", &[]);
    party_identification(w, scheme, number);
    w.open("cac:PartyLegalEntity", &[]);
    w.text("cbc:RegistrationName", legal_name);
    w.close();
    w.close();
}

fn party_identification(w: &mut XmlWriter, scheme: &str, number: &str) {
    w.open("cac:PartyIdentification", &[]);
    w.leaf("cbc:ID", &[("schemeID", scheme)], number);
    w.close();
}

fn shipment(w: &mut XmlWriter, s: &Shipment) {
    w.open("cac:Shipment", &[]);
    w.text("cbc:ID", SHIPMENT_ID);
    w.text("cbc:HandlingCode", &s.reason_code);
    w.text_opt("cbc:HandlingInstructions", s.reason_description.as_deref());
    if let (Some(weight), Some(unit)) = (&s.gross_weight, &s.weight_unit) {
        w.leaf("cbc:GrossWeightMeasure", &[("unitCode", unit.as_str())], weight.as_str());
    }
    w.text_opt("cbc:TotalTransportHandlingUnitQuantity", s.packages.as_deref());
    if s.m1_or_l_vehicle {
        w.text("cbc:SpecialInstructions", M1_OR_L_INDICATOR);
    }
    shipment_stage(w, s);
    w.open("cac:Delivery", &[]);
    address(w, "cac:DeliveryAddress", &s.destination);
    w.open("cac:Despatch", &[]);
    address(w, "cac:DespatchAddress", &s.origin);
    w.close();
    w.close();
    vehicles(w, s);
    w.close();
}

fn shipment_stage(w: &mut XmlWriter, s: &Shipment) {
    w.open("cac:ShipmentStage", &[]);
    w.text("cbc:TransportModeCode", &s.transport_mode);
    if let Some(start) = &s.start_date {
        w.open("cac:TransitPeriod", &[]);
        w.text("cbc:StartDate", &start.date_string());
        w.close();
    }
    if let Some(carrier) = &s.carrier {
        carrier_party(w, carrier);
    }
    for (index, driver) in s.drivers.iter().enumerate() {
        driver_person(w, driver, if index == 0 { "Principal" } else { "Secundario" });
    }
    w.close();
}

fn carrier_party(w: &mut XmlWriter, carrier: &Carrier) {
    w.open("cac:CarrierParty", &[]);
    party_identification(w, RUC_SCHEME, &carrier.ruc);
    w.open("cac:PartyLegalEntity", &[]);
    w.text("cbc:RegistrationName", &carrier.legal_name);
    w.text_opt("cbc:CompanyID", carrier.mtc_registration.as_deref());
    w.close();
    w.close();
}

fn driver_person(w: &mut XmlWriter, driver: &Driver, role: &str) {
    w.open("cac:DriverPerson", &[]);
    w.leaf("cbc:ID", &[("schemeID", driver.document_type.as_str())], &driver.document_number);
    w.text("cbc:FirstName", &driver.first_name);
    w.text("cbc:FamilyName", &driver.family_name);
    w.text("cbc:JobTitle", role);
    w.open("cac:IdentityDocumentReference", &[]);
    w.text("cbc:ID", &driver.license);
    w.close();
    w.close();
}

/// Ubigeo, annex establishment (its code, with the RUC it belongs to in `listID`) and address line.
fn address(w: &mut XmlWriter, name: &'static str, place: &Place) {
    w.open(name, &[]);
    w.text("cbc:ID", &place.ubigeo);
    if let Some(code) = place.establishment_code.as_deref().filter(|c| !c.is_empty()) {
        let ruc = place.establishment_ruc.as_deref().unwrap_or("");
        w.leaf("cbc:AddressTypeCode", &[("listID", ruc)], code);
    }
    w.open("cac:AddressLine", &[]);
    w.text("cbc:Line", &place.address);
    w.close();
    w.close();
}

/// The first plate is the main vehicle; the rest go as attached equipment.
fn vehicles(w: &mut XmlWriter, s: &Shipment) {
    let Some((main, secondary)) = s.vehicles.split_first() else { return };
    w.open("cac:TransportHandlingUnit", &[]);
    w.open("cac:TransportEquipment", &[]);
    w.text("cbc:ID", &main.plate);
    for vehicle in secondary {
        w.open("cac:AttachedTransportEquipment", &[]);
        w.text("cbc:ID", &vehicle.plate);
        w.close();
    }
    w.close();
    w.close();
}

fn despatch_line(w: &mut XmlWriter, number: usize, line: &DespatchLine) {
    let id = number.to_string();
    w.open("cac:DespatchLine", &[]);
    w.text("cbc:ID", &id);
    if let Some(quantity) = &line.quantity {
        w.leaf("cbc:DeliveredQuantity", &[("unitCode", line.unit.as_str())], &shortest(quantity));
    }
    // The schema requires an order line reference; SUNAT reads nothing from it.
    w.open("cac:OrderLineReference", &[]);
    w.text("cbc:LineID", &id);
    w.close();
    w.open("cac:Item", &[]);
    w.text("cbc:Description", &line.description);
    if let Some(code) = line.product_code.as_deref().filter(|c| !c.is_empty()) {
        w.open("cac:SellersItemIdentification", &[]);
        w.text("cbc:ID", code);
        w.close();
    }
    if let Some(code) = line.sunat_product_code.as_deref().filter(|c| !c.is_empty()) {
        w.open("cac:CommodityClassification", &[]);
        w.text("cbc:ItemClassificationCode", code);
        w.close();
    }
    w.close();
    w.close();
}
