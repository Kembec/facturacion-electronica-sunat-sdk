# CLAUDE.md — facturacion-electronica-sunat-sdk

SDK for SUNAT (Peru) electronic invoicing: one Rust core, builds for PHP, JS/WASM, Python and Go.
Covers invoices (01), receipts (03), credit (07) and debit (08) notes, daily summaries (RC), void
communications (RA) and the sender's despatch advice (09, built and signed; not sent). Contracts:
`docs/architecture.md`, `docs/input-model.md`.

## Rules of this repository

- A document type is accepted only when its output matches the compatibility vectors in
  `tests/vectors/` (C14N, `DigestValue`, `SignatureValue`) and SUNAT beta accepts it.
  The despatch advice (09) has no SUNAT test environment: it is held to its vectors, the OASIS UBL
  2.1 schema and one test per SUNAT rule code (`docs/input-model.md`).
- A production consumer adopts a version only after it passes the whole battery for the documents it
  emits, through the build of its own language, with a way back to its previous implementation.
- The core only canonicalizes and signs XML it generates itself.
- No certificates, private keys, SOL credentials or tokens in the repository, tests or logs, except the
  self-generated test certificate in `tests/vectors/test-cert/`, valid for no taxpayer.
- TLS verification is never disabled.
- Code identifiers and commit messages in English.
