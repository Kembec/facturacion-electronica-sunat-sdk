#!/usr/bin/env bash
# The `swift-xcframework` job of release.yml: uploads to the release of tag <tag> the xcframework zip that
# pin-release.sh pinned, downloaded again from the swift.yml run recorded in bindings/swift/release-source.json. It
# never rebuilds: the zip goes up only if its SHA-256 is the one in release-source.json and in the tagged Package.swift,
# and Package.swift's URL is this release's asset. Without release-source.json, or with another version in it, the
# tag has no pinned xcframework: it says so and uploads nothing. --dry-run checks everything and skips the upload.
#
#   bindings/swift/scripts/publish-release.sh v1.0.0-rc.3 [--dry-run]
set -euo pipefail
repo=$(cd "$(dirname "$0")/../../.." && pwd)
# shellcheck source=bindings/swift/scripts/release-zip.sh
. "$repo/bindings/swift/scripts/release-zip.sh"

[ $# -ge 1 ] && [ $# -le 2 ] || { echo "usage: $0 <tag> [--dry-run]" >&2; exit 2; }
tag=$1
dry_run=${2:-}
[ -z "$dry_run" ] || [ "$dry_run" = "--dry-run" ] || { echo "unknown option: $dry_run" >&2; exit 2; }
version=${tag#v}

source_json="$repo/bindings/swift/release-source.json"
if [ ! -f "$source_json" ]; then
  echo "no bindings/swift/release-source.json: $tag has no pinned xcframework, nothing to upload"
  exit 0
fi
pinned_version=$(jq -r '.version' "$source_json")
if [ "$pinned_version" != "$version" ]; then
  echo "bindings/swift/release-source.json pins $pinned_version, not $version: nothing to upload"
  exit 0
fi
run_id=$(jq -r '.run_id' "$source_json")
recorded=$(jq -r '.sha256' "$source_json")

line=$(grep -E "$RELEASE_LINE_PATTERN" "$repo/Package.swift")
url=$(sed -n 's/.*url: "\([^"]*\)".*/\1/p' <<<"$line")
checksum=$(sed -n 's/.*checksum: "\([0-9a-f]\{64\}\)".*/\1/p' <<<"$line")
expected_url=$(release_url "$version")
[ "$url" = "$expected_url" ] || { echo "Package.swift's release URL is '$url', not $expected_url" >&2; exit 1; }
[ "$checksum" = "$recorded" ] || {
  echo "Package.swift's checksum '$checksum' differs from release-source.json's $recorded" >&2
  exit 1
}

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
got=$(fetch_release_zip "$run_id" "$work")
[ "$got" = "$checksum" ] || { echo "run $run_id's $RELEASE_ZIP has SHA-256 $got, Package.swift says $checksum" >&2; exit 1; }

if [ "$dry_run" = "--dry-run" ]; then
  echo "dry run: $RELEASE_ZIP of run $run_id ($got) would go to the release $tag"
  exit 0
fi
gh release upload "$tag" "$work/$RELEASE_ZIP" --repo "$RELEASE_REPO"
echo "$tag: $expected_url ($got)"
