//! Text forms of the amounts the documents carry.
//!
//! Two forms appear in the compatibility vectors:
//! - money totals and line amounts with exactly two decimals (`n(12,2)` in SUNAT's guides),
//!   rounded half away from zero;
//! - quantities, unit prices and the tax rate as the shortest decimal that identifies the same
//!   IEEE 754 double (`50.00` -> `50`, `11.80` -> `11.8`), the way a binary float prints.

use sunat_model::Decimal;

/// `125.4` -> `125.40`, `0.125` -> `0.13`, `-1.005` -> `-1.01`.
pub(crate) fn two_decimals(value: &Decimal) -> String {
    let text = value.as_str();
    let (negative, unsigned) = match text.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, text),
    };
    let (int, frac) = unsigned.split_once('.').unwrap_or((unsigned, ""));
    let mut digits: Vec<u8> = int.bytes().chain(frac.bytes().chain(std::iter::repeat(b'0')).take(2)).collect();
    if frac.as_bytes().get(2).is_some_and(|d| *d >= b'5') {
        round_up(&mut digits);
    }
    let split = digits.len() - 2;
    let int_part = trim_leading_zeros(&digits[..split]);
    let result = format!("{int_part}.{}", String::from_utf8_lossy(&digits[split..]));
    if negative && result.bytes().any(|b| matches!(b, b'1'..=b'9')) {
        format!("-{result}")
    } else {
        result
    }
}

/// Adds one unit in the last place of a digit string, carrying to the left.
fn round_up(digits: &mut Vec<u8>) {
    for d in digits.iter_mut().rev() {
        if *d == b'9' {
            *d = b'0';
        } else {
            *d += 1;
            return;
        }
    }
    digits.insert(0, b'1');
}

fn trim_leading_zeros(int: &[u8]) -> String {
    let text = String::from_utf8_lossy(int);
    let trimmed = text.trim_start_matches('0');
    if trimmed.is_empty() {
        "0".to_owned()
    } else {
        trimmed.to_owned()
    }
}

/// Shortest round-trip text of the double nearest to `value`: positional between 1e-4 and 1e15,
/// exponent form (`1.0E-5`, `1.0E+15`) outside that range.
pub(crate) fn shortest(value: &Decimal) -> String {
    let number: f64 = value.as_str().parse().unwrap_or(0.0);
    if number == 0.0 {
        return if number.is_sign_negative() { "-0".into() } else { "0".into() };
    }
    let magnitude = number.abs();
    if (1e-4..1e15).contains(&magnitude) {
        return format!("{number}");
    }
    exponent_form(number)
}

fn exponent_form(number: f64) -> String {
    let text = format!("{number:e}");
    let (mantissa, exponent) = text.split_once('e').unwrap_or((&text, "0"));
    let mantissa = if mantissa.contains('.') { mantissa.to_owned() } else { format!("{mantissa}.0") };
    let exponent = if exponent.starts_with('-') { exponent.to_owned() } else { format!("+{exponent}") };
    format!("{mantissa}E{exponent}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(text: &str) -> Decimal {
        text.parse().unwrap()
    }

    #[test]
    fn two_decimals_rounds_half_away_from_zero() {
        let cases = [
            ("125.42", "125.42"),
            ("125.4", "125.40"),
            ("7", "7.00"),
            ("0.125", "0.13"),
            ("0.124", "0.12"),
            ("9.995", "10.00"),
            ("-1.005", "-1.01"),
            ("-0.001", "0.00"),
            ("007.10", "7.10"),
        ];
        for (input, expected) in cases {
            assert_eq!(two_decimals(&d(input)), expected, "{input}");
        }
    }

    #[test]
    fn shortest_prints_like_a_double() {
        let cases = [
            ("50.00", "50"),
            ("11.80", "11.8"),
            ("12.71", "12.71"),
            ("0.00", "0"),
            ("18.00", "18"),
            ("0.00001", "1.0E-5"),
            ("1000000000000000", "1.0E+15"),
            ("0.1", "0.1"),
        ];
        for (input, expected) in cases {
            assert_eq!(shortest(&d(input)), expected, "{input}");
        }
    }
}
