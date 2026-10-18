use crate::{Company, Decimal, Detraction, Document, DocumentType, IgvRetention, Invoice, Line, ModelError, Summary, SummaryLine, Voided, Voucher};

/// Catalog 07 codes the SDK renders.
const AFFECTATIONS: &[&str] = &[
    "10", "11", "12", "13", "14", "15", "16", "17", "20", "21", "30", "31", "32", "33", "34", "35", "36", "37", "40",
];

impl Document {
    /// Checks the scope and the identifiers the XML relies on. `from_json` runs it; callers that
    /// build a `Document` in code get the same checks from `sunat_ubl::build`.
    pub fn validate(&self) -> Result<(), ModelError> {
        match self {
            Document::Invoice(invoice) => {
                check_voucher(&invoice.voucher, &[DocumentType::Invoice, DocumentType::Receipt])?;
                require_text("$.operation_type", &invoice.operation_type)?;
                check_detraction(invoice)?;
                require(invoice.voucher.totals.sale_value.is_some(), "$.totals.sale_value")?;
                crate::allowances::check(invoice)?;
                crate::icbper::check(&invoice.voucher, None)
            }
            Document::Note(note) => {
                check_voucher(&note.voucher, &[DocumentType::CreditNote, DocumentType::DebitNote])?;
                if let Some(index) = note.voucher.lines.iter().position(|line| !line.discounts.is_empty()) {
                    let path = format!("$.lines[{index}].discounts");
                    return Err(ModelError::OutOfScope { path, feature: "line discounts in credit and debit notes" });
                }
                if let Some(index) = note.voucher.lines.iter().position(|line| !line.charges.is_empty()) {
                    let path = format!("$.lines[{index}].charges");
                    return Err(ModelError::OutOfScope { path, feature: "line charges in credit and debit notes" });
                }
                let totals = &note.voucher.totals;
                if totals.prepaid.is_some() {
                    return Err(ModelError::OutOfScope { path: "$.totals.prepaid".into(), feature: "advances in credit and debit notes" });
                }
                if totals.allowance_total.is_some() || totals.charge_total.is_some() {
                    let feature = "discount and charge totals in credit and debit notes";
                    return Err(ModelError::OutOfScope { path: "$.totals".into(), feature });
                }
                let reference = &note.reference;
                crate::ivap::check_note(&note.voucher, &reference.reason_code)?;
                crate::penalty_notes::check(note)?;
                require_text("$.note.reason_code", &reference.reason_code)?;
                require_text("$.note.reason", &reference.reason)?;
                require_text("$.note.affected_document_type", &reference.affected_document_type)?;
                require_text("$.note.affected_document_number", &reference.affected_document_number)?;
                crate::icbper::check(&note.voucher, Some(&reference.affected_document_type))
            }
            Document::Summary(summary) => check_summary(summary),
            Document::Voided(voided) => check_voided(voided),
            Document::Despatch(despatch) => crate::despatch_rules::check(despatch),
            Document::Retention(retention) => crate::retention_rules::check(retention),
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
    crate::references::check(voucher.order_reference.as_deref(), &voucher.despatch_references)?;
    crate::ivap::check(voucher)?;
    if let Some(retention) = &voucher.igv_retention {
        check_retention(voucher, retention)?;
    }
    voucher.lines.iter().enumerate().try_for_each(|(index, line)| check_line(index, line))
}

fn check_line(index: usize, line: &Line) -> Result<(), ModelError> {
    let path = format!("$.lines[{index}].affectation");
    if !AFFECTATIONS.contains(&line.affectation.as_str()) {
        return Err(ModelError::invalid(path, "not a catalog 07 code"));
    }
    require_text(&format!("$.lines[{index}].unit"), &line.unit)?;
    require_text(&format!("$.lines[{index}].description"), &line.description)
}

/// What SUNAT rejects in the IGV retention without reading its registers: the formats (3025, 2968,
/// 3016) and a customer without RUC (3262). Whether the customer is a designated retention agent and
/// the issuer is not (3262, 3269) depends on SUNAT's register; the arithmetic (3263, 3264) stays
/// with the caller, like every other amount.
fn check_retention(voucher: &Voucher, retention: &IgvRetention) -> Result<(), ModelError> {
    if voucher.customer.document_type != "6" {
        return Err(ModelError::invalid("$.igv_retention", "requires a customer with RUC, document_type \"6\" (SUNAT 3262)"));
    }
    let positive = |field: &str, value: &Decimal, int_max: usize, frac_max: usize| {
        let ok = !value.is_negative() && !value.is_zero() && fits(value, int_max, frac_max);
        let expected = format!("a positive amount with up to {int_max} integer digits and {frac_max} decimals");
        check_format(&format!("$.igv_retention.{field}"), value.as_str(), &expected, |_| ok)
    };
    positive("factor", &retention.factor, 3, 5)?;
    positive("amount", &retention.amount, 12, 2)?;
    positive("base_amount", &retention.base_amount, 12, 2)
}

/// Catalog 51 operation types subject to detraction. SUNAT requires the detraction blocks with them
/// (3127, 3034) and rejects them with any other type (3128).
const DETRACTION_OPERATIONS: &[&str] = &["1001", "1002", "1003", "1004"];

/// What SUNAT rejects in the detraction blocks: the code each special operation type demands (3129),
/// the catalog formats (3033, 3174), the account (3034) and the amount (3037). The rate and the
/// amount are the caller's: SUNAT does not check them against each other.
fn check_detraction(invoice: &Invoice) -> Result<(), ModelError> {
    let operation = invoice.operation_type.as_str();
    let subject = DETRACTION_OPERATIONS.contains(&operation);
    let detraction = match (&invoice.detraction, subject) {
        (None, false) => return Ok(()),
        (None, true) => return Err(ModelError::missing("$.detraction")),
        (Some(_), false) => {
            return Err(ModelError::invalid("$.detraction", "applies only with operation_type 1001 to 1004 (SUNAT 3128)"));
        }
        (Some(detraction), true) => detraction,
    };
    let feature = match operation {
        "1002" => Some("detraction on hydrobiological resources (operation type 1002)"),
        "1004" => Some("detraction on freight transport (operation type 1004)"),
        _ => None,
    };
    if let Some(feature) = feature {
        return Err(ModelError::OutOfScope { path: "$.operation_type".into(), feature });
    }
    check_detraction_fields(detraction, operation)
}

fn check_detraction_fields(detraction: &Detraction, operation: &str) -> Result<(), ModelError> {
    let three_digits = |s: &str| s.len() == 3 && digits(s);
    check_format("$.detraction.code", &detraction.code, "a catalog 54 code, 3 digits", three_digits)?;
    if operation == "1003" {
        check_format("$.detraction.code", &detraction.code, "\"028\" for operation type 1003 (SUNAT 3129)", |s| s == "028")?;
    }
    let means = &detraction.payment_means_code;
    check_format("$.detraction.payment_means_code", means, "a catalog 59 code, 3 digits", three_digits)?;
    let account = &detraction.account;
    check_format("$.detraction.account", account, "1 to 100 characters", |s| (1..=100).contains(&s.chars().count()))?;
    let positive = |field: &str, value: &Decimal, int_max: usize, frac_max: usize| {
        let ok = !value.is_negative() && !value.is_zero() && fits(value, int_max, frac_max);
        let expected = format!("a positive amount with up to {int_max} integer digits and {frac_max} decimals");
        check_format(&format!("$.detraction.{field}"), value.as_str(), &expected, |_| ok)
    };
    positive("percent", &detraction.percent, 3, 5)?;
    positive("amount", &detraction.amount, 12, 2)
}

/// At most `int_max` integer digits and `frac_max` decimals, as rendered (no leading or trailing zeros).
pub(crate) fn fits(value: &Decimal, int_max: usize, frac_max: usize) -> bool {
    let unsigned = value.as_str().trim_start_matches('-');
    let (int, frac) = unsigned.split_once('.').unwrap_or((unsigned, ""));
    int.trim_start_matches('0').len() <= int_max && frac.trim_end_matches('0').len() <= frac_max
}

fn check_summary(summary: &Summary) -> Result<(), ModelError> {
    check_issuer(&summary.issuer)?;
    check_format("$.number", &summary.number, "1 to 5 digits", correlative)?;
    check_format("$.currency", &summary.currency, "ISO 4217 code", currency)?;
    if summary.reference_date > summary.generation_date {
        return Err(ModelError::invalid("$.reference_date", "must not be after generation_date"));
    }
    require(!summary.lines.is_empty(), "$.lines")?;
    summary.lines.iter().enumerate().try_for_each(|(index, line)| {
        check_summary_line(index, line)?;
        crate::free_sales::check(index, line, summary)?;
        crate::icbper::check_summary_line(index, line, &summary.currency)
    })
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
    if let Some(name) = &line.customer_name {
        check_format(&path("customer_name"), name, "3 to 250 characters, no line breaks or tabs (SUNAT 2022)", summary_customer_name)?;
    }
    if let Some(rate) = &line.igv_rate {
        check_summary_rate(&path("igv_rate"), rate, note)?;
    }
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

/// Field 10.3 of the daily summary: any character but whitespace other than the space, not blank.
fn summary_customer_name(text: &str) -> bool {
    let length = text.chars().count();
    (3..=250).contains(&length) && !text.trim().is_empty() && !text.chars().any(|c| c.is_whitespace() && c != ' ')
}

/// The IGV rates SUNAT accepts on a summary line: 18 or 10.5 for a receipt, and also 10 for a note
/// (its receipt may be from 2025). Errors 3102 (format) and 3504 (value).
fn check_summary_rate(path: &str, rate: &Decimal, note: bool) -> Result<(), ModelError> {
    let ok = !rate.is_negative() && fits(rate, 3, 5);
    check_format(path, rate.as_str(), "a positive rate with up to 3 integer digits and 5 decimals (SUNAT 3102)", |_| ok)?;
    let value = rate.to_f64();
    let allowed: &[f64] = if note { &[18.0, 10.5, 10.0] } else { &[18.0, 10.5] };
    let expected = if note { "18, 10.5 or 10 (SUNAT 3504)" } else { "18 or 10.5 for a receipt (SUNAT 3504)" };
    check_format(path, rate.as_str(), expected, |_| allowed.contains(&value))
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

pub(crate) fn check_format(path: &str, value: &str, expected: &str, ok: impl Fn(&str) -> bool) -> Result<(), ModelError> {
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
