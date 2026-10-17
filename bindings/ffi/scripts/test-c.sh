#!/usr/bin/env bash
# Compiles tests/c/vectors.c against include/sunat_sdk.h and runs every fixture through the shared
# library; given the native libraries of the static one (`--print native-static-libs`), through
# the static library too. Unix only: CI runs the Windows variant with MSVC (.github/workflows/ffi.yml).
#
#   test-c.sh <directory with the built libraries> ["<native static libs>"]
set -euo pipefail
ffi="$(cd "$(dirname "$0")/.." && pwd)"
repo="$(cd "$ffi/../.." && pwd)"
libdir="$(cd "$1" && pwd)"
out="$(mktemp -d)"
trap 'rm -r -- "$out"' EXIT
cases=()
for fixture in "$repo"/fixtures/*.json; do
  name="${fixture##*/}"
  cases+=("${name%.json}")
done
flags=(-std=c99 -Wall -Wextra -Werror -I"$ffi/include")
"${CC:-cc}" "${flags[@]}" "$ffi/tests/c/vectors.c" -L"$libdir" -lsunat_sdk_ffi -Wl,-rpath,"$libdir" -o "$out/vectors-shared"
"$out/vectors-shared" "$repo" "${cases[@]}"
if [ $# -ge 2 ]; then
  # shellcheck disable=SC2086 # the native libraries are one flag each
  "${CC:-cc}" "${flags[@]}" "$ffi/tests/c/vectors.c" "$libdir/libsunat_sdk_ffi.a" $2 -o "$out/vectors-static"
  "$out/vectors-static" "$repo" "${cases[@]}"
fi
