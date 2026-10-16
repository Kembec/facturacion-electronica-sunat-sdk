# Input model

One JSON document per voucher, snake_case English field names. Decimal amounts are JSON strings
(`"118.00"`), never floats. Amounts are given by the caller: the SDK renders and validates, it does not
compute taxes.

Documents: invoice `01`, receipt `03`, credit note `07`, debit note `08`, UBL 2.1, IGV (affectations
10-16, 20, 21, 30-37, 40) and IVAP (affectation 17, see IVAP). Not supported, rejected with an
explicit error: ISC, ICBPER, other taxes, global discounts or charges, line charges, line
discounts other than catalog 53 code `00` or in notes, advances, detraction on operation types 1002 and
1004, perception.

The daily summary (RC), the void communication (RA) and the sender's despatch advice (09) have their
own sections below.

## Document

| Field | Notes |
|---|---|
| `kind` | `"invoice"` (01, 03) or `"note"` (07, 08) |
| `ubl_version` | `"2.1"` |
| `operation_type` | catalog 51, invoice only (`"0101"`; `"1001"` or `"1003"` with `detraction`) |
| `document_type` | `"01"`, `"03"`, `"07"`, `"08"` |
| `series` | `"F001"`, `"B001"` |
| `number` | string, no padding added |
| `issue_date` | ISO 8601 with offset, `"2026-10-15T10:30:00-05:00"` |
| `due_date` | optional, invoice only |
| `currency` | `"PEN"`, `"USD"` |
| `detraction` | invoice and receipt only, see Detraction |
| `payment_terms` | `{ "type": "Contado" }` or `{ "type": "Credito", "amount": "...", "currency": "PEN", "installments": [{ "amount": "...", "currency": "PEN", "due_date": "2026-11-15T00:00:00-05:00" }] }` |
| `issuer` | see Issuer |
| `customer` | see Customer |
| `totals` | see Totals |
| `lines` | see Line |
| `legends` | `[{ "code": "1000", "value": "..." }]` |
| `note` | only for 07/08, see Note |
| `order_reference` | optional, the purchase or service order number, see Related documents |
| `despatch_references` | optional, `[{ "document_type": "09", "number": "T001-45" }]`, see Related documents |
| `igv_retention` | optional, 01/03/07/08, see IGV retention |

## Related documents

`order_reference` renders as `cac:OrderReference/cbc:ID` and each `despatch_references` entry as a
`cac:DespatchDocumentReference` (`cbc:ID` the guide, `cbc:DocumentTypeCode` catalog 01), in 01, 03, 07 and 08. In a
note they go around `cac:BillingReference`, in UBL 2.1's order; the guide's optional `listAgencyName`, `listName` and
`listURI` are not written. SUNAT answers these only with observations and one error, and the SDK refuses what beta
observed on 16/10/2026, so nothing reaches SUNAT with a note on it:

| Field | Accepted | What SUNAT beta did otherwise |
|---|---|---|
| `order_reference` | 1 to 20 characters, no space, tab or line break | 4233 on `OC 1` (the validation sheet says a space is allowed), `  X`, `OC<tab>1` and 21 characters; no note on `ÑANDÚ/9.5_x`, 20 characters or a no-break space. Beta validates it on invoices only: 03, 07 and 08 passed `OC 1`, and the SDK applies the same rule to all four so a document does not turn into an observed one by changing type |
| `despatch_references[].document_type` | `"09"` (sender's guide) or `"31"` (carrier's) | 4005 on `"99"` |
| `despatch_references[].number` | `SERIES-NUMBER`, 1 to 8 digits, series `T`+3 uppercase letters or digits, `V`+3, `G`+3 digits, 4 digits, `EG01` to `EG04` or `EG07` | 4006 on `XX-5`, `E1-5` (the sheet's `[EG0][1-4]` read literally), `EG05-1`, `t001-1` and a 9-digit number; no note on `T001-00000012`, `V001-1` as type 09 or `T001-1` as type 31 |
| repeated guide | the same type and number once | error 2364 |

Vectors `01-invoice-references` (F001-6001), `03-receipt-references`, `07-credit-note-references` and
`08-debit-note-references` (6001 each) were accepted by beta with code 0 and no notes.

## Detraction

`{ "code": "037", "percent": "12.00", "amount": "141.60", "account": "00-741-123456",
"payment_means_code": "001" }`: `code` (catalog 54, 3 digits), `percent` (positive, up to 3 integer digits
and 5 decimals), `amount` (positive, up to 2 decimals, always in soles), `account` (the supplier's
Banco de la Nación account, 1 to 100 characters) and `payment_means_code` (catalog 59, 3 digits). Rendered
as `cac:PaymentMeans` and `cac:PaymentTerms` with `cbc:ID` `Detraccion`, before the form of payment;
the amount carries `currencyID="PEN"` whatever the document currency (SUNAT 3208).

The caller decides whether the sale is subject to detraction and computes the amount, as for every
amount; SUNAT checks neither the rate nor the amount against the total. The SDK rejects what SUNAT
rejects: `detraction` is required with operation types 1001 to 1004 (3127, 3034) and refused with any
other (3128); 1003 takes code `028` (3129); 1002 (hydrobiological resources) and 1004 (freight
transport) need line details the SDK does not model and are rejected as out of scope. Whether a code is
in catalog 54 (3033) or 59 (3174) is left to SUNAT. The legend `2006` («Operación sujeta a detracción»)
goes in `legends` like any other. Vectors `01-invoice-detraction` (F001-921) and
`03-receipt-detraction` (B001-921) were accepted by beta on 16/10/2026 with code 0 and no notes.

## IGV retention

`{ "factor": "0.03", "amount": "4.44", "base_amount": "148.00" }`: the IGV a customer designated as retention agent
withholds, informed in the document. Rendered as a global `cac:AllowanceCharge` with `ChargeIndicator` `false`,
catalog 53 code `62`, `MultiplierFactorNumeric` (the factor, positive, up to 3 integer digits and 5 decimals),
`Amount` and `BaseAmount` (positive, up to 2 decimals, in the document currency), right before `cac:TaxTotal`. It
changes no total: the customer pays `payable` minus `amount`.

The caller decides whether the retention applies and computes it, as for every amount: SUNAT checks the amount
against `base_amount` × `factor` within 1 (3263) and that `base_amount` is not above the total (3264). The SDK
rejects the formats (3025, 2968, 3016) and a customer without RUC (3262). Whether the customer is in SUNAT's
register of retention agents (3262) and the issuer is not (3269) is checked by SUNAT against its register, which
beta does not apply: on 16/10/2026 beta accepted, with code 0 and no notes, vectors `01-invoice-igv-retention`
(F001-622), `03-receipt-igv-retention` (B001-622), `07-credit-note-igv-retention` (FC01-622) and
`08-debit-note-igv-retention` (FD01-622), whose customer 20000000001 is not an agent. In production that customer
is rejected with 3262.

## Issuer

`ruc`, `legal_name`, `trade_name`, `email`, `telephone`, `address`: `ubigeo`, `country_code`,
`department`, `province`, `district`, `urbanization`, `street`, `establishment_code`.

## Customer

`document_type` (catalog 06), `document_number`, `legal_name`, optional `address` (same shape as the
issuer's), `email`, `telephone`.

## Totals

`taxed`, `unaffected`, `exempt`, `export`, `free`, `free_igv`, `igv`, `ivap_base`, `ivap`,
`total_taxes`, `sale_value`, `subtotal`, `rounding`, `payable`. An absent total is not rendered.

## IVAP

Sale of rice under the IVAP (Impuesto a la Venta de Arroz Pilado), in invoices, receipts and notes.
Every line takes `affectation` `"17"`, with the tax in the line's IGV fields (`igv_base`, `igv_rate`
`"4.00"`, `igv`); the document carries `totals.ivap_base` and `totals.ivap`, rendered as a
`cac:TaxSubtotal` with scheme `1016` `IVAP` `VAT`, and each line's subtotal takes the same scheme.

The SDK rejects what SUNAT rejects: a line with another affectation next to a line with 17 (2644), a
document with lines 17 and no `ivap_base` or `ivap`, those totals without lines 17, and a credit or
debit note with lines 17 whose `reason_code` is not `"12"`, «Ajustes afectos al IVAP» in catalogs 09
and 10 (3230: beta rejected a credit note with type 07 and a debit note with type 02). The amounts and the legend `2007`
(«Operación sujeta al IVAP») stay with the caller; the vectors carry it in `legends`. Vectors
`01-invoice-ivap` (F001-1701), `03-receipt-ivap` (B001-1701), `07-credit-note-ivap` (FC01-1702) and
`08-debit-note-ivap` (FD01-1702) were accepted by beta on 16/10/2026 with code 0 and no notes.

## Line

`unit` (catalog 03, `"NIU"`, `"ZZ"`), `quantity`, `product_code`, `sunat_product_code`, `description`,
`unit_value`, `igv_base`, `igv_rate` (`"18.00"`), `igv`, `affectation` (catalog 07), `total_taxes`,
`unit_price`, `line_value`, `free_unit_value` (free lines), `discounts` (invoice and receipt only).

`discounts`: `[{ "code": "00", "factor": "0.15", "amount": "12.71", "base_amount": "84.74" }]`, discounts
that reduce the line's IGV base (catalog 53 code `00`), each rendered as a `cac:AllowanceCharge` with
`ChargeIndicator` false. `factor` is positive with up to 3 integer digits and 5 decimals; `amount` and
`base_amount` are positive with up to 2 decimals; trailing zeros do not count and the three print in
their shortest form (`0.10` → `0.1`). The arithmetic stays with the caller, as for every amount: SUNAT
rejects an `amount` that differs from `base_amount` × `factor` by more than 1 (error 3290) and a
`line_value` that differs from `unit_value` × `quantity` minus the discounts by more than 1 (3271).
So `unit_value` is the value before the discount: SUNAT beta rejected an invoice whose `unit_value` was
already net of the declared discount (3271) and accepted the same receipt with observation 4288.

## Note (07, 08)

`reason_code` (catalog 09 or 10; `"12"` with IVAP lines), `reason`, `affected_document_type`, `affected_document_number`
(`"F001-123"`).

# Daily summary (RC) and void communication (RA)

Not supported, rejected with an explicit error: perception, ICBPER, ISC and IVAP in summary lines;
reversal summaries (RR); despatch (GRE).

## Summary (`kind: "summary"`, RC)

| Field | Notes |
|---|---|
| `number` | correlative of the day, string (`"1"`); the identifier is `RC-YYYYMMDD-<number>` with the generation date |
| `generation_date` | date the summary is generated, `"2026-10-15"` |
| `reference_date` | date of issue of the documents it reports, `"2026-10-14"` |
| `currency` | `"PEN"`, `"USD"` |
| `issuer` | see Issuer |
| `lines` | see Summary line |

Summary line: `document_type` (`"03"`, `"07"`, `"08"`), `document_number` (`"B001-1"`), `status`
(catalog 19: `"1"` add, `"2"` modify, `"3"` void), `customer_document_type` (catalog 06),
`customer_document_number`, optional `customer_name` (field 10.3, 3 to 250 characters without line breaks
or tabs), optional `igv_rate` (`"18.00"` or `"10.50"` for a receipt; a note also takes `"10.00"`),
optional `reference_document` (`{ "document_type": "03", "document_number":
"B001-1" }`, required for notes and rejected on receipts), and the amounts `total` (required), `taxed`,
`exempt`, `unaffected`, `export`, `free`, `other_charges`, `igv`. `igv` is always rendered (`0.00` when
absent); `taxed` is rendered whenever given, even `"0.00"`; `exempt`, `unaffected`, `export`, `free` and
`other_charges` only when non-zero, so `"0.00"` there is the same as absent (the vectors' rule). Summary
lines render `igv_rate` as the `cbc:Percent` of tax 1000 only when given, and `customer_name` as
`cac:Party/cac:PartyLegalEntity/cbc:RegistrationName` of the customer. `reference_date` must not be after
`generation_date`.

### SUNAT's rule changes dated 01/08/2026

SUNAT's validation rules (version of 24/04/2026, «Control de Cambios» rows 508-545) date these changes
01/08/2026; SUNAT beta did not apply them on 16/10/2026, so the SDK keeps the rules beta applies:

| Change | What SUNAT beta did on 16/10/2026 | In the SDK |
|---|---|---|
| Summary: `Percent` of the IGV (errors 2992, 3504, 3102) and the customer's name (10.3, error 2022) | accepted `RC-20261016-911` with both, code 0 | `igv_rate` and `customer_name`, optional; vector `rc-igv-rate-and-customer-name` |
| Summary: free sales split into catalog 11 codes 06-09 with the IGV of free sales (tax 9996), code 05 deactivated | rejected code 06 with error 2256 («no se encuentra en el catálogo 11») | `free` keeps code 05 |
| Invoice, receipt and notes: an invalid SUNAT product code becomes error 3496 | accepted with observation 3496 | `sunat_product_code` stays optional and unchecked |
| IGV rate 10.5 % for MYPE restaurants and lodging | accepted `F001-911` with `igv_rate` `10.50`, code 0, no observations | `igv_rate` as given; vector `01-invoice-igv-10-5`. Being on SUNAT's list of taxpayers allowed to apply it is not checked: beta does not check it either |

## Void communication (`kind: "voided"`, RA)

| Field | Notes |
|---|---|
| `number` | correlative of the day, string; the identifier is `RA-YYYYMMDD-<number>` with the communication date |
| `generation_date` | date of issue of the documents it voids |
| `communication_date` | date the communication is sent |
| `issuer` | see Issuer |
| `lines` | `[{ "document_type": "01", "series": "F001", "number": "123", "reason": "..." }]`; `01`, `07` or `08` (a receipt is voided in a summary with status `3`) |

`generation_date` must not be after `communication_date`. In both documents `number` is 1 to 5 digits and
dates are `YYYY-MM-DD`.

`build` renders both (`file_name` `RUC-RC-YYYYMMDD-N` or `RUC-RA-YYYYMMDD-N`). Both are sent with
`sendSummary` and return a ticket; the CDR comes from `getStatus` with that ticket.

# Despatch advice (09)

The sender's despatch advice (guía de remisión remitente): UBL 2.1 `DespatchAdvice`, SUNAT
customization 2.0. `build` renders it with `file_name` `RUC-09-SERIES-NUMBER` and `sign` signs it like
any other document. Sending it is not covered: SUNAT receives despatch advices only through its REST
service, not through `sendBill`.

Not supported, rejected with an explicit error: transfer reasons `08`, `09` and `18` (imports, exports,
itinerant issuer: ports, airports, containers and customs data), related documents `49` and `80`
(they need an item with its tariff heading) and any `document_type` other than `09`, such as the
carrier's despatch advice (31). The carrier's vehicle and driver registration indicator is not
rendered.

## Despatch (`kind: "despatch"`)

| Field | Notes |
|---|---|
| `ubl_version` | `"2.1"` (2110) |
| `document_type` | `"09"` (1051) |
| `series` | `T` and 3 uppercase letters or digits, `"T001"` (1001) |
| `number` | 1 to 8 digits, string |
| `issue_date` | ISO 8601 with offset |
| `note` | optional |
| `issuer` | the sender, see Issuer (`ruc` and `legal_name` required) |
| `recipient` | `{ "document_type", "document_number", "legal_name" }`, catalog 06 |
| `seller` | optional, same shape; checked for reasons 02, 07 and 13 |
| `buyer` | optional, same shape; checked for reasons 03 and 13 |
| `related_documents` | optional, `[{ "document_type", "document_number", "description", "issuer_ruc" }]`, catalog 61; `issuer_ruc` is required for 01, 03, 04, 09, 12 and 48 (3380) |
| `shipment` | see Shipment |
| `lines` | 1 to 9999: `{ "quantity", "unit", "description", "product_code", "sunat_product_code" }`; `quantity` positive with up to 10 decimals, `unit` catalog 03, `sunat_product_code` catalog 25 (up to 8 digits) |

## Shipment

| Field | Notes |
|---|---|
| `reason_code` | catalog 20: `01`-`07`, `13`, `14`, `17` |
| `reason_description` | required for reason `13` (3457) |
| `transport_mode` | catalog 18: `"01"` public (a carrier moves the goods), `"02"` private (the sender does) |
| `start_date` | `"2026-10-15"`, not before the issue date; required in private transport or with M1 or L vehicles (3406) |
| `gross_weight`, `weight_unit` | required; up to 12 integer digits and 3 decimals; `"KGM"` or `"TNE"` |
| `packages` | optional, number of packages or pallets |
| `m1_or_l_vehicle` | `true` when the goods travel in M1 or L category vehicles: then no driver, at most one plate, and the carrier is not required |
| `carrier` | public transport only, required unless `m1_or_l_vehicle`: `{ "ruc", "legal_name", "mtc_registration" }`; not the sender (2560) |
| `vehicles` | private transport (at least the main one), or one plate with `m1_or_l_vehicle`: `[{ "plate" }]`, the first is the main vehicle; 6 to 8 uppercase letters or digits |
| `drivers` | private transport without `m1_or_l_vehicle`, at least the main one: `[{ "document_type", "document_number", "first_name", "family_name", "license" }]`, the first is the main driver; `license` 9 or 10 uppercase letters or digits |
| `origin`, `destination` | `{ "ubigeo", "address", "establishment_code", "establishment_ruc" }`; the establishment code and RUC go together, and reason `04` (between establishments of the same company) requires both with the sender's RUC |

In public transport the carrier names its own vehicles and drivers, so the despatch advice carries
none (3354, 3455).

## What is checked, and what is not

SUNAT has no test environment for despatch advices: its beta does not receive them. A despatch advice
is checked here by:

1. Its SUNAT validation rules that read only the XML (sheet «Guía-Remitente2_0», type XSL), each
   rejected with the code SUNAT would answer: `{"error": "$.shipment.carrier.ruc: must not be the
   sender (SUNAT ERR-2560)"}`. Observations (4000 and above) are not enforced. One test per code in
   `crates/sunat-model/tests/despatch_rules.rs`.
2. Its vectors: `fixtures/09-despatch-sale.json` (reason 01, private transport) and
   `fixtures/09-despatch-between-establishments.json` (reason 04, public transport) give
   `tests/vectors/09-*.unsigned.xml` and the `DigestValue` and `SignatureValue` of
   `tests/vectors/09-*.signed.xml` through the core, the CLI, PHP, npm and Python. These two vectors
   were written by this SDK, not by another serializer: they guard against regressions, they do not
   prove compatibility. The independent verifier accepts their signatures.
3. The OASIS UBL 2.1 `DespatchAdvice` schema (`tests/xsd/ubl-2.1`), validated with lxml on both signed
   vectors (`crates/sunat-ubl/tests/xsd.rs`).

Not checked here, because SUNAT answers them from its registries or at reception: the reception date
(2108, 2329), the sender's, recipient's and carrier's taxpayer status, RENIEC identities, the MTC
registration, the registered establishments (3366, 3367, 3370, 3371, 3412) and whether the related
documents were issued.
