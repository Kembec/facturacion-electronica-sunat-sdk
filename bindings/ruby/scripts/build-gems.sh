#!/usr/bin/env bash
# One .gem per platform, each with the native library of that platform inside, into bindings/ruby/pkg.
#
#   build-gems.sh <natives>
#
# <natives> holds <rid>/native/<library>, as the `native` job of .github/workflows/ruby.yml leaves them; a platform
# whose library is missing is an error. The repository's LICENSE goes into every gem.
set -euo pipefail

natives=$(cd "${1:?usage: build-gems.sh <natives>}" && pwd)
here=$(cd "$(dirname "$0")/.." && pwd)
repo=$(cd "$here/../.." && pwd)

# RubyGems platform  <-  rid of the native build, and its file name.
platforms=(
  "x86_64-linux-gnu linux-x64 libsunat_sdk_ffi.so"
  "aarch64-linux-gnu linux-arm64 libsunat_sdk_ffi.so"
  "x86_64-darwin osx-x64 libsunat_sdk_ffi.dylib"
  "arm64-darwin osx-arm64 libsunat_sdk_ffi.dylib"
  "x64-mingw-ucrt win-x64 sunat_sdk_ffi.dll"
  "x86-mingw32 win-x86 sunat_sdk_ffi.dll"
)

mkdir -p "$here/pkg"
for row in "${platforms[@]}"; do
  read -r platform rid file <<<"$row"
  library="$natives/$rid/native/$file"
  [ -f "$library" ] || { echo "missing $library for $platform" >&2; exit 1; }
  stage="$here/stage/$platform"
  rm -rf "$stage"
  mkdir -p "$stage/lib/kembec/sunat_sdk/native"
  cp -R "$here/lib/." "$stage/lib/"
  rm -f "$stage/lib/kembec/sunat_sdk/native/"*
  cp "$library" "$stage/lib/kembec/sunat_sdk/native/$file"
  cp "$here/kembec-sunat_sdk.gemspec" "$here/README.md" "$stage/"
  cp "$repo/LICENSE" "$stage/LICENSE"
  (cd "$stage" && SUNAT_SDK_GEM_PLATFORM="$platform" gem build kembec-sunat_sdk.gemspec --quiet)
  mv "$stage"/kembec-sunat_sdk-*.gem "$here/pkg/"
done
ls -l "$here/pkg"
