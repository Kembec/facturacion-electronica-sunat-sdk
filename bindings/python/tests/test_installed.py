"""The module under test is the installed wheel, with the version of the Cargo workspace."""

from __future__ import annotations

import re
from importlib.metadata import version
from pathlib import Path

import sunat_sdk
from conftest import ROOT


def pep440(cargo_version: str) -> str:
    match = re.fullmatch(r"(\d+\.\d+\.\d+)(?:-(alpha|beta|rc)\.(\d+))?", cargo_version)
    assert match, f"unsupported workspace version {cargo_version}"
    release, stage, number = match.groups()
    return release + ({"alpha": "a", "beta": "b", "rc": "rc"}[stage] + number if stage else "")


def workspace_version() -> str:
    section = (ROOT / "Cargo.toml").read_text(encoding="utf-8").split("[workspace.package]")[1]
    return re.search(r'^version\s*=\s*"([^"]+)"', section, re.M).group(1)


def test_imported_from_site_packages_not_from_the_tree():
    module = Path(sunat_sdk.__file__).resolve()
    assert ROOT not in module.parents, module
    assert any(part in {"site-packages", "dist-packages"} for part in module.parts), module


def test_version_is_the_workspace_version():
    expected = pep440(workspace_version())
    assert version("facturacion-electronica-sunat-sdk") == expected
    assert sunat_sdk.__version__ == expected
