"""`exchange` against a local HTTP server: the POST it sends and the outcome it reads, no network."""

from __future__ import annotations

import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

import pytest

import sunat_sdk
from conftest import ROOT
from test_api import DOC, send_bill_answer

FAULT = (
    b'<soap-env:Envelope xmlns:soap-env="http://schemas.xmlsoap.org/soap/envelope/"><soap-env:Body>'
    b"<soap-env:Fault><faultcode>soap-env:Client.1033</faultcode>"
    b"<faultstring>El comprobante fue registrado previamente con otros datos</faultstring>"
    b"</soap-env:Fault></soap-env:Body></soap-env:Envelope>"
)
STATUS = {"ruc": "20100066603", "document_type": "01", "series": "F001", "number": 1}


class Sunat(BaseHTTPRequestHandler):
    answers: dict = {}
    seen: list = []

    def do_POST(self):  # noqa: N802 - http.server API
        body = self.rfile.read(int(self.headers["Content-Length"]))
        Sunat.seen.append((self.path, self.headers["SOAPAction"], body))
        status, payload, delay = Sunat.answers[self.path]
        time.sleep(delay)
        try:
            self.send_response(status)
            self.send_header("Content-Type", "text/xml")
            self.send_header("Content-Length", str(len(payload)))
            self.end_headers()
            self.wfile.write(payload)
        except OSError:  # the client gave up first: that is the case under test
            pass

    def log_message(self, *args):  # the request carries SOL credentials: nothing is printed
        pass


@pytest.fixture()
def sunat():
    server = ThreadingHTTPServer(("127.0.0.1", 0), Sunat)
    server.daemon_threads = True
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    Sunat.seen = []
    yield f"http://127.0.0.1:{server.server_address[1]}"
    server.shutdown()
    server.server_close()


def test_send_posts_the_envelope_and_reads_the_cdr(sunat):
    Sunat.answers = {"/bill": (200, send_bill_answer(), 0)}
    sent = sunat_sdk.exchange("send", DOC, "20100066603MODDATOS", "moddatos", f"{sunat}/bill")
    assert (sent["outcome"], sent["cdr"]["response_code"]) == ("cdr", "2047")
    path, action, body = Sunat.seen[0]
    assert (path, action) == ("/bill", '"urn:sendBill"')
    assert b"<fileName>20100066603-01-F001-1.zip</fileName>" in body


def test_a_soap_fault_is_a_result(sunat):
    Sunat.answers = {"/bill": (500, FAULT, 0)}
    sent = sunat_sdk.exchange("send", DOC, "20100066603MODDATOS", "moddatos", f"{sunat}/bill")
    assert (sent["outcome"], sent["code"], sent["class"]) == ("fault", "1033", "already_registered")


def test_an_http_status_without_verdict_for_a_query(sunat):
    Sunat.answers = {"/consult": (401, b"", 0)}
    outcome = sunat_sdk.exchange("status", STATUS, "u", "p", f"{sunat}/consult")
    assert (outcome["outcome"], outcome["reason"], outcome["http_status"]) == ("no_verdict", "http_status", 401)


def test_no_answer_is_no_verdict_for_a_query_and_an_error_for_a_send(sunat):
    Sunat.answers = {"/slow": (200, b"", 2)}
    outcome = sunat_sdk.exchange("status", STATUS, "u", "p", f"{sunat}/slow", timeout=0.5)
    assert (outcome["outcome"], outcome["reason"]) == ("no_verdict", "timeout")
    with pytest.raises(sunat_sdk.SunatSdkError, match="outcome is unknown"):
        sunat_sdk.exchange("send", DOC, "u", "p", f"{sunat}/slow", timeout=0.5)


def test_diagnose_points_the_beta_2800_at_the_customer_document_type():
    message = (ROOT / "crates/sunat-sdk/tests/data/error-fields/faultstring-2800.txt").read_text(encoding="utf-8").strip()
    found = sunat_sdk.diagnose("03", "2800", message)
    assert found["detail"] == {"node": "cbc:ID/schemeID", "value": "1"}
    assert found["fields"][0]["input_path"] == "customer.document_type"
    body = (
        '<e:Envelope xmlns:e="s"><e:Body><e:Fault><faultcode>e:Client.2800</faultcode>'
        f"<faultstring>{message.replace('&', '&amp;').replace('<', '&lt;')}</faultstring></e:Fault></e:Body></e:Envelope>"
    ).encode()
    out = sunat_sdk.soap_response("send", {"name": "20100066603-03-B001-1", "xml": "<a/>"}, 500, body)
    assert out["diagnosis"] == [found]
    ok = sunat_sdk.soap_timeout("ticket", {"ticket": "1"}, 5)
    assert "diagnosis" not in ok
