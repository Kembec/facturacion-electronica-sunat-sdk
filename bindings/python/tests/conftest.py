"""Shared fixtures. The suite tests the installed wheel: scripts/test_wheel.sh installs it in a fresh
venv and runs pytest from outside the repository."""

from __future__ import annotations

from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[3]
EXPECTED_CASES = 22


def read(relative: str) -> str:
    return (ROOT / relative).read_text(encoding="utf-8")


def cases() -> list[str]:
    names = sorted(p.stem for p in (ROOT / "fixtures").glob("*.json"))
    return [n for n in names if n[:2] in {"01", "03", "07", "08", "ra", "rc"}]


@pytest.fixture(scope="session")
def cert() -> str:
    return read("tests/vectors/test-cert/test-cert.pem")


@pytest.fixture(scope="session")
def key() -> str:
    return read("tests/vectors/test-cert/test-key.pem")
