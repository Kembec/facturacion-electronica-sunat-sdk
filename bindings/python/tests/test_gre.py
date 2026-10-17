"""`gre_send` and `gre_ticket` against a local HTTP server standing in for SUNAT's GRE REST API: the
calls they make and the outcome they read, replaying the core's recorded exchanges
(crates/sunat-sdk/tests/data/gre, sources in its README). SUNAT has no test environment for GRE: none of
these proves acceptance by SUNAT. No network."""

from __future__ import annotations

import json
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

import pytest

import sunat_sdk
from conftest import ROOT

DATA = ROOT / "crates/sunat-sdk/tests/data/gre"
GUIDE = {"name": "20000000001-09-T001-1", "xml": "<DespatchAdvice/>"}
CREDENTIALS = ("c04864ce-f454", "s/e=c", "20000000001MODUSER", "p&ss")


class Sunat(BaseHTTPRequestHandler):
    answers: dict = {}
    seen: list = []

    def do_POST(self):  # noqa: N802 - http.server API
        self._answer(self.rfile.read(int(self.headers["Content-Length"])))

    def do_GET(self):  # noqa: N802 - http.server API
        self._answer(b"")

    def _answer(self, body: bytes):
        Sunat.seen.append((self.command, self.path, dict(self.headers), body))
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

    def log_message(self, *args):  # requests carry the secret, the password and the token
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


TOKEN = {"/token": answer("derived-token-200")}


def test_send_asks_a_token_then_posts_the_zip_and_returns_the_ticket(sunat):
    Sunat.answers = {**TOKEN, "/api": answer("fork-send-200")}
    before = int(time.time())
    out = sunat_sdk.gre_send(GUIDE, *CREDENTIALS, **sunat)
    assert (out["outcome"], out["ticket"], out["received_at"]) == ("ticket", "a.b.c", "2023-01-10T20:10:50")
    assert out["zip_name"] == "20000000001-09-T001-1.zip" and len(out["hash_zip"]) == 64
    (_, token_path, _, token_body), (method, send_path, headers, send_body) = Sunat.seen
    assert token_path == "/token/c04864ce-f454/oauth2/token/"
    assert token_body.startswith(b"grant_type=password&") and b"password=p%26ss" in token_body
    assert (method, send_path) == ("POST", "/api/contribuyente/gem/comprobantes/20000000001-09-T001-1")
    assert headers["Authorization"] == "Bearer fake.token.for-tests"
    assert json.loads(send_body)["archivo"]["hashZip"] == out["hash_zip"]
    assert "fake.token" not in json.dumps(out) and "p&ss" not in json.dumps(out)
    token = sunat_sdk.gre_token_response(200, (DATA / "derived-token-200.json").read_bytes(), before)
    assert token["expires_at"] == before + 3600 and sunat_sdk.gre_token_usable(token, before)


def test_send_refused_is_an_api_error_with_the_validation_errors(sunat):
    Sunat.answers = {**TOKEN, "/api": answer("fork-send-422", 422)}
    out = sunat_sdk.gre_send(GUIDE, *CREDENTIALS, **sunat)
    assert (out["outcome"], out["stage"], out["general_error"]) == ("api_error", "send", "unprocessable_entity")
    assert out["errors"] == [{"code": "501", "description": "El valor de codCpe no permitido o no valido"}]


def test_send_without_an_answer_raises_because_the_guide_may_have_arrived(sunat):
    Sunat.answers = {**TOKEN, "/api": (502, b"<html/>", 0)}
    with pytest.raises(sunat_sdk.SunatSdkError, match="may have arrived"):
        sunat_sdk.gre_send(GUIDE, *CREDENTIALS, **sunat)
    Sunat.answers = {**TOKEN, "/api": answer("fork-send-200", 200, 3)}
    with pytest.raises(sunat_sdk.SunatSdkError, match="may have arrived"):
        sunat_sdk.gre_send(GUIDE, *CREDENTIALS, timeout=1.5, **sunat)


def test_a_refused_token_sends_nothing(sunat):
    Sunat.answers = {"/token": answer("derived-error-401", 401)}
    out = sunat_sdk.gre_send(GUIDE, *CREDENTIALS, **sunat)
    assert (out["outcome"], out["stage"], out["general_error"]) == ("api_error", "token", "unauthorized")
    assert [path for _, path, _, _ in Sunat.seen] == ["/token/c04864ce-f454/oauth2/token/"]


@pytest.mark.parametrize(
    "recording,state,generated",
    [("fork-ticket-0", "processed", True), ("fork-ticket-98", "in_process", False), ("fork-ticket-99", "processed_with_errors", False)],
)
def test_ticket_reads_0_98_and_99(sunat, recording, state, generated):
    Sunat.answers = {**TOKEN, "/api": answer(recording)}
    out = sunat_sdk.gre_ticket("a.b.c", *CREDENTIALS, **sunat)
    assert (out["outcome"], out["ticket"], out["state"], out["cdr_generated"]) == ("ticket_status", "a.b.c", state, generated)
    method, path, headers, _ = Sunat.seen[1]
    assert (method, path) == ("GET", "/api/contribuyente/gem/comprobantes/envios/a.b.c")
    assert "Content-Type" not in headers
    if state == "processed":
        assert (out["cdr"]["reference_id"], out["cdr"]["class"]) == ("T001-1", "observed")
    if state == "processed_with_errors":
        assert out["error"] == {"code": "1345", "description": "El RUC no valido", "class": "exception"}
        assert "cdr" not in out


def test_ticket_errors_and_silence_keep_the_ticket(sunat):
    Sunat.answers = {**TOKEN, "/api": answer("manual-error-422-ticket", 422)}
    missing = sunat_sdk.gre_ticket("a.b.c", *CREDENTIALS, **sunat)
    assert (missing["outcome"], missing["errors"][0]["code"], missing["ticket"]) == ("api_error", "166", "a.b.c")

    Sunat.answers = {**TOKEN, "/api": answer("manual-error-500", 500)}
    assert sunat_sdk.gre_ticket("a.b.c", *CREDENTIALS, **sunat)["general_error"] == "internal_server_error"

    Sunat.answers = {"/token": answer("derived-error-401", 401)}
    refused = sunat_sdk.gre_ticket("a.b.c", *CREDENTIALS, **sunat)
    assert (refused["outcome"], refused["stage"], refused["ticket"]) == ("api_error", "token", "a.b.c")

    Sunat.answers = {**TOKEN, "/api": answer("fork-ticket-98", 200, 3)}
    late = sunat_sdk.gre_ticket("a.b.c", *CREDENTIALS, timeout=1.5, **sunat)
    assert (late["outcome"], late["stage"], late["reason"], late["ticket"]) == ("no_verdict", "ticket", "timeout", "a.b.c")


def test_a_bad_input_makes_no_call(sunat):
    with pytest.raises(sunat_sdk.SunatSdkError, match="RUC-09"):
        sunat_sdk.gre_send({**GUIDE, "name": "20000000001-31-V001-1"}, *CREDENTIALS, **sunat)
    with pytest.raises(sunat_sdk.SunatSdkError, match="ticket"):
        sunat_sdk.gre_ticket("a/b", *CREDENTIALS, **sunat)
    with pytest.raises(sunat_sdk.SunatSdkError, match="RUC"):
        sunat_sdk.gre_send(GUIDE, CREDENTIALS[0], CREDENTIALS[1], "MODUSER", CREDENTIALS[3], **sunat)
    assert Sunat.seen == []
