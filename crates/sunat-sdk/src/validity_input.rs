//! The query's input checked and turned into the manual's body (§C): a query SUNAT would refuse
//! for its shape is refused here, before any network call.

use serde_json::{json, Value};

/// Voucher types the SDK emits; the manual also lists 04, R1 and R7, which the SDK does not issue.
const DOCUMENT_TYPES: &[&str] = &["01", "03", "07", "08"];
/// `numero` is `an…8` in the manual.
const MAX_NUMBER: u64 = 99_999_999;

/// `{"numRuc", "codComp", "numeroSerie", "numero", "fechaEmision", "monto"}`, with `numero` and
/// `monto` as strings, the way the manual's example sends them.
pub fn query_body(input: &Value) -> Result<Value, String> {
    let ruc = ruc(input, "ruc")?;
    let document_type = text(input, "document_type")?;
    if !DOCUMENT_TYPES.contains(&document_type) {
        return Err(format!("`document_type` must be one of {}", DOCUMENT_TYPES.join(", ")));
    }
    let series = text(input, "series")?;
    if series.len() != 4 || !series.bytes().all(|b| b.is_ascii_alphanumeric()) {
        return Err("`series` must be 4 letters or digits".into());
    }
    Ok(json!({
        "numRuc": ruc,
        "codComp": document_type,
        "numeroSerie": series,
        "numero": number(input)?.to_string(),
        "fechaEmision": issue_date(text(input, "issue_date")?)?,
        "monto": total(text(input, "total")?)?,
    }))
}

/// The RUC in the URL, the one that asks: `requester_ruc`, or the issuer's `ruc` when absent.
pub fn requester_ruc(input: &Value) -> Result<&str, String> {
    if input.get("requester_ruc").is_some_and(|v| !v.is_null()) {
        return ruc(input, "requester_ruc");
    }
    ruc(input, "ruc")
}

/// The client id goes in the token URL's path: only the characters SOL issues (a UUID).
pub fn validate_client_id(client_id: &str) -> Result<(), String> {
    if client_id.is_empty() || !client_id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-') {
        return Err("the client id must be the letters, digits and hyphens SOL issues".into());
    }
    Ok(())
}

fn text<'a>(input: &'a Value, field: &str) -> Result<&'a str, String> {
    input[field].as_str().ok_or_else(|| format!("missing string field `{field}`"))
}

fn ruc<'a>(input: &'a Value, field: &str) -> Result<&'a str, String> {
    let value = text(input, field)?;
    if value.len() != 11 || !value.bytes().all(|b| b.is_ascii_digit()) {
        return Err(format!("`{field}` must be 11 digits"));
    }
    Ok(value)
}

fn number(input: &Value) -> Result<u64, String> {
    input["number"]
        .as_u64()
        .or_else(|| input["number"].as_str().filter(|s| s.bytes().all(|b| b.is_ascii_digit())).and_then(|s| s.parse().ok()))
        .filter(|n| (1..=MAX_NUMBER).contains(n))
        .ok_or_else(|| format!("`number` must be an integer from 1 to {MAX_NUMBER}"))
}

/// `YYYY-MM-DD`, alone or as the start of the document's ISO 8601 `issue_date`, to `dd/mm/yyyy`.
fn issue_date(value: &str) -> Result<String, String> {
    let bad = || "`issue_date` must start with a date YYYY-MM-DD".to_string();
    let date = value.get(..10).ok_or_else(bad)?;
    let rest = &value[10..];
    if !(rest.is_empty() || rest.starts_with('T')) {
        return Err(bad());
    }
    let parts: Vec<&str> = date.split('-').collect();
    let [year, month, day] = parts.as_slice() else { return Err(bad()) };
    let digits = |s: &str, len: usize| s.len() == len && s.bytes().all(|b| b.is_ascii_digit());
    if !(digits(year, 4) && digits(month, 2) && digits(day, 2)) {
        return Err(bad());
    }
    let (m, d): (u32, u32) = (month.parse().map_err(|_| bad())?, day.parse().map_err(|_| bad())?);
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return Err(bad());
    }
    Ok(format!("{day}/{month}/{year}"))
}

/// A decimal string with up to 2 decimals (`n(8,2)`), written with exactly 2. Never rounded.
fn total(value: &str) -> Result<String, String> {
    let bad = || "`total` must be a decimal string with up to 8 integer digits and 2 decimals".to_string();
    let (integer, decimals) = value.split_once('.').unwrap_or((value, ""));
    let digits = |s: &str| s.bytes().all(|b| b.is_ascii_digit());
    if integer.is_empty() || integer.len() > 8 || decimals.len() > 2 || !digits(integer) || !digits(decimals) {
        return Err(bad());
    }
    if value.ends_with('.') {
        return Err(bad());
    }
    Ok(format!("{integer}.{decimals:0<2}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input() -> Value {
        json!({ "ruc": "20100066603", "document_type": "03", "series": "B001", "number": 749, "issue_date": "2018-06-29T10:30:00-05:00", "total": "236" })
    }

    #[test]
    fn body_follows_the_manual_example() {
        let body = query_body(&input()).unwrap();
        assert_eq!(
            body,
            json!({ "numRuc": "20100066603", "codComp": "03", "numeroSerie": "B001", "numero": "749", "fechaEmision": "29/06/2018", "monto": "236.00" })
        );
        assert_eq!(requester_ruc(&input()).unwrap(), "20100066603");
        let mut other = input();
        other["requester_ruc"] = json!("20555555555");
        assert_eq!(requester_ruc(&other).unwrap(), "20555555555");
    }

    #[test]
    fn shapes_sunat_would_refuse_are_refused_here() {
        let with = |field: &str, value: Value| {
            let mut i = input();
            i[field] = value;
            query_body(&i).unwrap_err()
        };
        assert!(with("document_type", json!("09")).contains("01, 03, 07, 08"));
        assert!(with("ruc", json!("2010006660")).contains("11 digits"));
        assert!(with("series", json!("B01")).contains("series"));
        assert!(with("number", json!(0)).contains("number"));
        assert!(with("number", json!("1e3")).contains("number"));
        assert!(with("number", json!(100_000_000)).contains("number"));
        assert!(with("issue_date", json!("29/06/2018")).contains("issue_date"));
        assert!(with("issue_date", json!("2018-13-01")).contains("issue_date"));
        assert!(with("total", json!("236.005")).contains("total"));
        assert!(with("total", json!("236.")).contains("total"));
        assert!(with("total", json!(236)).contains("total"));
        assert!(with("total", json!("-1.00")).contains("total"));
        assert_eq!(total("0.5").unwrap(), "0.50");
        assert_eq!(issue_date("2026-10-16").unwrap(), "16/10/2026");
    }

    #[test]
    fn client_id_cannot_leave_its_path_segment() {
        assert!(validate_client_id("c04864ce-f454-48e1-b41d-8730e575b905").is_ok());
        for bad in ["", "a/b", "a?b", "../x", "a b"] {
            assert!(validate_client_id(bad).is_err(), "{bad}");
        }
    }
}
