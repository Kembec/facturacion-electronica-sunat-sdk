"""Every fixture through the installed package: the unsigned XML equals its vector under inclusive
C14N 1.0 (lxml, independent of the core), and signing it gives the vector's DigestValue and
SignatureValue. Same checks as crates/sunat-ubl/tests/vectors.rs and bindings/wasm/test/vectors.mjs."""

from __future__ import annotations

import json
import re

import pytest
from lxml import etree

import sunat_sdk
from conftest import EXPECTED_CASES, cases, read

DECLARATION = '<?xml version="1.0" encoding="UTF-8"?>\n'
CASES = cases()


def c14n(xml: str) -> bytes:
    return etree.tostring(etree.fromstring(xml.encode("utf-8")), method="c14n", exclusive=False, with_comments=False)


def element_text(xml: str, element: str) -> str:
    match = re.search(rf"<{element}>([^<]*)<", xml)
    assert match, f"no {element}"
    return re.sub(r"\s+", "", match.group(1))


def test_every_fixture_is_covered():
    assert len(CASES) == EXPECTED_CASES, CASES


@pytest.mark.parametrize("case", CASES)
def test_build_and_sign_reproduce_the_vector(case, cert, key):
    built = sunat_sdk.build(read(f"fixtures/{case}.json"))
    assert built["file_name"].split("-")[1].lower() == case.split("-")[0], "file_name document type"

    assert c14n(built["xml"]) == c14n(read(f"tests/vectors/{case}.unsigned.xml")), "C14N"
    assert built["xml"].startswith(DECLARATION)
    assert built["xml"][len(DECLARATION):].encode("utf-8") == c14n(built["xml"]), "output already canonical"

    signed = sunat_sdk.sign(built["xml"], cert, key)
    expected = read(f"tests/vectors/{case}.signed.xml")
    assert signed["digest_value"] == element_text(expected, "ds:DigestValue")
    assert signed["signature_value"] == element_text(expected, "ds:SignatureValue")
    assert 'Id="signatureKG"' in signed["xml"]


def test_build_takes_a_mapping_like_its_json():
    text = read(f"fixtures/{CASES[0]}.json")
    assert sunat_sdk.build(json.loads(text)) == sunat_sdk.build(text)
