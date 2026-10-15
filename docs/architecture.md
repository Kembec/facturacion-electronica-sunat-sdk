# Architecture (phase 1)

Sans-IO core: crates build and parse bytes; only `bindings/` do network or file I/O. That keeps one core
for native builds, WebAssembly (edge) and language bindings. TLS verification is never disabled.

| Crate | Owns | Public contract |
|---|---|---|
| `sunat-model` | input types (`docs/input-model.md`), serde, validation of scope | `Document` (`Invoice`, `Note`), `Document::from_json(&str) -> Result<Document, ModelError>`, `Document::file_name() -> String` (`RUC-TIPO-SERIE-NUMERO`) |
| `sunat-ubl` | UBL 2.1 rendering of 01/03/07/08, from SUNAT's XML guides | `build(&Document) -> Result<Vec<u8>, UblError>`: XML declaration + root element in canonical form, with the empty `ext:UBLExtensions/ext:UBLExtension/ext:ExtensionContent` that receives the signature |
| `sunat-xmldsig` | enveloped XML-DSig as SUNAT requires it: RSA-SHA1, SHA1 digest, inclusive C14N, `Reference URI=""`, signature in the first empty `ext:ExtensionContent` | `sign(xml, cert_pem, key_pem)`, `sign_with_id(..)`, `verify(xml) -> Result<Verification, SignError>` |
| `sunat-transport` | zip, SOAP envelopes (WS-Security UsernameToken), response and CDR parsing | `zip_document(name, xml)`, `send_bill_envelope(sol_user_full, sol_password, zip_name, zip)`, `parse_send_bill_response(&[u8]) -> SendBillOutcome { Cdr(Vec<u8>) \| Fault { code, message } }`, `read_cdr(&[u8]) -> Cdr { reference_id, response_code, description, notes }`, `get_status_cdr_envelope(...)`, `parse_get_status_cdr_response(...)` |
| `sunat-sdk` | facade and response classification | re-exports; `classify(code) -> ResponseClass` (0 accepted; 100-1999 exception; 1033 already registered; 1032 number voided or rejected; 2000-3999 rejected; 2323/2324 void-communication codes; ≥4000 observed) |
| `bindings/cli` | `sunat-sdk` binary: JSON on stdin, JSON on stdout; HTTP with TLS verification | `build`, `sign`, `zip`, `send`, `status`, `cdr`; endpoints and credentials from flags or environment, never in code |
| `bindings/php` | Composer package for PHP consumers, calling the CLI | mirrors the CLI commands |

Signature identifiers default to the ones in SUNAT's invoice guide example (`IDSignKG`, `#signatureKG`,
`signatureKG`).

## Acceptance of a document type

1. Compatibility: for every fixture in `fixtures/`, C14N of the SDK's unsigned XML equals
   `tests/vectors/<case>.unsigned.xml`.
2. Signature: signing that XML with `tests/vectors/test-cert/` gives the same `DigestValue` and
   `SignatureValue` as `tests/vectors/<case>.signed.xml`, and an independent verifier accepts it.
3. SUNAT beta: accepted (`ResponseCode` 0 or ≥4000) with the public beta credentials.

`tests/vectors/test-cert/` is a self-generated test certificate, valid for no taxpayer. No other
certificate or credential enters the repository.
