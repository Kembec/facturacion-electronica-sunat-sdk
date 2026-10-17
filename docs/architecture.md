# Architecture

Sans-IO core: crates build and parse bytes; only `bindings/` do network or file I/O. That keeps one core
for native builds, WebAssembly (edge) and language bindings. TLS verification is never disabled.

| Crate | Owns | Public contract |
|---|---|---|
| `sunat-model` | input types (`docs/input-model.md`), serde, validation of scope | `Document` (`Invoice`, `Note`, `Summary`, `Voided`, `Despatch`), `Document::from_json(&str) -> Result<Document, ModelError>`, `Document::file_name() -> String` (`RUC-TIPO-SERIE-NUMERO`, `RUC-RC-YYYYMMDD-N`, `RUC-RA-YYYYMMDD-N`, `RUC-09-SERIE-NUMERO`); a despatch advice is checked against SUNAT's rules with their error codes |
| `sunat-ubl` | UBL 2.1 rendering of 01/03/07/08, from SUNAT's XML guides; UBL 2.0 daily summary (RC) and void communication (RA); UBL 2.1 sender's despatch advice (09) | `build(&Document) -> Result<Vec<u8>, UblError>`: XML declaration + root element in canonical form, with the empty `ext:UBLExtensions/ext:UBLExtension/ext:ExtensionContent` that receives the signature |
| `sunat-xmldsig` | enveloped XML-DSig as SUNAT requires it: RSA-SHA1, SHA1 digest, inclusive C14N, `Reference URI=""`, signature in the first empty `ext:ExtensionContent` | `sign(xml, cert_pem, key_pem)`, `sign_with_id(..)`, `verify(xml) -> Result<Verification, SignError>` |
| `sunat-transport` | zip, SOAP envelopes (WS-Security UsernameToken), response and CDR parsing | `zip_document(name, xml)`, `send_bill_envelope(sol_user_full, sol_password, zip_name, zip)`, `parse_send_bill_response(&[u8]) -> SendBillOutcome { Cdr(Vec<u8>) \| Fault { code, message } }`, `read_cdr(&[u8]) -> Cdr { reference_id, response_code, description, notes }`, `get_status_cdr_envelope(...)`, `parse_get_status_cdr_response(...)` |
| `sunat-sdk` | facade, response classification, and the web-service calls as JSON (`exchange`) | `exchange::request(Exchange, input, user, password) -> envelope` and `exchange::read(Exchange, input, Reply) -> outcome`, the JSON of `docs/cli.md` for `send`, `summary`, `ticket` and `status`, shared by the CLI and the WebAssembly binding; `validity` (Consulta Integrada) and `gre` (despatch advice by REST: token, send with `hashZip`, ticket), each as `*_request` → `{"method", "url", "headers", "body"}` and `*_read` of the answer; re-exports; `classify(code) -> ResponseClass` and `classify_cdr(code, notes)`, which makes a code 0 with a 4xxx note observed (0 accepted; 100-1999 exception; 1033 already registered; 1032 number voided or rejected; 2000-3999 rejected; 2323/2324 void-communication codes; ≥4000 observed) |
| `bindings/cli` | `sunat-sdk` binary (and the `sunat_sdk_cli` library holding its dispatch, shared with `bindings/ffi`): JSON on stdin, JSON on stdout; HTTP with TLS verification | `build`, `sign`, `zip`, `send`, `summary`, `ticket`, `status`, `validity`, `gre-send`, `gre-ticket`, `cdr`; endpoints and credentials from flags or environment, never in code |
| `bindings/ffi` | C ABI (`cdylib`, `staticlib`, generated header) for the languages that load a library (VB6, Visual FoxPro, PowerBuilder, .NET, Java, Perl, Ruby, Swift, Objective-C): the CLI's dispatch in process | `sunat_sdk_call(request, &response)` with `{"command", "flags", "input"}`, the response is the CLI's output; `docs/c-abi.md`, decision in `docs/adr/0001-c-abi-over-cli-wrappers.md` |
| `bindings/php` | Composer package for PHP consumers, calling the CLI | mirrors the CLI commands |
| `bindings/wasm` | npm package: the core compiled to WebAssembly, no I/O | `build`, `sign`, `zip`, `cdr`; `soapRequest` gives the `fetch` options of an exchange and `soapResponse` / `soapTimeout` read what came back into the CLI's outcome: the caller does the POST |
| `bindings/python` | PyPI package `facturacion-electronica-sunat-sdk` (`import sunat_sdk`): abi3 wheels (PyO3, maturin) over the same in-memory commands as `bindings/wasm` | `build`, `sign`, `zip`, `cdr`, `soap_request`, `soap_response`, `soap_timeout` with the CLI's JSON (mapping or JSON text in, `dict` out); `exchange` does the POST with `urllib` and verified TLS |

Signature identifiers default to the ones in SUNAT's invoice guide example (`IDSignKG`, `#signatureKG`,
`signatureKG`).

## Acceptance of a document type

1. Compatibility: for every fixture in `fixtures/`, C14N of the SDK's unsigned XML equals
   `tests/vectors/<case>.unsigned.xml`.
2. Signature: signing that XML with `tests/vectors/test-cert/` gives the same `DigestValue` and
   `SignatureValue` as `tests/vectors/<case>.signed.xml`, and an independent verifier accepts it.
3. SUNAT beta: accepted (`ResponseCode` 0 or ≥4000) with the public beta credentials.

The sender's despatch advice (09) cannot meet 3: SUNAT has no test environment for despatch
advices. Its vectors are written by the SDK itself, so 1 and 2 guard against regressions; in their
place it passes the OASIS UBL 2.1 schema and one test per SUNAT rule code it can check offline
(`docs/input-model.md`, «Despatch advice (09)»).

`tests/vectors/test-cert/` is a self-generated test certificate, valid for no taxpayer. No other
certificate or credential enters the repository.
