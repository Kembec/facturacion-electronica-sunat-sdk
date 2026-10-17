"""SUNAT (Peru) electronic invoicing: build, sign and zip vouchers and read SUNAT's answers.

The same Rust core and the same commands as the npm package (``build``, ``sign``, ``zip``, ``cdr``,
``soap_request``, ``soap_response``, ``soap_timeout``) with the same input and output shapes
(``docs/cli.md`` of the repository). Inputs are a JSON string or a mapping; results are the parsed
JSON (a ``dict``). :func:`exchange` does the POST to SUNAT with :mod:`urllib` and verified TLS, and
:func:`validity` asks the Consulta Integrada (token, then query) the same way. :func:`gre_send` and
:func:`gre_ticket` deliver the despatch advice (09) by SUNAT's GRE REST API, which has no test environment.
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
    "gre_send",
    "gre_send_request",
    "gre_send_response",
    "gre_ticket",
    "gre_ticket_request",
    "gre_ticket_response",
    "gre_timeout",
    "gre_token_request",
    "gre_token_response",
    "gre_token_usable",
    "sign",
    "soap_request",
    "soap_response",
    "soap_timeout",
    "validity",
    "validity_request",
    "validity_response",
    "validity_timeout",
    "validity_token_request",
    "validity_token_response",
    "zip",
    "__version__",
]

try:
    __version__ = version("facturacion-electronica-sunat-sdk")
except PackageNotFoundError:  # imported from a build tree without metadata
    __version__ = "0.0.0"

Json = Union[str, Mapping[str, Any]]
Exchange = Literal["send", "summary", "ticket", "status"]
GreStage = Literal["token", "send", "ticket"]


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


def validity_token_request(client_id: str, client_secret: str, base: Optional[str] = None) -> Dict[str, Any]:
    """Consulta Integrada token request -> ``{method, url, headers, body}``. The body carries the secret."""
    return json.loads(_native.validity_token_request(client_id, client_secret, base))


def validity_token_response(http_status: int, body: bytes) -> Dict[str, Any]:
    """The token answer -> ``{outcome: token, access_token, token_type, expires_in}`` or ``no_verdict``."""
    return json.loads(_native.validity_token_response(http_status, bytes(body)))


def validity_request(input: Json, access_token: str, base: Optional[str] = None) -> Dict[str, Any]:
    """The query for the CLI's ``validity`` input -> ``{method, url, headers, body}``."""
    return json.loads(_native.validity_request(_text(input), access_token, base))


def validity_response(http_status: int, body: bytes) -> Dict[str, Any]:
    """The query answer -> ``validity``, ``query_error`` or ``no_verdict``, as the CLI prints it."""
    return json.loads(_native.validity_response(http_status, bytes(body)))


def validity_timeout(stage: Literal["token", "query"], seconds: int) -> Dict[str, Any]:
    """No answer within ``seconds`` for the token or the query call -> ``no_verdict``."""
    return json.loads(_native.validity_timeout(stage, int(seconds)))


def gre_token_request(client_id: str, client_secret: str, user: str, password: str, token_base: str) -> Dict[str, Any]:
    """GRE token request (``grant_type=password``) -> ``{method, url, headers, body}``.

    ``user`` is the RUC followed by the SOL user; ``token_base`` has no default. The body carries the
    client secret and the SOL password: never log it.
    """
    return json.loads(_native.gre_token_request(client_id, client_secret, user, password, token_base))


def gre_token_response(http_status: int, body: bytes, received_at: int) -> Dict[str, Any]:
    """The token answer and the Unix seconds when it arrived -> ``token`` (with ``expires_at``), ``api_error`` or ``no_verdict``."""
    return json.loads(_native.gre_token_response(http_status, bytes(body), int(received_at)))


def gre_token_usable(token: Json, now: int) -> bool:
    """Whether a ``token`` outcome can still be used at ``now`` (Unix seconds): renewed ten minutes before expiry."""
    return _native.gre_token_usable(_text(token), int(now))


def gre_send_request(input: Json, access_token: str, base: str) -> Dict[str, Any]:
    """The send of ``{name, xml | xml_base64}`` (the signed guide) -> ``{method, url, headers, body, zip_name, hash_zip}``."""
    return json.loads(_native.gre_send_request(_text(input), access_token, base))


def gre_send_response(http_status: int, body: bytes) -> Dict[str, Any]:
    """The send answer -> ``ticket`` or ``api_error``; raises when it is not GRE-M's: the guide may have arrived."""
    return json.loads(_native.gre_send_response(http_status, bytes(body)))


def gre_ticket_request(ticket: str, access_token: str, base: str) -> Dict[str, Any]:
    """The ticket query (a GET) -> ``{method, url, headers, body}``."""
    return json.loads(_native.gre_ticket_request(ticket, access_token, base))


def gre_ticket_response(http_status: int, body: bytes, ticket: str) -> Dict[str, Any]:
    """The ticket answer -> ``ticket_status`` (0, 98, 99), ``api_error`` or ``no_verdict``, all with ``ticket``."""
    return json.loads(_native.gre_ticket_response(http_status, bytes(body), ticket))


def gre_timeout(stage: GreStage, seconds: int, ticket: Optional[str] = None) -> Dict[str, Any]:
    """No answer within ``seconds`` -> ``no_verdict`` for ``token`` and ``ticket`` (with the ticket); raises for ``send``."""
    return json.loads(_native.gre_timeout(stage, int(seconds), ticket))


from ._http import exchange, validity  # noqa: E402 - uses the functions above
from ._gre import gre_send, gre_ticket  # noqa: E402 - uses the functions above
