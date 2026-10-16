"""Validates XML files against an XSD with lxml: `validate_xsd.py SCHEMA FILE...`.

Prints `OK <file>` for each valid file and the schema's errors for each invalid one; exits 1 if any
file is invalid. Used by `tests/xsd.rs`.
"""

import sys

from lxml import etree


def main(schema_path, files):
    schema = etree.XMLSchema(etree.parse(schema_path))
    failed = False
    for path in files:
        document = etree.parse(path)
        if schema.validate(document):
            print(f"OK {path}")
            continue
        failed = True
        for error in schema.error_log:
            print(f"INVALID {path}:{error.line}: {error.message}")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1], sys.argv[2:]))
