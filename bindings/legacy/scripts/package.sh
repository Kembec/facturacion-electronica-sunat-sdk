#!/usr/bin/env bash
# Stages the zip for VB6, Visual FoxPro and PowerBuilder from the two Windows DLLs the CI tested:
#
#   package.sh <x86 sunat_sdk_ffi.dll> <x64 sunat_sdk_ffi.dll> <output directory>
#
# Writes <output>/sunat-sdk-legacy-windows-<version>/ and prints its name; the workflow uploads it as
# the artifact, which GitHub serves as that zip. Fails if a DLL is of the wrong machine, or if a
# source file would not open in its IDE: VB6 reads CRLF only and all three read the ANSI code page,
# so the sources are ASCII with CRLF.
set -euo pipefail

x86_dll=$1
x64_dll=$2
output=$3
repo=$(cd "$(dirname "$0")/../../.." && pwd)
legacy="$repo/bindings/legacy"

version=$(sed -n '/^\[workspace.package\]/,/^\[/s/^version = "\(.*\)"/\1/p' "$repo/Cargo.toml")
[ -n "$version" ] || { echo "no version in Cargo.toml" >&2; exit 1; }
name="sunat-sdk-legacy-windows-$version"
stage="$output/$name"
rm -rf "$stage"
mkdir -p "$stage/dll/x86" "$stage/dll/x64" "$stage/vb6" "$stage/vfp" "$stage/powerbuilder" "$stage/sample"

# The PE machine field: 0x14c is i386, 0x8664 is x86-64.
machine() {
  python3 -c 'import struct,sys
data = open(sys.argv[1], "rb").read(4096)
pe = struct.unpack_from("<I", data, 0x3c)[0]
assert data[pe:pe + 4] == b"PE\0\0", "not a PE file"
print(hex(struct.unpack_from("<H", data, pe + 4)[0]))' "$1"
}
[ "$(machine "$x86_dll")" = 0x14c ] || { echo "$x86_dll is not a 32-bit DLL" >&2; exit 1; }
[ "$(machine "$x64_dll")" = 0x8664 ] || { echo "$x64_dll is not a 64-bit DLL" >&2; exit 1; }
cp "$x86_dll" "$stage/dll/x86/sunat_sdk_ffi.dll"
cp "$x64_dll" "$stage/dll/x64/sunat_sdk_ffi.dll"

cp "$legacy/vb6/SunatSdk.bas" "$legacy/vb6/SunatJson.bas" "$legacy/vb6/Sample.bas" "$legacy/vb6/Sample.vbp" "$stage/vb6/"
cp "$legacy/vfp/sunat_sdk.prg" "$legacy/vfp/sample.prg" "$stage/vfp/"
cp "$legacy/powerbuilder/n_sunat_sdk.sru" "$legacy/powerbuilder/f_sunat_sdk_sample.srf" "$stage/powerbuilder/"
# The test certificate is self-generated and valid for no taxpayer (tests/vectors/test-cert/README.md).
cp "$repo/fixtures/01-invoice-taxed-cash.json" "$repo/tests/vectors/01-invoice-taxed-cash.signed.xml" \
  "$repo/tests/vectors/test-cert/test-cert.pem" "$repo/tests/vectors/test-cert/test-key.pem" "$stage/sample/"
cp "$legacy/README.md" "$repo/LICENSE" "$repo/NOTICE" "$stage/"

for file in "$stage"/vb6/* "$stage"/vfp/* "$stage"/powerbuilder/*; do
  if LC_ALL=C grep -q '[^[:print:][:space:]]' "$file"; then
    echo "$file: not ASCII; the IDE reads the ANSI code page" >&2
    exit 1
  fi
  if [ "$(tr -cd '\n' < "$file" | wc -c)" -ne "$(grep -c $'\r$' "$file")" ]; then
    echo "$file: a line without CRLF (check bindings/legacy/.gitattributes)" >&2
    exit 1
  fi
done

(cd "$output" && find "$name" -type f | sort | xargs sha256sum) >&2
echo "$name"
