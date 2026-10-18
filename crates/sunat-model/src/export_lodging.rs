//! Guest data of the export lines (catalog 55, codes 4000 to 4009): required with 0202 (3136 to 3145)
//! and, the four of the tour package, with 0205 (3136 to 3139); then their formats (4280, 4281, 4282).
//! More than 60 days per entry is not lodging export (art. 33 num. 4 of the IGV law), but SUNAT does
//! not check it and the SDK does not decide what is an export: it passes.

use crate::export::{is_country, IDENTITY_DOCUMENTS};
use crate::{Date, Line, Lodging, ModelError};

type Field = fn(&Lodging) -> Option<&String>;

/// In the order of SUNAT's codes 3136 to 3145; `true` marks the four 0205 requires too.
const REQUIRED: &[(&str, &str, bool, Field)] = &[
    ("guest_document_number", "3136", true, |l| l.guest_document_number.as_ref()),
    ("guest_document_type", "3137", true, |l| l.guest_document_type.as_ref()),
    ("passport_country", "3138", true, |l| l.passport_country.as_ref()),
    ("guest_name", "3139", true, |l| l.guest_name.as_ref()),
    ("residence_country", "3140", false, |l| l.residence_country.as_ref()),
    ("country_entry_date", "3141", false, |l| l.country_entry_date.as_ref()),
    ("check_in_date", "3142", false, |l| l.check_in_date.as_ref()),
    ("check_out_date", "3143", false, |l| l.check_out_date.as_ref()),
    ("consumption_date", "3144", false, |l| l.consumption_date.as_ref()),
    ("stay_days", "3145", false, |l| l.stay_days.as_ref()),
];

const DATES: &[(&str, Field)] = &[
    ("country_entry_date", |l| l.country_entry_date.as_ref()),
    ("check_in_date", |l| l.check_in_date.as_ref()),
    ("check_out_date", |l| l.check_out_date.as_ref()),
    ("consumption_date", |l| l.consumption_date.as_ref()),
];

pub(crate) fn reject_any(lines: &[Line], reason: &str) -> Result<(), ModelError> {
    match lines.iter().position(|line| line.lodging.is_some()) {
        Some(index) => Err(ModelError::invalid(format!("$.lines[{index}].lodging"), reason)),
        None => Ok(()),
    }
}

pub(crate) fn check(index: usize, line: &Line, operation: &str) -> Result<(), ModelError> {
    let base = format!("$.lines[{index}].lodging");
    let tour = match operation {
        "0202" => false,
        "0205" => true,
        _ if line.lodging.is_some() => return Err(ModelError::invalid(base, "applies only with operation_type 0202 or 0205")),
        _ => return Ok(()),
    };
    let empty = Lodging::default();
    let lodging = line.lodging.as_ref().unwrap_or(&empty);
    for (field, code, in_tour, get) in REQUIRED {
        let required = !tour || *in_tour;
        match (get(lodging), required) {
            (None, true) => {
                let reason = format!("operation type {operation} requires the guest's {field}");
                return Err(ModelError::rule(code, format!("{base}.{field}"), reason));
            }
            (Some(_), false) => return Err(ModelError::invalid(format!("{base}.{field}"), "applies to operation type 0202 only")),
            _ => {}
        }
    }
    check_formats(&base, lodging)
}

fn check_formats(base: &str, lodging: &Lodging) -> Result<(), ModelError> {
    let path = |field: &str| format!("{base}.{field}");
    let bad = |field: &str, expected: &str, value: &str| {
        Err(ModelError::rule("4280", path(field), format!("expected {expected}, got {value:?}")))
    };
    if let Some(kind) = &lodging.guest_document_type {
        if !IDENTITY_DOCUMENTS.contains(&kind.as_str()) || kind == "-" {
            return bad("guest_document_type", "a catalog 06 code", kind);
        }
    }
    for (field, value) in [("passport_country", &lodging.passport_country), ("residence_country", &lodging.residence_country)] {
        if let Some(country) = value.as_deref().filter(|c| !is_country(c)) {
            return bad(field, "a catalog 04 country", country);
        }
    }
    if let Some(name) = lodging.guest_name.as_deref().filter(|n| !(3..=200).contains(&n.chars().count())) {
        return bad("guest_name", "3 to 200 characters", name);
    }
    if let Some(number) = lodging.guest_document_number.as_deref().filter(|n| !(3..=20).contains(&n.chars().count())) {
        return bad("guest_document_number", "3 to 20 characters", number);
    }
    check_dates(base, lodging)?;
    if let Some(days) = lodging.stay_days.as_deref().filter(|d| !(1..=4).contains(&d.len()) || !d.bytes().all(|b| b.is_ascii_digit())) {
        return Err(ModelError::rule("4281", path("stay_days"), format!("expected 1 to 4 digits, got {days:?}")));
    }
    Ok(())
}

fn check_dates(base: &str, lodging: &Lodging) -> Result<(), ModelError> {
    for (field, get) in DATES {
        if let Some(text) = get(lodging) {
            text.parse::<Date>().map_err(|_| ModelError::invalid(format!("{base}.{field}"), format!("expected YYYY-MM-DD, got {text:?}")))?;
        }
    }
    let (Some(check_in), Some(check_out)) = (&lodging.check_in_date, &lodging.check_out_date) else { return Ok(()) };
    if check_out.as_str() < check_in.as_str() {
        let reason = format!("check-out {check_out} is before check-in {check_in}");
        return Err(ModelError::rule("4282", format!("{base}.check_out_date"), reason));
    }
    Ok(())
}
