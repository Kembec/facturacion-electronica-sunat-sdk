use crate::{Company, Document, DocumentType, Line, ModelError, Summary, SummaryLine, Voided, Voucher};

/// Catalog 07 codes phase 1 renders. 17 (IVAP) is excluded: it needs IVAP totals.
const AFFECTATIONS: &[&str] = &[
    "10", "11", "12", "13", "14", "15", "16", "20", "21", "30", "31", "32", "33", "34", "35", "36", "37", "40",
];

impl Document {
    /// Checks the scope and the identifiers the XML relies on. `from_json` runs it; callers that
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
            Document::Summary(summary) => check_summary(summary),
            Document::Voided(voided) => check_voided(voided),
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
    check_issuer(&voucher.issuer)?;
    check_format("$.series", &voucher.series, "4 uppercase letters or digits", series)?;
    check_format("$.number", &voucher.number, "1 to 8 digits", voucher_number)?;
    check_format("$.currency", &voucher.currency, "ISO 4217 code", currency)?;
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

fn check_summary(summary: &Summary) -> Result<(), ModelError> {
    check_issuer(&summary.issuer)?;
    check_format("$.number", &summary.number, "1 to 5 digits", correlative)?;
    check_format("$.currency", &summary.currency, "ISO 4217 code", currency)?;
    if summary.reference_date > summary.generation_date {
        return Err(ModelError::invalid("$.reference_date", "must not be after generation_date"));
    }
    require(!summary.lines.is_empty(), "$.lines")?;
    summary.lines.iter().enumerate().try_for_each(|(index, line)| check_summary_line(index, line))
}

fn check_summary_line(index: usize, line: &SummaryLine) -> Result<(), ModelError> {
    let path = |field: &str| format!("$.lines[{index}].{field}");
    let note = match line.document_type.as_str() {
        "03" => false,
        "07" | "08" => true,
        _ => return Err(ModelError::invalid(path("document_type"), "expected \"03\", \"07\" or \"08\"")),
    };
    check_format(&path("document_number"), &line.document_number, "SERIES-NUMBER", document_number)?;
    check_format(&path("status"), &line.status, "catalog 19: \"1\", \"2\" or \"3\"", |s| matches!(s, "1" | "2" | "3"))?;
    require_text(&path("customer_document_type"), &line.customer_document_type)?;
    require_text(&path("customer_document_number"), &line.customer_document_number)?;
    match (&line.reference_document, note) {
        (Some(_), false) => Err(ModelError::invalid(path("reference_document"), "applies to notes (07, 08) only")),
        (None, true) => Err(ModelError::missing(path("reference_document"))),
        (Some(reference), true) => {
            require_text(&path("reference_document.document_type"), &reference.document_type)?;
            let number = path("reference_document.document_number");
            check_format(&number, &reference.document_number, "SERIES-NUMBER", document_number)
        }
        (None, false) => Ok(()),
    }
}

fn check_voided(voided: &Voided) -> Result<(), ModelError> {
    check_issuer(&voided.issuer)?;
    check_format("$.number", &voided.number, "1 to 5 digits", correlative)?;
    if voided.generation_date > voided.communication_date {
        return Err(ModelError::invalid("$.generation_date", "must not be after communication_date"));
    }
    require(!voided.lines.is_empty(), "$.lines")?;
    voided.lines.iter().enumerate().try_for_each(|(index, line)| {
        let path = |field: &str| format!("$.lines[{index}].{field}");
        // Receipts are voided in a daily summary with status 3, not here.
        check_format(&path("document_type"), &line.document_type, "\"01\", \"07\" or \"08\"", |s| {
            matches!(s, "01" | "07" | "08")
        })?;
        check_format(&path("series"), &line.series, "4 uppercase letters or digits", series)?;
        check_format(&path("number"), &line.number, "1 to 8 digits", voucher_number)?;
        require_text(&path("reason"), &line.reason)
    })
}

fn check_issuer(issuer: &Company) -> Result<(), ModelError> {
    check_format("$.issuer.ruc", &issuer.ruc, "11 digits", |s| s.len() == 11 && digits(s))?;
    require_text("$.issuer.legal_name", &issuer.legal_name)
}

fn digits(text: &str) -> bool {
    text.bytes().all(|b| b.is_ascii_digit())
}

fn series(text: &str) -> bool {
    text.len() == 4 && text.bytes().all(|b| b.is_ascii_uppercase() || b.is_ascii_digit())
}

fn voucher_number(text: &str) -> bool {
    (1..=8).contains(&text.len()) && digits(text)
}

/// `SERIES-NUMBER`, e.g. `B001-1`.
fn document_number(text: &str) -> bool {
    text.split_once('-').is_some_and(|(s, n)| series(s) && voucher_number(n))
}

/// Correlative of the day in `RC-YYYYMMDD-N` and `RA-YYYYMMDD-N`.
fn correlative(text: &str) -> bool {
    (1..=5).contains(&text.len()) && digits(text)
}

fn currency(text: &str) -> bool {
    text.len() == 3 && text.bytes().all(|b| b.is_ascii_uppercase())
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
