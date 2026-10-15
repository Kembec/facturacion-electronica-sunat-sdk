use std::fmt;

use sunat_model::ModelError;

/// Why a document could not be rendered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UblError {
    /// The document is outside phase 1 or misses a field the XML needs.
    Model(ModelError),
    /// A rendering option is unusable.
    InvalidOption(&'static str),
}

impl fmt::Display for UblError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Model(error) => write!(f, "{error}"),
            Self::InvalidOption(reason) => write!(f, "invalid option: {reason}"),
        }
    }
}

impl std::error::Error for UblError {}

impl From<ModelError> for UblError {
    fn from(error: ModelError) -> Self {
        Self::Model(error)
    }
}
