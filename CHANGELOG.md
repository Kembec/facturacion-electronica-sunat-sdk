# Changelog

What changes for whoever integrates the SDK, by version. The format follows
[Keep a Changelog 1.1.0](https://keepachangelog.com/en/1.1.0/); versions and what counts as a breaking change are in
[`docs/releasing.md`](docs/releasing.md#versions). The public contract is the input model
([`docs/input-model.md`](docs/input-model.md)), the CLI's JSON and exit codes ([`docs/cli.md`](docs/cli.md)), the public
API of each binding and the C ABI ([`docs/c-abi.md`](docs/c-abi.md)). An entry marked **Breaking** says how to migrate.

Each entry names its pull request (`#n`) or, before pull requests, its commit.

## [Unreleased]

## [1.0.0-rc.1] - 2026-10-18

The first release candidate: the contract of `0.1.0-beta.8`, unchanged.

### Changed

- Release policy: `1.0.0-rc.N` candidates, and `1.0.0` after 15 days of use in production by a real issuer, with its
  CDRs monitored and every rejection the SDK caused fixed ([`docs/releasing.md`](docs/releasing.md#versions)). It
  replaces «two betas in a row without a **Breaking** entry» (#70).
- The CPAN version of a candidate takes `_(100 + N)`, so it sorts after the betas: `1.0.0-rc.1` is `0.999999_101` (#70).

## [0.1.0-beta.8] - 2026-10-18

### Added

- Advances (F04) in invoices and receipts: `advances[]` and `totals.prepaid` render the `cac:AdditionalDocumentReference`,
  the `cac:PrepaidPayment`, the discount 04, 05 or 06 and the `PrepaidAmount`, with the SUNAT checks (2505, 2521, 3217,
  2503, 2968, 3307, 3220, 2509, 3275, 3274, 3277, 3279, 3280, 3291) and, in the beta, vectors accepted with code 0
  (`fixtures/advances/beta/`). `advances` no longer fails as out of scope; in a note it still does (#67).
- The .NET package `Kembec.SunatSdk` is published to NuGet with the tag: `release.yml` packs it with the six native
  builds (`nuget-package` artifact) and `publish-nuget` uploads it behind the `release` environment (#64).

### Changed

- **Breaking** — Rust only: `Invoice` gains `advances` and `Totals` gains `prepaid`. The JSON input and the
  other bindings are unchanged for a document without advances. Migration: add `advances: vec![]` and `prepaid: None`
  where you build these structs (#67).
- **Breaking**: `igv_rate` is required on every summary (RC) line. The writer always puts tax 1000 on the line, and the
  SUNAT Resumen Diario sheet rejects it without `cbc:Percent` (error 2992, «Tasa del tributo faltante para código
  1000»; the beta rejected a line without it on 17/10/2026). A line without it now fails before sending with SUNAT code
  2992 at `$.lines[n].igv_rate`, whatever its status or amounts. Migration: add `"igv_rate": "18.00"` (or `"10.50"`;
  a note also takes `"10.00"`) to every line of the summaries you build; the RC fixtures and their vectors carry it
  (#66).
- ICBPER sent to the SUNAT beta (17/10/2026), which `0.1.0-beta.7` had only checked locally: the five valid vectors
  (invoice, receipt, free-bags receipt, credit note, daily summary) were accepted with code 0 and no notes; 4318 came
  back as an observation and 3306 as a rejection, as the SDK says; 4237 (stale rate) was accepted with no note, so
  it is a rule of the SDK alone. The daily summary line needed `igv_rate`: without it the beta answers 2992. Answers
  in `fixtures/icbper/beta/` (#65).

### Fixed

- The rejection of a summary reaches the integrator: the beta puts its message, with non-ASCII letters, in the status
  line of an HTTP 200 (`HTTP/1.1 200 Error en la linea: 1. Tasa del tributo faltante para código 1000.: 2992 ...`), and
  the CLI's HTTP client (`ureq` 2) refused that line, so `send`, `summary`, `ticket` and `status` printed only
  `transport error: Bad Status`. The CLI now uses `ureq` 3, whose parser accepts the reason phrase RFC 9112 allows
  (`obs-text`); the SOAP fault in the body gives its code and `diagnosis`. TLS verification is unchanged (#66).

## [0.1.0-beta.7] - 2026-10-17

### Added

- `diagnosis` in the .NET and Java bindings: `SendResult`, `SummaryResult`, `TicketResult` and `StatusResult` add
  `Diagnosis` (.NET) and `getDiagnosis()` (Java), a read-only list of `Diagnosis` (code, `Detail` with node and value,
  `Fields` with `InputPath`, `Xpath` and `Certainty`), empty when the answer has none. Their batteries parse the beta's
  real 2800 and a CDR with two coded notes (`bindings/dotnet/tests/data`, `bindings/java/tests/data`).
- `diagnosis` in the Ruby and Perl bindings: the `send_bill`, `summary`, `ticket` and `status` results add `diagnosis`
  (Ruby `Diagnosis`, Perl `Kembec::SunatSdk::Diagnosis`), a list with `code`, `detail` (`node`, `value`) and `fields`
  (`input_path`, `xpath`, `certainty`), empty when the answer has none; the beta's 2800 gives `customer.document_type`
  on detail `cbc:ID/schemeID` = `1`. The legacy README (VB6, Visual FoxPro, PowerBuilder) says where `diagnosis` is in
  the JSON and how to read it.
- `diagnosis` in the Swift and Objective-C packages: `SendResult`, `SummaryResult`, `TicketResult` and `StatusResult`
  add `diagnosis`, a list of `Diagnosis` (`code`, `detail`, `fields` with `inputPath`, `xpath` and `certainty`), empty
  when the response carries none; Objective-C's `KSSSendResult`, `KSSSummaryResult`, `KSSTicketResult` and
  `KSSStatusResult` add `diagnosis` with `KSSDiagnosis`, `KSSDiagnosisDetail` and `KSSDiagnosisField`. The batteries of
  both read the beta's 2800 and point it at `customer.document_type`.
- `diagnosis` in the Python binding (`soap_response`, `soap_timeout`, `exchange`, and `diagnose(document_type, code,
  message)`) and through the C ABI, whose `send`, `summary`, `ticket` and `status` run the CLI's commands. The rule
  that picks the document type lives in `sunat_sdk::diagnosis::with_diagnosis`, shared by the CLI and wasm.
- Rules that take effect on 01/01/2027 (RS 143-2026 art. 3.4 over RS 048-2026), each applied by the date of its norm
  from a table of the SDK with its source: in the daily summary, free sales by catalog 11 code (`free_taxed`,
  `free_exempt`, `free_unaffected`, `free_export` for 06 to 09) and their IGV (`free_igv`, tax 9996) from a
  `reference_date` of 2027-01-01, with `free` (05) rejected from then and 06 to 09 rejected (2256) before; debit note
  13 «Penalidades» from an `issue_date` of 2027-01-01 on unaffected lines only (3507), `03` rejected from then (2524)
  and `13` before. Documents dated before the cut build the same XML as today. The SUNAT product code (3496) stays
  unenforced: it has no norm ([`docs/input-model.md`](docs/input-model.md#rules-that-take-effect-on-01012027)).

### Changed

- **Breaking** — Rust only: `SummaryLine` gains `free_taxed`, `free_exempt`, `free_unaffected`, `free_export` and
  `free_igv`, `Line` gains `icbper` and `icbper_factor`, and `Totals` and `SummaryLine` gain `icbper`, so code that
  builds them with a struct literal no longer compiles. Migration: add the new fields as `None`. The JSON input, the
  CLI and the other bindings do not change for documents without ICBPER or free sales split by code.
- **Breaking** — Despatch advice (09) in public transport carries the day the goods are handed to the carrier, which
  SUNAT requires since 2026-06-01 (field 39-A of RS 108-2026): `shipment.carrier_handover_date`, rendered as
  `cac:LoadingTransportEvent/cbc:OccurrenceDate`. Without it the SDK answers 3617 locally, before sending; a date before
  the issue date is 3618 and a malformed one 3619. Private transport takes no such field
  ([`docs/input-model.md`](docs/input-model.md#shipment)). Migration: a public-transport despatch advice that built
  before now needs `carrier_handover_date`; SUNAT already rejected it without one.
- ICBPER (tax 7152, plastic bags) in invoices, receipts, credit and debit notes and daily summary lines, which the
  SDK used to reject: `lines[].icbper`, `lines[].icbper_factor` and `totals.icbper` render the line and document
  subtotals 7152, and a summary line's `icbper` its own `cac:TaxTotal`. The rate per bag is a dated table of the SDK
  (S/ 0.50 from 2023) read on the year of `issue_date`. Checked before sending with SUNAT's codes (2033, 2048, 2892,
  3238, 2949, 3292, 3294, 3306, 3279, 3280; a receipt gets the observation code), and the observations 4318 (amount
  ≠ rate × bags) and 4237 (rate not the one in force) are rejected too. PEN only. Not sent to SUNAT beta yet
  (#55, [`docs/input-model.md`](docs/input-model.md#icbper)).

## [0.1.0-beta.6] - 2026-10-17

### Added

- Which input field a SUNAT code points at: `sunat_sdk::diagnose(document_type, code, message)` gives the node and value
  SUNAT names in the message, if any, and the candidate fields from SUNAT's validation rules of 26.08.2026 (01, 03, 07,
  08). The CLI's `send`, `summary`, `ticket` and `status` add `diagnosis` to their JSON, one element per code in the
  answer ([`docs/cli.md`](docs/cli.md#diagnosis)).
- `diagnosis` in the PHP and WebAssembly bindings: PHP's `SendResult`, `StatusResult`, `SummaryResult` and
  `TicketResult` add `getDiagnosis()`, a list of `Diagnosis` (`getCode()`, `getDetail()`, `getFields()`), empty when the
  binary sends none; wasm's `soapResponse` and `soapTimeout` add `diagnosis` as the CLI does, `diagnose(documentType,
  code, message)` diagnoses one code, and `index.d.ts` types both (#52).
- Retention voucher (20): `kind: "retention"`, UBL 2.0 `Retention` built, signed and zipped (`RUC-20-R###-N.zip`)
  through the core and the CLI; SUNAT's «Retenciones1_0» rules that read only the document are checked before
  sending, each with its code (2619 rate of the regime, 2623 amounts and exchange rate, 2626 payment number, 2628 and
  2629 totals, among others). The send term (2600) is a parameter of `Retention::check_send_deadline` (#51). Sent by
  the CLI to SUNAT beta (`otroscpe`): the three valid vectors accepted with code 0; 2619 and 2626 answered as the SDK
  predicts; a line that breaks both the exchange rate and its amounts is now reported as 2715 before 2623, as the beta
  answers ([`docs/input-model.md`](docs/input-model.md), `fixtures/retention/beta/`).

### Changed

- **Breaking** — Rust only: `Line` gains `charges`, `Invoice` gains `global_adjustments` and `Totals` gains
  `allowance_total` and `charge_total`, so code that builds them with a struct literal no longer compiles. Migration:
  add `charges: vec![]`, `global_adjustments: vec![]`, `allowance_total: None` and `charge_total: None` (or
  `..Default::default()` in `Totals`). `LineDiscount` stays, as another name for `AllowanceCharge`. The JSON input,
  the CLI and the other bindings do not change (#54).
- Catalog 53 discounts and charges in invoices and receipts, which the SDK used to reject: line discount `01` and
  line charges `47` and `48` (`lines[].charges`), and global discounts `02`, `03` and charges `45`, `46`, `49`, `50`
  (`global_adjustments`), rendered as `cac:AllowanceCharge` with `ChargeIndicator` derived from the code;
  `totals.allowance_total` and `totals.charge_total` render as `cbc:AllowanceTotalAmount` and
  `cbc:ChargeTotalAmount`. A document with any discount or charge has its arithmetic checked before sending, each rule
  with SUNAT's code (3290 and 3307 `amount` = `base_amount` × `factor`, 3271 line value, 3277 net IGV base, 3291 IGV,
  3278, 3279, 3280, 3300, 3301); a receipt gets the observation code. The FISE (45) stays outside the IGV base. Not
  sent to SUNAT beta yet ([`docs/input-model.md`](docs/input-model.md#discounts-and-charges)).
- `sunat_model::LineDiscount` is now an alias of `AllowanceCharge`; `Line` gains `charges`, `Invoice` gains
  `global_adjustments` and `Totals` gains `allowance_total` and `charge_total`: Rust code that builds these structs
  field by field adds them (empty or `None`). JSON input is unchanged for documents without discounts or charges.

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

[Unreleased]: https://github.com/Kembec/facturacion-electronica-sunat-sdk/compare/v1.0.0-rc.1...HEAD
[1.0.0-rc.1]: https://github.com/Kembec/facturacion-electronica-sunat-sdk/compare/v0.1.0-beta.8...v1.0.0-rc.1
[0.1.0-beta.8]: https://github.com/Kembec/facturacion-electronica-sunat-sdk/compare/v0.1.0-beta.7...v0.1.0-beta.8
[0.1.0-beta.7]: https://github.com/Kembec/facturacion-electronica-sunat-sdk/compare/v0.1.0-beta.6...v0.1.0-beta.7
[0.1.0-beta.6]: https://github.com/Kembec/facturacion-electronica-sunat-sdk/compare/v0.1.0-beta.5...v0.1.0-beta.6
[0.1.0-beta.5]: https://github.com/Kembec/facturacion-electronica-sunat-sdk/compare/v0.1.0-beta.4...v0.1.0-beta.5
[0.1.0-beta.4]: https://github.com/Kembec/facturacion-electronica-sunat-sdk/compare/v0.1.0-beta.3...v0.1.0-beta.4
[0.1.0-beta.3]: https://github.com/Kembec/facturacion-electronica-sunat-sdk/compare/v0.1.0-beta.2...v0.1.0-beta.3
[0.1.0-beta.2]: https://github.com/Kembec/facturacion-electronica-sunat-sdk/compare/v0.1.0-beta.1...v0.1.0-beta.2
[0.1.0-beta.1]: https://github.com/Kembec/facturacion-electronica-sunat-sdk/releases/tag/v0.1.0-beta.1
