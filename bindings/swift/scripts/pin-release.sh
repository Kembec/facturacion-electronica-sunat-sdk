#!/usr/bin/env bash
# Before tagging v<version>: pins in Package.swift the URL and checksum of the xcframework zip built by run <run-id> of
# swift.yml (the run of the release/<version> branch), and records that run in bindings/swift/release-source.json.
# The `swift-xcframework` job of release.yml uploads that same zip to the tag's release, without rebuilding it
# (docs/releasing.md). Commit both files before tagging: SwiftPM reads the pin from the tagged commit.
#
#   bindings/swift/scripts/pin-release.sh 1.0.0-rc.3 18712345678
set -euo pipefail
repo=$(cd "$(dirname "$0")/../../.." && pwd)
# shellcheck source=bindings/swift/scripts/release-zip.sh
. "$repo/bindings/swift/scripts/release-zip.sh"

[ $# -eq 2 ] || { echo "usage: $0 <version> <swift.yml run id>" >&2; exit 2; }
version=$1
run_id=$2
[[ $version =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$ ]] || { echo "not a version: $version" >&2; exit 2; }
[[ $run_id =~ ^[0-9]+$ ]] || { echo "not a run id: $run_id" >&2; exit 2; }

manifest="$repo/Package.swift"
[ "$(grep -cE "$RELEASE_LINE_PATTERN" "$manifest")" = 1 ] || { echo "Package.swift: no single 'let release' line" >&2; exit 1; }

work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
checksum=$(fetch_release_zip "$run_id" "$work")
url=$(release_url "$version")

line="let release: (url: String, checksum: String)? = (url: \"$url\", checksum: \"$checksum\")"
awk -v line="$line" '/^let release: \(url: String, checksum: String\)\? = /{print line; next} {print}' "$manifest" >"$work/Package.swift"
cat "$work/Package.swift" >"$manifest"

jq -n --arg version "$version" --argjson run_id "$run_id" --arg sha256 "$checksum" \
  '{version: $version, run_id: $run_id, sha256: $sha256}' >"$repo/bindings/swift/release-source.json"

echo "Package.swift: $url ($checksum)"
echo "bindings/swift/release-source.json: run $run_id"
