//! Checks on the raw JSON before it is typed: unsupported features get a named error instead
//! of a generic "unknown field", and no string may hold a character XML 1.0 forbids.

use serde_json::Value;

use crate::ModelError;

const DOCUMENT_KEYS: &[(&str, &str)] = &[
    ("isc", "ISC"),
    ("icbper", "ICBPER"),
    ("ivap", "IVAP"),
    ("other_taxes", "other taxes"),
    ("discounts", "global discounts"),
    ("global_discounts", "global discounts"),
    ("charges", "global charges"),
    ("global_charges", "global charges"),
    ("advances", "advances"),
    ("prepayments", "advances"),
    ("perception", "perception"),
];

const TOTALS_KEYS: &[(&str, &str)] = &[
    ("isc", "ISC"),
    ("isc_base", "ISC"),
    ("icbper", "ICBPER"),
    ("other_taxes", "other taxes"),
    ("other_taxes_base", "other taxes"),
    ("discounts", "global discounts"),
    ("global_discount", "global discounts"),
    ("other_discounts", "global discounts"),
    ("charges", "global charges"),
    ("other_charges", "global charges"),
    ("advances", "advances"),
    ("prepaid", "advances"),
];

const LINE_KEYS: &[(&str, &str)] = &[
    ("isc", "ISC"),
    ("isc_base", "ISC"),
    ("isc_rate", "ISC"),
    ("isc_system", "ISC"),
    ("icbper", "ICBPER"),
    ("icbper_factor", "ICBPER"),
    ("other_tax", "other taxes"),
    ("other_tax_base", "other taxes"),
    ("other_tax_rate", "other taxes"),
];

pub(crate) fn check(root: &Value) -> Result<(), ModelError> {
    check_text(root, "$")?;
    reject_keys(root, "$", DOCUMENT_KEYS)?;
    if let Some(totals) = root.get("totals") {
        reject_keys(totals, "$.totals", TOTALS_KEYS)?;
    }
    if let Some(Value::Array(lines)) = root.get("lines") {
        for (index, line) in lines.iter().enumerate() {
            reject_keys(line, &format!("$.lines[{index}]"), LINE_KEYS)?;
        }
    }
    Ok(())
}

pub(crate) fn reject_keys(value: &Value, path: &str, keys: &[(&str, &'static str)]) -> Result<(), ModelError> {
    let Value::Object(map) = value else { return Ok(()) };
    for (key, feature) in keys {
        if map.contains_key(*key) {
            return Err(ModelError::OutOfScope { path: format!("{path}.{key}"), feature });
        }
    }
    Ok(())
}

pub(crate) fn check_text(value: &Value, path: &str) -> Result<(), ModelError> {
    match value {
        Value::String(text) if !is_xml_text(text) => {
            Err(ModelError::invalid(path, "contains a character XML 1.0 does not allow"))
        }
        Value::Array(items) => items
            .iter()
            .enumerate()
            .try_for_each(|(index, item)| check_text(item, &format!("{path}[{index}]"))),
        Value::Object(map) => map.iter().try_for_each(|(key, item)| check_text(item, &format!("{path}.{key}"))),
        _ => Ok(()),
    }
}

/// XML 1.0 `Char`: tab, newline, carriage return and everything from U+0020 except U+FFFE/U+FFFF.
pub fn is_xml_text(text: &str) -> bool {
    text.chars().all(|c| matches!(c, '\t' | '\n' | '\r') || (c >= ' ' && c != '\u{FFFE}' && c != '\u{FFFF}'))
}
