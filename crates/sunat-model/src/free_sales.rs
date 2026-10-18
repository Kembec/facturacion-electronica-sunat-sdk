//! Free sales in a daily summary (RC), by the date of its `reference_date`: until 2026-12-31 catalog 11
//! code 05 (`free`); from 2027-01-01 codes 06 to 09 (`free_taxed`, `free_exempt`, `free_unaffected`,
//! `free_export`) and the IGV of free sales (`free_igv`, tax 9996, field 25), and 05 is repealed
//! (RS 048-2026, annexes IX and X; dates in `rule_dates`). SUNAT beta on 16/10/2026 still answered 2256
//! to code 06, which is the old rule.

use crate::rule_dates::{applies, from, FREE_SALES_06_09};
use crate::{Decimal, ModelError, Summary, SummaryLine};

/// `0.00` is the same as absent: the XML does not render it.
fn given(amount: &Option<Decimal>) -> bool {
    amount.as_ref().is_some_and(|a| !a.is_zero())
}

pub(crate) fn check(index: usize, line: &SummaryLine, summary: &Summary) -> Result<(), ModelError> {
    let path = |field: &str| format!("$.lines[{index}].{field}");
    let date = summary.reference_date.date_string();
    let since = from(FREE_SALES_06_09);
    if applies(FREE_SALES_06_09, &date) {
        if given(&line.free) {
            let reason = format!("catalog 11 code 05 is repealed from {since}: split it in free_taxed, free_exempt, free_unaffected and free_export (codes 06 to 09)");
            return Err(ModelError::invalid(path("free"), reason));
        }
        return Ok(());
    }
    let new_codes = [
        ("free_taxed", "06", &line.free_taxed),
        ("free_exempt", "07", &line.free_exempt),
        ("free_unaffected", "08", &line.free_unaffected),
        ("free_export", "09", &line.free_export),
    ];
    for (field, code, amount) in new_codes {
        if given(amount) {
            let reason = format!("catalog 11 code {code} exists from {since}, and reference_date is {date}: use free (code 05) until then");
            return Err(ModelError::rule("2256", path(field), reason));
        }
    }
    if given(&line.free_igv) {
        let reason = format!("the IGV of free sales (tax 9996) is reported from {since}, and reference_date is {date}");
        return Err(ModelError::invalid(path("free_igv"), reason));
    }
    Ok(())
}
