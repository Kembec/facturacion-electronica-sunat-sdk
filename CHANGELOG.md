# Changelog

What changes for whoever integrates the SDK, by version. The format follows
[Keep a Changelog 1.1.0](https://keepachangelog.com/en/1.1.0/); versions and what counts as a breaking change are in
[`docs/releasing.md`](docs/releasing.md#versions). The public contract is the input model
([`docs/input-model.md`](docs/input-model.md)), the CLI's JSON and exit codes ([`docs/cli.md`](docs/cli.md)), the public
API of each binding and the C ABI ([`docs/c-abi.md`](docs/c-abi.md)). An entry marked **Breaking** says how to migrate.

Each entry names its pull request (`#n`) or, before pull requests, its commit.

## [Unreleased]

### Added

- Which input field a SUNAT code points at: `sunat_sdk::diagnose(document_type, code, message)` gives the node and value
  SUNAT names in the message, if any, and the candidate fields from SUNAT's validation rules of 26.08.2026 (01, 03, 07,
  08). The CLI's `send`, `summary`, `ticket` and `status` add `diagnosis` to their JSON, one element per code in the
  answer ([`docs/cli.md`](docs/cli.md#diagnosis)).
- Retention voucher (20): `kind: "retention"`, UBL 2.0 `Retention` built, signed and zipped (`RUC-20-R###-N.zip`)
  through the core and the CLI; SUNAT's «Retenciones1_0» rules that read only the document are checked before
  sending, each with its code (2619 rate of the regime, 2623 amounts and exchange rate, 2626 payment number, 2628 and
  2629 totals, among others). The send term (2600) is a parameter of `Retention::check_send_deadline`. Not sent to
  SUNAT beta yet; sending by `otroscpe` comes next (#51).

## [0.1.0-beta.5] - 2026-10-17

### Added

- Sender's despatch advice (09) by SUNAT's GRE REST API: `gre-send` and `gre-ticket` in the CLI and the core
  (token by password, send with `hash_zip`, ticket query). SUNAT has no test environment for GRE, so nothing here is
  labelled accepted by SUNAT (#30).
- GRE in the bindings: PHP `greSend()`/`greTicket()`, Python `gre_send`/`gre_ticket` and the `gre_*` helpers,
  JavaScript `greTokenRequest` … `greTimeout` without I/O (#31).
- PHP `summary()` and `ticket()` for daily summaries and void communications, with `SummaryResult` and
  `TicketResult`, so PHP covers the whole RC/RA flow the CLI already had (#46).
- C ABI (`bindings/ffi`): every CLI command in process, flags in the request and no environment, on six platforms;
  contract in `docs/c-abi.md` (#36).
- .NET package `Kembec.SunatSdk` (C# and VB.NET; .NET Framework 4.6.2+, .NET Standard 2.0, .NET 10+) over the C ABI,
  built and tested in CI; not on NuGet yet (#37).
- Java package `com.kembec:sunat-sdk` (Java 8+) over the C ABI through JNA, with the native builds in the jar; not on
  Maven Central yet (#38).
- Ruby gem `kembec-sunat_sdk` over the C ABI, one gem per platform, Ruby 3.2 to 4.0 (#39, #42).
- Perl distribution `Kembec-SunatSdk` (module `Kembec::SunatSdk`, Perl 5.26+) over the C ABI (#40).
- Windows zip for VB6, Visual FoxPro and PowerBuilder over the C ABI, 32 and 64 bits (#41).
- Swift package and Objective-C client (`KembecSunatSDKObjC`, macOS 10.15+, iOS 13+) over the C ABI, with the
  xcframework and the podspec (#43).
- The five library crates are published on crates.io, from this release on (#44, #48).

### Changed

- **Breaking** — The five library crates are renamed `kembec-sunat-model`, `kembec-sunat-transport`,
  `kembec-sunat-sdk`, `kembec-sunat-xmldsig` and `kembec-sunat-ubl`, and their libraries `kembec_sunat_*`.
  Migration: depend on `kembec-sunat-sdk` (and the others) instead of `sunat-sdk`, or keep the short key with
  `package = "kembec-sunat-sdk"`, and change `use sunat_sdk::…` to `use kembec_sunat_sdk::…` (#44).
- **Breaking** — A CDR with code 0 and any note is now `observed`, whatever the note's code; before, only notes of 4000
  and above were, and 2404 or 2988 came out `accepted`. Migration: treat `observed` as accepted with notes to read,
  as before; code that keyed on `accepted` to mean "no notes" now sees those CDRs as `observed` (#45).
- `status` without a verdict: resend the same signed XML with the same number while the sending deadline is open
  (SUNAT answers 1033, 1032 or 0140 for a document already on record); a `summary` with no answer is checked with
  `validity` and resent under the same name (#34).

### Fixed

- The PHP fixture-count test names the 38 vectors it checks (#35).

## [0.1.0-beta.4] - 2026-10-16

### Added

- `validity` (SUNAT's Consulta Integrada) for 01, 03, 07 and 08: in the CLI, PHP (`ValidityResult`,
  `ApiCredentials`), JavaScript (`validityTokenRequest` … `validityTimeout`) and Python (`validity`, `validity_*`)
  (#24).
- Sender's despatch advice (09): shipment rules, UBL rendering, XSD validation and signing; no sending yet (#23).
- Detraction in invoices and receipts with operation 1001 and 1003 (#26).
- IGV retention in the voucher (catalog 53 code 62) in 01, 03, 07 and 08 (#27).
- IVAP (affectation 17) in 01, 03, 07 and 08 (#28).
- Purchase order and related despatch guide references in 01, 03, 07 and 08 (#25).

## [0.1.0-beta.3] - 2026-10-16

Built from `main` (#18 to #22), whose first commit `19a6be4` carries the work merged after beta.1; beta.2 was cut
from a side branch, and everything it added is here too.

### Added

- Daily summaries (RC) and void communications (RA): `summary` sends them and returns a ticket, `ticket` reads its
  state (`processed`, `in_process`, `processed_with_errors`) and its CDR (`19a6be4`).
- Line discounts (catalog 53 code `00`) in invoices and receipts (`19a6be4`).
- JavaScript: `zip`, `cdr`, and the SUNAT exchange without I/O (`soapRequest`, `soapResponse`, `soapTimeout`) for
  `send`, `summary`, `ticket` and `status` (`19a6be4`).
- Python package `facturacion-electronica-sunat-sdk` on PyPI: abi3 wheels over the same core, and `exchange` for the
  POST to SUNAT (#19).
- Optional `igv_rate` and `customer_name` in daily summary lines, and IGV 10.5 % in invoices (#20).
- PHP `StatusResult::isNoVerdict()`, `getNoVerdictReason()` and `getHttpStatus()` (`19a6be4`).

### Changed

- **Breaking** — `ticket` and `status` print `{"outcome": "no_verdict", "reason": "http_status" | "timeout", …}` with
  exit 0 when SUNAT answers an HTTP status other than 200/500 or the wait runs out; in beta.2 that was an error.
  Migration: check `outcome` before reading the CDR, and on `no_verdict` ask again with the same data; never send the
  document again because of it (`19a6be4`).
- **Breaking** — A CDR with code 0 and a note of 4000 and above has `class` `observed` instead of `accepted`.
  Migration: treat `observed` as accepted with notes to read (`19a6be4`).

## [0.1.0-beta.2] - 2026-10-16

### Added

- The wait on SUNAT in `send` and `status` is configurable: `--timeout` or `SUNAT_HTTP_TIMEOUT_SECONDS` in the CLI
  (whole seconds, 1 to 300, default 90), `$httpTimeoutSeconds` in the PHP constructor (`da368d4`).
- Signing reads a combined PEM (certificate and key in one file, in either order, LF or CRLF, with `Bag Attributes`,
  the key as PKCS#8 or PKCS#1) (`da368d4`).

### Changed

- RSA-SHA1 signing no longer depends on the `rsa` crate; signatures are byte for byte the same (`da368d4`, `7d3c986`).

### Fixed

- npm publishing takes the tarball as `./npm/*.tgz` (`b602a58`).

## [0.1.0-beta.1] - 2026-10-16

### Added

- Invoices (01), receipts (03), credit notes (07) and debit notes (08) in UBL 2.1 with IGV: build from the JSON input
  model, XML-DSig signing, zip, `send` and `status` to SUNAT, and the CDR read and classified (`703a8df`).
- The CLI `sunat-sdk` (`build`, `sign`, `zip`, `send`, `status`, `cdr`) as a static Linux binary with its SHA-256
  (`ea30563`, `8921926`).
- PHP package on Packagist over the CLI binary, with a verified binary installer (`ea30563`, `a8bef31`).
- npm package with the WebAssembly binding (`build` and `sign` in memory) for Node and the browser (`01ad245`).
- Compatibility vectors: every fixture gives the exact `DigestValue` and `SignatureValue` of its vector (`703a8df`,
  `0b840c9`).

[Unreleased]: https://github.com/Kembec/facturacion-electronica-sunat-sdk/compare/v0.1.0-beta.5...HEAD
[0.1.0-beta.5]: https://github.com/Kembec/facturacion-electronica-sunat-sdk/compare/v0.1.0-beta.4...v0.1.0-beta.5
[0.1.0-beta.4]: https://github.com/Kembec/facturacion-electronica-sunat-sdk/compare/v0.1.0-beta.3...v0.1.0-beta.4
[0.1.0-beta.3]: https://github.com/Kembec/facturacion-electronica-sunat-sdk/compare/v0.1.0-beta.2...v0.1.0-beta.3
[0.1.0-beta.2]: https://github.com/Kembec/facturacion-electronica-sunat-sdk/compare/v0.1.0-beta.1...v0.1.0-beta.2
[0.1.0-beta.1]: https://github.com/Kembec/facturacion-electronica-sunat-sdk/releases/tag/v0.1.0-beta.1
