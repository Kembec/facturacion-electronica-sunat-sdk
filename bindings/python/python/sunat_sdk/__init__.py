"""SUNAT (Peru) electronic invoicing: build, sign and zip vouchers and read SUNAT's answers.

The same Rust core and the same commands as the npm package (``build``, ``sign``, ``zip``, ``cdr``,
``soap_request``, ``soap_response``, ``soap_timeout``) with the same input and output shapes
(``docs/cli.md`` of the repository). Inputs are a JSON string or a mapping; results are the parsed
JSON (a ``dict``). :func:`exchange` does the POST to SUNAT with :mod:`urllib` and verified TLS.
"""

from __future__ import annotations

import json
from importlib.metadata import PackageNotFoundError, version
from typing import Any, Dict, Literal, Mapping, Optional, Union

from . import _native
from ._native import SunatSdkError

__all__ = [
    "SunatSdkError",
    "build",
    "cdr",
    "exchange",
    "sign",
    "soap_request",
    "soap_response",
    "soap_timeout",
    "zip",
    "__version__",
]

try:
    __version__ = version("facturacion-electronica-sunat-sdk")
except PackageNotFoundError:  # imported from a build tree without metadata
    __version__ = "0.0.0"

Json = Union[str, Mapping[str, Any]]
Exchange = Literal["send", "summary", "ticket", "status"]


def _text(value: Json) -> str:
    return value if isinstance(value, str) else json.dumps(value, ensure_ascii=False)


def build(document: Json) -> Dict[str, Any]:
    """Renders the unsigned XML of a document (``docs/input-model.md``) -> ``{file_name, xml}``."""
    return json.loads(_native.build(_text(document)))


def sign(xml: str, cert_pem: str, key_pem: str, signature_id: Optional[str] = None) -> Dict[str, Any]:
    """Signs an unsigned XML with an unencrypted RSA key -> ``{xml, digest_value, signature_value}``.

    ``signature_id`` defaults to ``signatureKG``.
    """
    return json.loads(_native.sign(xml, cert_pem, key_pem, signature_id))


def zip(input: Json) -> Dict[str, Any]:  # noqa: A001 - the command's name in every binding
    """``{name, xml | xml_base64}`` -> ``{file_name, zip_base64}``."""
    return json.loads(_native.zip(_text(input)))


def cdr(input: Json) -> Dict[str, Any]:
    """``{cdr_base64}`` (zip or XML) -> ``{reference_id, response_code, description, notes, class}``."""
    return json.loads(_native.cdr(_text(input)))


def soap_request(exchange: Exchange, input: Json, user: str, password: str) -> Dict[str, Any]:
    """The POST of an exchange -> ``{method, headers, body}``. The body carries the SOL credentials."""
    return json.loads(_native.soap_request(exchange, _text(input), user, password))


def soap_response(exchange: Exchange, input: Json, http_status: int, body: bytes) -> Dict[str, Any]:
    """SUNAT's HTTP status and body, with the input given to :func:`soap_request` -> the outcome."""
    return json.loads(_native.soap_response(exchange, _text(input), http_status, bytes(body)))


def soap_timeout(exchange: Exchange, input: Json, seconds: int) -> Dict[str, Any]:
    """No answer within ``seconds`` -> ``no_verdict`` for ticket and status; raises for send and summary."""
    return json.loads(_native.soap_timeout(exchange, _text(input), int(seconds)))


from ._http import exchange  # noqa: E402 - uses the functions above
