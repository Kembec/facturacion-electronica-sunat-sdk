# GRE REST exchanges

SUNAT has no test environment for the despatch advice's REST API, so these are the only answers the
tests can replay. Nothing here was recorded against SUNAT and nothing proves acceptance by SUNAT.

| Prefix | Source |
|---|---|
| `manual-*` | Printed in SUNAT's «Manual de servicios web, plataforma nueva GRE» (05/10/2022), §1.3: the general 500 error and the 422 of a ticket query without ticket (`codError` 166), verbatim |
| `derived-*` | Built from the fields GRE-M names (§1.2 b, §1.3) with made-up values: a token (fake, valid for nothing) and a 401 in the general error shape of annex I |
| `fork-*` | From the MIT-licensed Greenter client (`greenter/ws`, `tests/Ws/Api/GreSenderTest.php`, Copyright (c) 2021 Giancarlos Salas), a **secondary source**: the ticket of a send, a 422 whose items use `cod`/`msg`, a ticket in process (98), a ticket processed with error 1345 (99, no CDR) and a ticket with its CDR (`arcCdr` is the base64 of `tests/Resources/cdrGRE.zip`, a CDR of guide T001-1 accepted with note 4398) |

The JSON field names of `fork-*` follow the `attributeMap` of `greenter/gre-api` 1.0 (`numTicket`,
`fecRecepcion`, `codRespuesta`, `indCdrGenerado`, `arcCdr`, `error.numError`, `error.desError`).
