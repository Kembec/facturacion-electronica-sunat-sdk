//! Which field of the input JSON a SUNAT rejection or observation points at.
//!
//! The table is `data/error-fields.json`, generated from SUNAT's validation rules («Reglas de
//! validación», updated 26.08.2026) by `data/generate-error-fields.py`: per code and document type, the
//! UBL node the rule checks and the input field that writes it (`docs/input-model.md`), `"sdk"` when
//! the SDK writes the node itself, or none when the input model has no such field.

use std::sync::OnceLock;

use serde_json::{json, Value};

const ERROR_FIELDS: &str = include_str!("../data/error-fields.json");

/// What SUNAT's message names, observed in beta as `(nodo: "cbc:ID/schemeID" valor: "1")`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Detail {
    /// The last element and, if any, its attribute, as SUNAT writes it.
    pub node: String,
    /// The value SUNAT read in that node.
    pub value: String,
}

/// One candidate field for a code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    /// The input path (`customer.document_type`, `lines[].unit_price`), `"sdk"` when the SDK writes
    /// the node, or `None` when the input model has no field for it.
    pub input_path: Option<String>,
    /// The node the rule checks; `None` for rules on the file, the signature or the service.
    pub xpath: Option<String>,
    /// `fuente` when the node is the rule's own and one field writes it; `inferido` otherwise.
    pub certainty: String,
}

/// A code SUNAT answered, with the fields that may have caused it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnosis {
    pub code: String,
    pub detail: Option<Detail>,
    /// The table's rows for the code and document type. When `detail` names a node, only the rows
    /// that end in it, unless none does: then all of them.
    pub fields: Vec<Field>,
}

impl Diagnosis {
    /// `{"code", "detail": {"node", "value"} | null, "fields": [{"input_path", "xpath", "certainty"}]}`.
    pub fn to_json(&self) -> Value {
        let detail = self.detail.as_ref().map(|d| json!({ "node": d.node, "value": d.value }));
        let fields: Vec<Value> = self
            .fields
            .iter()
            .map(|f| json!({ "input_path": f.input_path, "xpath": f.xpath, "certainty": f.certainty }))
            .collect();
        json!({ "code": self.code, "detail": detail, "fields": fields })
    }
}

struct Row {
    code: String,
    document_types: Vec<String>,
    field: Field,
}

/// Diagnoses `code` (`"2800"`, `"0109"`) with its `message`, for `document_type` (`"01"`, `"03"`,
/// `"07"`, `"08"`; any other type has no rows in the table).
pub fn diagnose(document_type: &str, code: &str, message: &str) -> Diagnosis {
    let code = code.trim();
    let key = table_code(code);
    let detail = detail(message);
    let candidates: Vec<&Row> = rows()
        .iter()
        .filter(|row| row.code == key && row.document_types.iter().any(|t| t == document_type))
        .collect();
    let on_node: Vec<&Row> = match &detail {
        Some(detail) => candidates.iter().copied().filter(|row| ends_in(row.field.xpath.as_deref(), &detail.node)).collect(),
        None => Vec::new(),
    };
    let chosen = if on_node.is_empty() { candidates } else { on_node };
    Diagnosis { code: code.to_string(), detail, fields: chosen.into_iter().map(|row| row.field.clone()).collect() }
}

/// The `diagnosis` array for an outcome of `exchange::read` or `exchange::cdr`: one element per code
/// in the answer — a fault's code, a CDR's `response_code` when it is not 0, and each CDR note that
/// starts with a code (`"4287 - ..."`). `None` when the outcome is neither a fault nor carries a CDR.
pub fn diagnosis_json(document_type: &str, outcome: &Value) -> Option<Value> {
    let mut found = Vec::new();
    if outcome["outcome"] == "fault" {
        found.push(diagnose(document_type, str_of(&outcome["code"]), str_of(&outcome["message"])));
    }
    let cdr = &outcome["cdr"];
    if outcome["outcome"] != "fault" && cdr.is_null() {
        return None;
    }
    let response_code = str_of(&cdr["response_code"]).trim();
    if !response_code.is_empty() && table_code(response_code) != "0000" {
        found.push(diagnose(document_type, response_code, str_of(&cdr["description"])));
    }
    for note in cdr["notes"].as_array().into_iter().flatten().filter_map(Value::as_str) {
        if let Some((code, message)) = coded_note(note) {
            found.push(diagnose(document_type, code, message));
        }
    }
    Some(Value::Array(found.iter().map(Diagnosis::to_json).collect()))
}

fn str_of(value: &Value) -> &str {
    value.as_str().unwrap_or_default()
}

/// `"4287 - El precio ..."` → `("4287", "El precio ...")`; `None` without a leading number.
fn coded_note(note: &str) -> Option<(&str, &str)> {
    let note = note.trim();
    let digits = note.find(|c: char| !c.is_ascii_digit()).unwrap_or(note.len());
    if digits == 0 {
        return None;
    }
    let rest = note[digits..].trim_start();
    let message = rest.strip_prefix('-').unwrap_or(rest).trim();
    Some((&note[..digits], message))
}

/// The table writes codes with four digits (`"0109"`).
fn table_code(code: &str) -> String {
    match code.parse::<u32>() {
        Ok(number) => format!("{number:04}"),
        Err(_) => code.to_string(),
    }
}

/// `(nodo: "<node>" valor: "<value>")` in the message, the form observed in beta.
fn detail(message: &str) -> Option<Detail> {
    let (node, rest) = quoted_after(message, "nodo:")?;
    let (value, _) = quoted_after(rest, "valor:")?;
    Some(Detail { node: node.to_string(), value: value.to_string() })
}

/// The text between the quotes that follow `label`, and what comes after them.
fn quoted_after<'a>(text: &'a str, label: &str) -> Option<(&'a str, &'a str)> {
    let after = text[text.find(label)? + label.len()..].trim_start().strip_prefix('"')?;
    let end = after.find('"')?;
    Some((&after[..end], &after[end + 1..]))
}

/// `cbc:ID@schemeID`, `cbc:ID/@schemeID` and `cbc:ID/schemeID` are the same node.
fn normalize(path: &str) -> String {
    path.replace("/@", "/").replace('@', "/")
}

fn ends_in(xpath: Option<&str>, node: &str) -> bool {
    let (Some(xpath), node) = (xpath.map(normalize), normalize(node.trim())) else {
        return false;
    };
    !node.is_empty() && (xpath == node || xpath.ends_with(&format!("/{node}")))
}

fn rows() -> &'static [Row] {
    static ROWS: OnceLock<Vec<Row>> = OnceLock::new();
    ROWS.get_or_init(|| {
        let table: Vec<Value> = serde_json::from_str(ERROR_FIELDS).expect("data/error-fields.json is a JSON array");
        table.iter().map(row).collect()
    })
}

fn row(value: &Value) -> Row {
    let text = |field: &str| value[field].as_str().map(str::to_string);
    Row {
        code: text("code").unwrap_or_default(),
        document_types: value["document_types"].as_array().into_iter().flatten().filter_map(Value::as_str).map(str::to_string).collect(),
        field: Field { input_path: text("input_path"), xpath: text("xpath"), certainty: text("certainty").unwrap_or_default() },
    }
}

#[cfg(test)]
#[path = "diagnosis_tests.rs"]
mod tests;
