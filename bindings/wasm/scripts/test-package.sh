#!/usr/bin/env bash
# Installs the packed tarball in a clean project, as a user would, and checks it there: the
# declarations type-check through `exports`, and the 12 voucher vectors (01, 03, 07, 08) are built
# and signed like their signed XML in Node (`node` condition) and in headless Chrome (`default`).
# Usage: bindings/wasm/scripts/test-package.sh <tarball>
set -euo pipefail

[ $# -eq 1 ] || { echo "usage: $0 <tarball>" >&2; exit 2; }
tarball="$(cd "$(dirname "$1")" && pwd)/$(basename "$1")"
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
tests="$root/bindings/wasm/test"
consumer="$(mktemp -d)"
# A second try covers a Chrome helper that was still dying when the first removal ran.
trap 'rm -r "$consumer" 2>/dev/null || { sleep 1; rm -r "$consumer"; } || true' EXIT

cp "$tests"/{vectors,node-consumer,browser,browser-page}.mjs "$tests/types-consumer.mts" "$consumer/"
cd "$consumer"
echo '{"name":"consumer","private":true,"type":"module"}' > package.json
npm install --no-audit --no-fund --loglevel=error "$tarball" typescript@5.6.3

echo "== types ($(npx tsc --version))"
npx tsc --noEmit --strict --target es2022 --module nodenext --moduleResolution nodenext --lib es2022,dom types-consumer.mts
echo "ok types"
echo "== node $(node --version)"
node node-consumer.mjs "$root"
echo "== browser"
node browser.mjs "$root"
