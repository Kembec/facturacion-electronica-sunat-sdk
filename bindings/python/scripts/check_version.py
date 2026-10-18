"""Fails when the Python package version differs from the Cargo workspace version.

maturin takes the version from Cargo.toml and writes it as PEP 440 (1.0.0-rc.2 -> 1.0.0rc2). This
checks that pyproject.toml declares no version of its own, that every built artifact in `--dist`
carries that PEP 440 version, and, given a tag, that the tag is `v` + the workspace version.
Usage: python bindings/python/scripts/check_version.py [--dist <dir>] [tag]
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
STAGES = {"alpha": "a", "beta": "b", "rc": "rc"}


def workspace_version() -> str:
    text = (ROOT / "Cargo.toml").read_text(encoding="utf-8")
    section = text.split("[workspace.package]", 1)[1].split("\n[", 1)[0]
    match = re.search(r'^version\s*=\s*"([^"]+)"', section, re.M)
    if not match:
        sys.exit("version: Cargo.toml has no [workspace.package] version")
    return match.group(1)


def pep440(cargo_version: str) -> str:
    match = re.fullmatch(r"(\d+\.\d+\.\d+)(?:-(alpha|beta|rc)\.(\d+))?", cargo_version)
    if not match:
        sys.exit(f"version: {cargo_version} is not X.Y.Z or X.Y.Z-(alpha|beta|rc).N")
    release, stage, number = match.groups()
    return release + (STAGES[stage] + number if stage else "")


def artifact_version(name: str) -> str | None:
    if name.endswith(".whl"):
        return name.split("-")[1]
    if name.endswith(".tar.gz"):
        return name[: -len(".tar.gz")].rsplit("-", 1)[1]
    return None


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--dist", type=Path)
    parser.add_argument("tag", nargs="?")
    args = parser.parse_args()

    cargo = workspace_version()
    expected = pep440(cargo)
    errors = []

    pyproject = (ROOT / "bindings/python/pyproject.toml").read_text(encoding="utf-8")
    if re.search(r"^version\s*=", pyproject, re.M) or 'dynamic = ["version"]' not in pyproject:
        errors.append("bindings/python/pyproject.toml must take the version from Cargo (dynamic = [\"version\"])")

    if args.dist is not None:
        artifacts = sorted(p.name for p in args.dist.iterdir() if artifact_version(p.name) is not None)
        if not artifacts:
            errors.append(f"no wheel or sdist in {args.dist}")
        for name in artifacts:
            if artifact_version(name) != expected:
                errors.append(f"{name} is not version {expected}")

    if args.tag is not None and args.tag != f"v{cargo}":
        errors.append(f"tag {args.tag} does not match the workspace version {cargo}")

    for error in errors:
        print(f"version: {error}", file=sys.stderr)
    if errors:
        return 1
    print(f"version {expected} (Cargo {cargo}){f' matches {args.tag}' if args.tag else ''}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
