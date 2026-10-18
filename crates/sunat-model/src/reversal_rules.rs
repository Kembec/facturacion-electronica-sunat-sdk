//! SUNAT's validation rules for the reversal summary (RR) of a purchase settlement (04) that can be checked
//! on the document itself (sheet «ResumenReversiones1_0» of SUNAT's validation rules, 24/04/2026). Whether
//! the settlement exists, was accepted, is not reversed yet and has no payments registered (2750, 2398,
//! 2751, 2471) only SUNAT knows.

use std::collections::HashSet;

use crate::despatch_rules::{alnum_upper, blank, digits, digits_between, rule};
use crate::{ModelError, Voided};

/// Days SUNAT allows from the settlement's accepted CDR to the reception of its reversal (sheet RR, ERR-2958,
/// from 01/08/2026; RS 244-2019 art. 3.2 says seven).
const REVERSAL_DAYS: i64 = 5;

/// The settlement must reach SUNAT the day it is issued or the next one (2108), so its CDR may be one day
/// after `generation_date`. Rejecting at `REVERSAL_DAYS` from the issue date would refuse a reversal SUNAT
/// accepts; the SDK rejects only what SUNAT rejects in every case.
const CDR_LAG_DAYS: i64 = 1;

pub(crate) fn check(r: &Voided) -> Result<(), ModelError> {
    if !(r.issuer.ruc.len() == 11 && digits(&r.issuer.ruc)) {
        return Err(ModelError::invalid("$.issuer.ruc", format!("expected 11 digits, got {:?}", r.issuer.ruc)));
    }
    if blank(&r.issuer.legal_name) {
        return Err(ModelError::missing("$.issuer.legal_name"));
    }
    rule(digits_between(&r.number, 1, 5), "2220", "$.number", "must be 1 to 5 digits (RR-YYYYMMDD-N)")?;
    let elapsed = r.communication_date.days_after(&r.generation_date);
    rule(elapsed >= 0, "2671", "$.generation_date", "must not be after communication_date")?;
    let reason = "a purchase settlement is reversed within 5 days of its CDR (SUNAT validation sheet RR, from \
                  01/08/2026; RS 244-2019 art. 3.2 says 7): communication_date is more than 6 days after generation_date";
    rule(elapsed <= REVERSAL_DAYS + CDR_LAG_DAYS, "2958", "$.communication_date", reason)?;
    if r.lines.is_empty() {
        return Err(ModelError::missing("$.lines"));
    }
    let mut seen = HashSet::new();
    r.lines.iter().enumerate().try_for_each(|(index, line)| {
        let path = |field: &str| format!("$.lines[{index}].{field}");
        rule(line.document_type == "04", "2308", path("document_type"), "must be \"04\" (purchase settlement)")?;
        rule(series(&line.series), "2345", path("series"), "must be L and 3 uppercase letters or digits")?;
        rule(digits_between(&line.number, 1, 8), "2312", path("number"), "must be 1 to 8 digits")?;
        let number = line.number.trim_start_matches('0');
        rule(seen.insert((line.series.as_str(), number)), "2348", path("number"), "the settlement is already in this summary")?;
        rule(!blank(&line.reason), "2315", path("reason"), "required")?;
        rule(line.reason.trim().chars().count() >= 3, "4203", path("reason"), "must be at least 3 characters")
    })
}

fn series(text: &str) -> bool {
    text.len() == 4 && text.starts_with('L') && alnum_upper(&text[1..])
}
