# CLAUDE.md — facturacion-electronica-sunat-sdk

SDK for SUNAT (Peru) electronic invoicing: one Rust core, builds for PHP, JS/WASM, Python and Go.
Covers invoices (01), receipts (03), credit (07) and debit (08) notes, daily summaries (RC) and void
communications (RA). Contracts: `docs/architecture.md`, `docs/input-model.md`.

## Rules of this repository

- A document type is accepted only when its output matches the compatibility vectors in
  `tests/vectors/` (C14N, `DigestValue`, `SignatureValue`) and SUNAT beta accepts it.
- A production consumer adopts a version only after it passes the whole battery for the documents it
  emits, through the build of its own language, with a way back to its previous implementation.
- The core only canonicalizes and signs XML it generates itself.
- No certificates, private keys, SOL credentials or tokens in the repository, tests or logs, except the
  self-generated test certificate in `tests/vectors/test-cert/`, valid for no taxpayer.
- TLS verification is never disabled.
- Code identifiers and commit messages in English.
