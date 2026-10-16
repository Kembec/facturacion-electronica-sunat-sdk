"""`validity` against a local HTTP server standing in for SUNAT's two APIs: the calls it makes and the
outcome it reads, with the answers the Consulta Integrada manual prints. No network."""

from __future__ import annotations

import json
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

import pytest

import sunat_sdk
from conftest import ROOT

DATA = ROOT / "crates/sunat-sdk/tests/data/validity"
QUERY = {"ruc": "20100066603", "document_type": "03", "series": "B001", "number": 7, "issue_date": "2026-10-15", "total": "118"}
CLIENT_ID = "c04864ce-f454-48e1-b41d-8730e575b905"


class Sunat(BaseHTTPRequestHandler):
    answers: dict = {}
    seen: list = []

    def do_POST(self):  # noqa: N802 - http.server API
        body = self.rfile.read(int(self.headers["Content-Length"]))
        Sunat.seen.append((self.path, dict(self.headers), body))
        status, payload, delay = next(a for prefix, a in Sunat.answers.items() if self.path.startswith(prefix))
        time.sleep(delay)
        try:
            self.send_response(status)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(payload)))
            self.end_headers()
            self.wfile.write(payload)
        except OSError:  # the client gave up first
            pass

    def log_message(self, *args):  # requests carry the secret and the token: nothing is printed
        pass


@pytest.fixture()
def sunat():
    server = ThreadingHTTPServer(("127.0.0.1", 0), Sunat)
    server.daemon_threads = True
    threading.Thread(target=server.serve_forever, daemon=True).start()
    Sunat.seen = []
    base = f"http://127.0.0.1:{server.server_address[1]}"
    yield {"token_base": f"{base}/token", "base": f"{base}/api"}
    server.shutdown()
    server.server_close()


def answer(name: str, status: int = 200, delay: float = 0) -> tuple:
    return (status, (DATA / f"{name}.json").read_bytes(), delay)


def test_token_then_query_reads_the_manual_answer(sunat):
    Sunat.answers = {"/token": answer("manual-token-200"), "/api": answer("manual-query-1")}
    out = sunat_sdk.validity(QUERY, CLIENT_ID, "secret==", **sunat)
    assert (out["outcome"], out["state"], len(out["observations"])) == ("validity", "accepted", 2)
    (token_path, _, token_body), (query_path, query_headers, query_body) = Sunat.seen
    assert token_path == f"/token/{CLIENT_ID}/oauth2/token/"
    assert b"grant_type=client_credentials" in token_body and b"client_secret=secret%3D%3D" in token_body
    assert query_path == "/api/20100066603/validarcomprobante"
    assert query_headers["Authorization"] == "Bearer test-access-token-not-a-real-jwt"
    assert json.loads(query_body)["fechaEmision"] == "15/10/2026"
    assert "test-access-token" not in json.dumps(out)


@pytest.mark.parametrize("code,state", [("0", "not_reported"), ("2", "voided"), ("3", "printed_authorized"), ("4", "printed_not_authorized")])
def test_each_state(sunat, code, state):
    Sunat.answers = {"/token": answer("manual-token-200"), "/api": answer(f"derived-query-{code}")}
    assert sunat_sdk.validity(QUERY, CLIENT_ID, "s", **sunat)["state"] == state


def test_errors_and_silence(sunat):
    Sunat.answers = {"/token": answer("derived-token-401", 401)}
    refused = sunat_sdk.validity(QUERY, CLIENT_ID, "s", **sunat)
    assert (refused["outcome"], refused["stage"], refused["http_status"]) == ("no_verdict", "token", 401)

    Sunat.answers = {"/token": answer("manual-token-200"), "/api": answer("manual-query-422", 422)}
    malformed = sunat_sdk.validity(QUERY, CLIENT_ID, "s", **sunat)
    assert (malformed["outcome"], malformed["error_code"]) == ("query_error", "99")

    Sunat.answers = {"/token": answer("manual-token-200"), "/api": answer("derived-query-401", 401)}
    assert sunat_sdk.validity(QUERY, CLIENT_ID, "s", **sunat)["stage"] == "query"

    Sunat.answers = {"/token": answer("manual-token-200"), "/api": answer("manual-query-1", 200, 3)}
    late = sunat_sdk.validity(QUERY, CLIENT_ID, "s", timeout=1.5, **sunat)
    assert (late["outcome"], late["stage"], late["reason"]) == ("no_verdict", "query", "timeout")


def test_a_bad_input_makes_no_call(sunat):
    with pytest.raises(sunat_sdk.SunatSdkError, match="document_type"):
        sunat_sdk.validity({**QUERY, "document_type": "09"}, CLIENT_ID, "s", **sunat)
    assert Sunat.seen == []
