"""GRE (despatch advice, 09) by SUNAT's REST API, with the standard library: token, then the send or
the ticket query, both inside one wait, as the CLI's ``gre-send`` and ``gre-ticket`` do.

SUNAT has no test environment for GRE: nothing here has been accepted by SUNAT. The URL bases have no
default and always come from the caller. TLS is verified with the platform's default context and never
disabled. Nothing here logs: the token body carries the client secret and the SOL password.
"""

from __future__ import annotations

import math
import socket
import ssl
import time
import urllib.error
import urllib.request
from typing import Any, Dict, Optional, Tuple

from . import (
    Json,
    gre_send_request,
    gre_send_response,
    gre_ticket_request,
    gre_ticket_response,
    gre_timeout,
    gre_token_request,
    gre_token_response,
)

DEFAULT_TIMEOUT_SECONDS = 60


def gre_send(
    input: Json,
    client_id: str,
    client_secret: str,
    user: str,
    password: str,
    token_base: str,
    base: str,
    timeout: float = DEFAULT_TIMEOUT_SECONDS,
    context: Optional[ssl.SSLContext] = None,
) -> Dict[str, Any]:
    """Sends the signed guide: the CLI's ``gre-send`` output.

    ``input`` is ``{name: "RUC-09-SERIE-NUMERO", xml | xml_base64}``; ``client_id`` and
    ``client_secret`` are the GRE pair of «Credenciales de API SUNAT»; ``user`` is the RUC followed by
    the SOL user. Returns ``ticket`` (with ``zip_name`` and ``hash_zip``: store the ticket first, it is
    the only handle on the outcome), ``api_error``, or ``no_verdict`` of the token stage (nothing was
    sent). Without an answer to the send itself it raises :class:`SunatSdkError`: the guide may have
    arrived, so send the same file again, never the operation with another number.
    """
    deadline = time.monotonic() + timeout
    gre_send_request(input, "unused", base)  # the guide is checked before the token is spent
    tls = context if context is not None else ssl.create_default_context()
    token = _token(client_id, client_secret, user, password, token_base, timeout, tls)
    if token["outcome"] != "token":
        return token
    left = deadline - time.monotonic()
    if left < 1:
        return gre_timeout("token", math.ceil(timeout))
    call = gre_send_request(input, token["access_token"], base)
    answer = _fetch(call, left, tls)
    if answer is None:
        return gre_timeout("send", math.ceil(left))
    out = gre_send_response(*answer)
    if out["outcome"] == "ticket":
        out["zip_name"] = call["zip_name"]
        out["hash_zip"] = call["hash_zip"]
    return out


def gre_ticket(
    ticket: str,
    client_id: str,
    client_secret: str,
    user: str,
    password: str,
    token_base: str,
    base: str,
    timeout: float = DEFAULT_TIMEOUT_SECONDS,
    context: Optional[ssl.SSLContext] = None,
) -> Dict[str, Any]:
    """Asks for the outcome of a sent guide: the CLI's ``gre-ticket`` output, always with ``ticket``.

    ``ticket_status`` with ``state`` ``processed`` (0, with the CDR when SUNAT attaches it),
    ``in_process`` (98: ask again later with the same ticket, never send again) or
    ``processed_with_errors`` (99, with ``error``); ``api_error``; or ``no_verdict`` with the
    ``stage`` that got no answer. The token and the secrets are never returned.
    """
    deadline = time.monotonic() + timeout
    gre_ticket_request(ticket, "unused", base)
    tls = context if context is not None else ssl.create_default_context()
    token = _token(client_id, client_secret, user, password, token_base, timeout, tls)
    if token["outcome"] != "token":
        return {**token, "ticket": ticket}
    left = deadline - time.monotonic()
    if left < 1:
        return {**gre_timeout("token", math.ceil(timeout)), "ticket": ticket}
    answer = _fetch(gre_ticket_request(ticket, token["access_token"], base), left, tls)
    if answer is None:
        return gre_timeout("ticket", math.ceil(left), ticket)
    return gre_ticket_response(answer[0], answer[1], ticket)


def _token(
    client_id: str, client_secret: str, user: str, password: str, token_base: str, timeout: float, tls: ssl.SSLContext
) -> Dict[str, Any]:
    answer = _fetch(gre_token_request(client_id, client_secret, user, password, token_base), timeout, tls)
    if answer is None:
        return gre_timeout("token", math.ceil(timeout))
    return gre_token_response(answer[0], answer[1], int(time.time()))


def _fetch(call: Dict[str, Any], timeout: float, tls: ssl.SSLContext) -> Optional[Tuple[int, bytes]]:
    """The HTTP status and body, whatever the status; None when the wait ran out."""
    data = call["body"].encode("utf-8") if call["body"] else None
    request = urllib.request.Request(call["url"], data=data, headers=call["headers"], method=call["method"])
    try:
        with urllib.request.urlopen(request, timeout=timeout, context=tls) as response:
            return response.status, response.read()
    except urllib.error.HTTPError as error:
        with error:
            return error.code, error.read()
    except (socket.timeout, TimeoutError):
        return None
    except urllib.error.URLError as error:
        if isinstance(error.reason, (socket.timeout, TimeoutError)):
            return None
        raise
