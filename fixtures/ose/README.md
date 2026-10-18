Synthetic answers of an OSE (operador de servicios electrónicos), written for these tests: no OSE's
CDR or fault was found published. They follow the ApplicationResponse and fault shapes of SUNAT's OSE
technical annex — the document number in `cac:DocumentResponse/cac:DocumentReference/cbc:ID`, each
observation in `cac:Response/cac:Status` (`StatusReasonCode`, `StatusReason`), and a rejection as a
SOAP fault with the code in `faultstring` and the description in `detail`. Unsigned; RUCs, numbers and
hashes are placeholders.

- `ose-cdr-invoice-accepted.xml`, `ose-cdr-summary-accepted.xml`: code 0, no observations.
- `ose-cdr-invoice-observed.xml`: code 0 with one observation (4252).
- `ose-cdr-nonzero-code.xml`: an answer with `ResponseCode` 1, which the annex does not allow.
- `ose-fault-rejected.xml`: rejection 2108 as an OSE writes it.
- `ose-fault-alpha-code.xml`: a fault whose code is the OSE's own, not one of SUNAT's.
- `sunat-fault-contrast.xml`: the same rejection 2108 as SUNAT writes it, for contrast.
