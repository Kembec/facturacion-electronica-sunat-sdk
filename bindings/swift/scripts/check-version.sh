#!/usr/bin/env bash
# One version everywhere a consumer reads it: the Rust workspace (and so the native library), the Swift package, the
# Objective-C package and its podspec. The native library is checked against the packages at load time; this catches
# the packages before anything is built.
set -euo pipefail
repo=$(cd "$(dirname "$0")/../../.." && pwd)

cargo=$(sed -n '/^\[workspace.package\]/,/^\[/s/^version = "\(.*\)"/\1/p' "$repo/Cargo.toml")
swift=$(sed -n 's/.*static let packageVersion = "\(.*\)"/\1/p' "$repo/bindings/swift/Sources/KembecSunatSDK/SunatSDKClient.swift")
objc=$(sed -n 's/^NSString \*const KSSPackageVersion = @"\(.*\)";/\1/p' "$repo/bindings/objc/Sources/KembecSunatSDKObjC/KSSClient.m")
podspec=$(sed -n 's/^ *s\.version *= *"\(.*\)"/\1/p' "$repo/KembecSunatSDKObjC.podspec")

status=0
for pair in "Swift:$swift" "Objective-C:$objc" "podspec:$podspec"; do
  if [ -z "$cargo" ] || [ "${pair#*:}" != "$cargo" ]; then
    echo "${pair%%:*} version '${pair#*:}' differs from Cargo's '$cargo'" >&2
    status=1
  fi
done
[ $status -ne 0 ] || echo "ok $cargo: Cargo, Swift, Objective-C, podspec"
exit $status
