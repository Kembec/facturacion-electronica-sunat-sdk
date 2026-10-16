//! SUNAT's validation rules for the sender's despatch advice that can be checked on the document
//! itself (sheet «Guía-Remitente2_0» of SUNAT's validation rules, type XSL): header, parties, related
//! documents and lines. The shipment is in `despatch_transport`. Each rejection carries the error code
//! SUNAT would answer; observations (codes 4000 and above) are not enforced.

use crate::despatch_transport::{check_shipment, Reason};
use crate::{Decimal, Despatch, DespatchLine, DespatchParty, ModelError, RelatedDocument};

/// Catalog 06 codes.
pub(crate) const IDENTITY_TYPES: &[&str] = &["0", "1", "4", "6", "7", "A", "B", "C", "D", "E", "F", "G"];

/// Catalog 61 codes a sender's despatch advice may relate to.
const RELATED_TYPES: &[&str] =
    &["01", "03", "04", "09", "12", "48", "49", "50", "52", "71", "72", "73", "74", "75", "76", "77", "78", "80", "81"];

/// Related documents whose issuer's RUC is required (3380).
const RELATED_WITH_ISSUER: &[&str] = &["01", "03", "04", "09", "12", "48"];

pub(crate) fn check(despatch: &Despatch) -> Result<(), ModelError> {
    check_header(despatch)?;
    let reason = check_shipment(despatch)?;
    check_recipient(despatch, reason)?;
    check_seller(despatch, reason)?;
    check_buyer(despatch, reason)?;
    check_related(despatch, reason)?;
    check_lines(&despatch.lines)
}

fn check_header(d: &Despatch) -> Result<(), ModelError> {
    rule(d.ubl_version == "2.1", "2110", "$.ubl_version", "must be \"2.1\"")?;
    rule(d.document_type == "09", "1051", "$.document_type", "must be \"09\"")?;
    rule(series(&d.series), "1001", "$.series", "must be T and 3 uppercase letters or digits")?;
    rule(digits_between(&d.number, 1, 8), "1001", "$.number", "must be 1 to 8 digits")?;
    let ruc = &d.issuer.ruc;
    if !(ruc.len() == 11 && digits(ruc)) {
        return Err(ModelError::invalid("$.issuer.ruc", format!("expected 11 digits, got {ruc:?}")));
    }
    rule(!blank(&d.issuer.legal_name), "1037", "$.issuer.legal_name", "required")
}

fn check_recipient(d: &Despatch, reason: Reason) -> Result<(), ModelError> {
    let r = &d.recipient;
    let at = |field: &str| format!("$.recipient.{field}");
    rule(!r.document_type.is_empty(), "2759", at("document_type"), "required")?;
    rule(IDENTITY_TYPES.contains(&r.document_type.as_str()), "2760", at("document_type"), "not a catalog 06 code")?;
    if reason.is(&["06", "17"]) {
        rule(r.document_type == "6", "3417", at("document_type"), "must be 6 (RUC) for reasons 06 and 17")?;
    }
    rule(!r.document_number.is_empty(), "2757", at("document_number"), "required")?;
    rule(identity_number(&r.document_type, &r.document_number), "2758", at("document_number"), identity_format(&r.document_type))?;
    rule(!blank(&r.legal_name), "2761", at("legal_name"), "required")?;
    let is_sender = is_issuer(d, r);
    if reason.is(&["01", "03", "05", "06", "09", "14", "17"]) {
        rule(!is_sender, "2555", at("document_number"), "must not be the sender for this reason")?;
    }
    if reason.is(&["02", "04", "07"]) {
        rule(is_sender, "2554", at("document_number"), "must be the sender for this reason")?;
    }
    Ok(())
}

/// The supplier: SUNAT checks it for reasons 02, 07 and 13; for other reasons it only observes it (4054).
fn check_seller(d: &Despatch, reason: Reason) -> Result<(), ModelError> {
    let Some(s) = &d.seller else { return Ok(()) };
    if !reason.is(&["02", "07", "13"]) {
        return Ok(());
    }
    let at = |field: &str| format!("$.seller.{field}");
    rule(!s.document_type.is_empty(), "2552", at("document_type"), "required")?;
    let allowed: &[&str] = match reason.0 {
        "02" => &["1", "4", "6", "7"],
        "07" => &["6"],
        _ => IDENTITY_TYPES,
    };
    rule(allowed.contains(&s.document_type.as_str()), "3447", at("document_type"), "not allowed for this reason")?;
    rule(!s.document_number.is_empty(), "2723", at("document_number"), "required")?;
    rule(identity_number(&s.document_type, &s.document_number), "2724", at("document_number"), identity_format(&s.document_type))?;
    rule(!blank(&s.legal_name), "3449", at("legal_name"), "required")?;
    if reason.is(&["02", "07"]) {
        rule(!is_issuer(d, s), "3448", at("document_number"), "must not be the sender")?;
    }
    Ok(())
}

/// The buyer: SUNAT checks it for reasons 03 and 13; for other reasons it only observes it (4377).
fn check_buyer(d: &Despatch, reason: Reason) -> Result<(), ModelError> {
    let Some(b) = &d.buyer else { return Ok(()) };
    if !reason.is(&["03", "13"]) {
        return Ok(());
    }
    let at = |field: &str| format!("$.buyer.{field}");
    rule(!b.document_type.is_empty(), "3331", at("document_type"), "required")?;
    rule(IDENTITY_TYPES.contains(&b.document_type.as_str()), "3332", at("document_type"), "not a catalog 06 code")?;
    rule(!b.document_number.is_empty(), "3333", at("document_number"), "required")?;
    rule(identity_number(&b.document_type, &b.document_number), "3337", at("document_number"), identity_format(&b.document_type))?;
    if reason.is(&["03"]) {
        rule(!blank(&b.legal_name), "3339", at("legal_name"), "required")?;
        rule(!is_issuer(d, b), "3334", at("document_number"), "must not be the sender")?;
        let same = b.document_type == d.recipient.document_type && b.document_number == d.recipient.document_number;
        rule(!same, "3335", at("document_number"), "must not be the recipient")?;
    }
    Ok(())
}

fn check_related(d: &Despatch, reason: Reason) -> Result<(), ModelError> {
    for (index, doc) in d.related_documents.iter().enumerate() {
        let at = |field: &str| format!("$.related_documents[{index}].{field}");
        let kind = doc.document_type.as_str();
        rule(!kind.is_empty(), "3376", at("document_type"), "required")?;
        rule(RELATED_TYPES.contains(&kind), "2692", at("document_type"), "not a catalog 61 code for a sender's despatch advice")?;
        rule(!doc.document_number.is_empty(), "3403", at("document_number"), "required")?;
        rule(related_number(kind, &doc.document_number), "3441", at("document_number"), "does not have the format of its type")?;
        if matches!(kind, "50" | "52") {
            rule(reason.is(&["13"]), "3445", at("document_type"), "a DAM or DS only goes with reasons 08, 09 and 13")?;
        }
        // 49 and 80 need an item with its tariff heading (concept 7020), which this SDK does not render.
        rule(!matches!(kind, "49" | "80"), "3352", at("document_type"), "needs an item with its tariff heading")?;
        let repeated = d.related_documents[..index]
            .iter()
            .any(|other| other.document_type == doc.document_type && other.document_number == doc.document_number);
        rule(!repeated, "3340", at("document_number"), "duplicated")?;
        check_related_issuer(d, reason, doc, &at("issuer_ruc"))?;
    }
    Ok(())
}

fn check_related_issuer(d: &Despatch, reason: Reason, doc: &RelatedDocument, path: &str) -> Result<(), ModelError> {
    let kind = doc.document_type.as_str();
    let issuer = doc.issuer_ruc.as_deref().unwrap_or("");
    if RELATED_WITH_ISSUER.contains(&kind) {
        rule(!issuer.is_empty(), "3380", path, "required for this document type")?;
    }
    if issuer.is_empty() {
        return Ok(());
    }
    rule(issuer.len() == 11 && digits(issuer), "3409", path, "must be 11 digits")?;
    let sender = issuer == d.issuer.ruc;
    let must_be_sender = (reason.is(&["01", "03"]) && matches!(kind, "01" | "03" | "12"))
        || kind == "09"
        || (reason.is(&["02"]) && matches!(kind, "04" | "48"));
    rule(!must_be_sender || sender, "3381", path, "must be the sender's RUC")?;
    if reason.is(&["06"]) && matches!(kind, "01" | "03" | "12") {
        let recipient = issuer == d.recipient.document_number;
        rule(recipient, "3381", path, "must be the recipient's RUC for reason 06")?;
    }
    Ok(())
}

fn check_lines(lines: &[DespatchLine]) -> Result<(), ModelError> {
    if lines.is_empty() {
        return Err(ModelError::missing("$.lines"));
    }
    rule(lines.len() <= 9999, "2023", "$.lines", "at most 9999 items")?;
    for (index, line) in lines.iter().enumerate() {
        let at = |field: &str| format!("$.lines[{index}].{field}");
        let Some(quantity) = &line.quantity else { return Err(ModelError::rule("2580", at("quantity"), "required")) };
        rule(positive_fits(quantity, 12, 10), "2780", at("quantity"), "a positive quantity with up to 12 integer digits and 10 decimals")?;
        rule(!line.unit.is_empty(), "2883", at("unit"), "required")?;
        rule(!blank(&line.description), "2781", at("description"), "required")?;
        if let Some(code) = line.sunat_product_code.as_deref().filter(|c| !c.is_empty()) {
            let ok = digits_between(code, 1, 8) && !all_zeros(code);
            rule(ok, "3002", at("sunat_product_code"), "up to 8 digits, not only zeros")?;
        }
    }
    Ok(())
}

fn is_issuer(d: &Despatch, party: &DespatchParty) -> bool {
    party.document_type == "6" && party.document_number == d.issuer.ruc
}

/// The related document's number by its catalog 61 type (3441).
fn related_number(kind: &str, number: &str) -> bool {
    let split = number.split_once('-');
    let voucher = |letter: u8, electronic: &[&str]| {
        split.is_some_and(|(s, n)| {
            let valid_series = (s.len() == 4 && s.as_bytes()[0] == letter && alnum_upper(&s[1..]))
                || electronic.contains(&s)
                || digits_between(s, 1, 4);
            valid_series && digits_between(n, 1, 8) && !all_zeros(n)
        })
    };
    match kind {
        "01" => voucher(b'F', &["E001"]),
        "03" => voucher(b'B', &["EB01"]),
        "04" => voucher(b'L', &["E001"]),
        "09" => split.is_some_and(|(s, n)| {
            ((s.len() == 4 && s.starts_with('T') && alnum_upper(&s[1..])) || matches!(s, "EG07" | "EG02"))
                && digits_between(n, 1, 8)
                && !all_zeros(n)
        }),
        "12" => split.is_some_and(|(s, n)| (1..=20).contains(&s.len()) && (1..=20).contains(&n.len())),
        "48" => split.is_some_and(|(s, n)| digits_between(s, 1, 4) && digits_between(n, 1, 7) && !all_zeros(n)),
        "49" | "80" => digits_between(number, 1, 15) && !all_zeros(number),
        "81" => (1..=20).contains(&number.chars().count()),
        "50" | "52" => customs_declaration(number),
        _ => (1..=100).contains(&number.chars().count()) && !number.chars().any(char::is_whitespace),
    }
}

/// `[0-9]{3}-[0-9]{4}-[0-9]{2}-[0-9]{1,6}`: customs office, year, regime, correlative.
fn customs_declaration(number: &str) -> bool {
    let parts: Vec<&str> = number.split('-').collect();
    parts.len() == 4
        && digits_between(parts[0], 3, 3)
        && digits_between(parts[1], 4, 4)
        && digits_between(parts[2], 2, 2)
        && digits_between(parts[3], 1, 6)
}

/// The number of a catalog 06 document: a RUC has 11 digits, a DNI 8, the rest up to 15 characters
/// without whitespace.
pub(crate) fn identity_number(kind: &str, number: &str) -> bool {
    match kind {
        "6" => number.len() == 11 && digits(number),
        "1" => number.len() == 8 && digits(number),
        _ => (1..=15).contains(&number.chars().count()) && !number.chars().any(char::is_whitespace),
    }
}

pub(crate) fn identity_format(kind: &str) -> &'static str {
    match kind {
        "6" => "a RUC has 11 digits",
        "1" => "a DNI has 8 digits",
        _ => "up to 15 characters without whitespace",
    }
}

/// `T` followed by 3 uppercase letters or digits.
fn series(text: &str) -> bool {
    text.len() == 4 && text.starts_with('T') && alnum_upper(&text[1..])
}

pub(crate) fn alnum_upper(text: &str) -> bool {
    text.bytes().all(|b| b.is_ascii_uppercase() || b.is_ascii_digit())
}

pub(crate) fn digits(text: &str) -> bool {
    text.bytes().all(|b| b.is_ascii_digit())
}

pub(crate) fn digits_between(text: &str, min: usize, max: usize) -> bool {
    (min..=max).contains(&text.len()) && digits(text)
}

pub(crate) fn all_zeros(text: &str) -> bool {
    text.bytes().all(|b| b == b'0')
}

/// Empty or only spaces.
pub(crate) fn blank(text: &str) -> bool {
    text.trim().is_empty()
}

/// Positive, with at most `int_max` integer digits and `frac_max` decimals.
pub(crate) fn positive_fits(value: &Decimal, int_max: usize, frac_max: usize) -> bool {
    let text = value.as_str();
    let (int, frac) = text.split_once('.').unwrap_or((text, ""));
    !value.is_negative() && !value.is_zero() && int.trim_start_matches('0').len() <= int_max && frac.len() <= frac_max
}

/// Rejects with SUNAT's `code` unless `ok`.
pub(crate) fn rule(ok: bool, code: &'static str, path: impl Into<String>, reason: &str) -> Result<(), ModelError> {
    if ok {
        return Ok(());
    }
    Err(ModelError::rule(code, path, reason))
}
