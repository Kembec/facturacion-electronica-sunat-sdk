# Input model

One JSON document per voucher, snake_case English field names. Decimal amounts are JSON strings
(`"118.00"`), never floats. Amounts are given by the caller: the SDK renders and validates, it does not
compute taxes.

Documents: invoice `01`, receipt `03`, credit note `07`, debit note `08`, UBL 2.1, IGV (affectations
10-16, 20, 21, 30-37, 40), IVAP (affectation 17, see IVAP) and ICBPER (tax 7152, see ICBPER). Not
supported, rejected with an explicit error: ISC, other taxes, catalog 53 codes other than those of Discounts and
charges, discounts and charges in notes, advances in notes, detraction on operation types 1002 and 1004,
perception.

The daily summary (RC), the void communication (RA), the sender's despatch advice (09) and the
retention voucher (20) have their own sections below.

## Document

| Field | Notes |
|---|---|
| `kind` | `"invoice"` (01, 03) or `"note"` (07, 08) |
| `ubl_version` | `"2.1"` |
| `operation_type` | catalog 51, invoice only (`"0101"`; `"1001"` or `"1003"` with `detraction`; `"0200"` to `"0208"`, see Export) |
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
| `global_adjustments` | optional, invoice and receipt only, see Discounts and charges |
| `advances` | optional, invoice and receipt only, see Advances |
| `service_use_country` | operation types 0201 and 0208 only, see Export |

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

In 01, 03, 07 and 08 the free-text fields SUNAT observes by format are refused before signing, with the field and the
SUNAT code, and never trimmed or rewritten: SUNAT's sheet (2026-04-24) allows any character including the space and no
other whitespace (line feed, carriage return, tab, vertical tab, form feed). An empty value writes no tag and is not
checked. Beta observed 4094 on a receipt whose `issuer.address.street` had a line break (2026-10-18).

| Field | Accepted | SUNAT |
|---|---|---|
| `issuer.legal_name` | 1 to 1500 characters in 01 and 03, 3 to 1500 in 07 and 08 | 4338 |
| `issuer.trade_name` | 1 to 1500 characters in 01 and 03, 3 to 1500 in 07 and 08 | 4092 |
| `issuer.address.street` | 3 to 200 characters | 4094 |
| `issuer.address.urbanization` | 1 to 25 characters | 4095 |
| `issuer.address.province` | 1 to 30 characters | 4096 |
| `issuer.address.department` | 1 to 30 characters | 4097 |
| `issuer.address.district` | 1 to 30 characters | 4098 |
| `lines[].product_code` | 1 to 30 characters | 4269 in 01 and 03, 4234 in 07 and 08 |

The delivery address of an itinerant sale (4236, 4238 to 4241) and the trip detail of operation 1004 (4270) are not
written by the SDK, so they have no field here.

## Customer

`document_type` (catalog 06), `document_number`, `legal_name`, optional `address` (same shape as the
issuer's), `email`, `telephone`.

## Totals

`taxed`, `unaffected`, `exempt`, `export`, `free`, `free_igv`, `igv`, `ivap_base`, `ivap`, `icbper`,
`total_taxes`, `sale_value`, `subtotal`, `allowance_total`, `charge_total`, `rounding`, `prepaid`, `payable`. An
absent total is not rendered. `allowance_total` and `charge_total` (`cbc:AllowanceTotalAmount`,
`cbc:ChargeTotalAmount`) carry the discounts and charges that do not change the IGV base, see
Discounts and charges. `prepaid` (`cbc:PrepaidAmount`) is the sum of the advances' `paid_amount`, see Advances.
`total_taxes` is `igv` + `ivap` + `icbper`, within 1, in 01, 03, 07 and 08: 3294 in an invoice and in a note on one,
4301 in a receipt and in a note on anything else (the SDK rejects that observation too).

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
`unit_price`, `line_value`, `free_unit_value` (free lines), `discounts` and `charges` (invoice and
receipt only, see Discounts and charges), `icbper` and `icbper_factor` (see ICBPER), `lodging` (operation types
0202 and 0205, see Export).

`igv_base` equals `line_value` to the cent on every line, in every operation type: the SDK does not model ISC, and
without it SUNAT compares the two exactly. Otherwise 3272 in an invoice and in a note that modifies one, and 4294 in a
receipt and in a note that modifies one (an observation for SUNAT, rejected here).

In an invoice and a receipt, every line is also checked before signing, within SUNAT's tolerance of 1:

- `unit_price` = (`line_value` + `total_taxes` − discounts `01` + charges `48`) ÷ `quantity`, on a line without
  `free_unit_value`: 3270 in an invoice («Factura2_0» row 183), 4287 in a receipt («Boleta2_0» row 176, an
  observation, rejected here); a `unit_price` with up to 10 decimals is compared exactly;
- `igv` = `igv_base` × `igv_rate`, rounded half up to the cent, on a line with affectation 10 to 17, free ones
  included: 3103 in both (an error; rows 213 and 206).

## ICBPER

The tax on plastic bags (Ley 30884, art. 12; tax 7152 `ICBPER` `OTH`), in invoices, receipts and both
notes, in PEN only: in another currency it is out of scope, because the law has it paid in soles and
SUNAT's rules check the rate only in PEN. The bags go on their own line, sold (affectation 10) or given
away (11 to 16, with `free_unit_value`): the tax is due either way. Whether the sale is subject to it
(an establishment selling to the final consumer, a bag to carry what was sold) is the caller's call.

| Field | Notes |
|---|---|
| `lines[].quantity` | the bags: a whole number from 1 to 99999 (2892); rendered as `cbc:BaseUnitMeasure` `NIU` |
| `lines[].icbper_factor` | PEN per bag, required with `icbper` (3238): the rate in force on the date of issue (4237) |
| `lines[].icbper` | `quantity` × `icbper_factor`, positive with up to 2 decimals (2033, 4318) |
| `lines[].total_taxes` | the line's IGV plus its `icbper` (3292) |
| `totals.icbper` | Σ `lines[].icbper` (3306), positive (2048) |
| `totals.total_taxes` | `igv` + `ivap` + `icbper` (3294) |
| `totals.subtotal`, `totals.payable` | include `icbper` (3279, 3280); `sale_value` and the IGV base do not |

The rate is a dated table of the SDK, `crates/sunat-model/src/icbper_rates.rs`, read on the year of
`issue_date` in Lima (31/12 23:30 -05:00 is still that year): S/ 0.10 from 01/08/2019, 0.20 in 2020,
0.30 in 2021, 0.40 in 2022 and 0.50 from 2023. Its source is the schedule of Ley 30884, art. 12, and
SUNAT's table for tax 7152 in catalog 05 of its SFS structures (June 2026); the 2023 row stays
`verified-partial` until no later rule changing art. 12 is ruled out. Before 01/08/2019 the tax does not
exist (2949).

The line renders one more `cac:TaxSubtotal` after its IGV one, with `cbc:TaxAmount`,
`cbc:BaseUnitMeasure` and `cbc:PerUnitAmount` (at least two decimals, `0.50`) and no `TaxableAmount` or
`Percent`; the document's `cac:TaxTotal` gets a 7152 subtotal with only its amount.

Checked only in a document with ICBPER, in the order above, each with the code an invoice and a note on
an invoice get; a receipt or a note on another document gets the observation (4293, 4301, 4310, 4312,
4321) and the SDK rejects it too. SUNAT only observes 4318 and 4237; the SDK rejects them, so nothing
reaches SUNAT with a note. A free line's `total_taxes` is not checked: SUNAT's sum leaves its IGV (9996)
out and `03-receipt-free-line.json` puts it in; the SUNAT beta accepted `v3` with it in. Vectors:
`fixtures/icbper/` (`v1` to `v5` valid, `n1` to `n3` rejected with 4318, 3306 and 4237). Sent to the SUNAT beta on
17/10/2026 (answers in `fixtures/icbper/beta/`): `v1` to `v5` accepted with code 0 and no notes (`v5` as a summary
with its ticket), `n1` observed with 4318, `n2` rejected with 3306, and `n3` (rate 0.10 in 2026) accepted with no
note: the beta does not apply 4237, the SDK still rejects it.

## Discounts and charges

Catalog 53, invoice and receipt only. Each one is `{ "code": "00", "factor": "0.15", "amount": "12.71",
"base_amount": "84.74" }` and renders as one `cac:AllowanceCharge`; `ChargeIndicator` is not an input,
it follows from the code (so SUNAT's 3114 cannot happen).

| Where | Code | IGV base | Goes to |
|---|---|---|---|
| `lines[].discounts` | `00` | reduces the line's | `line_value` (minus) |
| `lines[].discounts` | `01` | no | `totals.allowance_total` |
| `lines[].charges` | `47` | raises the line's | `line_value` (plus) |
| `lines[].charges` | `48` | no | `totals.charge_total` |
| `global_adjustments` | `02` discount | reduces the document's | `taxed`, `sale_value`, `igv` (minus) |
| `global_adjustments` | `49` charge | raises the document's | `taxed`, `sale_value`, `igv` (plus) |
| `global_adjustments` | `03` discount | no | `totals.allowance_total` |
| `global_adjustments` | `45` FISE, `46` recargo al consumo, `50` other | no | `totals.charge_total` |

The FISE (45) stays outside the IGV base, as SUNAT's rules have it (3301); whether it belongs in the
base under article 14 of the IGV law is the issuer's decision with its adviser.

The caller gives every amount; the SDK checks them, with SUNAT's tolerance of 1, only in a document that
has a discount or a charge (a document without them is checked as before). In the order of SUNAT's
sheet, each with the code an invoice gets (a receipt gets the observation in brackets, and the SDK
rejects it too):

- codes: required (3072 global, 3073 line); a line code at document level 3071 (4291); a global code on
  a line 4268; outside catalog 53, 3071 or 2954; codes `04` to `07`, `20`, `51` to `54` and `61` to
  `64` are out of scope, and `62` goes in `igv_retention`;
- formats: `factor` positive with up to 3 integer digits and 5 decimals, `amount` and `base_amount`
  positive with up to 2 decimals, trailing zeros not counted (global 3025, 2968, 3016; line 3052, 2955,
  3053); the FISE needs `base_amount` (3092) and `amount` (3074) above zero; `allowance_total` and
  `charge_total` up to 2 decimals (2065, 2064);
- `amount` = `base_amount` × `factor`, rounded half up to the cent: 3290 (4289) line, 3307 (4322) global;
- line: `line_value` = `unit_value` × `quantity` − discounts `00` + charges `47` (3271, 4288), so
  `unit_value` is the value before them: beta rejected an invoice whose `unit_value` was already net of
  the declared discount (3271) and accepted the same receipt with observation 4288;
- `allowance_total` = line `01` + global `03` (3300, 4307); `charge_total` = line `48` + global `45`, `46`,
  `50` (3301, 4308); `payable` = `subtotal` + `charge_total` − `allowance_total` + `rounding` (3280, 4312);
- `taxed` = Σ `line_value` of the lines with affectation 10 − global `02` + global `49` (3277, 4299);
  `igv` = the lines' rate × that base (3291, 4290); `sale_value` = Σ `line_value` of the lines with
  affectation 10, 17, 20, 30 or 40 − `02` + `49` (3278, 4309); `subtotal` = `sale_value` + `total_taxes`
  (3279, 4310).

A global `02` or `49` with lines at two IGV rates (18 and 10.5) is out of scope: the sheet does not say
how it splits between them. Each line keeps its own IGV over its own base; only the document's IGV is net
of `02` and `49`. Vectors: `fixtures/allowances/` (`inv-01` to `inv-06` valid, `inv-R1` to `inv-R3`
rejected with 3277, 3307 and 3290). Not sent to SUNAT beta yet.

## Advances

A sale paid in advance is two documents. The advance is an ordinary invoice or receipt for what was received
(`operation_type` `0101`: catalog 51 has no code of its own for it). The final invoice or receipt carries the
whole sale and lists each advance in `advances`; the SDK renders from that list a `cac:AdditionalDocumentReference`
(the advance's document, `DocumentStatusCode` its number), a `cac:PrepaidPayment` and a `cac:AllowanceCharge` with
catalog 53 code `04`, `05` or `06`, in the same order. Invoice and receipt only; in a note, `advances` and
`totals.prepaid` are out of scope. `global_adjustments` does not take 04, 05 or 06, and `prepayments` is not a name.

| Field of `advances[]` | |
|---|---|
| `document_type` | `"02"` invoice or `"03"` receipt of the advance (2505) |
| `document_number` | `SERIES-NUMBER`: `F`+3, `E001` or up to 4 digits for an invoice, `B`+3, `EB01` or up to 4 digits for a receipt; then 1 to 8 digits (2521) |
| `paid_amount` | what was received, IGV included; positive, 12 integer digits and 2 decimals (2503) |
| `affectation` | `"10"` taxed (discount 04), `"20"` exempt (05), `"30"` unaffected (06) |
| `value` | sale value of the advance, without IGV (equal to `paid_amount` in 20 and 30): the `Amount` of the discount (2968) |
| `issuer_ruc` | optional, 11 digits; the document's issuer when absent |
| `base_amount`, `factor` | optional, both or neither; `value` = `base_amount` × `factor`, within 1 (3307, 4322) |

With `L` the sum of the lines' value, `A` the advances 04 and `P` the sum of `paid_amount`:

- `totals.prepaid` = `P`, above zero (3220) and equal to the sum (2509);
- `taxed` = `L` − `A` − 02 + 49 (3277, 4299) and `igv` = rate × `taxed` (3291, 4290): n2 shows the beta rejects the IGV over `L`;
- `sale_value` = `L` − 02 + 49: the advance does not lower it (3278, 4309);
- `subtotal` = `sale_value` + `total_taxes` + rate × `A`, that is, the IGV over the gross base (3279, 4310); the net version was rejected (s1);
- `payable` = `subtotal` + `charge_total` − `allowance_total` − `prepaid` + `rounding` (3280, 4312); `allowance_total` does not include 04, 05 or 06;
- `exempt` = Σ exempt lines − advances 05 and `unaffected` = Σ unaffected lines − advances 06 (3275, 3274; 4297, 4296).

A sale covered in full has `payable` `"0.00"`, `taxed` `"0.00"` and still renders the IGV subtotal at zero
(without it the beta answers 2638). An advance with an affectation other than 10, 20 or 30, the same advance twice,
and advances with lines at two IGV rates are rejected. The SDK does not check that the advance exists or is
accepted (3218): the beta does not either. Vectors and the beta's answers: `fixtures/advances/`, `fixtures/advances/beta/`.

## Export

Operation types `0200` to `0208` of catalog 51, in invoices and receipts. They share one set of rules: every line
takes affectation `"40"` (2642), `igv` `"0.00"` (3110) and the IGV rate of the period, not zero (3102); the totals
carry only `export` = Σ `line_value` (3003, 3273), with `taxed`, `exempt`, `unaffected` and `igv` absent or
`"0.00"` (3107), `total_taxes` zero (3294), `sale_value` = `export` (3278), `subtotal` = `sale_value` (3279) and
`payable` = `subtotal` + `charge_total` − `allowance_total` (3280). The customer is a catalog 06 document or `"-"`
(2800): a DNI is 8 digits (2801), a foreign document up to 15 characters without spaces (2802), and a RUC (`"6"`)
is rejected with 0200, 0201 and 0204, and with any export in a receipt (2800). Lodging (0202) and tour package (0205)
are invoice only (3206). Advances, IGV retention, free lines and ICBPER in an export are out of scope.

| Type | What changes |
|---|---|
| `0201`, `0208` services used abroad | `service_use_country`: catalog 04 country other than `"PE"`, required (3098, 3099). It renders `cac:Delivery/cac:DeliveryLocation/cac:Address/cac:Country` and no address line |
| `0202` lodging of non-domiciled guests | `lines[].lodging` on every line with the ten fields below (3136 to 3145) |
| `0205` tour package | `lines[].lodging` with `guest_name`, `guest_document_type`, `guest_document_number` and `passport_country` only (3136 to 3139) |
| `0200`, `0203`, `0204`, `0206`, `0207` | nothing beyond the shared rules |

Catalog 04 is the list in `crates/sunat-model/data/catalog-04.json`: SUNAT points to ISO 3166-1 without listing it,
and the SDK takes ISO 3166-1 alpha-2 in force (249 codes), the list the beta applied: it rejected `AN`, withdrawn but
on the page the SUNAT sheet cites, with 3099. `PE` is in it: only 3099 excludes it.

| Field of `lines[].lodging` | Catalog 55 | |
|---|---|---|
| `passport_country` | 4000 | catalog 04 (4280) |
| `residence_country` | 4001 | catalog 04 (4280) |
| `country_entry_date`, `check_in_date`, `check_out_date`, `consumption_date` | 4002, 4003, 4004, 4006 | `YYYY-MM-DD`; check-out not before check-in (4282) |
| `stay_days` | 4005 | 1 to 4 digits (4281) |
| `guest_name` | 4007 | 3 to 200 characters (4280) |
| `guest_document_type` | 4008 | catalog 06 (4280) |
| `guest_document_number` | 4009 | 3 to 20 characters (4280) |

Web hosting, SaaS or software for a customer abroad is a service: `0201` (or `0208` if part of it is provided
abroad), not `0202`. `0202` is the lodging of tourists, and SUNAT rejects it without the guest data (3136 to 3145).
Lodging beyond 60 days per entry into the country is not an export (art. 33 num. 4 of the IGV law): invoice that
part apart; the SDK does not reject `stay_days` above 60, because SUNAT does not check it. Whether the sale is an export
(the customer's domicile, where the service is used, the exporters' register SUNAT checks with 0201) stays with the
caller. `service_use_country` outside 0201 and 0208, and `lodging` outside 0202 and 0205 or in a note, are rejected.
Vectors and the beta's answers: `fixtures/export/`, `fixtures/export/beta/`; the beta accepted 0208 without the country
(n1), the SDK still rejects it with 3098, as the validation sheet says. 0203 to 0207 were accepted with code 0 and no
notes; as the country of use, `RS` was accepted and `AN` came back as 3099.

## Note (07, 08)

`reason_code` (catalog 09 or 10; `"12"` with IVAP lines), `reason`, `affected_document_type`, `affected_document_number`
(`"F001-123"`).

# Daily summary (RC) and void communication (RA)

Not supported, rejected with an explicit error: despatch (GRE). The
reversal summary (RR) of a purchase settlement is under [Purchase settlement (04)](#reversal-summary-kind-reversal-rr).

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
or tabs), required `igv_rate` (`"18.00"` or `"10.50"` for a receipt; a note also takes `"10.00"`; missing is 2992),
optional `reference_document` (`{ "document_type": "03", "document_number":
"B001-1" }`, required for notes and rejected on receipts), and the amounts `total` (required), `taxed`,
`exempt`, `unaffected`, `export`, `free`, `free_taxed`, `free_exempt`, `free_unaffected`, `free_export`, `free_igv`, `other_charges`, `igv`, `icbper` (in PEN, positive, 2048; `total`
includes it, and it renders as its own `cac:TaxTotal` 7152 after the IGV one). `ivap` (tax 1016), `isc` (2000) and `other_taxes` (9999) are positive PEN amounts (2048), each its own `cac:TaxTotal`; a line with `ivap` takes neither `igv` nor `igv_rate` (2355) and then needs no `igv_rate`. The line's `cac:TaxTotal` go in the order 1000 or 1016, 2000, 9999, 9996, 7152; `cbc:Percent` only on 1000 and 9996. `igv` is always rendered (`0.00` when
absent) unless the line carries `ivap`, whose 1016 takes the place of 1000 (2278); `taxed` is rendered whenever given, even `"0.00"`; `exempt`, `unaffected`, `export`, `free` and
`other_charges` only when non-zero, so `"0.00"` there is the same as absent (the vectors' rule). Summary
lines render `igv_rate` as the `cbc:Percent` of tax 1000, on every line with tax 1000 whatever its status or amounts (the Resumen Diario sheet answers 2992 to «Código de tributo 1000» without the tag), and `customer_name` as
`cac:Party/cac:PartyLegalEntity/cbc:RegistrationName` of the customer. `reference_date` must not be after
`generation_date`.

`perception` (optional) carries the IGV perception of a receipt: `system_code` (catalog 22: `"01"` 2 %, `"02"` 1 %,
`"03"` 0.5 %), `base`, `amount` and `total_cashed`, all positive PEN (2893, 2895, 2897). The rate is not input: the SDK
takes it from the code, so `amount` must be `base` times that rate and `total_cashed` the line's `total` plus `amount`
(each within 1, as SUNAT's 2608); a `percent` field is rejected as unknown. It renders as
`sac:SUNATPerceptionSummaryDocumentReference`, with the rate as `sac:SUNATPerceptionPercent`, between the customer (or
`cac:BillingReference`) and `cac:Status`. Rejected: any other `system_code` (2517), a line that is
not a receipt (03) or whose `status` is `"2"` (2986), and a summary whose `currency` is not `"PEN"` (2685).

### SUNAT's rule changes dated 01/08/2026

SUNAT's validation rules (version of 24/04/2026, «Control de Cambios» rows 508-545) date these changes
01/08/2026; SUNAT beta did not apply them on 16/10/2026, so the SDK keeps the rules beta applies:

| Change | What SUNAT beta did on 16/10/2026 | In the SDK |
|---|---|---|
| Summary: `Percent` of the IGV (errors 2992, 3504, 3102) and the customer's name (10.3, error 2022) | accepted `RC-20261016-911` with both, code 0 | `igv_rate` required on every line (2992 locally); `customer_name` optional; vector `rc-igv-rate-and-customer-name`. On 17/10/2026 beta accepted `rc-exempt-and-unaffected` and `rc-receipts-added` with `Percent` 18.00 on every line, exempt and unaffected included (`fixtures/rc-beta/`) |
| Summary: free sales split into catalog 11 codes 06-09 with the IGV of free sales (tax 9996), code 05 deactivated | rejected code 06 with error 2256 («no se encuentra en el catálogo 11») | dated 01/01/2027, see «Rules that take effect on 01/01/2027» below: `free` (05) until then, `free_taxed`, `free_exempt`, `free_unaffected`, `free_export` and `free_igv` from then |
| Invoice, receipt and notes: an invalid SUNAT product code becomes error 3496 | accepted with observation 3496 | `sunat_product_code` stays optional and unchecked |
| IGV rate 10.5 % for MYPE restaurants and lodging | accepted `F001-911` with `igv_rate` `10.50`, code 0, no observations | `igv_rate` as given; vector `01-invoice-igv-10-5`. Being on SUNAT's list of taxpayers allowed to apply it is not checked: beta does not check it either |

### Rules that take effect on 01/01/2027

RS 143-2026 (art. 3.4) moves RS 048-2026 from 01/08/2026 to 01/01/2027, and its annexes carry these rules. The SDK
applies each by the date its norm names, from a table of the SDK with the norm or lead behind every date: until
2026-12-31 the rule of today, from 2027-01-01 the new one.

| Rule | Until 2026-12-31 | From 2027-01-01 | Decided by |
|---|---|---|---|
| Free sales in the summary (catalog 11) | `free` renders code 05; `free_taxed`, `free_exempt`, `free_unaffected`, `free_export` (06 to 09) are rejected with 2256, `free_igv` too | 06 to 09 render, in that order after 01 to 04; `free_igv` renders its own `cac:TaxTotal` with tax 9996 after the IGV one; `free` (05) is rejected | `reference_date` |
| Debit note 13 «Penalidades» (catalog 10) | `13` is rejected; penalties go under `03` | `13` only with every line on affectation 30 to 37 and no taxed, exempt or export total (3507); `03` is rejected (2524) | `issue_date` (Lima) |
| SUNAT product code (3496) | not rejected | not rejected: no norm names it, so the SDK does not enforce it; the date sits in the table as a lead | `issue_date` |

Not part of this change: 3503 (amount of the credit note against the modified document) needs the totals of that
document as input and comes separately. The position of the 9996 `cac:TaxTotal` among the line's elements, and
whether the IGV rate (`igv_rate`) becomes required for taxed amounts, follow the annexes' text; neither has been
seen answered by SUNAT.

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
any other document. SUNAT receives despatch advices only through its REST service, not through `sendBill`: the CLI sends
the signed guide with `gre-send` and reads its ticket with `gre-ticket` ([`cli.md`](cli.md)), and PHP and Python wrap the same two calls (JavaScript gives the
requests and reads the answers, without the network). SUNAT has no test environment for them, so none of it
has been accepted by SUNAT.

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
| `carrier_handover_date` | `"2026-10-15"`, the day the goods are handed to the carrier; public transport only, where it is required (3617), a date (3619) and the same day as the issue date or later (3618). Rendered as `cac:ShipmentStage/cac:LoadingTransportEvent/cbc:OccurrenceDate`. Not allowed in private transport |
| `gross_weight`, `weight_unit` | required; up to 12 integer digits and 3 decimals; `"KGM"` or `"TNE"` |
| `packages` | optional, number of packages or pallets |
| `m1_or_l_vehicle` | `true` when the goods travel in M1 or L category vehicles: then no driver, at most one plate, and the carrier is not required |
| `carrier` | public transport only, required unless `m1_or_l_vehicle`: `{ "ruc", "legal_name", "mtc_registration" }`; not the sender (2560) |
| `vehicles` | private transport (at least the main one), or one plate with `m1_or_l_vehicle`: `[{ "plate" }]`, the first is the main vehicle; 6 to 8 uppercase letters or digits |
| `drivers` | private transport without `m1_or_l_vehicle`, at least the main one: `[{ "document_type", "document_number", "first_name", "family_name", "license" }]`, the first is the main driver; `license` 9 or 10 uppercase letters or digits |
| `origin`, `destination` | `{ "ubigeo", "address", "establishment_code", "establishment_ruc" }`; the establishment code and RUC go together, and reason `04` (between establishments of the same company) requires both with the sender's RUC |

In public transport the carrier names its own vehicles and drivers, so the despatch advice carries
none (3354, 3455).

`carrier_handover_date` is field 39-A of RS 108-2026/SUNAT (annex II), in force since 2026-06-01. The norm
fixes no relation between it and `start_date`, so the SDK checks none: ERR-3616 (start of transfer after the handover
date in a single GRE) comes only from a secondary source and is not validated. Codes 3617 to 3619 are not in the
validation-rules sheet this repository follows; they come from SUNAT's change control as reproduced by Nubefact.

## What is checked, and what is not

SUNAT has no test environment for despatch advices: its beta does not receive them. A despatch advice
is checked here by:

1. Its SUNAT validation rules that read only the XML (sheet «Guía-Remitente2_0», type XSL), each
   rejected with the code SUNAT would answer: `{"error": "$.shipment.carrier.ruc: must not be the
   sender (SUNAT ERR-2560)"}`. Observations (4000 and above) are not enforced, except the free-text
   formats below. One test per code in `crates/sunat-model/tests/despatch_rules.rs`.
2. Its vectors: `fixtures/09-despatch-sale.json` (reason 01, private transport) and
   `fixtures/09-despatch-between-establishments.json` (reason 04, public transport) give
   `tests/vectors/09-*.unsigned.xml` and the `DigestValue` and `SignatureValue` of
   `tests/vectors/09-*.signed.xml` through the core, the CLI, PHP, npm and Python. These two vectors
   were written by this SDK, not by another serializer: they guard against regressions, they do not
   prove compatibility. The independent verifier accepts their signatures.
3. The OASIS UBL 2.1 `DespatchAdvice` schema (`tests/xsd/ubl-2.1`), validated with lxml on both signed
   vectors (`crates/sunat-ubl/tests/xsd.rs`).

The free-text fields SUNAT observes by format are refused before signing, with the field and the SUNAT code, and
never trimmed or rewritten, as in 01, 03, 07 and 08 (Issuer): the sheet allows any character including the space and
no other whitespace (line feed, carriage return, tab, vertical tab, form feed), and most rows refuse a value of only
spaces. The line's `description` is the exception: it allows line breaks and tabs and only its length is measured. An
empty value is not checked by format: the SDK writes no `note` or `product_code` for it, rows R!48 and R!161 exempt
it, and the errors that require a field answer first (1037, 2761, 2563, 3360, 3361, 2577, 2574, 2781); an empty
`seller.legal_name` outside reasons 02, 07 and 13 is not refused. SUNAT has no test environment for despatch
advices: the sheet is the only source. One test of the borders per row in
`crates/sunat-model/tests/despatch_text.rs`.

| Field | Accepted | SUNAT (sheet row) |
|---|---|---|
| `note` | 1 to 250 characters, not only spaces | 4186 (R!24) |
| `issuer.legal_name` | 1 to 250 characters, not only spaces | 4338 (R!37) |
| `related_documents[].description` | 1 to 120 characters, not only spaces | 4372 (R!48) |
| `recipient.legal_name` | 1 to 250 characters, not only spaces | 4152 (R!114) |
| `seller.legal_name` | 1 to 250 characters, not only spaces, with any reason | 4106 (R!138) |
| `buyer.legal_name` | 1 to 250 characters, not only spaces, with any reason | 4381 (R!161) |
| `shipment.reason_description` | with reason 13: 3 to 100 characters, at least 3 of them letters | 4190 (R!171, R!172) |
| `shipment.carrier.legal_name` | public transport without M1 or L vehicles: 3 to 250 characters, not only spaces | 4165 (R!241) |
| `shipment.carrier.mtc_registration` | public transport without M1 or L vehicles: up to 20 uppercase letters or digits, not only zeros | 4392 (R!245) |
| `shipment.drivers[].first_name`, `.family_name` | 1 to 250 characters, not only spaces, main and secondary drivers | 4409 (R!316, R!321, R!352, R!357) |
| `shipment.origin.address` | 3 to 500 characters, not only spaces | 4076 (R!373) |
| `shipment.destination.address` | 3 to 500 characters, not only spaces | 4068 (R!400) |
| `lines[].description` | 3 to 500 characters, line breaks and tabs allowed | 4084 (R!449) |
| `lines[].product_code` | 1 to 30 characters | 4085 (R!450) |

Not checked here, because SUNAT answers them from its registries or at reception: the reception date
(2108, 2329), the sender's, recipient's and carrier's taxpayer status, RENIEC identities, the MTC
registration, the registered establishments (3366, 3367, 3370, 3371, 3412) and whether the related
documents were issued.

# Retention voucher (20)

The comprobante de retención electrónico: UBL 2.0 `Retention`, SUNAT customization 1.0. `build`
renders it with `file_name` `RUC-20-SERIES-NUMBER` and `sign` signs it like any other document; `zip`
gives `RUC-20-SERIES-NUMBER.zip`. The CLI sends it with `send` to SUNAT's `otroscpe` service (`sendBill`).

As everywhere else, the caller gives every amount and the SDK reconciles them: it does not compute
the retention, it checks it. All amounts of the voucher itself are in PEN.

Not supported, rejected with an explicit error: related documents of type `07` and `20` (SUNAT allows
them but they carry no payment and stay out of the totals) and the reversal summary (RR).

## Retention (`kind: "retention"`)

| Field | Notes |
|---|---|
| `ubl_version` | `"2.0"` |
| `document_type` | `"20"` |
| `series` | `R` and 3 uppercase letters or digits, `"R001"` (1001) |
| `number` | 1 to 8 digits, string; zero padding is kept as given (`"0000001"`) |
| `issue_date` | ISO 8601 with offset; gives `IssueDate` and `IssueTime` |
| `issuer` | the retention agent, see Issuer; `country_code` must be `PE` (2548). The address is written with `CityName` = province and `CountrySubentity` = department, as SUNAT's sheet says; an `urbanization` of `"-"` is left out |
| `supplier` | `{ "document_type": "6", "document_number", "legal_name" }`: a RUC of 11 digits, not the issuer's (2620) |
| `regime` | catalog 23: `"01"` (3 %) or `"02"` (6 %, only for related documents issued up to 28/02/2014, 2985) (2618) |
| `rate` | the regime's: `"3.00"` for `01`, `"6.00"` for `02` (2619) |
| `note` | optional, up to 250 characters |
| `total_retained` | PEN, the exact sum of the lines' `retention.amount`, no tolerance (2628) |
| `total_paid` | PEN, the exact sum of the lines' `retention.net_paid` plus `rounding_amount`, no tolerance (2629) |
| `rounding_amount` | optional, PEN, absolute value up to 1 (3303) |
| `lines` | at least one, one per payment of a related document; see Retention line |

## Retention line

| Field | Notes |
|---|---|
| `related_document` | `{ "document_type", "number", "issue_date", "currency", "total" }`; type `"01"`, `"08"` or `"12"` (2692; a receipt `03` is rejected); `number` is `SERIES-NUMBER` (2694); `total` positive (2696) |
| `payment` | `{ "number"?, "date", "amount" }` in the related document's currency. `number` up to 9 digits, unique for the same related document within the voucher (2626); by default 1, 2, … in the order the document appears. All payment dates in one month (2661), not after the voucher's issue date and not before the first day of its month, or the related document's issue date when it is from that month (2625) |
| `retention` | `{ "date"?, "amount", "net_paid" }` in PEN; `date` defaults to the payment date. `amount` = `payment.amount` × rate (× `exchange_rate.rate` when not PEN) and `net_paid` = `payment.amount` (× `exchange_rate.rate`) − `amount`, each within ±1 PEN (2623) |
| `exchange_rate` | `{ "source_currency", "target_currency", "rate", "date" }`, required when the related document is not in PEN (2719, 2721, 2722): `source_currency` is the document's currency (2749), `target_currency` is `"PEN"` (2715), `rate` up to 4 integer digits and 6 decimals (2716) |

## What is checked, and what is not

The SUNAT validation rules of sheet «Retenciones1_0» that read only the document are checked in the
sheet's order, except that a line's exchange rate goes before the amounts it converts (the beta answered 2715, not
2623, to a line that breaks both), each rejected with the code SUNAT would answer (`{"error": "$.rate: must be the
regime's rate (01: 3.00, 02: 6.00) (SUNAT ERR-2619)"}`); one test per code in
`crates/sunat-model/tests/retention_rules.rs`. The vectors are `fixtures/retention/`: `v1` to `v3`
(PEN, several documents and payments, USD) build, sign and pass the independent verifier
(`crates/sunat-ubl/tests/retention.rs`); `v4` (6 % with regime 01), `v5` (USD amounts without the
exchange rate, target currency USD) and `v6` (totals summed before rounding the lines) are rejected
with 2619, 2715 then 2623, and 2628 then 2629. On 17/10/2026 the SUNAT beta (`otroscpe`) accepted `v1` to `v3`
with code 0 and no notes, rejected `v4` with 2619, `v5` with 2715 and a payment number repeated for the same
document with 2626, and accepted `v6` with code 0 although its totals differ by 0.01 from the sum of the lines: the beta does not apply 2628 and 2629, and the SDK keeps rejecting that locally because SUNAT's validation sheet allows no difference. An
identical resend of an accepted voucher got code 0 again, not 1033: the beta keeps no record, so what production
answers to a resend is still unproven. The signed XML sent, the CLI's answer and the CDR of each are in
`fixtures/retention/beta/`. There is no SUNAT schema for `Retention` in the repository.

The send term (2600) is not part of `validate`, which has no clock: `Retention::check_send_deadline(now,
max_days)` checks it with the term as a parameter. Note: RS 274-2015 art. 41 gives 7 calendar days
from the day after the issue date, but SUNAT's rules defer it to «the regulation in force» since
01/11/2024 without citing it; pass the term, do not assume it.

Not checked here, because SUNAT answers them from its registries: whether the issuer is a retention
agent (2617), whether the supplier's RUC exists (2621) and its condition (4091, an observation), and
whether the numbering was already used (1033). The S/ 700 threshold and the exclusions of RS
037-2002 are the agent's decision, not the SDK's.

# Purchase settlement (04)

The liquidación de compra electrónica: UBL 2.1 `SelfBilledInvoice`, SUNAT customization 2.0. The buyer issues it, with
its RUC, for a purchase from a natural person without RUC (RCP art. 6 num. 1.3). `build` renders it with `file_name`
`RUC-04-SERIES-NUMBER`; the CLI sends it with `send` to the invoice `billService`. The roles of an invoice are reversed:
the issuer is `cac:AccountingCustomerParty` and the seller `cac:AccountingSupplierParty`.

Whether a settlement may be issued (the seller's condition, the goods, the 75 UIT ceiling: SUNAT 2451) and the rules
that query SUNAT's registries (the seller's DNI 2450 to 2462, the buyer's RUC 2010, 2011, 2463) are the caller's and
SUNAT's. As everywhere else, the caller gives every amount and the SDK reconciles them.

Not supported, rejected with an explicit error: operation type `0502` (advances) and `totals.prepaid`; a series that
starts with a digit (printed settlements); free lines (9996) and other taxes (9999).

## Settlement (`kind: "self_billed"`)

| Field | Notes |
|---|---|
| `ubl_version` | `"2.1"` (2074) |
| `operation_type` | catalog 51: `"0501"` purchase, `"0503"` purchase of gold (3206) |
| `document_type` | `"04"` (1003) |
| `series`, `number` | `L` and 3 uppercase letters or digits; 1 to 8 digits (1001) |
| `issue_date`, `currency` | as in an invoice; `currency` ISO 4217 (3088). `SelfBilled::check_send_deadline` (Rust) checks the term: the issue date or the next day (2108), never before it (2329) |
| `issuer` | the buyer, see Issuer; `country_code`, when given, `PE` (4041) |
| `seller` | `{ "document_type", "document_number", "legal_name", "address" }`. `document_type` `"1"` DNI, `"4"` carné de extranjería or `"7"` passport (2800); `document_number` required (2014): a DNI has 8 digits, the others up to 15 characters without whitespace (2802); `legal_name` 1 to 1500 characters (2021, 2022) |
| `seller.address` | `street` 3 to 250 characters, not starting with a space or a tab (2452, 2593); `ubigeo` (2453); `address_type`, catalog 60 (2456, 2457); `country_code`, `department`, `province`, `district`, `urbanization` optional |
| `operation_place` | where the purchase took place (`cac:DeliveryTerms`), required: without it, 2458 on `$.operation_place.location_type`, as the beta answers; `street` as the seller's (2454, 2778), `ubigeo` of 6 digits (2455, 2917), `location_type`, catalog 60 (2458, 2459) |
| `lines` | at least one; see Settlement line |
| `totals` | `taxed`, `exempt`, `unaffected`, `igv`, `income_tax_retention_base`, `income_tax_retention`, `total_taxes` (the IGV, without the retention: 4301), `sale_value` (Σ `line_value`: 4309), `subtotal` (`sale_value` + IGV: 4310), `rounding` (absolute value up to 1: 4314), `payable` (`subtotal` − IGV − retention + `rounding`: 4312). The retention base and amount are the sums of the lines' (4345, 4346). Each within ±1, as the sheet allows |
| `legends`, `despatch_references` | as in an invoice |

The catalog 60 list is not in the sources the SDK was written from: `address_type` and `location_type` are checked as
two digits.

## Settlement line

As an invoice line (`unit`, `quantity`, `product_code`, `description`, `unit_value`, `igv_base`, `igv_rate`, `igv`,
`total_taxes`, `line_value`), with these differences:

| Field | Notes |
|---|---|
| `sunat_product_code` | required, catalog 25, 8 digits (3506, 3496). The beta observes 4337 on a code that is not at class level (`11140000`) |
| `unit_price` | required (2028, 2367) |
| `affectation` | `"10"` taxed, `"20"` exempt, `"30"` unaffected (2040); `igv` = `igv_base` × `igv_rate` ±1 (3103), 0 when not taxed (3110); `igv_base` is the `line_value` (4294) |
| `total_taxes` | the line's IGV (4293) |
| `income_tax_retention` | required: `{ "base", "rate", "amount" }` (2464). `base` is the `line_value` (4294); `rate` `"1.50"`, or `"4.00"` for gold (4237); `amount` = `base` × `rate` / 100 ±1 (2464, 2033). It renders a second `TaxSubtotal` with tax 3000 (`IR`, `TOX`) |
| `item_properties` | operation type `0503` only: `[{ "code", "value" }]` with codes `6000`, `6004`, `6005` and `6006`, each with a value (2466 to 2469, 3064) |

## What is checked, and what is not

The rules of sheet «LiquidacionCompra2_0» that read only the document, each rejected with the code SUNAT would answer,
the totals' observations included; one test per code in `crates/sunat-model/tests/self_billed_rules.rs`. The vectors
are `fixtures/self-billed/`: `v1` (0501, retention 1.5 %) and `v2` (0503, gold, 4 %) build, sign and pass the
independent verifier (`crates/sunat-ubl/tests/self_billed.rs`); `n1` (a seller with RUC) fails with 2800 and `n2`
(no `operation_place`) with 2458. On 18/10/2026 the SUNAT beta accepted `v1` with code 0 and observation 4337 (the
vector's product code) and `v2` with code 0 and no notes, rejected `n1` with 2800, and rejected `n2` with 2458, the code the SDK gives: the
beta checks the place's location type before its address (`fixtures/self-billed/beta/`).

## Reversal summary (`kind: "reversal"`, RR)

The baja of a purchase settlement: the void communication's UBL 2.0 `VoidedDocuments` (same fields as
[`voided`](#void-communication-kind-voided-ra)) with identifier `RR-YYYYMMDD-<number>` (communication date) and
`file_name` `RUC-RR-YYYYMMDD-N`. In Rust it is `Document::Reversal(Voided)`. The CLI sends it with `summary` to SUNAT's
**`otroscpe`** service and asks its `ticket` there: on 18/10/2026 the beta's invoice `billService` answered 0151 to
the RR, and `otroscpe` gave a ticket whose `getStatus` was code 0 (`fixtures/self-billed/beta/v3-*`).

| Field | Notes |
|---|---|
| `number` | 1 to 5 digits (2220) |
| `generation_date` | date of issue of the settlement it reverses (`ReferenceDate`); not after `communication_date` (2671) |
| `communication_date` | date the summary is sent (`IssueDate`); at most 6 days after `generation_date` (2958, below) |
| `issuer` | the buyer that issued the settlement, see Issuer |
| `lines` | `[{ "document_type": "04", "series": "L001", "number": "1", "reason": "..." }]`: type `04` (2308), series `L` and 3 uppercase letters or digits (2345), number 1 to 8 digits (2312), each settlement once (2348), `reason` not blank (2315) and of 3 characters or more (4203) |

The deadline: SUNAT's sheet «ResumenReversiones1_0» (ERR-2958, from 01/08/2026) allows 5 days from the settlement's
accepted CDR, where RS 244-2019 art. 3.2 says 7 from its issue. The SDK does not know the CDR's date, which may be the
day after the issue (2108), so it rejects with 2958 only when `communication_date` is more than 6 days after
`generation_date`; a summary sent on day 6 may still be rejected by SUNAT. Only SUNAT checks that the settlement
exists, was accepted, is not already reversed and has no payments registered (2750, 2398, 2751, 2471). One test per
code in `crates/sunat-model/tests/reversal_rules.rs`; `v3` builds, signs and passes the independent verifier
(`crates/sunat-ubl/tests/self_billed.rs`), and `n3` (7 days) fails with 2958.
