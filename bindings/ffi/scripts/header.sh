#!/usr/bin/env bash
# Writes include/sunat_sdk.h from src/lib.rs with cbindgen; with --check, fails if the committed
# header differs from what the source generates. CBINDGEN overrides the binary (pinned in CI).
set -euo pipefail
here="$(cd "$(dirname "$0")/.." && pwd)"
tmp="$(mktemp -d)"
trap 'rm -r -- "$tmp"' EXIT
# cbindgen reads only `extern "C"` items. The exports are `extern "system"`, which the header
# spells with SUNAT_SDK_CALL (cbindgen.toml); the copy only renames the ABI so cbindgen sees them.
sed 's/extern "system"/extern "C"/' "$here/src/lib.rs" > "$tmp/lib.rs"
"${CBINDGEN:-cbindgen}" --quiet --config "$here/cbindgen.toml" --output "$tmp/sunat_sdk.h" "$tmp/lib.rs"
# MSVC takes the calling convention after the return type, GCC and Clang accept it there too:
# cbindgen can only prefix the declaration, so the macro is moved next to the name.
sed -E -i.bak 's/^SUNAT_SDK_CALL (.*[ *])(sunat_sdk_[a-z_]+\()/\1SUNAT_SDK_CALL \2/' "$tmp/sunat_sdk.h"
if [ "${1:-}" = "--check" ]; then
  diff -u "$here/include/sunat_sdk.h" "$tmp/sunat_sdk.h" || { echo "include/sunat_sdk.h is stale: run bindings/ffi/scripts/header.sh" >&2; exit 1; }
else
  cp "$tmp/sunat_sdk.h" "$here/include/sunat_sdk.h"
fi
