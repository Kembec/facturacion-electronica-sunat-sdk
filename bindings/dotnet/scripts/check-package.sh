#!/usr/bin/env bash
# What a consumer gets from the .nupkg: the version of the Rust workspace, the three frameworks, the six native
# builds, the .NET Framework targets and the README; and the oldest glibc the Linux builds load on.
#
#   check-package.sh <package.nupkg>
set -euo pipefail

nupkg=${1:?usage: check-package.sh <package.nupkg>}
here=$(cd "$(dirname "$0")/.." && pwd)
repo=$(cd "$here/../.." && pwd)
# The Linux builds come from manylinux2014 (.github/workflows/dotnet.yml): no symbol newer than this.
max_glibc=2.17

cargo_version=$(sed -n '/^\[workspace.package\]/,/^\[/s/^version = "\(.*\)"/\1/p' "$repo/Cargo.toml")
package_version=$(sed -n 's:.*<Version>\(.*\)</Version>.*:\1:p' "$here/src/Kembec.SunatSdk/Kembec.SunatSdk.csproj")
[ -n "$cargo_version" ] && [ "$cargo_version" = "$package_version" ] \
  || { echo "package version $package_version differs from Cargo's $cargo_version" >&2; exit 1; }
case $(basename "$nupkg") in
  "Kembec.SunatSdk.$package_version.nupkg") ;;
  *) echo "unexpected package name $(basename "$nupkg")" >&2; exit 1 ;;
esac

entries=$(unzip -Z1 "$nupkg")
missing=0
for entry in \
  README.md \
  lib/net462/Kembec.SunatSdk.dll lib/net462/Kembec.SunatSdk.xml \
  lib/netstandard2.0/Kembec.SunatSdk.dll lib/netstandard2.0/Kembec.SunatSdk.xml \
  lib/net10.0/Kembec.SunatSdk.dll lib/net10.0/Kembec.SunatSdk.xml \
  build/Kembec.SunatSdk.targets buildTransitive/Kembec.SunatSdk.targets \
  runtimes/win-x64/native/sunat_sdk_ffi.dll runtimes/win-x86/native/sunat_sdk_ffi.dll \
  runtimes/linux-x64/native/libsunat_sdk_ffi.so runtimes/linux-arm64/native/libsunat_sdk_ffi.so \
  runtimes/osx-x64/native/libsunat_sdk_ffi.dylib runtimes/osx-arm64/native/libsunat_sdk_ffi.dylib
do
  grep -qxF "$entry" <<<"$entries" || { echo "missing in the package: $entry" >&2; missing=1; }
done
[ "$missing" = 0 ] || exit 1

work=$(mktemp -d)
trap 'rm -r "$work"' EXIT
unzip -q "$nupkg" 'runtimes/*' -d "$work"
for rid in linux-x64 linux-arm64; do
  glibc=$(readelf -V -W "$work/runtimes/$rid/native/libsunat_sdk_ffi.so" | grep -o 'GLIBC_[0-9.]*' | sed 's/GLIBC_//' | sort -uV | tail -1)
  newest=$(printf '%s\n%s\n' "$glibc" "$max_glibc" | sort -V | tail -1)
  [ "$newest" = "$max_glibc" ] || { echo "$rid needs glibc $glibc, above $max_glibc" >&2; exit 1; }
  echo "$rid: needs glibc $glibc or later"
done
echo "ok: Kembec.SunatSdk $package_version, $(wc -l <<<"$entries" | tr -d ' ') entries"
