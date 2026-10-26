//! Line data of a detraction on hydrobiological resources (1002) or freight transport (1004).
//!
//! 1002: one `cac:AdditionalItemProperty` per catalog 55 code, 3001 to 3006, in `cac:Item`; `Name` is the
//! catalog description (4235 if empty), 3005 goes in `UsabilityPeriod/StartDate` and 3006 in
//! `ValueQuantity` in metric tons (3135 if absent).
//! 1004: the line's `cac:Delivery`, children in the order of UBL 2.1 `DeliveryType` (`DeliveryLocation`,
//! `Despatch`, `DeliveryTerms`), with the three reference values 01, 02 and 03 (3124 to 3126).

use sunat_model::{FishingSale, FreightPlace, FreightService};

use crate::amount::two_decimals;
use crate::writer::XmlWriter;

const METRIC_TONS: &str = "TNE";

type Text = fn(&FishingSale) -> Option<&String>;

const FISHING_TEXTS: &[(&str, &str, Text)] = &[
    ("3001", "Detracciones: Recursos Hidrobiológicos-Matrícula de la embarcación", |f| f.vessel_registration.as_ref()),
    ("3002", "Detracciones: Recursos Hidrobiológicos-Nombre de la embarcación", |f| f.vessel_name.as_ref()),
    ("3003", "Detracciones: Recursos Hidrobiológicos-Tipo de especie vendida", |f| f.species.as_ref()),
    ("3004", "Detracciones: Recursos Hidrobiológicos-Lugar de descarga", |f| f.landing_place.as_ref()),
];
const LANDING_DATE: (&str, &str) = ("3005", "Detracciones: Recursos Hidrobiológicos-Fecha de descarga");
const QUANTITY: (&str, &str) = ("3006", "Detracciones: Recursos Hidrobiológicos-Cantidad de especie vendida");

pub(crate) fn fishing_properties(w: &mut XmlWriter, fishing: &FishingSale) {
    for (code, name, get) in FISHING_TEXTS {
        if let Some(value) = get(fishing) {
            property(w, code, name, |w| w.text("cbc:Value", value));
        }
    }
    if let Some(date) = &fishing.landing_date {
        property(w, LANDING_DATE.0, LANDING_DATE.1, |w| {
            w.open("cac:UsabilityPeriod", &[]);
            w.text("cbc:StartDate", date);
            w.close();
        });
    }
    if let Some(quantity) = &fishing.quantity {
        property(w, QUANTITY.0, QUANTITY.1, |w| {
            w.leaf("cbc:ValueQuantity", &[("unitCode", METRIC_TONS)], &two_decimals(quantity));
        });
    }
}

fn property(w: &mut XmlWriter, code: &str, name: &str, body: impl FnOnce(&mut XmlWriter)) {
    w.open("cac:AdditionalItemProperty", &[]);
    w.text("cbc:Name", name);
    w.text("cbc:NameCode", code);
    body(w);
    w.close();
}

/// Catalog of `DeliveryTerms/ID` for 1004: 01 service, 02 effective load, 03 nominal payload.
pub(crate) fn freight_delivery(w: &mut XmlWriter, freight: &FreightService, currency: &str) {
    w.open("cac:Delivery", &[]);
    if let Some(destination) = &freight.destination {
        w.open("cac:DeliveryLocation", &[]);
        address(w, "cac:Address", destination);
        w.close();
    }
    w.open("cac:Despatch", &[]);
    if let Some(detail) = &freight.trip_detail {
        w.text("cbc:Instructions", detail);
    }
    if let Some(origin) = &freight.origin {
        address(w, "cac:DespatchAddress", origin);
    }
    w.close();
    let terms = [
        ("01", &freight.reference_value),
        ("02", &freight.effective_load_value),
        ("03", &freight.nominal_load_value),
    ];
    for (id, amount) in terms {
        let Some(amount) = amount else { continue };
        w.open("cac:DeliveryTerms", &[]);
        w.text("cbc:ID", id);
        w.leaf("cbc:Amount", &[("currencyID", currency)], &two_decimals(amount));
        w.close();
    }
    w.close();
}

fn address(w: &mut XmlWriter, element: &'static str, place: &FreightPlace) {
    w.open(element, &[]);
    if let Some(ubigeo) = &place.ubigeo {
        w.text("cbc:ID", ubigeo);
    }
    if let Some(line) = &place.address {
        w.open("cac:AddressLine", &[]);
        w.text("cbc:Line", line);
        w.close();
    }
    w.close();
}
