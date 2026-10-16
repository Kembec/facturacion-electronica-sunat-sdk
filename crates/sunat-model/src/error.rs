use std::fmt;

/// Why an input document was rejected. `path` is a JSON path such as `$.lines[2].affectation`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModelError {
    /// The input is not valid JSON or does not match the input model.
    Json(String),
    /// The input uses a feature the SDK does not render.
    OutOfScope { path: String, feature: &'static str },
    /// A required value is absent.
    Missing { path: String },
    /// A value is present but not acceptable.
    Invalid { path: String, reason: String },
    /// The document breaks one of SUNAT's validation rules, checked before sending: `code` is the
    /// error SUNAT would answer (`"1001"`).
    Rule { path: String, code: &'static str, reason: String },
}

impl ModelError {
    pub(crate) fn invalid(path: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::Invalid { path: path.into(), reason: reason.into() }
    }

    pub(crate) fn missing(path: impl Into<String>) -> Self {
        Self::Missing { path: path.into() }
    }

    pub(crate) fn rule(code: &'static str, path: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::Rule { path: path.into(), code, reason: reason.into() }
    }

    /// SUNAT's error code for a `Rule` error.
    pub fn sunat_code(&self) -> Option<&'static str> {
        match self {
            Self::Rule { code, .. } => Some(code),
            _ => None,
        }
    }
}

impl fmt::Display for ModelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json(message) => write!(f, "invalid input: {message}"),
            Self::OutOfScope { path, feature } => {
                write!(f, "{path}: {feature} is out of scope")
            }
            Self::Missing { path } => write!(f, "{path}: required"),
            Self::Invalid { path, reason } => write!(f, "{path}: {reason}"),
            Self::Rule { path, code, reason } => write!(f, "{path}: {reason} (SUNAT ERR-{code})"),
        }
    }
}

impl std::error::Error for ModelError {}
