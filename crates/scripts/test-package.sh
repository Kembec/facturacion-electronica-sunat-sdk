#!/usr/bin/env bash
# Runs the battery on the crates as crates.io gets them: each `.crate` file is unpacked and checked to carry only its
# library, its tests' code, README and notices; then the unpacked crates, wired to each other through
# `[patch.crates-io]`, run their own tests against the repository's data (fixtures/, tests/vectors/, tests/xsd/ and
# each crate's tests/data/, which stay out of the packages). What compiles is the packaged code and manifest only.
# Set SUNAT_XMLDSIG_PYTHON to a Python with lxml and cryptography, or the schema and independent-verifier tests skip.
# Usage: crates/scripts/test-package.sh <directory with the .crate files> <crate>...
set -euo pipefail

[ $# -ge 2 ] || { echo "usage: $0 <directory with the .crate files> <crate>..." >&2; exit 2; }
packages="$(cd "$1" && pwd)"
shift
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
version="$(cargo metadata --no-deps --locked --format-version 1 --manifest-path "$root/Cargo.toml" \
  | jq -r --arg name "$1" '.packages[] | select(.name == $name) | .version')"

allowed='^(\.cargo_vcs_info\.json|Cargo\.toml|Cargo\.toml\.orig|Cargo\.lock|LICENSE|NOTICE|README\.md|src/.+\.rs|tests/[^/]+\.(rs|py)|tests/.+\.rs)$'
members=""
patches=""
mkdir "$work/crates"
for crate in "$@"; do
  file="$packages/$crate-$version.crate"
  echo "== $crate-$version.crate ($(wc -c < "$file") bytes)"
  if tar -tzf "$file" | sed "s|^$crate-$version/||" | grep -vE "$allowed"; then
    echo "files outside the library, its tests' code and its notices in $crate" >&2
    exit 1
  fi
  tar -xzf "$file" -C "$work/crates"
  mv "$work/crates/$crate-$version" "$work/crates/$crate"
  source_dir="$root/crates/${crate#kembec-}"  # the folders keep the sunat-* names
  if [ -d "$source_dir/tests/data" ]; then
    mkdir -p "$work/crates/$crate/tests"
    cp -R "$source_dir/tests/data" "$work/crates/$crate/tests/data"
  fi
  members="$members\"crates/$crate\", "
  patches="$patches$crate = { path = \"crates/$crate\" }"$'\n'
done

# The repository's data, at the paths the tests expect relative to their crate.
cp -R "$root/fixtures" "$work/fixtures"
mkdir "$work/tests"
cp -R "$root/tests/vectors" "$root/tests/xsd" "$work/tests/"
# The dependency versions CI tested with; cargo drops the entries of the members left out here.
cp "$root/Cargo.lock" "$work/Cargo.lock"
cat > "$work/Cargo.toml" <<EOF
[workspace]
resolver = "2"
members = [$members]

[patch.crates-io]
$patches
EOF

cargo test --manifest-path "$work/Cargo.toml" --workspace
