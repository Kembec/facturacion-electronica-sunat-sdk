//! Classification of SUNAT response codes (CDR `ResponseCode` or fault code), per the
//! programmer's manual §4.1 and the codes 1032/1033 (number) and 2323/2324 (void communication).

/// What a response code means for the document that was sent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResponseClass {
    /// 0: accepted without observations.
    Accepted,
    /// 4000 and above, or a CDR with code 0 and a note, whatever its code: accepted with
    /// observations.
    Observed,
    /// 100-1999 except 1032 and 1033: the file was not processed; the document counts as not
    /// reported and may be sent again after fixing it.
    Exception,
    /// 1033: the number is already registered with other data. Recover its CDR with
    /// getStatusCdr; do not resend or reuse the number.
    AlreadyRegistered,
    /// 1032: the number is already reported and voided or rejected. Do not reuse it; issue the
    /// operation with a new number.
    NumberVoidedOrRejected,
    /// 2000-3999 except 2323 and 2324: rejected; for invoices and notes the number is spent.
    Rejected,
    /// 2323 (a voided document is already in an earlier void communication) or 2324 (the void
    /// communication id was already presented): a rejection of that void communication, not a
    /// statement that an invoice, receipt or note is registered.
    VoidAlreadyReported,
    /// 1-99: outside the ranges the manual defines.
    Unknown,
}

impl ResponseClass {
    /// Stable snake_case name, for JSON and logs.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Accepted => "accepted",
            Self::Observed => "observed",
            Self::Exception => "exception",
            Self::AlreadyRegistered => "already_registered",
            Self::NumberVoidedOrRejected => "number_voided_or_rejected",
            Self::Rejected => "rejected",
            Self::VoidAlreadyReported => "void_already_reported",
            Self::Unknown => "unknown",
        }
    }

    /// True when SUNAT registered the document as valid (0 or an observation).
    pub fn is_valid_document(self) -> bool {
        matches!(self, Self::Accepted | Self::Observed)
    }
}

/// Classifies a numeric response code.
pub fn classify(code: u32) -> ResponseClass {
    match code {
        0 => ResponseClass::Accepted,
        1032 => ResponseClass::NumberVoidedOrRejected,
        1033 => ResponseClass::AlreadyRegistered,
        2323 | 2324 => ResponseClass::VoidAlreadyReported,
        100..=1999 => ResponseClass::Exception,
        2000..=3999 => ResponseClass::Rejected,
        4000.. => ResponseClass::Observed,
        _ => ResponseClass::Unknown,
    }
}

/// Classifies a code as SUNAT writes it (`"0109"`, `"0"`). `None` if it is not a number.
pub fn classify_code(code: &str) -> Option<ResponseClass> {
    code.trim().parse::<u32>().ok().map(classify)
}

/// Classifies a CDR by its `ResponseCode` and its `cbc:Note`s. Every note of a CDR is a warning
/// that is not a rejection (manual annex B.10, `"4287 - ..."`), so code 0 with any note is
/// `Observed`. The code range does not decide it: the validation sheet turned 2404 (a note that
/// modifies a paper voucher) and 2988 (a daily summary line) from errors into observations
/// without renumbering them. `None` if the code is not a number.
pub fn classify_cdr(response_code: &str, notes: &[String]) -> Option<ResponseClass> {
    let class = classify_code(response_code)?;
    let observed = notes.iter().any(|note| !note.trim().is_empty());
    if class == ResponseClass::Accepted && observed {
        return Some(ResponseClass::Observed);
    }
    Some(class)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ResponseClass::*;

    #[test]
    fn covers_every_range_and_special_code() {
        let cases = [
            (0, Accepted),
            (1, Unknown),
            (99, Unknown),
            (100, Exception),
            (109, Exception),
            (1031, Exception),
            (1032, NumberVoidedOrRejected),
            (1033, AlreadyRegistered),
            (1034, Exception),
            (1999, Exception),
            (2000, Rejected),
            (2322, Rejected),
            (2323, VoidAlreadyReported),
            (2324, VoidAlreadyReported),
            (2325, Rejected),
            (3999, Rejected),
            (4000, Observed),
            (4031, Observed),
            (u32::MAX, Observed),
        ];
        for (code, expected) in cases {
            assert_eq!(classify(code), expected, "code {code}");
        }
    }

    #[test]
    fn classifies_codes_as_written_by_sunat() {
        assert_eq!(classify_code("0109"), Some(Exception));
        assert_eq!(classify_code("0"), Some(Accepted));
        assert_eq!(classify_code("1033"), Some(AlreadyRegistered));
        assert_eq!(classify_code(""), None);
        assert_eq!(classify_code("CDR"), None);
    }

    #[test]
    fn a_cdr_with_code_0_and_an_observation_note_is_observed() {
        let notes = |list: &[&str]| list.iter().map(|n| n.to_string()).collect::<Vec<_>>();
        assert_eq!(classify_cdr("0", &[]), Some(Accepted));
        assert_eq!(classify_cdr("0", &notes(&["4287 - El precio unitario difiere"])), Some(Observed));
        assert_eq!(classify_cdr("0", &notes(&["sin código", " 4001-Observación"])), Some(Observed));
        assert_eq!(classify_cdr("0", &notes(&["", "  "])), Some(Accepted), "a blank note says nothing");
        assert_eq!(classify_cdr("0", &notes(&["Texto libre sin código"])), Some(Observed));
        assert_eq!(
            classify_cdr("0", &notes(&["2404 - Documento afectado por la nota electronica no se encuentra autorizado"])),
            Some(Observed),
            "an OBSERV rule below 4000 still observes"
        );
        assert_eq!(classify_cdr("0", &notes(&["2988 - El comprobante (fisico) ..."])), Some(Observed));
        assert_eq!(classify_cdr("2047", &notes(&["4287 - ..."])), Some(Rejected), "a note never softens a rejection");
        assert_eq!(classify_cdr("4031", &[]), Some(Observed));
        assert_eq!(classify_cdr("CDR", &notes(&["4287 - ..."])), None);
    }

    #[test]
    fn only_accepted_and_observed_are_valid_documents() {
        assert!(Accepted.is_valid_document() && Observed.is_valid_document());
        for class in [Exception, AlreadyRegistered, NumberVoidedOrRejected, Rejected, VoidAlreadyReported, Unknown] {
            assert!(!class.is_valid_document());
        }
        assert_eq!(VoidAlreadyReported.as_str(), "void_already_reported");
    }
}
