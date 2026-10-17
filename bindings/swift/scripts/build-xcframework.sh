#!/usr/bin/env bash
# Packs the static libraries of the C ABI into bindings/swift/artifacts/SunatSDKFFI.xcframework, the binary target of
# the Swift and Objective-C products (Package.swift, KembecSunatSDKObjC.podspec), with its zip and the zip's checksum.
# Three slices: macOS (arm64 + x86_64), iOS (arm64) and the iOS simulator (arm64 + x86_64). Static libraries: nothing to
# sign, no rpath, nothing to embed in an iOS app. macOS with Xcode only.
#
#   build-xcframework.sh <cargo target dir>
#
# The target dir holds <triple>/release/libsunat_sdk_ffi.a for aarch64-apple-darwin, x86_64-apple-darwin,
# aarch64-apple-ios, aarch64-apple-ios-sim and x86_64-apple-ios (.github/workflows/swift.yml builds them).
set -euo pipefail

targets=$(cd "${1:?usage: build-xcframework.sh <cargo target dir>}" && pwd)
swift=$(cd "$(dirname "$0")/.." && pwd)
repo=$(cd "$swift/../.." && pwd)
out="$swift/artifacts"
xcframework="$out/SunatSDKFFI.xcframework"
lib=libsunat_sdk_ffi.a
[ ! -e "$xcframework" ] || { echo "$xcframework exists: move it away first" >&2; exit 1; }

work=$(mktemp -d)
trap 'rm -r -- "$work"' EXIT

# The headers in a subdirectory named as the module: two xcframeworks with a module.modulemap at the root of Headers
# collide in the same include directory of an Xcode build.
mkdir -p "$work/Headers/SunatSDKFFI"
cp "$repo/bindings/ffi/include/sunat_sdk.h" "$work/Headers/SunatSDKFFI/"
cat > "$work/Headers/SunatSDKFFI/module.modulemap" <<'MAP'
module SunatSDKFFI {
    header "sunat_sdk.h"
    export *
}
MAP

slice() { # slice <name> <triple>...: one library, fat when there are two architectures
  local name=$1
  shift
  mkdir -p "$work/$name"
  local inputs=()
  for triple in "$@"; do inputs+=("$targets/$triple/release/$lib"); done
  if [ ${#inputs[@]} -eq 1 ]; then cp "${inputs[0]}" "$work/$name/$lib"; else lipo -create "${inputs[@]}" -output "$work/$name/$lib"; fi
  lipo -info "$work/$name/$lib"
}
slice macos aarch64-apple-darwin x86_64-apple-darwin
slice ios aarch64-apple-ios
slice ios-simulator aarch64-apple-ios-sim x86_64-apple-ios

mkdir -p "$out"
xcodebuild -create-xcframework \
  -library "$work/macos/$lib" -headers "$work/Headers" \
  -library "$work/ios/$lib" -headers "$work/Headers" \
  -library "$work/ios-simulator/$lib" -headers "$work/Headers" \
  -output "$xcframework"

# The zip a release attaches and Package.swift points to; its checksum is the SHA-256 SwiftPM and the podspec check.
rm -f -- "$xcframework.zip"
(cd "$out" && ditto -c -k --keepParent SunatSDKFFI.xcframework SunatSDKFFI.xcframework.zip)
checksum=$(swift package compute-checksum "$xcframework.zip")
echo "$checksum" > "$xcframework.zip.sha256"
du -sh "$xcframework" "$xcframework.zip"
echo "checksum $checksum"
