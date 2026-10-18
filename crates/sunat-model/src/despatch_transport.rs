//! SUNAT's validation rules for the shipment of a sender's despatch advice (sheet
//! «Guía-Remitente2_0», type XSL): reason, weight, transport mode, start date, carrier, vehicles,
//! drivers, origin and destination. Each rejection carries the error code SUNAT would answer.
//!
//! The SDK does not render the carrier's vehicle and driver registration indicator
//! (`SUNAT_Envio_IndicadorVehiculoConductoresTransp`): in public transport without M1 or L vehicles
//! the carrier names its own vehicles and drivers, so the despatch advice carries none (3354, 3455).

use crate::despatch_rules::{all_zeros, alnum_upper, blank, digits, digits_between, positive_fits, rule, IDENTITY_TYPES};
use crate::{Date, Despatch, Driver, ModelError, Place, Shipment};

/// Catalog 20 codes the SDK renders.
const REASONS: &[&str] = &["01", "02", "03", "04", "05", "06", "07", "13", "14", "17"];

/// Catalog 20 codes that need data this SDK does not render (ports, airports, containers, DAM).
const REASONS_OUT_OF_SCOPE: &[&str] = &["08", "09", "18"];

/// The transfer reason (catalog 20) of a despatch advice that passed `check_shipment`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Reason<'a>(pub &'a str);

impl Reason<'_> {
    pub(crate) fn is(&self, codes: &[&str]) -> bool {
        codes.contains(&self.0)
    }
}

/// Which parts of the transport the despatch advice must, or must not, carry.
#[derive(Clone, Copy)]
struct Mode {
    public: bool,
    m1_or_l: bool,
}

impl Mode {
    /// Private transport in regular vehicles: the sender names its vehicles and drivers.
    fn names_vehicles(self) -> bool {
        !self.public && !self.m1_or_l
    }

    /// Public transport in regular vehicles: the sender names its carrier.
    fn names_carrier(self) -> bool {
        self.public && !self.m1_or_l
    }
}

pub(crate) fn check_shipment(d: &Despatch) -> Result<Reason<'_>, ModelError> {
    let s = &d.shipment;
    let reason = check_reason(s)?;
    check_weight(s)?;
    let mode = check_mode(s)?;
    check_start_date(d, mode)?;
    check_handover_date(d, mode)?;
    check_carrier(d, mode)?;
    check_vehicles(s, mode)?;
    check_drivers(s, mode)?;
    check_place(d, reason, &s.origin, Side::Origin)?;
    check_place(d, reason, &s.destination, Side::Destination)?;
    Ok(reason)
}

fn check_reason(s: &Shipment) -> Result<Reason<'_>, ModelError> {
    let code = s.reason_code.as_str();
    rule(!code.is_empty(), "3404", "$.shipment.reason_code", "required")?;
    if REASONS_OUT_OF_SCOPE.contains(&code) {
        return Err(ModelError::OutOfScope {
            path: "$.shipment.reason_code".into(),
            feature: "transfer reasons 08, 09 and 18 (imports, exports, itinerant issuer)",
        });
    }
    rule(REASONS.contains(&code), "3405", "$.shipment.reason_code", "not a catalog 20 code")?;
    if code == "13" {
        let described = s.reason_description.as_deref().is_some_and(|text| !blank(text));
        rule(described, "3457", "$.shipment.reason_description", "required for reason 13")?;
    }
    Ok(Reason(code))
}

fn check_weight(s: &Shipment) -> Result<(), ModelError> {
    let Some(weight) = &s.gross_weight else {
        return Err(ModelError::rule("2880", "$.shipment.gross_weight", "required"));
    };
    rule(positive_fits(weight, 12, 3), "2523", "$.shipment.gross_weight", "a positive weight with up to 12 integer digits and 3 decimals")?;
    let unit = s.weight_unit.as_deref().unwrap_or("");
    rule(!unit.is_empty(), "2881", "$.shipment.weight_unit", "required")?;
    rule(matches!(unit, "KGM" | "TNE"), "2523", "$.shipment.weight_unit", "must be KGM or TNE")
}

fn check_mode(s: &Shipment) -> Result<Mode, ModelError> {
    let code = s.transport_mode.as_str();
    rule(!code.is_empty(), "2532", "$.shipment.transport_mode", "required")?;
    rule(matches!(code, "01" | "02"), "2773", "$.shipment.transport_mode", "not a catalog 18 code")?;
    Ok(Mode { public: code == "01", m1_or_l: s.m1_or_l_vehicle })
}

/// Required, and not before the issue date in Lima, in private transport and with M1 or L vehicles
/// (3406, 3343). In public transport SUNAT only observes it.
fn check_start_date(d: &Despatch, mode: Mode) -> Result<(), ModelError> {
    if mode.public && !mode.m1_or_l {
        return Ok(());
    }
    let Some(start) = &d.shipment.start_date else {
        return Err(ModelError::rule("3406", "$.shipment.start_date", "required in private transport or with M1 or L vehicles"));
    };
    let issued = d.issue_date.to_lima().date_string();
    rule(start.date_string() >= issued, "3343", "$.shipment.start_date", "must not be before the issue date")
}

/// The day the goods go to the carrier (field 39-A of RS 108-2026, in force since 2026-06-01):
/// required in public transport (3617), a `YYYY-MM-DD` date (3619), the same day as the issue date
/// or later (3618). The norm fixes no relation with `start_date`, so none is checked.
fn check_handover_date(d: &Despatch, mode: Mode) -> Result<(), ModelError> {
    let path = "$.shipment.carrier_handover_date";
    let given = d.shipment.carrier_handover_date.as_deref().filter(|text| !blank(text));
    if !mode.public {
        return match d.shipment.carrier_handover_date {
            None => Ok(()),
            Some(_) => Err(ModelError::invalid(path, "only in public transport")),
        };
    }
    let Some(text) = given else {
        return Err(ModelError::rule("3617", path, "required in public transport"));
    };
    let Ok(handover) = text.parse::<Date>() else {
        return Err(ModelError::rule("3619", path, "must be a date (YYYY-MM-DD)"));
    };
    let issued = d.issue_date.to_lima().date_string();
    rule(handover.date_string() >= issued, "3618", path, "must not be before the issue date")
}

fn check_carrier(d: &Despatch, mode: Mode) -> Result<(), ModelError> {
    let carrier = d.shipment.carrier.as_ref();
    if !mode.public {
        return rule(carrier.is_none(), "3347", "$.shipment.carrier", "private transport has no carrier");
    }
    if !mode.names_carrier() {
        return Ok(());
    }
    let ruc = carrier.map_or("", |c| c.ruc.as_str());
    rule(!ruc.is_empty(), "2558", "$.shipment.carrier.ruc", "required in public transport")?;
    if !(ruc.len() == 11 && digits(ruc)) {
        return Err(ModelError::invalid("$.shipment.carrier.ruc", format!("expected 11 digits, got {ruc:?}")));
    }
    rule(ruc != d.issuer.ruc, "2560", "$.shipment.carrier.ruc", "must not be the sender")?;
    let named = carrier.is_some_and(|c| !blank(&c.legal_name));
    rule(named, "2563", "$.shipment.carrier.legal_name", "required in public transport")
}

fn check_vehicles(s: &Shipment, mode: Mode) -> Result<(), ModelError> {
    if mode.names_vehicles() {
        rule(!s.vehicles.is_empty(), "2566", "$.shipment.vehicles", "the main vehicle's plate is required in private transport")?;
    } else if mode.m1_or_l {
        rule(s.vehicles.len() <= 1, "3453", "$.shipment.vehicles[1]", "no secondary vehicles with M1 or L vehicles")?;
    } else {
        rule(s.vehicles.is_empty(), "3354", "$.shipment.vehicles", "in public transport the carrier names its vehicles")?;
    }
    for (index, vehicle) in s.vehicles.iter().enumerate() {
        let plate = vehicle.plate.as_str();
        let ok = (6..=8).contains(&plate.len()) && alnum_upper(plate) && !all_zeros(plate);
        rule(ok, "2567", format!("$.shipment.vehicles[{index}].plate"), "6 to 8 uppercase letters or digits, not only zeros")?;
    }
    Ok(())
}

fn check_drivers(s: &Shipment, mode: Mode) -> Result<(), ModelError> {
    if !mode.names_vehicles() {
        rule(s.drivers.is_empty(), "3455", "$.shipment.drivers", "drivers go only in private transport in regular vehicles")?;
        return Ok(());
    }
    rule(!s.drivers.is_empty(), "3357", "$.shipment.drivers", "the main driver is required in private transport")?;
    for (index, driver) in s.drivers.iter().enumerate() {
        check_driver(driver, &format!("$.shipment.drivers[{index}]"))?;
    }
    Ok(())
}

fn check_driver(driver: &Driver, at: &str) -> Result<(), ModelError> {
    let kind = driver.document_type.as_str();
    let number = driver.document_number.as_str();
    rule(!kind.is_empty(), "2570", format!("{at}.document_type"), "required")?;
    rule(IDENTITY_TYPES.contains(&kind) && kind != "6", "2571", format!("{at}.document_type"), "a catalog 06 code other than 6 (RUC)")?;
    rule(!number.is_empty(), "2568", format!("{at}.document_number"), "required")?;
    let ok = if kind == "1" {
        number.len() == 8 && digits(number)
    } else {
        (1..=15).contains(&number.chars().count()) && !number.chars().any(char::is_whitespace)
    };
    rule(ok, "2569", format!("{at}.document_number"), "a DNI has 8 digits; other documents up to 15 characters without whitespace")?;
    rule(!blank(&driver.first_name), "3360", format!("{at}.first_name"), "required")?;
    rule(!blank(&driver.family_name), "3361", format!("{at}.family_name"), "required")?;
    let license = driver.license.as_str();
    rule(!license.is_empty(), "2572", format!("{at}.license"), "required")?;
    let ok = (9..=10).contains(&license.len()) && alnum_upper(license) && !all_zeros(license);
    rule(ok, "2573", format!("{at}.license"), "9 or 10 uppercase letters or digits, not only zeros")
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Side {
    Origin,
    Destination,
}

impl Side {
    fn path(self) -> &'static str {
        match self {
            Side::Origin => "$.shipment.origin",
            Side::Destination => "$.shipment.destination",
        }
    }
}

fn check_place(d: &Despatch, reason: Reason, place: &Place, side: Side) -> Result<(), ModelError> {
    let at = |field: &str| format!("{}.{field}", side.path());
    rule(!place.ubigeo.is_empty(), "2775", at("ubigeo"), "required")?;
    rule(digits_between(&place.ubigeo, 6, 6), "2776", at("ubigeo"), "must be 6 digits")?;
    let address_code = if side == Side::Origin { "2577" } else { "2574" };
    rule(!blank(&place.address), address_code, at("address"), "required")?;
    check_establishment(d, reason, place, side)
}

fn check_establishment(d: &Despatch, reason: Reason, place: &Place, side: Side) -> Result<(), ModelError> {
    let at = |field: &str| format!("{}.{field}", side.path());
    let code = place.establishment_code.as_deref().unwrap_or("");
    let ruc = place.establishment_ruc.as_deref().unwrap_or("");
    let code_rule = if side == Side::Origin { "3365" } else { "3369" };
    rule(!(code.is_empty() && !ruc.is_empty()), code_rule, at("establishment_code"), "required with establishment_ruc")?;
    rule(!(code.is_empty() && reason.is(&["04"])), code_rule, at("establishment_code"), "required for reason 04")?;
    rule(code.is_empty() || !ruc.is_empty(), "3410", at("establishment_ruc"), "required with establishment_code")?;
    if ruc.is_empty() {
        return Ok(());
    }
    rule(ruc.len() == 11 && digits(ruc), "3409", at("establishment_ruc"), "must be 11 digits")?;
    let sender = ruc == d.issuer.ruc;
    let not_sender: &[&str] = if side == Side::Origin { &["02", "07"] } else { &["01", "03", "05", "06", "14", "17"] };
    if reason.is(not_sender) {
        rule(!sender, "3411", at("establishment_ruc"), "must not be the sender's for this reason")?;
    }
    if reason.is(&["04"]) {
        rule(sender, "3414", at("establishment_ruc"), "must be the sender's for reason 04")?;
    }
    Ok(())
}
