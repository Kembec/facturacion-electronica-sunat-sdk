# Error code → input field

| File | Source |
|---|---|
| `error-fields.source.json` | The code → UBL node → input field table, 711 rows for invoices (01), receipts (03), credit (07) and debit (08) notes, read from SUNAT's «Reglas de validación» updated 26.08.2026 (sheets `Factura2_0`, `Boleta2_0`, `NotaCredito2_0`, `NotaDebito2_0`, `General`, `Firma`; each row names its `source_rows`). The last 9 rows, type `RR`, are the reversal summary of a purchase settlement (sheet `ResumenReversiones1_0`, 24.04.2026), one per code the SDK checks for `kind: "reversal"`. The 26.08 workbook was taken from a provider's mirror (sha256 `cb5e871c…2da73b6`); the one at SUNAT's own URL is of 24.04.2026 and differs only by code 3209 in 07 and 08. `input_path` follows `docs/input-model.md`: a field, `"sdk"` (the SDK writes the node: constants, catalogue attributes, signature, file name) or null (the input model has no such field). `certainty` is `fuente` when the node is the rule's and one field writes it, `inferido` when the rule is a computation over several nodes or the row groups several; the reason follows `INFERIDO:` in `note` |
| `faultstring-2800.txt` | The `faultstring` of a SUNAT beta rejection (17/10/2026, an invoice with `customer.document_type` 1 where 6 is required), as sent |
| `fault-2335.xml` | A SUNAT beta fault 2335 (altered document, digest mismatch), whole envelope |

`crates/sunat-sdk/data/error-fields.json` is generated from the source by
`python3 -I crates/sunat-sdk/data/generate-error-fields.py`; the test `error_fields_match_their_source` fails when
they drift apart. The `(nodo: "…" valor: "…")` detail of a message is not documented by SUNAT: it is the form observed
in the beta answer above.
