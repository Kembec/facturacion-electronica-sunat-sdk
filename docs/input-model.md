# Input model (phase 1)

One JSON document per voucher, snake_case English field names. Decimal amounts are JSON strings
(`"118.00"`), never floats. Amounts are given by the caller: the SDK renders and validates, it does not
compute taxes.

Phase 1 scope: invoice `01`, receipt `03`, credit note `07`, debit note `08`, UBL 2.1, IGV only
(affectations 10-16, 20, 21, 30-37, 40; 17 IVAP is out of phase 1). Out of phase 1, rejected with an
explicit error: ISC, ICBPER, IVAP, other taxes, global discounts or charges, advances, detraction,
perception, despatch references.

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
`unit_price`, `line_value`, `free_unit_value` (free lines).

## Note (07, 08)

`reason_code` (catalog 09 or 10), `reason`, `affected_document_type`, `affected_document_number`
(`"F001-123"`).

# Phase 2: daily summary (RC) and void communication (RA)

Out of phase 2, rejected with an explicit error: perception, ICBPER, ISC and IVAP in summary lines;
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
`customer_document_number`, optional `reference_document` (`{ "document_type": "03", "document_number":
"B001-1" }`, for notes), and the amounts `total`, `taxed`, `exempt`, `unaffected`, `export`, `free`,
`other_charges`, `igv`. `igv` is always rendered (`0.00` when absent); any amount given, even `"0.00"`, is
rendered; an absent amount other than `igv` is not. Summary lines carry no tax percentage.

## Void communication (`kind: "voided"`, RA)

| Field | Notes |
|---|---|
| `number` | correlative of the day, string; the identifier is `RA-YYYYMMDD-<number>` with the communication date |
| `generation_date` | date of issue of the documents it voids |
| `communication_date` | date the communication is sent |
| `issuer` | see Issuer |
| `lines` | `[{ "document_type": "01", "series": "F001", "number": "123", "reason": "..." }]` |

Both are sent with `sendSummary` and return a ticket; the CDR comes from `getStatus` with that ticket.
