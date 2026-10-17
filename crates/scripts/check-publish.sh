#!/usr/bin/env bash
# Checks the workspace before any crate is packaged: the crates given are exactly the members crates.io would take
# (every other member says `publish = false`), each carries the metadata its crates.io and docs.rs pages need, and
# each requires the others at exactly the workspace version, so a release cannot mix versions.
# Usage: crates/scripts/check-publish.sh <crate>...
set -euo pipefail

[ $# -ge 1 ] || { echo "usage: $0 <crate>..." >&2; exit 2; }
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
meta="$(cargo metadata --no-deps --locked --format-version 1 --manifest-path "$root/Cargo.toml")"
names="$(printf '%s\n' "$@" | jq -R . | jq -sc 'sort')"

publishable="$(jq -c '[.packages[] | select(.publish == null) | .name] | sort' <<<"$meta")"
if [ "$publishable" != "$names" ]; then
  echo "crates.io would take $publishable; expected $names (a member that is not published says publish = false)" >&2
  exit 1
fi

problems="$(jq -r --argjson names "$names" '
  [.packages[] | select(.name as $n | $names | index($n))] as $published
  | ($published[0].version) as $version
  | $published[]
  | . as $package
  | .name as $crate
  | ( (["description", "license", "repository", "homepage", "readme", "rust_version"][]
        | select($package[.] == null)
        | "\($crate): no \(.)"),
      (select((.keywords | length) == 0) | "\($crate): no keywords"),
      (select((.categories | length) == 0) | "\($crate): no categories"),
      (select(.version != $version) | "\($crate): version \(.version), expected \($version)"),
      (.dependencies[]
        | select(.name as $n | $names | index($n))
        | select(.req != "=" + $version)
        | "\($crate) requires \(.name) \(.req), expected =\($version)") )
' <<<"$meta")"
if [ -n "$problems" ]; then
  echo "$problems" >&2
  exit 1
fi
echo "ok: $# crates, $(jq -r '.packages[] | select(.publish == null) | .version' <<<"$meta" | sort -u)"
