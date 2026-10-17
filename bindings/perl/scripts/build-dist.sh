#!/usr/bin/env bash
# The CPAN distribution Kembec-SunatSdk-<version>.tar.gz, with the native library of every platform it serves inside,
# into bindings/perl/dist. Makefile.PL installs only the one of the machine that installs it.
#
#   build-dist.sh <natives>
#
# <natives> holds <rid>/native/<library>, as the `native` job of .github/workflows/perl.yml leaves them; a platform
# whose library is missing is an error. The repository's LICENSE and one fixture (for t/10-build.t) go in as copies.
set -euo pipefail

natives=$(cd "${1:?usage: build-dist.sh <natives>}" && pwd)
here=$(cd "$(dirname "$0")/.." && pwd)
repo=$(cd "$here/../.." && pwd)

# rid of the native build and its file name: the builds Kembec::SunatSdk::Platform lists as supported.
platforms=(
  "linux-x64 libsunat_sdk_ffi.so"
  "linux-arm64 libsunat_sdk_ffi.so"
  "osx-x64 libsunat_sdk_ffi.dylib"
  "osx-arm64 libsunat_sdk_ffi.dylib"
  "win-x64 sunat_sdk_ffi.dll"
)

stage="$here/stage"
rm -rf "$stage" "$here/dist"
mkdir -p "$stage/t/data" "$here/dist"
cp -R "$here/lib" "$here/Makefile.PL" "$here/MANIFEST.SKIP" "$here/Changes" "$here/README.md" "$stage/"
cp "$here"/t/*.t "$stage/t/"
rm -rf "$stage/lib/Kembec/SunatSdk/native"
for row in "${platforms[@]}"; do
  read -r rid file <<<"$row"
  library="$natives/$rid/native/$file"
  [ -f "$library" ] || { echo "missing $library" >&2; exit 1; }
  mkdir -p "$stage/lib/Kembec/SunatSdk/native/$rid"
  cp "$library" "$stage/lib/Kembec/SunatSdk/native/$rid/$file"
done
cp "$repo/fixtures/01-invoice-taxed-cash.json" "$stage/t/data/"
cp "$repo/LICENSE" "$stage/LICENSE"

# The packing machine needs no native library of its own: any one of them lets Makefile.PL write the Makefile.
(
  cd "$stage"
  KEMBEC_SUNAT_SDK_LIBRARY="$stage/lib/Kembec/SunatSdk/native/linux-x64/libsunat_sdk_ffi.so" perl Makefile.PL
  make manifest
  make dist
)
mv "$stage"/Kembec-SunatSdk-*.tar.gz "$here/dist/"
ls -l "$here/dist"
