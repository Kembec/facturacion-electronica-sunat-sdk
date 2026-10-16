# CLI contract (`sunat-sdk`)

JSON on stdin, JSON on stdout, one JSON object per call. On error: exit code 1 and
`{"error": "<message>"}` on stderr; nothing on stdout. Usage errors: same shape. Secrets (SOL password,
private key) are read from flags, environment or files and are never printed.

## `build`

stdin: one document (`docs/input-model.md`).
stdout: `{"file_name": "20000000001-01-F001-1", "xml": "<?xml version=\"1.0\" encoding=\"utf-8\"?>..."}`
(the unsigned XML, UTF-8, exactly as `sunat_ubl::build` returns it).

## `sign`

stdin: `{"xml": "<unsigned XML>", "signature_id": "signatureKG"}` (`signature_id` optional, default
`signatureKG`).
Flags: `--cert <path>` (or `SUNAT_CERT_PATH`) and `--key <path>` (or `SUNAT_KEY_PATH`), both PEM; the
key must be unencrypted RSA.
stdout: `{"xml": "<signed XML>", "digest_value": "...", "signature_value": "..."}`.

## `zip`, `send`, `summary`, `ticket`, `status`, `cdr`

As implemented in `bindings/cli/src/commands.rs`; `send`, `summary` and `ticket` read the endpoint from
`--endpoint` or `SUNAT_ENDPOINT` (billService), `status` from `--endpoint` or `SUNAT_STATUS_ENDPOINT`
(billConsultService); all four read the credentials from flags or `SUNAT_SOL_USER`, `SUNAT_SOL_PASSWORD`,
and the HTTP wait from `--timeout` or `SUNAT_HTTP_TIMEOUT_SECONDS`.

- `summary`: stdin `{"name": "RUC-RC|RA-YYYYMMDD-N", "xml" | "xml_base64"}` (the signed daily summary or
  void communication) → `{"outcome": "ticket", "ticket": "..."}` or a fault. Store the ticket before
  anything else: it is the only handle on the outcome.
- `ticket`: stdin `{"ticket"}` → `{"outcome": "ticket_status", "status_code", "state", "cdr"?,
  "cdr_zip_base64"?}`; `state` is `processed` (0), `in_process` (98: ask again with the same ticket,
  never send again) or `processed_with_errors` (99).
- `ticket` and `status` are queries: when SUNAT answers an HTTP status other than 200/500 (it has
  answered 401 to a valid getStatus that succeeded when repeated) or the wait runs out, they print
  `{"outcome": "no_verdict", "reason": "http_status" | "timeout", "http_status": 401 | null, "message"}` (`ticket`
  adds the same `ticket`) with exit 0. It says nothing about the document: ask again, never send again.
  `send` and `summary` keep those cases as errors, since a send that got no answer may have arrived.
- A CDR's `class` is `observed` for code 0 with a note of 4000 and above.

## `validity` (Consulta Integrada)

SUNAT's REST «Consulta Integrada» (manual v2.0, 22/11/2018) for 01, 03, 07 and 08: an OAuth token
(`client_credentials`) and one query. Credentials of «Credenciales de API SUNAT» in SOL (not the SOL
user, not the GRE ones): `--client-id`/`SUNAT_API_CLIENT_ID`, `--client-secret`/`SUNAT_API_CLIENT_SECRET`.
`--timeout` bounds both calls together. **There is no test environment**: the defaults are production;
`--token-endpoint`/`SUNAT_VALIDITY_TOKEN_ENDPOINT` and `--endpoint`/`SUNAT_VALIDITY_ENDPOINT` replace the
URL bases (https only). Tests replay the manual's answers (`crates/sunat-sdk/tests/data/validity/`).

stdin: `{"ruc", "document_type", "series", "number", "issue_date" (YYYY-MM-DD or ISO 8601), "total"
(PayableAmount, up to 2 decimals), "requester_ruc"?}`; an input SUNAT would refuse exits 1 before any call.

- `{"outcome": "validity", "state_code", "state", "taxpayer_state_code", "domicile_condition_code",
  "observations", "message"}`; `state`: `not_reported` (0), `accepted` (1), `voided` (2),
  `printed_authorized` (3), `printed_not_authorized` (4), null otherwise. `observations` say where the data
  sent differs (the manual's example is `accepted` with a different issue date).
- `{"outcome": "query_error", "http_status", "error_code", "message"}`: `success: false` with HTTP 200, 400
  or 422 (`99` = malformed). About the query, not the voucher.
- `{"outcome": "no_verdict", "stage": "token" | "query", "reason", "http_status", "message"}`: any other
  HTTP status (401 included) or the wait ran out. Ask again.

Deciding without issuing again: no outcome is a reason to issue the operation with another number.
`not_reported` does not prove the voucher never arrived (a summary's ticket may be in process): reconcile
with `ticket` or `status` first; `voided` is spent; `printed_*` is outside the contract for an electronic
series. The token and the secret never reach stdout or stderr.

## Consumers

- `bindings/php`: Composer package; runs the binary with `proc_open`, writes the JSON to stdin, reads
  stdout and stderr, maps a non-zero exit to a typed exception carrying the `error` message.
- `validity` in the bindings: wasm `validityTokenRequest`/`validityTokenResponse`, `validityRequest`/
  `validityResponse`, `validityTimeout` (fetch options carry the `url`); Python the same as `validity_*`,
  plus `sunat_sdk.validity(...)` that makes both calls; PHP `SunatSdk::validity()` with `ApiCredentials`.
- `bindings/wasm`: exposes `build`, `sign`, `zip` and `cdr` in memory with the same input and output
  shapes. It does no network I/O: `soapRequest(exchange, input, user, password)` returns the `fetch`
  options (`method`, `headers`, `body`) for `send`, `summary`, `ticket` or `status` with this command's
  stdin as `input`; the caller POSTs them to the endpoint and passes the HTTP status and body to
  `soapResponse(exchange, input, status, body)`, or the seconds waited to `soapTimeout`, which return
  this command's stdout for the same answer, `no_verdict` included, and throw where it exits 1.
