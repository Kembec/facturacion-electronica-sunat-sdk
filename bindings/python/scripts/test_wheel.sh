#!/usr/bin/env bash
# Installs a built wheel (or the sdist) in a fresh venv outside the repository and runs the pytest
# suite against it, so the module under test is the installed one and never bindings/python/python.
# Usage: bindings/python/scripts/test_wheel.sh <wheel-or-sdist> [python]
set -euo pipefail

[ $# -ge 1 ] || { echo "usage: $0 <wheel-or-sdist> [python]" >&2; exit 2; }
artifact="$(cd "$(dirname "$1")" && pwd)/$(basename "$1")"
python="${2:-python3}"
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
work="$(mktemp -d)"
trap 'rm -r "$work"' EXIT

"$python" -m venv "$work/venv"
bin="$work/venv/bin"
[ -d "$bin" ] || bin="$work/venv/Scripts"
"$bin/python" -m pip install --quiet --disable-pip-version-check "$artifact" pytest==9.1.1 lxml==6.0.2
cd "$work"
# Default (prepend) import mode puts only tests/ on sys.path, so `sunat_sdk` comes from the venv;
# test_installed.py asserts it.
"$bin/python" -m pytest "$root/bindings/python/tests" -p no:cacheprovider -q --rootdir "$work"
