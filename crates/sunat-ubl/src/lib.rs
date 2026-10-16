//! sunat-ubl: UBL 2.1 XML for invoices (01), receipts (03), credit notes (07) and debit notes (08),
//! written from SUNAT's «Guía de elaboración de documentos electrónicos XML - UBL 2.1» for each
//! document type; and the UBL 2.0 daily summary (RC) and void communication (RA).
//!
//! The output is an XML declaration followed by the root element in canonical form (inclusive
//! Canonical XML 1.0): the bytes after the declaration equal their own C14N, so what is hashed
//! when signing is exactly what was built. The empty `ext:UBLExtensions/ext:UBLExtension/
//! ext:ExtensionContent` is where `sunat-xmldsig` puts the signature.
//!
//! Amounts are rendered as given; this crate does not compute taxes or totals.

mod amount;
mod daily;
mod document;
mod error;
mod lines;
mod parties;
mod references;
mod tax;
mod totals;
mod writer;

pub use error::UblError;
use sunat_model::Document;

/// `cbc:ID` of `cac:Signature` used by the compatibility vectors.
pub const DEFAULT_SIGNATURE_REFERENCE_ID: &str = "IDSignKG";
/// `Id` of the `ds:Signature` the document points to (`cbc:URI` is `#` + this value); the same
/// default `sunat-xmldsig` signs with.
pub const DEFAULT_SIGNATURE_ID: &str = "signatureKG";

/// Identifiers of the signature that the document references.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Options {
    pub signature_reference_id: String,
    pub signature_id: String,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            signature_reference_id: DEFAULT_SIGNATURE_REFERENCE_ID.to_owned(),
            signature_id: DEFAULT_SIGNATURE_ID.to_owned(),
        }
    }
}

/// Checks the document's scope and renders it with the default signature identifiers.
pub fn build(document: &Document) -> Result<Vec<u8>, UblError> {
    build_with(document, &Options::default())
}

/// Checks the document's scope and renders it.
pub fn build_with(document: &Document, options: &Options) -> Result<Vec<u8>, UblError> {
    document.validate()?;
    if options.signature_id.is_empty() || options.signature_reference_id.is_empty() {
        return Err(UblError::InvalidOption("signature identifiers must not be empty"));
    }
    Ok(document::render(document, options))
}
