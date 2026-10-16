# Input model

One JSON document per voucher, snake_case English field names. Decimal amounts are JSON strings
(`"118.00"`), never floats. Amounts are given by the caller: the SDK renders and validates, it does not
compute taxes.

Documents: invoice `01`, receipt `03`, credit note `07`, debit note `08`, UBL 2.1, IGV only
(affectations 10-16, 20, 21, 30-37, 40; 17 IVAP is not supported). Not supported, rejected with an
explicit error: ISC, ICBPER, IVAP, other taxes, global discounts or charges, line charges, line
discounts other than catalog 53 code `00` or in notes, advances, detraction, perception, despatch
references.

## Document

| Field | Notes |
|---|---|
| `kind` | `"invoice"` (01, 03) or `"note"` (07, 08) |
| `ubl_version` | `"2.1"` |
| `operation_type` | catalog 51, invoice only (`"0101"`) |
| `document_type` | `"01"`, `"03"`, `"07"`, `"08"` |
| `series` | `"F001"`, `"B001"` |
| `number` | string, no padding added |
| `issue_date` | ISO 8601 with offset, `"2026-10-15T10:30:00-05:00"` |
| `due_date` | optional, invoice only |
| `currency` | `"PEN"`, `"USD"` |
| `payment_terms` | `{ "type": "Contado" }` or `{ "type": "Credito", "amount": "...", "currency": "PEN", "installments": [{ "amount": "...", "currency": "PEN", "due_date": "2026-11-15T00:00:00-05:00" }] }` |
| `issuer` | see Issuer |
| `customer` | see Customer |
| `totals` | see Totals |
| `lines` | see Line |
| `legends` | `[{ "code": "1000", "value": "..." }]` |
| `note` | only for 07/08, see Note |

## Issuer

`ruc`, `legal_name`, `trade_name`, `email`, `telephone`, `address`: `ubigeo`, `country_code`,
`department`, `province`, `district`, `urbanization`, `street`, `establishment_code`.

## Customer

`document_type` (catalog 06), `document_number`, `legal_name`, optional `address` (same shape as the
issuer's), `email`, `telephone`.

## Totals

`taxed`, `unaffected`, `exempt`, `export`, `free`, `free_igv`, `igv`, `total_taxes`, `sale_value`,
`subtotal`, `rounding`, `payable`. An absent total is not rendered.

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

`reason_code` (catalog 09 or 10), `reason`, `affected_document_type`, `affected_document_number`
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
