//! Free-text fields SUNAT observes by format in 01, 03, 07 and 08: «alfanumérico de N a M caracteres
//! (se considera cualquier carácter incluido espacio, no se permite ningún otro whitespace character:
//! salto de línea, tab, fin de línea, etc.)», only when the tag exists. The SDK writes none of these
//! tags for an empty value, so an empty value is left alone. The sheet allows the space; beta, on
//! 4233, accepted a no-break space, so only ASCII whitespace other than the space is refused. A value
//! is refused with its field, never trimmed or rewritten: SUNAT would keep the note on the voucher.
//! The despatch advice's fields (09) are in `despatch_text`, with the same `Text`.

use crate::{Company, DocumentType, Line, ModelError};

/// Issuer fields (sheet rows of `Factura2_0`, `Boleta2_0`, `NotaCredito2_0`, `NotaDebito2_0`).
pub(crate) fn check_issuer(issuer: &Company, document_type: DocumentType) -> Result<(), ModelError> {
    // 01 and 03 say «hasta 1500»; 07 and 08 say «de 3 hasta 1500».
    let name_min = if matches!(document_type, DocumentType::CreditNote | DocumentType::DebitNote) { 3 } else { 1 };
    check("$.issuer.legal_name", Some(&issuer.legal_name), name_min, 1500, "4338")?;
    check("$.issuer.trade_name", issuer.trade_name.as_deref(), name_min, 1500, "4092")?;
    let address = &issuer.address;
    check("$.issuer.address.street", address.street.as_deref(), 3, 200, "4094")?;
    check("$.issuer.address.urbanization", address.urbanization.as_deref(), 1, 25, "4095")?;
    check("$.issuer.address.province", address.province.as_deref(), 1, 30, "4096")?;
    check("$.issuer.address.department", address.department.as_deref(), 1, 30, "4097")?;
    check("$.issuer.address.district", address.district.as_deref(), 1, 30, "4098")
}

/// `cac:SellersItemIdentification/cbc:ID`: 4269 in 01 and 03, 4234 in 07 and 08, both 1 to 30.
pub(crate) fn check_lines(lines: &[Line], document_type: DocumentType) -> Result<(), ModelError> {
    let code = if matches!(document_type, DocumentType::CreditNote | DocumentType::DebitNote) { "4234" } else { "4269" };
    lines.iter().enumerate().try_for_each(|(index, line)| check(&format!("$.lines[{index}].product_code"), line.product_code.as_deref(), 1, 30, code))
}

fn check(path: &str, value: Option<&str>, min: usize, max: usize, code: &str) -> Result<(), ModelError> {
    check_format(path, value, Text::spaced(min, max), code)
}

/// A free-text format as SUNAT's sheets word it: a length in characters, whether whitespace other
/// than the space is allowed, and whether a value of only spaces is.
#[derive(Clone, Copy)]
pub(crate) struct Text {
    min: usize,
    max: usize,
    whitespace: bool,
    only_spaces: bool,
}

impl Text {
    /// «se considera cualquier carácter incluido espacio, no se permite ningún otro whitespace».
    pub(crate) const fn spaced(min: usize, max: usize) -> Self {
        Self { min, max, whitespace: false, only_spaces: true }
    }

    /// «permite whitespace character: espacio, salto de línea, fin de línea, tab».
    pub(crate) const fn any_whitespace(min: usize, max: usize) -> Self {
        Self { min, max, whitespace: true, only_spaces: true }
    }

    /// «Tampoco se permite solo espacios en blanco».
    pub(crate) const fn not_only_spaces(self) -> Self {
        Self { only_spaces: false, ..self }
    }

    fn accepts(self, value: &str) -> bool {
        (self.min..=self.max).contains(&value.chars().count())
            && (self.whitespace || !value.chars().any(forbidden))
            && (self.only_spaces || !value.chars().all(|c| c == ' '))
    }

    fn describe(self) -> String {
        let whitespace = if self.whitespace { "line breaks and tabs allowed" } else { "spaces allowed, no line break or tab" };
        let spaces = if self.only_spaces { "" } else { ", not only spaces" };
        format!("{} to {} characters, {whitespace}{spaces}", self.min, self.max)
    }
}

/// Refuses a non-empty `value` outside `format` with SUNAT's observation `code`; an empty value is
/// left alone (the SDK writes no tag for it, or another rule already requires it).
pub(crate) fn check_format(path: &str, value: Option<&str>, format: Text, code: &str) -> Result<(), ModelError> {
    let Some(value) = value.filter(|v| !v.is_empty()) else { return Ok(()) };
    if format.accepts(value) {
        return Ok(());
    }
    Err(ModelError::invalid(path, format!("expected {} (SUNAT {code}), got {value:?}", format.describe())))
}

/// Line feed, carriage return, tab, vertical tab and form feed: the whitespace SUNAT names, and the
/// rest of Java's `\s` class besides the space.
fn forbidden(c: char) -> bool {
    matches!(c, '\n' | '\r' | '\t' | '\u{0B}' | '\u{0C}')
}
