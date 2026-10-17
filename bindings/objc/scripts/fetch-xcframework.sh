#!/usr/bin/env bash
# The podspec's prepare_command: puts SunatSDKFFI.xcframework where the podspec and Package.swift expect it,
# bindings/swift/artifacts. It downloads the zip that Package.swift's `release` names and checks it against the same
# checksum SwiftPM checks (SHA-256), so CocoaPods and SwiftPM link the same bytes. Until a release fills `release`, it
# takes the xcframework already unzipped there from the CI artifact (bindings/objc/README.md).
set -euo pipefail
repo=$(cd "$(dirname "$0")/../../.." && pwd)
out="$repo/bindings/swift/artifacts"
xcframework="$out/SunatSDKFFI.xcframework"

line=$(grep -E '^let release: \(url: String, checksum: String\)\? = ' "$repo/Package.swift")
url=$(sed -n 's/.*url: "\([^"]*\)".*/\1/p' <<<"$line")
checksum=$(sed -n 's/.*checksum: "\([0-9a-f]\{64\}\)".*/\1/p' <<<"$line")

if [ -z "$url" ]; then
  [ -d "$xcframework" ] && { echo "SunatSDKFFI.xcframework: the one in bindings/swift/artifacts (no release in Package.swift)"; exit 0; }
  echo "Package.swift names no released xcframework yet: unzip the CI artifact into bindings/swift/artifacts" >&2
  exit 1
fi
[ -n "$checksum" ] || { echo "Package.swift's release has no SHA-256 checksum" >&2; exit 1; }
case "$url" in https://*) ;; *) echo "the xcframework URL must be https: $url" >&2; exit 1 ;; esac

mkdir -p "$out"
zip="$out/SunatSDKFFI.xcframework.zip"
curl --proto '=https' --tlsv1.2 -fsSL --retry 3 -o "$zip" "$url"
got=$(shasum -a 256 "$zip" | cut -d' ' -f1)
[ "$got" = "$checksum" ] || { echo "SunatSDKFFI.xcframework.zip: SHA-256 $got, Package.swift says $checksum" >&2; exit 1; }
[ ! -e "$xcframework" ] || mv "$xcframework" "$(mktemp -d)/"
ditto -x -k "$zip" "$out"
echo "SunatSDKFFI.xcframework: $url ($checksum)"
