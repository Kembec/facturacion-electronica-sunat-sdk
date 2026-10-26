#!/usr/bin/env bash
# Publishes the crates given to crates.io one at a time, each after every crate of the set it requires, skipping a
# version crates.io already has, and waits until the sparse index serves each one before the next is published.
# A batch `cargo publish -p … -p …` stops at the first crate that already exists, and its 60 s wait for the index can
# expire with a warning, so the next crate fails to resolve its dependency: on 1.0.0-rc.8 crates.io took model,
# transport and xmldsig at 07:35:50Z and its index still did not serve them 17 minutes later, so ubl and sdk failed.
# "Already published" is asked to the API, not to the index, which can lag behind it; a re-run finishes the rest.
# Usage: crates/scripts/publish.sh [--dry-run] <crate>...   (PUBLISH_INDEX_TIMEOUT seconds, default 1800)
set -euo pipefail

dry_run=""
if [ "${1:-}" = "--dry-run" ]; then dry_run="--dry-run"; shift; fi
[ $# -ge 1 ] || { echo "usage: $0 [--dry-run] <crate>..." >&2; exit 2; }
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
timeout="${PUBLISH_INDEX_TIMEOUT:-1800}"
agent="kembec-sunat-sdk release (https://github.com/Kembec/facturacion-electronica-sunat-sdk)"
meta="$(cargo metadata --no-deps --locked --format-version 1 --manifest-path "$root/Cargo.toml")"
names="$(printf '%s\n' "$@" | jq -R . | jq -sc .)"

# The sparse index path of a crate name: 1/, 2/, 3/<c>/ or <ab>/<cd>/ (crates.io index layout).
index_url() {
  local name; name="$(tr '[:upper:]' '[:lower:]' <<<"$1")"
  case ${#name} in
    1) echo "https://index.crates.io/1/$name" ;;
    2) echo "https://index.crates.io/2/$name" ;;
    3) echo "https://index.crates.io/3/${name:0:1}/$name" ;;
    *) echo "https://index.crates.io/${name:0:2}/${name:2:2}/$name" ;;
  esac
}

# The API answers 200 for a version crates.io accepted, whether or not the index serves it yet.
published() {
  curl -fsS -o /dev/null -A "$agent" "https://crates.io/api/v1/crates/$1/$2" 2>/dev/null
}

# What cargo resolves a dependency from: the next crate can be published only once this serves the version.
indexed() {
  curl -fsS -A "$agent" -H 'Cache-Control: no-cache' "$(index_url "$1")" 2>/dev/null \
    | jq -e --arg v "$2" 'select(.vers == $v)' >/dev/null
}

# Dependency order from the manifests, not from the caller: a crate goes once every crate of the set it requires went.
order=()
pending=("$@")
while [ ${#pending[@]} -gt 0 ]; do
  next=()
  for crate in "${pending[@]}"; do
    deps="$(jq -r --arg c "$crate" --argjson names "$names" '
      .packages[] | select(.name == $c) | .dependencies[] | .name | select(. as $n | $names | index($n))' <<<"$meta")"
    ready=1
    for dep in $deps; do
      [[ " ${order[*]-} " == *" $dep "* ]] || ready=0
    done
    if [ $ready -eq 1 ]; then order+=("$crate"); else next+=("$crate"); fi
  done
  if [ ${#next[@]} -eq ${#pending[@]} ]; then
    echo "no dependency order for: ${next[*]}" >&2
    exit 1
  fi
  pending=("${next[@]+"${next[@]}"}")
done

for crate in "${order[@]}"; do
  version="$(jq -r --arg c "$crate" '.packages[] | select(.name == $c) | .version' <<<"$meta")"
  if published "$crate" "$version"; then
    echo "skip: $crate $version is already on crates.io"
  else
    echo "publish: $crate $version"
    cargo publish --locked $dry_run -p "$crate" --manifest-path "$root/Cargo.toml"
  fi
  [ -z "$dry_run" ] || continue
  waited=0
  until indexed "$crate" "$version"; do
    if [ $waited -ge "$timeout" ]; then
      echo "$crate $version is on crates.io but not in its index after ${timeout}s; re-run to publish the rest" >&2
      exit 1
    fi
    sleep 10
    waited=$((waited + 10))
  done
  echo "indexed: $crate $version after ${waited}s"
done
