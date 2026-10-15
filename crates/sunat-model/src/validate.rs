use crate::{Document, DocumentType, Line, ModelError, Voucher};

/// Catalog 07 codes phase 1 renders. 17 (IVAP) is excluded: it needs IVAP totals.
const AFFECTATIONS: &[&str] = &[
    "10", "11", "12", "13", "14", "15", "16", "20", "21", "30", "31", "32", "33", "34", "35", "36", "37", "40",
];

impl Document {
    /// Checks the scope and the identifiers phase 1 relies on. `from_json` runs it; callers that
    /// build a `Document` in code get the same checks from `sunat_ubl::build`.
    pub fn validate(&self) -> Result<(), ModelError> {
        match self {
            Document::Invoice(invoice) => {
                check_voucher(&invoice.voucher, &[DocumentType::Invoice, DocumentType::Receipt])?;
                require_text("$.operation_type", &invoice.operation_type)?;
                require(invoice.voucher.totals.sale_value.is_some(), "$.totals.sale_value")
            }
            Document::Note(note) => {
                check_voucher(&note.voucher, &[DocumentType::CreditNote, DocumentType::DebitNote])?;
                let reference = &note.reference;
                require_text("$.note.reason_code", &reference.reason_code)?;
                require_text("$.note.reason", &reference.reason)?;
                require_text("$.note.affected_document_type", &reference.affected_document_type)?;
                require_text("$.note.affected_document_number", &reference.affected_document_number)
            }
        }
    }
}

fn check_voucher(voucher: &Voucher, allowed: &[DocumentType]) -> Result<(), ModelError> {
    if voucher.ubl_version != "2.1" {
        return Err(ModelError::invalid("$.ubl_version", "only \"2.1\" is supported"));
    }
    if !allowed.contains(&voucher.document_type) {
        return Err(ModelError::invalid("$.document_type", "does not match kind"));
    }
    check_format("$.issuer.ruc", &voucher.issuer.ruc, "11 digits", |s| s.len() == 11 && digits(s))?;
    check_format("$.series", &voucher.series, "4 uppercase letters or digits", |s| {
        s.len() == 4 && s.bytes().all(|b| b.is_ascii_uppercase() || b.is_ascii_digit())
    })?;
    check_format("$.number", &voucher.number, "1 to 8 digits", |s| (1..=8).contains(&s.len()) && digits(s))?;
    check_format("$.currency", &voucher.currency, "ISO 4217 code", |s| {
        s.len() == 3 && s.bytes().all(|b| b.is_ascii_uppercase())
    })?;
    require_text("$.issuer.legal_name", &voucher.issuer.legal_name)?;
    require_text("$.customer.document_type", &voucher.customer.document_type)?;
    require_text("$.customer.document_number", &voucher.customer.document_number)?;
    require(voucher.totals.total_taxes.is_some(), "$.totals.total_taxes")?;
    require(voucher.totals.payable.is_some(), "$.totals.payable")?;
    require(!voucher.lines.is_empty(), "$.lines")?;
    voucher.lines.iter().enumerate().try_for_each(|(index, line)| check_line(index, line))
}

fn check_line(index: usize, line: &Line) -> Result<(), ModelError> {
    let path = format!("$.lines[{index}].affectation");
    if line.affectation == "17" {
        return Err(ModelError::OutOfScope { path, feature: "IVAP (affectation 17)" });
    }
    if !AFFECTATIONS.contains(&line.affectation.as_str()) {
        return Err(ModelError::invalid(path, "not a catalog 07 code"));
    }
    require_text(&format!("$.lines[{index}].unit"), &line.unit)?;
    require_text(&format!("$.lines[{index}].description"), &line.description)
}

fn digits(text: &str) -> bool {
    text.bytes().all(|b| b.is_ascii_digit())
}

fn check_format(path: &str, value: &str, expected: &str, ok: impl Fn(&str) -> bool) -> Result<(), ModelError> {
    if ok(value) {
        return Ok(());
    }
    Err(ModelError::invalid(path, format!("expected {expected}, got {value:?}")))
}

fn require(present: bool, path: &str) -> Result<(), ModelError> {
    if present {
        return Ok(());
    }
    Err(ModelError::missing(path))
}

fn require_text(path: &str, value: &str) -> Result<(), ModelError> {
    require(!value.is_empty(), path)
}
