use std::fmt;

/// Failure to build or parse a SUNAT payload. Never carries credentials.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransportError {
    /// The document name is not `RUC-TIPO-SERIE-NUMERO`.
    InvalidName(String),
    /// An argument of a SOAP call is out of its format (RUC, type or series).
    InvalidArgument(&'static str),
    Zip(String),
    /// The zip holds no `.xml` entry.
    NoXmlInZip,
    Xml(String),
    Base64(String),
    /// A required element is absent or empty in the response or CDR.
    MissingElement(&'static str),
}

impl fmt::Display for TransportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidName(name) => write!(f, "invalid document name `{name}`: expected RUC-TIPO-SERIE-NUMERO"),
            Self::InvalidArgument(what) => write!(f, "invalid argument: {what}"),
            Self::Zip(msg) => write!(f, "zip error: {msg}"),
            Self::NoXmlInZip => write!(f, "zip contains no xml file"),
            Self::Xml(msg) => write!(f, "xml error: {msg}"),
            Self::Base64(msg) => write!(f, "base64 error: {msg}"),
            Self::MissingElement(name) => write!(f, "missing element `{name}`"),
        }
    }
}

impl std::error::Error for TransportError {}
