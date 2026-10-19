# Changelog

What changes for whoever integrates the SDK, by version. The format follows
[Keep a Changelog 1.1.0](https://keepachangelog.com/en/1.1.0/); versions and what counts as a breaking change are in
[`docs/releasing.md`](docs/releasing.md#versions). The public contract is the input model
([`docs/input-model.md`](docs/input-model.md)), the CLI's JSON and exit codes ([`docs/cli.md`](docs/cli.md)), the public
API of each binding and the C ABI ([`docs/c-abi.md`](docs/c-abi.md)). An entry marked **Breaking** says how to migrate.

Each entry names its pull request (`#n`) or, before pull requests, its commit.

## [Unreleased]

### Breaking

- Invoices (01) and receipts (03) are checked before signing for three line rules of the 2026-04-24 sheet, within
  SUNAT's tolerance of 1: a receipt line with a discount or a charge for `line_value` = `unit_value` × `quantity` −
  discounts `00` + charges `47` (observation 4288, «Boleta2_0» row 274; invoices were already held to 3271); every
  line for `unit_price` = (`line_value` + `total_taxes` − discounts `01` + charges `48`) ÷ `quantity`, free lines
  aside (3270 in an invoice, row 183; observation 4287 in a receipt, row 176); and every line with affectation 10 to
  17 for `igv` = `igv_base` × `igv_rate` (error 3103 in both, rows 213 and 206). Such a document used to be signed and
  was rejected by SUNAT (3103, 3270, 3271) or accepted with an observation (4287, 4288); observations are rejected
  here like the errors. Fix the amounts at the source (`docs/input-model.md`, Line) (#PR).

### Changed

- Every invoice and receipt the SDK builds from `fixtures/` is validated, signed, against the OASIS UBL 2.1 `Invoice`
  schema (`tests/xsd/ubl-2.1/maindoc/UBL-Invoice-2.1.xsd`), as the despatch advices already were (#PR).

## [1.0.0-rc.5] - 2026-10-19

### Breaking

- The sender's despatch advice (09) refuses, before signing, the free text SUNAT's «Guía-Remitente2_0» sheet observes
  by format: `note` (4186), the issuer's, recipient's, seller's and buyer's `legal_name` (4338, 4152, 4106, 4381), a
  related document's `description` (4372), `shipment.reason_description` with reason 13 (4190), the carrier's
  `legal_name` and `mtc_registration` (4165, 4392), the drivers' `first_name` and `family_name` (4409), the origin and
  destination `address` (4076, 4068) and a line's `description` and `product_code` (4084, 4085). A value with a line
  break, a tab or another whitespace other than the space, outside the sheet's length, or of only spaces where the
  sheet says so, used to be signed and would have reached SUNAT as an observation; a line's `description` still
  allows line breaks and is only measured. SUNAT has no test environment for despatch advices, so the sheet is the
  only source. The value is refused with its field, never rewritten: fix it at the source (`docs/input-model.md`,
  Despatch advice (09)) (#86).
- Credit (07) and debit (08) notes are checked before signing for `totals.total_taxes` = `igv` + `ivap` + `icbper`,
  within 1, as invoices and receipts are since #78: SUNAT answers 3294 for a note on an invoice (01) and 4301 for a note
  on anything else (observation, rejected here like the errors; NotaCredito2_0 rows 286-287, NotaDebito2_0 rows
  269-270 of the 2026-04-24 sheet). Until now only notes with ICBPER were checked, so a note with a mismatched total
  was signed and SUNAT rejected or observed it. Send `total_taxes` as the sum of the note's global taxes (#85).

### Added

- `sunat-sdk --version` and `-V` print `sunat-sdk <version>` on one line and exit 0; they used to answer with the
  usage and exit 1 (`docs/cli.md`, `--version`) (#87).

## [1.0.0-rc.4] - 2026-10-18

### Breaking

- 01, 03, 07 and 08 refuse, before signing, the issuer's `legal_name` (4338), `trade_name` (4092), `address.street`
  (4094), `address.urbanization` (4095), `address.province` (4096), `address.department` (4097) and
  `address.district` (4098), and a line's `product_code` (4269 in 01 and 03, 4234 in 07 and 08), when they carry a line
  break, a tab or another whitespace other than the space, or fall outside the length SUNAT's sheet sets. They used to
  reach SUNAT and come back accepted with an observation (beta answered 4094 to a street with a line break on
  2026-10-18). The value is refused with its field, never rewritten: fix it at the source (`docs/input-model.md`,
  Issuer) (#83).

### Fixed

- The daily summary writes the line's `igv_rate` as `cbc:Percent` in the IGV of free sales (tax 9996) too, as the
  Resumen Diario sheet of the 2026-04-24 rules asks (2992, 3504). Free sales enter the summary from 2027-01-01, so no
  summary sent today changes (#82).

## [1.0.0-rc.3] - 2026-10-18

### Added

- The purchase settlement (04), UBL 2.1 `SelfBilledInvoice`, operation types 0501 and 0503: `kind: "self_billed"` with
  `seller` (DNI, carné de extranjería or passport), `operation_place`, the income tax retention per line (tax 3000) and
  the four gold properties, with the SUNAT checks that read only the document
  ([`docs/input-model.md`](docs/input-model.md#purchase-settlement-04)). In the beta, `v1` and `v2` were accepted with
  code 0 (`fixtures/self-billed/beta/`) (#74).
- The reversal summary (RR) of a purchase settlement: `kind: "reversal"`, the void communication's `VoidedDocuments`
  named `RR-YYYYMMDD-N` with type 04 lines, checked with 2308, 2345, 2312, 2348, 2315, 4203, 2671, 2220 and 2958 (more
  than 6 days after `generation_date`: SUNAT allows 5 from the settlement's CDR)
  ([`docs/input-model.md`](docs/input-model.md#reversal-summary-kind-reversal-rr)). `summary` and `ticket` accept the
  name `RUC-RR-YYYYMMDD-N`; it goes to SUNAT's `otroscpe` service: in the beta (18/10/2026) the invoice `billService`
  answered 0151 and `otroscpe` gave a ticket with status 0 (`fixtures/self-billed/beta/v3-*`) (#75).
- `diagnosis` of a reversal summary (`RUC-RR-...`) points 2220, 2671, 2308, 2345, 2958, 2312, 2348, 2315 and 4203 at
  their `kind: "reversal"` field (`crates/sunat-sdk/data/error-fields.json`, 9 rows of type `RR`) (#75).
- Vectors for the export types 0203 to 0207 in `fixtures/export/` (v5 to v9), built, signed and checked by the
  independent verifier; in the beta (18/10/2026) all five were accepted with code 0 and no notes (#76).
- 3272 and 4294: `lines[].igv_base` must equal `lines[].line_value` to the cent, in every operation type (the SDK does
  not model ISC). 3272 in an invoice and in a note that modifies one; 4294 in a receipt and in a note that modifies one,
  an observation for SUNAT that the SDK rejects (#76).
- Swift Package Manager from a tag: a release attaches `SunatSDKFFI.xcframework.zip`, the zip `swift.yml` built and
  whose URL and checksum the tagged `Package.swift` names (`bindings/swift/scripts/pin-release.sh`) (#77).

### Changed

- **Breaking** — Rust only: `Document` gains the variant `SelfBilled`. A `match` on `Document` adds the arm (or `_`);
  the JSON input and the bindings do not change (#74).
- **Breaking** — Rust only: `Document` gains the variant `Reversal(Voided)` for the reversal summary (RR). A `match` on
  `Document` adds the arm (or `_`); `Voided` gains `reversal_identifier()`. The JSON input only adds the kind (#75).
- Catalog 04 is a list, not a format: `service_use_country` (3099) and the guest's `passport_country` and
  `residence_country` (4280) take ISO 3166-1 alpha-2 in force (`crates/sunat-model/data/catalog-04.json`, with its
  sources). A two-letter code outside it, such as `ZZ` or the withdrawn `AN` and `CS`, is now rejected. In the beta,
  `RS` was accepted and `AN` came back as 3099 (#76).
- `diagnose` maps 3098 and 3099 to `service_use_country` (#76).

### Fixed

- Every invoice and receipt is checked before signing for `totals.total_taxes` = `igv` + `ivap` + `icbper`, within 1
  (one unit of the currency, like the other totals): SUNAT answers 3294 for an invoice and 4301 for a receipt (rejected here like the errors). Until now only exports
  and ICBPER documents were checked, so a plain invoice with a mismatched total was signed and rejected by SUNAT (#78).

## [1.0.0-rc.2] - 2026-10-18

### Added

- Export in invoices and receipts, operation types 0200 to 0208 of catalog 51: `service_use_country` (0201, 0208)
  renders `cac:Delivery` with the country only, and `lines[].lodging` (0202 with the ten guest fields, 0205 with four)
  renders the catalog 55 properties 4000 to 4009. Checked before sending: 3206, 3098, 3099, 2800, 2801, 2802, 2642,
  3110, 3102, 3136 to 3145, 4280, 4281, 4282, 3107, 3003, 3273, 3294, 3278, 3279 and 3280; advances, IGV retention,
  free lines and ICBPER in an export are out of scope. `stay_days` above 60 is not rejected: the SDK does not decide
  what is an export. In the beta (18/10/2026) 0200, 0201, 0208 and 0202 were accepted with code 0 and no notes, and
  2800 and 3145 came back as the SDK says; 0208 without the country was accepted, and the SDK keeps 3098 as the
  validation sheet says. Answers in `fixtures/export/beta/` (#72).
- An OSE's answer reads like SUNAT's (`read_cdr`, the fault parsers): the CDR's number comes from
  `cac:DocumentReference/cbc:ID` when `cac:Response/cbc:ReferenceID` is missing, each `cac:Response/cac:Status` enters
  `notes` as `"<code> - <reason>"` (so a code 0 with an observation is `observed` and gets its `diagnosis`), and a
  fault's `message` is the text of `detail` when it has one. SUNAT's own answers read as before. Synthetic vectors in
  `fixtures/ose/` (#69).

### Changed

- **Breaking** — Rust only: `Line` gains `lodging` and `Invoice` gains `service_use_country`. The JSON input and the
  other bindings are unchanged for a document without them. Migration: add `lodging: None` and
  `service_use_country: None` where you build these structs (#72).

### Fixed

- `docs/cli.md` no longer tells you to send a despatch advice again after a `gre-send` with no answer as if SUNAT
  documented it: its GRE API manual does not say what a repeated send returns; the guide's rules reject a number
  already registered (1033, 1032), and the first CDR is looked up in SUNAT Operaciones en Línea (#73).

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

[Unreleased]: https://github.com/Kembec/facturacion-electronica-sunat-sdk/compare/v1.0.0-rc.5...HEAD
[1.0.0-rc.5]: https://github.com/Kembec/facturacion-electronica-sunat-sdk/compare/v1.0.0-rc.4...v1.0.0-rc.5
[1.0.0-rc.4]: https://github.com/Kembec/facturacion-electronica-sunat-sdk/compare/v1.0.0-rc.3...v1.0.0-rc.4
[1.0.0-rc.3]: https://github.com/Kembec/facturacion-electronica-sunat-sdk/compare/v1.0.0-rc.2...v1.0.0-rc.3
[1.0.0-rc.2]: https://github.com/Kembec/facturacion-electronica-sunat-sdk/compare/v1.0.0-rc.1...v1.0.0-rc.2
[1.0.0-rc.1]: https://github.com/Kembec/facturacion-electronica-sunat-sdk/compare/v0.1.0-beta.8...v1.0.0-rc.1
[0.1.0-beta.8]: https://github.com/Kembec/facturacion-electronica-sunat-sdk/compare/v0.1.0-beta.7...v0.1.0-beta.8
[0.1.0-beta.7]: https://github.com/Kembec/facturacion-electronica-sunat-sdk/compare/v0.1.0-beta.6...v0.1.0-beta.7
[0.1.0-beta.6]: https://github.com/Kembec/facturacion-electronica-sunat-sdk/compare/v0.1.0-beta.5...v0.1.0-beta.6
[0.1.0-beta.5]: https://github.com/Kembec/facturacion-electronica-sunat-sdk/compare/v0.1.0-beta.4...v0.1.0-beta.5
[0.1.0-beta.4]: https://github.com/Kembec/facturacion-electronica-sunat-sdk/compare/v0.1.0-beta.3...v0.1.0-beta.4
[0.1.0-beta.3]: https://github.com/Kembec/facturacion-electronica-sunat-sdk/compare/v0.1.0-beta.2...v0.1.0-beta.3
[0.1.0-beta.2]: https://github.com/Kembec/facturacion-electronica-sunat-sdk/compare/v0.1.0-beta.1...v0.1.0-beta.2
[0.1.0-beta.1]: https://github.com/Kembec/facturacion-electronica-sunat-sdk/releases/tag/v0.1.0-beta.1
