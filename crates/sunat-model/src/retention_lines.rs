//! SUNAT's rules for each line of the retention voucher (sheet «Retenciones1_0», data 37 to 54): the
//! related document, its payment, the amounts retained and paid, and the exchange rate. Amounts are
//! compared exactly, in millionths, never as floating point: the totals have no tolerance (2628,
//! 2629) and a line's amounts have one of plus or minus 1 PEN (2623).

use crate::despatch_rules::{digits, positive_fits, rule};
use crate::{Date, Decimal, ModelError, RetainedDocument, Retention, RetentionLine};

/// Fixed point of `units`: 6 decimals, the most any amount of the voucher carries (the exchange rate).
pub(crate) const SCALE: i128 = 1_000_000;

/// Related document types the SDK renders. 07 and 20 are allowed by SUNAT but carry no payment
/// and stay out of the totals: out of scope.
const RELATED_TYPES: &[&str] = &["01", "08", "12"];
const OUT_OF_SCOPE_TYPES: &[&str] = &["07", "20"];

/// The currency of the voucher's own amounts.
const PEN: &str = "PEN";

/// Regime 02 (6 %) only covers related documents issued up to this day (SUNAT 2985).
const REGIME_02_LAST_DAY: &str = "2014-02-28";

/// `value × SCALE`, exact; `None` with more than 6 decimals or more than 18 integer digits.
pub(crate) fn units(value: &Decimal) -> Option<i128> {
    let text = value.as_str();
    let (negative, unsigned) = match text.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, text),
    };
    let (int, frac) = unsigned.split_once('.').unwrap_or((unsigned, ""));
    if frac.len() > 6 || int.trim_start_matches('0').len() > 18 {
        return None;
    }
    let scaled = int.parse::<i128>().ok()? * SCALE + format!("{frac:0<6}").parse::<i128>().ok()?;
    Some(if negative { -scaled } else { scaled })
}

/// `rate` is the regime's rate in units (3 % is `3 * SCALE`).
pub(crate) fn check_lines(r: &Retention, rate: i128) -> Result<(), ModelError> {
    let issued: Date = r.issue_date.to_lima().date_string().parse()?;
    let first_period = period(r.lines[0].payment.date);
    let mut seen: Vec<(&str, u64)> = Vec::new();
    for (index, line) in r.lines.iter().enumerate() {
        let at = |field: &str| format!("$.lines[{index}].{field}");
        check_document(r, &line.related_document, &at)?;
        check_payment_date(line, issued, &first_period, &at)?;
        let number = r.payment_number(index);
        rule(number <= 999_999_999, "2734", at("payment.number"), "up to 9 digits")?;
        let key = (line.related_document.number.as_str(), number);
        rule(!seen.contains(&key), "2626", at("payment.number"), "repeats a payment number of the same related document")?;
        seen.push(key);
        // The exchange rate before the amounts it converts: the beta answered 2715, not 2623, to a
        // line that breaks both (17/10/2026).
        check_exchange_rate(line, &at)?;
        check_amounts(line, rate, &at)?;
    }
    Ok(())
}

fn check_document(r: &Retention, d: &RetainedDocument, at: &dyn Fn(&str) -> String) -> Result<(), ModelError> {
    rule(!d.document_type.is_empty(), "2691", at("related_document.document_type"), "required")?;
    if OUT_OF_SCOPE_TYPES.contains(&d.document_type.as_str()) {
        let feature = "related documents of type 07 and 20 (no payment, out of the totals)";
        return Err(ModelError::OutOfScope { path: at("related_document.document_type"), feature });
    }
    let reason = "must be \"01\", \"08\" or \"12\"";
    rule(RELATED_TYPES.contains(&d.document_type.as_str()), "2692", at("related_document.document_type"), reason)?;
    rule(!d.number.is_empty(), "2693", at("related_document.number"), "required")?;
    let ok = if d.document_type == "12" { ticket_number(&d.number) } else { voucher_number(&d.number) };
    rule(ok, "2694", at("related_document.number"), "not a valid SERIES-NUMBER for its type")?;
    if r.regime == "02" {
        let last: Date = REGIME_02_LAST_DAY.parse()?;
        rule(d.issue_date <= last, "2985", at("related_document.issue_date"), "regime 02 only covers documents issued up to 2014-02-28")?;
    }
    rule(positive_fits(&d.total, 12, 2), "2696", at("related_document.total"), "must be positive, up to 12 integer digits and 2 decimals")?;
    if !(d.currency.len() == 3 && d.currency.bytes().all(|b| b.is_ascii_uppercase())) {
        return Err(ModelError::invalid(at("related_document.currency"), "ISO 4217 code"));
    }
    Ok(())
}

/// All payments in one month (2661), and each between the first day of the voucher's month, or the
/// related document's issue date when it is from that month, and the voucher's issue date (2625).
fn check_payment_date(line: &RetentionLine, issued: Date, first_period: &str, at: &dyn Fn(&str) -> String) -> Result<(), ModelError> {
    let paid = line.payment.date;
    rule(period(paid) == first_period, "2661", at("payment.date"), "all payment dates must be in the same month")?;
    let document_issued = line.related_document.issue_date;
    let from = if period(document_issued) == period(issued) {
        document_issued
    } else {
        format!("{}-01", period(issued)).parse()?
    };
    let reason = "must be between the first day of the voucher's month (or the related document's issue date, if later) and the voucher's issue date";
    rule(from <= paid && paid <= issued, "2625", at("payment.date"), reason)
}

/// Retained = paid × rate [× exchange rate], net = paid [× exchange rate] − retained, both in PEN
/// within ±1 (2623). Without a usable exchange rate the comparison waits for 2719/2721/2716.
fn check_amounts(line: &RetentionLine, rate: i128, at: &dyn Fn(&str) -> String) -> Result<(), ModelError> {
    let payment = &line.payment.amount;
    rule(positive_fits(payment, 12, 2), "2736", at("payment.amount"), "must be positive, up to 12 integer digits and 2 decimals")?;
    let amounts = &line.retention;
    rule(positive_fits(&amounts.amount, 12, 2), "2740", at("retention.amount"), "must be positive, up to 12 integer digits and 2 decimals")?;
    let (Some(paid), Some(retained), Some(factor)) = (units(payment), units(&amounts.amount), factor(line)) else {
        return positive_net(line, at);
    };
    let paid_pen = paid * factor / SCALE;
    let expected = paid_pen * rate / (100 * SCALE);
    let reason = "must be payment.amount × rate (× exchange_rate.rate when not PEN), ±1 PEN";
    rule((retained - expected).abs() <= SCALE, "2623", at("retention.amount"), reason)?;
    positive_net(line, at)?;
    let net = units(&amounts.net_paid).unwrap_or_default();
    let reason = "must be payment.amount (× exchange_rate.rate when not PEN) − retention.amount, ±1 PEN";
    rule((net - (paid_pen - retained)).abs() <= SCALE, "2623", at("retention.net_paid"), reason)
}

fn positive_net(line: &RetentionLine, at: &dyn Fn(&str) -> String) -> Result<(), ModelError> {
    let reason = "must be positive, up to 12 integer digits and 2 decimals";
    rule(positive_fits(&line.retention.net_paid, 12, 2), "2746", at("retention.net_paid"), reason)
}

/// PEN per unit of the related document's currency: 1 in PEN, else the exchange rate when it fits.
fn factor(line: &RetentionLine) -> Option<i128> {
    if line.related_document.currency == PEN {
        return Some(SCALE);
    }
    let rate = line.exchange_rate.as_ref()?.rate.as_ref()?;
    positive_fits(rate, 4, 6).then(|| units(rate)).flatten()
}

fn check_exchange_rate(line: &RetentionLine, at: &dyn Fn(&str) -> String) -> Result<(), ModelError> {
    let currency = line.related_document.currency.as_str();
    let foreign = currency != PEN;
    let Some(x) = &line.exchange_rate else {
        return rule(!foreign, "2719", at("exchange_rate"), "required when the related document is not in PEN");
    };
    let at = |field: &str| at(&format!("exchange_rate.{field}"));
    match x.source_currency.as_deref() {
        None => rule(!foreign, "2719", at("source_currency"), "required when the related document is not in PEN")?,
        Some(source) => rule(source == currency, "2749", at("source_currency"), "must be the related document's currency")?,
    }
    rule(x.target_currency.as_deref() == Some(PEN), "2715", at("target_currency"), "must be \"PEN\"")?;
    match &x.rate {
        None => rule(!foreign, "2721", at("rate"), "required when the related document is not in PEN")?,
        Some(rate) => rule(positive_fits(rate, 4, 6), "2716", at("rate"), "must be positive, up to 4 integer digits and 6 decimals")?,
    }
    rule(x.date.is_some() || !foreign, "2722", at("date"), "required when the related document is not in PEN")
}

/// `YYYY-MM`.
fn period(date: Date) -> String {
    date.date_string()[..7].to_owned()
}

/// `(E001|(F|R)[A-Z0-9]{3}|[0-9]{4})-(?!0+$)[0-9]{1,8}`.
fn voucher_number(text: &str) -> bool {
    let Some((series, number)) = text.split_once('-') else { return false };
    let upper_alnum = |s: &str| s.bytes().all(|b| b.is_ascii_uppercase() || b.is_ascii_digit());
    let series_ok = series == "E001"
        || (series.len() == 4 && (series.starts_with('F') || series.starts_with('R')) && upper_alnum(&series[1..]))
        || (series.len() == 4 && digits(series));
    series_ok && (1..=8).contains(&number.len()) && digits(number) && !number.bytes().all(|b| b == b'0')
}

/// `[a-zA-Z0-9-]{1,20}(-[0-9]{1,20})`.
fn ticket_number(text: &str) -> bool {
    let Some((prefix, number)) = text.rsplit_once('-') else { return false };
    let prefix_ok = (1..=20).contains(&prefix.len()) && prefix.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-');
    prefix_ok && (1..=20).contains(&number.len()) && digits(number)
}
