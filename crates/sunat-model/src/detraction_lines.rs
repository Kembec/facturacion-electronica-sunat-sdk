//! Line data of the detraction operations that need it: the catch of 1002 (catalog 55 codes 3001 to
//! 3006, `cac:AdditionalItemProperty`) and the trip of 1004 (the line's `cac:Delivery`). SUNAT rejects
//! a missing field with its own code on every line (3063, 3130 to 3134; 3116 to 3126) and an amount of
//! `DeliveryTerms` that is not positive with 2 decimals (3122, 3123). The text formats SUNAT only
//! observes (4280, 4281, 4235) and the reference values against the amount (D.S. 020-2021-MTC) are
//! not checked: SUNAT does not reject them.

use crate::validate::fits;
use crate::{Date, Decimal, FishingSale, FreightPlace, FreightService, Invoice, Line, ModelError};

type Text<T> = fn(&T) -> Option<&String>;

const FISHING_TEXT: &[(&str, &str, Text<FishingSale>)] = &[
    ("vessel_registration", "3063", |f| f.vessel_registration.as_ref()),
    ("vessel_name", "3130", |f| f.vessel_name.as_ref()),
    ("species", "3131", |f| f.species.as_ref()),
    ("landing_place", "3132", |f| f.landing_place.as_ref()),
    ("landing_date", "3134", |f| f.landing_date.as_ref()),
];

type Amount = fn(&FreightService) -> Option<&Decimal>;

const FREIGHT_AMOUNTS: &[(&str, &str, Amount)] = &[
    ("reference_value", "3124", |f| f.reference_value.as_ref()),
    ("effective_load_value", "3125", |f| f.effective_load_value.as_ref()),
    ("nominal_load_value", "3126", |f| f.nominal_load_value.as_ref()),
];

/// Every line of a 1002 carries `fishing` and every line of a 1004 `freight`; any other operation
/// type takes neither.
pub(crate) fn check(invoice: &Invoice) -> Result<(), ModelError> {
    let operation = invoice.operation_type.as_str();
    invoice.voucher.lines.iter().enumerate().try_for_each(|(index, line)| check_line(index, line, operation))
}

/// Notes carry neither: they are not detraction operations.
pub(crate) fn reject_any(lines: &[Line]) -> Result<(), ModelError> {
    for (index, line) in lines.iter().enumerate() {
        if line.fishing.is_some() {
            return Err(ModelError::invalid(format!("$.lines[{index}].fishing"), "applies to invoices with operation_type 1002 only"));
        }
        if line.freight.is_some() {
            return Err(ModelError::invalid(format!("$.lines[{index}].freight"), "applies to invoices with operation_type 1004 only"));
        }
    }
    Ok(())
}

fn check_line(index: usize, line: &Line, operation: &str) -> Result<(), ModelError> {
    let base = format!("$.lines[{index}]");
    if operation != "1002" && line.fishing.is_some() {
        return Err(ModelError::invalid(format!("{base}.fishing"), "applies only with operation_type 1002"));
    }
    if operation != "1004" && line.freight.is_some() {
        return Err(ModelError::invalid(format!("{base}.freight"), "applies only with operation_type 1004"));
    }
    match operation {
        "1002" => check_fishing(&format!("{base}.fishing"), line.fishing.as_ref()),
        "1004" => check_freight(&format!("{base}.freight"), line.freight.as_ref()),
        _ => Ok(()),
    }
}

fn check_fishing(base: &str, fishing: Option<&FishingSale>) -> Result<(), ModelError> {
    let Some(fishing) = fishing else {
        return Err(ModelError::rule("3063", base, "operation type 1002 requires the catch (catalog 55 codes 3001 to 3006) on every line"));
    };
    for (field, code, get) in FISHING_TEXT {
        require_text(code, &format!("{base}.{field}"), get(fishing).map(String::as_str))?;
    }
    if let Some(date) = &fishing.landing_date {
        date.parse::<Date>().map_err(|_| ModelError::invalid(format!("{base}.landing_date"), format!("expected YYYY-MM-DD, got {date:?}")))?;
    }
    if fishing.quantity.is_none() {
        return Err(ModelError::rule("3133", format!("{base}.quantity"), "operation type 1002 requires the quantity of the species"));
    }
    Ok(())
}

fn check_freight(base: &str, freight: Option<&FreightService>) -> Result<(), ModelError> {
    let empty = FreightService::default();
    let freight = freight.unwrap_or(&empty);
    check_place(&format!("{base}.origin"), freight.origin.as_ref(), ("3116", "3117"))?;
    check_place(&format!("{base}.destination"), freight.destination.as_ref(), ("3118", "3119"))?;
    require_text("3120", &format!("{base}.trip_detail"), freight.trip_detail.as_deref())?;
    for (field, code, get) in FREIGHT_AMOUNTS {
        let path = format!("{base}.{field}");
        let Some(value) = get(freight) else {
            return Err(ModelError::rule(code, path, format!("operation type 1004 requires {field}")));
        };
        if value.is_negative() || value.is_zero() || !fits(value, 12, 2) {
            let reason = format!("expected a positive amount with up to 12 integer digits and 2 decimals (SUNAT 3122, 3123), got {:?}", value.as_str());
            return Err(ModelError::invalid(path, reason));
        }
    }
    Ok(())
}

fn check_place(base: &str, place: Option<&FreightPlace>, (ubigeo, address): (&'static str, &'static str)) -> Result<(), ModelError> {
    let empty = FreightPlace::default();
    let place = place.unwrap_or(&empty);
    require_text(ubigeo, &format!("{base}.ubigeo"), place.ubigeo.as_deref())?;
    require_text(address, &format!("{base}.address"), place.address.as_deref())
}

/// An empty text renders an empty element, which SUNAT reads as missing.
fn require_text(code: &'static str, path: &str, value: Option<&str>) -> Result<(), ModelError> {
    match value {
        Some(text) if !text.trim().is_empty() => Ok(()),
        _ => Err(ModelError::rule(code, path, "required by the operation type")),
    }
}
