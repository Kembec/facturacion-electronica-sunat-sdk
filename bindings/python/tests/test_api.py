"""The rest of the surface, as bindings/wasm/src/api_tests.rs checks it."""

from __future__ import annotations

import base64

import pytest

import sunat_sdk
from conftest import ROOT, cases, read

DOC = {"name": "20100066603-01-F001-1", "xml": "<Invoice/>"}


def cdr_base64() -> str:
    data = (ROOT / "crates/sunat-transport/tests/data/R-20150147718-01-FT01-982.zip").read_bytes()
    return base64.b64encode(data).decode("ascii")


def send_bill_answer() -> bytes:
    return (
        '<S:Envelope xmlns:S="s"><S:Body><n:sendBillResponse xmlns:n="n">'
        f"<applicationResponse>{cdr_base64()}</applicationResponse>"
        "</n:sendBillResponse></S:Body></S:Envelope>"
    ).encode("utf-8")


def test_custom_signature_id(cert, key):
    signed = sunat_sdk.sign(read(f"tests/vectors/{cases()[0]}.unsigned.xml"), cert, key, "customSig")
    assert 'Id="customSig"' in signed["xml"]


def test_invalid_input_raises_sunat_sdk_error(cert):
    with pytest.raises(sunat_sdk.SunatSdkError):
        sunat_sdk.build("{not json")
    with pytest.raises(ValueError):
        sunat_sdk.sign("<a/>", cert, "not a key")


def test_sign_error_does_not_carry_key_material(cert, key):
    broken = key.replace("PRIVATE KEY", "BROKEN KEY")
    with pytest.raises(sunat_sdk.SunatSdkError) as error:
        sunat_sdk.sign(read(f"tests/vectors/{cases()[0]}.unsigned.xml"), cert, broken)
    assert key.splitlines()[1][1:] not in str(error.value)


def test_zip_and_cdr():
    assert sunat_sdk.zip(DOC)["file_name"] == "20100066603-01-F001-1.zip"
    cdr = sunat_sdk.cdr({"cdr_base64": cdr_base64()})
    assert (cdr["response_code"], cdr["class"]) == ("2047", "rejected")


def test_soap_request_and_response_match_the_cli():
    post = sunat_sdk.soap_request("send", DOC, "20100066603MODDATOS", "moddatos")
    assert post["method"] == "POST"
    assert post["headers"] == {"Content-Type": "text/xml; charset=utf-8", "SOAPAction": '"urn:sendBill"'}
    assert "<fileName>20100066603-01-F001-1.zip</fileName>" in post["body"]

    sent = sunat_sdk.soap_response("send", DOC, 200, send_bill_answer())
    assert (sent["outcome"], sent["cdr"]["class"]) == ("cdr", "rejected")
    assert sent["cdr"] == sunat_sdk.cdr({"cdr_base64": cdr_base64()})

    unauthorized = sunat_sdk.soap_response("ticket", {"ticket": "1760620000123"}, 401, b"")
    assert (unauthorized["outcome"], unauthorized["ticket"]) == ("no_verdict", "1760620000123")
    assert sunat_sdk.soap_timeout("status", {}, 45)["reason"] == "timeout"
    with pytest.raises(sunat_sdk.SunatSdkError, match="outcome is unknown"):
        sunat_sdk.soap_timeout("send", DOC, 45)
    with pytest.raises(sunat_sdk.SunatSdkError):
        sunat_sdk.soap_request("zip", DOC, "u", "p")
