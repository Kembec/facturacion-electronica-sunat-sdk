#!/usr/bin/env python3
"""Generates data/error-fields.json from tests/data/error-fields/error-fields.source.json.

The source is the code -> UBL node -> input field table read from SUNAT's validation rules
(«Reglas de validación», updated 26.08.2026; see tests/data/error-fields/README.md). The output keeps,
per row, what `sunat_sdk::diagnose` needs: `code`, `document_types`, `xpath`, `input_path`, `also`
and `certainty`. A source row whose credit (07) or debit (08) note xpath differs from the invoice one
is split, one row per distinct xpath, so every row carries the node of its own document types. A
source xpath in parentheses is a note, not a node: it becomes null.

Usage: python3 -I crates/sunat-sdk/data/generate-error-fields.py
The test `error_fields_match_their_source` fails when the output and the source drift apart.
"""
import json
import pathlib

CRATE = pathlib.Path(__file__).resolve().parent.parent
SOURCE = CRATE / "tests" / "data" / "error-fields" / "error-fields.source.json"
OUTPUT = CRATE / "data" / "error-fields.json"


def xpath_for(row, document_type):
    return row.get(f"xpath_{document_type}") or row["xpath"]


def rows_of(row):
    groups = {}
    for document_type in row["document_types"]:
        groups.setdefault(xpath_for(row, document_type), []).append(document_type)
    for xpath, document_types in groups.items():
        yield {
            "code": row["code"],
            "document_types": document_types,
            "xpath": None if xpath is None or xpath.startswith("(") else xpath,
            "input_path": row["input_path"],
            "also": row["also"],
            "certainty": row["certainty"],
        }


def main():
    source = json.loads(SOURCE.read_text(encoding="utf-8"))
    rows = [out for row in source for out in rows_of(row)]
    lines = ",\n".join(json.dumps(row, ensure_ascii=False, separators=(",", ":")) for row in rows)
    OUTPUT.write_text(f"[\n{lines}\n]\n", encoding="utf-8")
    print(f"{OUTPUT.relative_to(CRATE)}: {len(rows)} rows from {len(source)}")


if __name__ == "__main__":
    main()
