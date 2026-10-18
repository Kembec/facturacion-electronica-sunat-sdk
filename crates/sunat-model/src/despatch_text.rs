//! Free-text fields SUNAT observes by format in the sender's despatch advice (sheet
//! «Guía-Remitente2_0», rows in brackets), on the tags the SDK writes. SUNAT has no test environment
//! for despatch advices, so the sheet is the only source. A value is refused with its field and the
//! observation's code, never trimmed or rewritten. These run after `despatch_rules`, so a value its
//! errors already refuse (a blank name, 2761) answers with the error.

use crate::despatch_rules::{all_zeros, alnum_upper};
use crate::text_format::{check_format, Text};
use crate::{Despatch, DespatchParty, ModelError};

/// «alfanumérico de hasta N caracteres … Tampoco se permite solo espacios en blanco».
const fn name(max: usize) -> Text {
    Text::spaced(1, max).not_only_spaces()
}

pub(crate) fn check(d: &Despatch) -> Result<(), ModelError> {
    check_format("$.note", d.note.as_deref(), name(250), "4186")?; // R!24
    check_format("$.issuer.legal_name", Some(&d.issuer.legal_name), name(250), "4338")?; // R!37
    for (index, doc) in d.related_documents.iter().enumerate() {
        check_format(&format!("$.related_documents[{index}].description"), Some(&doc.description), name(120), "4372")?; // R!48
    }
    check_party("$.recipient", Some(&d.recipient), "4152")?; // R!114
    check_party("$.seller", d.seller.as_ref(), "4106")?; // R!138
    check_party("$.buyer", d.buyer.as_ref(), "4381")?; // R!161
    check_shipment(d)?;
    for (index, line) in d.lines.iter().enumerate() {
        let at = |field: &str| format!("$.lines[{index}].{field}");
        check_format(&at("description"), Some(&line.description), Text::any_whitespace(3, 500), "4084")?; // R!449
        check_format(&at("product_code"), line.product_code.as_deref(), Text::spaced(1, 30), "4085")?; // R!450
    }
    Ok(())
}

fn check_party(path: &str, party: Option<&DespatchParty>, code: &str) -> Result<(), ModelError> {
    let Some(party) = party else { return Ok(()) };
    check_format(&format!("{path}.legal_name"), Some(&party.legal_name), name(250), code)
}

fn check_shipment(d: &Despatch) -> Result<(), ModelError> {
    let s = &d.shipment;
    if s.reason_code == "13" {
        let description = s.reason_description.as_deref();
        check_format("$.shipment.reason_description", description, Text::spaced(3, 100), "4190")?; // R!171
        let letters = description.map_or(0, |text| text.chars().filter(|c| c.is_alphabetic()).count());
        if description.is_some_and(|text| !text.is_empty()) && letters < 3 {
            let reason = format!("expected at least 3 letters (SUNAT 4190), got {:?}", description.unwrap_or(""));
            return Err(ModelError::invalid("$.shipment.reason_description", reason)); // R!172
        }
    }
    if s.transport_mode == "01" && !s.m1_or_l_vehicle {
        if let Some(carrier) = &s.carrier {
            let name = Text::spaced(3, 250).not_only_spaces();
            check_format("$.shipment.carrier.legal_name", Some(&carrier.legal_name), name, "4165")?; // R!241
            check_mtc(carrier.mtc_registration.as_deref())?; // R!245
        }
    }
    for (index, driver) in s.drivers.iter().enumerate() {
        let at = |field: &str| format!("$.shipment.drivers[{index}].{field}");
        check_format(&at("first_name"), Some(&driver.first_name), name(250), "4409")?; // R!316, R!352
        check_format(&at("family_name"), Some(&driver.family_name), name(250), "4409")?; // R!321, R!357
    }
    let address = Text::spaced(3, 500).not_only_spaces();
    check_format("$.shipment.origin.address", Some(&s.origin.address), address, "4076")?; // R!373
    // R!400 exempts reason 18, which the SDK refuses as out of scope.
    check_format("$.shipment.destination.address", Some(&s.destination.address), address, "4068") // R!400
}

/// The carrier's MTC registration: up to 20 uppercase letters or digits, not only zeros.
fn check_mtc(value: Option<&str>) -> Result<(), ModelError> {
    let Some(value) = value.filter(|v| !v.is_empty()) else { return Ok(()) };
    if value.len() <= 20 && alnum_upper(value) && !all_zeros(value) {
        return Ok(());
    }
    let reason = format!("expected up to 20 uppercase letters or digits, not only zeros (SUNAT 4392), got {value:?}");
    Err(ModelError::invalid("$.shipment.carrier.mtc_registration", reason))
}
