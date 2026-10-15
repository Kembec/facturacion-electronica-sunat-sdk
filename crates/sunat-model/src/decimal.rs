use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer};

use crate::ModelError;

const MAX_LEN: usize = 64;

/// A decimal amount exactly as the caller wrote it (`-?\d+(\.\d+)?`).
///
/// The text is kept verbatim: how an amount is rendered depends on the digits given, not on a
/// normalized value.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Decimal(String);

impl Decimal {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// `0`, `0.00` and `-0.0` are zero.
    pub fn is_zero(&self) -> bool {
        self.0.bytes().all(|b| matches!(b, b'0' | b'.' | b'-'))
    }

    pub fn is_negative(&self) -> bool {
        self.0.starts_with('-') && !self.is_zero()
    }

    /// The nearest IEEE 754 double.
    pub fn to_f64(&self) -> f64 {
        self.0.parse().unwrap_or(0.0)
    }
}

impl FromStr for Decimal {
    type Err = ModelError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        if text.len() > MAX_LEN || !is_decimal(text) {
            return Err(ModelError::invalid("$", format!("not a decimal string: {text:?}")));
        }
        Ok(Self(text.to_owned()))
    }
}

fn is_decimal(text: &str) -> bool {
    let unsigned = text.strip_prefix('-').unwrap_or(text);
    let (int, frac) = match unsigned.split_once('.') {
        Some((int, frac)) => (int, Some(frac)),
        None => (unsigned, None),
    };
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    digits(int) && frac.map_or(true, digits)
}

impl fmt::Display for Decimal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for Decimal {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        text.parse().map_err(|_| {
            serde::de::Error::custom(format!("amount must be a decimal string, got {text:?}"))
        })
    }
}
