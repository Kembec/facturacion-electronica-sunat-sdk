"""The POST to SUNAT, with the standard library: what the npm README leaves to the caller's ``fetch``.

TLS is verified with the platform's default context and never disabled. Nothing here logs: the
request body carries the SOL credentials.
"""

from __future__ import annotations

import math
import socket
import ssl
import urllib.error
import urllib.request
from typing import Any, Dict, Optional

from . import Exchange, Json, soap_request, soap_response, soap_timeout

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
