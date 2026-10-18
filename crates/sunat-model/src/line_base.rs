//! The IGV base of a line against its value (SUNAT 3272 and 4294): without ISC, which the SDK does
//! not model, `cbc:TaxableAmount` of the line equals `cbc:LineExtensionAmount` to the cent, in every
//! operation type. An error (3272) in an invoice and in a note that modifies one; an observation
//! (4294) in a receipt and in a note that modifies one, rejected here like the errors: SUNAT accepts
//! the document, but with a base that does not add up.

use crate::allowances::{cents, show_cents};
use crate::retention_lines::units;
use crate::{DocumentType, ModelError, Voucher};

/// `affected` is the document type a note modifies; `None` for an invoice or a receipt.
pub(crate) fn check(voucher: &Voucher, affected: Option<&str>) -> Result<(), ModelError> {
    let receipt = match affected {
        Some(kind) => kind == "03",
        None => voucher.document_type == DocumentType::Receipt,
    };
    let code = if receipt { "4294" } else { "3272" };
    for (index, line) in voucher.lines.iter().enumerate() {
        let (Some(base), Some(value)) = (units(&line.igv_base), units(&line.line_value)) else { continue };
        if cents(base) != cents(value) {
            let reason = format!("must equal line_value ({}) without ISC", show_cents(cents(value)));
            return Err(ModelError::rule(code, format!("$.lines[{index}].igv_base"), reason));
        }
    }
    Ok(())
}
