//! The JSON document as written, before it is split into `Invoice` or `Note`; summaries and void
//! communications go to `daily`.

use serde::Deserialize;
use serde_json::Value;

use crate::{
    Client, Company, DateTime, DespatchReference, Document, DocumentType, Invoice, Legend, Line, ModelError, Note, NoteReference,
    PaymentTerms, Totals, Voucher,
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawDocument {
    kind: String,
    ubl_version: String,
    operation_type: Option<String>,
    document_type: String,
    series: String,
    number: String,
    issue_date: DateTime,
    due_date: Option<DateTime>,
    currency: String,
    payment_terms: Option<PaymentTerms>,
    issuer: Company,
    customer: Client,
    totals: Totals,
    lines: Vec<Line>,
    #[serde(default)]
    legends: Vec<Legend>,
    note: Option<NoteReference>,
    order_reference: Option<String>,
    #[serde(default)]
    despatch_references: Vec<DespatchReference>,
}

pub(crate) fn parse(json: &str) -> Result<Document, ModelError> {
    let value: Value = serde_json::from_str(json).map_err(|e| ModelError::Json(e.to_string()))?;
    if let Some(kind @ ("summary" | "voided")) = value.get("kind").and_then(Value::as_str) {
        let kind = kind.to_owned();
        return crate::daily::parse(&kind, value);
    }
    crate::scope::check(&value)?;
    let raw: RawDocument = serde_json::from_value(value).map_err(|e| ModelError::Json(e.to_string()))?;
    raw.into_document()
}

impl RawDocument {
    fn into_document(self) -> Result<Document, ModelError> {
        let document_type = DocumentType::from_code(&self.document_type)
            .ok_or_else(|| ModelError::invalid("$.document_type", "expected \"01\", \"03\", \"07\" or \"08\""))?;
        let voucher = Voucher {
            ubl_version: self.ubl_version,
            document_type,
            series: self.series,
            number: self.number,
            issue_date: self.issue_date,
            currency: self.currency,
            payment_terms: self.payment_terms,
            issuer: self.issuer,
            customer: self.customer,
            totals: self.totals,
            lines: self.lines,
            legends: self.legends,
            order_reference: self.order_reference,
            despatch_references: self.despatch_references,
        };
        match self.kind.as_str() {
            "invoice" => {
                reject(self.note.is_some(), "$.note", "applies to notes (07, 08) only")?;
                let operation_type = self.operation_type.ok_or_else(|| ModelError::missing("$.operation_type"))?;
                Ok(Document::Invoice(Invoice { voucher, operation_type, due_date: self.due_date }))
            }
            "note" => {
                reject(self.operation_type.is_some(), "$.operation_type", "applies to invoices only")?;
                reject(self.due_date.is_some(), "$.due_date", "applies to invoices only")?;
                let reference = self.note.ok_or_else(|| ModelError::missing("$.note"))?;
                Ok(Document::Note(Note { voucher, reference }))
            }
            _ => Err(ModelError::invalid("$.kind", "expected \"invoice\", \"note\", \"summary\" or \"voided\"")),
        }
    }
}

fn reject(present: bool, path: &str, reason: &str) -> Result<(), ModelError> {
    if present {
        return Err(ModelError::invalid(path, reason));
    }
    Ok(())
}
