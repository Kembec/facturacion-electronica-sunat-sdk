//! Rice under the IVAP (affectation 17). SUNAT beta on 16/10/2026 accepted invoices, receipts and
//! notes whose lines all take 17, with the IVAP totals as `TaxSubtotal` 1016, and rejected notes with
//! any type other than 12, «Ajustes afectos al IVAP» (3230).

use crate::{ModelError, Voucher};

const IVAP: &str = "17";

/// Every line under the IVAP or none (2644), and the IVAP totals exactly when the lines carry it.
pub(crate) fn check(voucher: &Voucher) -> Result<(), ModelError> {
    let totals = &voucher.totals;
    let has_totals = totals.ivap_base.is_some() || totals.ivap.is_some();
    if !uses_ivap(voucher) {
        if has_totals {
            return Err(ModelError::invalid("$.totals.ivap_base", "applies only with lines of affectation 17"));
        }
        return Ok(());
    }
    if let Some(index) = voucher.lines.iter().position(|line| line.affectation != IVAP) {
        let path = format!("$.lines[{index}].affectation");
        return Err(ModelError::invalid(path, "\"17\" like the other lines: an IVAP document takes no other affectation (SUNAT 2644)"));
    }
    if totals.ivap_base.is_none() {
        return Err(ModelError::missing("$.totals.ivap_base"));
    }
    if totals.ivap.is_none() {
        return Err(ModelError::missing("$.totals.ivap"));
    }
    Ok(())
}

/// A credit or debit note on IVAP lines takes type 12 in catalogs 09 and 10 (3230).
pub(crate) fn check_note(voucher: &Voucher, reason_code: &str) -> Result<(), ModelError> {
    if uses_ivap(voucher) && reason_code != "12" {
        return Err(ModelError::invalid("$.note.reason_code", "\"12\" (Ajustes afectos al IVAP) when the lines take affectation 17 (SUNAT 3230)"));
    }
    Ok(())
}

fn uses_ivap(voucher: &Voucher) -> bool {
    voucher.lines.iter().any(|line| line.affectation == IVAP)
}
