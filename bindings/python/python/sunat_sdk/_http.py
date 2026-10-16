"""The POST to SUNAT, with the standard library: what the npm README leaves to the caller's ``fetch``.

TLS is verified with the platform's default context and never disabled. Nothing here logs: the
request body carries the SOL credentials.
"""

from __future__ import annotations

import math
import socket
import time
import ssl
import urllib.error
import urllib.request
from typing import Any, Dict, Optional

from . import (
    Exchange,
    Json,
    soap_request,
    soap_response,
    soap_timeout,
    validity_request,
    validity_response,
    validity_timeout,
    validity_token_request,
    validity_token_response,
)

DEFAULT_TIMEOUT_SECONDS = 60


def exchange(
    kind: Exchange,
    input: Json,
    user: str,
    password: str,
    endpoint: str,
    timeout: float = DEFAULT_TIMEOUT_SECONDS,
    context: Optional[ssl.SSLContext] = None,
) -> Dict[str, Any]:
    """One call to SUNAT's web service: the CLI's output for ``send``, ``summary``, ``ticket`` or ``status``.

    ``user`` is the RUC followed by the SOL user. ``send``, ``summary`` and ``ticket`` go to the
    billService URL and ``status`` to the billConsultService URL. A SOAP fault is a result
    (``outcome: fault``). Without an answer, ``ticket`` and ``status`` return ``no_verdict`` and
    ``send`` and ``summary`` raise :class:`SunatSdkError`: the document may have arrived.
    ``context`` replaces the default verified TLS context (for example, to add a CA).
    """
    post = soap_request(kind, input, user, password)
    request = urllib.request.Request(
        endpoint, data=post["body"].encode("utf-8"), headers=post["headers"], method=post["method"]
    )
    tls = context if context is not None else ssl.create_default_context()
    try:
        with urllib.request.urlopen(request, timeout=timeout, context=tls) as response:
            return soap_response(kind, input, response.status, response.read())
    except urllib.error.HTTPError as error:
        with error:
            return soap_response(kind, input, error.code, error.read())
    except (socket.timeout, TimeoutError):
        return soap_timeout(kind, input, math.ceil(timeout))
    except urllib.error.URLError as error:
        if isinstance(error.reason, (socket.timeout, TimeoutError)):
            return soap_timeout(kind, input, math.ceil(timeout))
        raise


def validity(
    input: Json,
    client_id: str,
    client_secret: str,
    timeout: float = DEFAULT_TIMEOUT_SECONDS,
    context: Optional[ssl.SSLContext] = None,
    token_base: Optional[str] = None,
    base: Optional[str] = None,
) -> Dict[str, Any]:
    """The Consulta Integrada for one voucher: the CLI's ``validity`` output.

    Asks a token with the «Credenciales de API SUNAT» and then the query, both inside ``timeout``.
    ``input`` is ``{ruc, document_type, series, number, issue_date, total, requester_ruc?}``. Without
    an answer (401 included) it returns ``no_verdict`` with the ``stage`` that failed. The token and
    the secret are never returned. ``token_base`` and ``base`` replace SUNAT's URL bases.
    """
    deadline = time.monotonic() + timeout
    token_call = validity_token_request(client_id, client_secret, token_base)
    validity_request(input, "unused", base)  # the input is checked before the token is spent
    tls = context if context is not None else ssl.create_default_context()
    token = _rest(token_call, timeout, tls, "token")
    if token["outcome"] != "token":
        return token
    left = deadline - time.monotonic()
    if left < 1:
        return validity_timeout("query", math.ceil(timeout))
    return _rest(validity_request(input, token["access_token"], base), left, tls, "query")


def _rest(call: Dict[str, Any], timeout: float, tls: ssl.SSLContext, stage: str) -> Dict[str, Any]:
    read = validity_token_response if stage == "token" else validity_response
    request = urllib.request.Request(
        call["url"], data=call["body"].encode("utf-8"), headers=call["headers"], method=call["method"]
    )
    try:
        with urllib.request.urlopen(request, timeout=timeout, context=tls) as response:
            return read(response.status, response.read())
    except urllib.error.HTTPError as error:
        with error:
            return read(error.code, error.read())
    except (socket.timeout, TimeoutError):
        return validity_timeout(stage, math.ceil(timeout))
    except urllib.error.URLError as error:
        if isinstance(error.reason, (socket.timeout, TimeoutError)):
            return validity_timeout(stage, math.ceil(timeout))
        raise
