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

## Consumers

- `bindings/php`: Composer package; runs the binary with `proc_open`, writes the JSON to stdin, reads
  stdout and stderr, maps a non-zero exit to a typed exception carrying the `error` message.
- `bindings/wasm`: exposes `build`, `sign`, `zip` and `cdr` in memory with the same input and output
  shapes. It does no network I/O: `soapRequest(exchange, input, user, password)` returns the `fetch`
  options (`method`, `headers`, `body`) for `send`, `summary`, `ticket` or `status` with this command's
  stdin as `input`; the caller POSTs them to the endpoint and passes the HTTP status and body to
  `soapResponse(exchange, input, status, body)`, or the seconds waited to `soapTimeout`, which return
  this command's stdout for the same answer, `no_verdict` included, and throw where it exits 1.
