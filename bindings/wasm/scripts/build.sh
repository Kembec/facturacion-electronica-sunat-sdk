#!/usr/bin/env bash
# Builds the npm package contents into bindings/wasm/npm: one wasm-bindgen `--target web` build,
# loaded by node.js (fs + initSync) and web.js (URL + init). Needs the wasm32-unknown-unknown target
# and wasm-bindgen-cli at the exact version pinned in bindings/wasm/Cargo.toml.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
package="$root/bindings/wasm/npm"

cargo build --manifest-path "$root/Cargo.toml" -p sunat-sdk-wasm --target wasm32-unknown-unknown --release --locked
rm -rf "$package/wasm"
wasm-bindgen --target web --out-dir "$package/wasm" \
  "$root/target/wasm32-unknown-unknown/release/sunat_sdk_wasm.wasm"
# npm packs LICENSE by itself only when it sits in the package root; NOTICE goes through `files`.
cp "$root/LICENSE" "$root/NOTICE" "$package/"
