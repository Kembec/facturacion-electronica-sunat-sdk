#!/usr/bin/env bash
# What a consumer gets from Maven: the version of the Rust workspace, the jar with its six native builds, Java 8
# bytecode, the manifest, the sources and javadoc jars and the pom; and the oldest glibc the Linux builds load on.
#
#   check-package.sh <artifacts-dir>
set -euo pipefail

artifacts=${1:?usage: check-package.sh <artifacts-dir>}
here=$(cd "$(dirname "$0")/.." && pwd)
repo=$(cd "$here/../.." && pwd)
# The Linux builds come from manylinux2014 (.github/workflows/java.yml): no symbol newer than this.
max_glibc=2.17

cargo_version=$(sed -n '/^\[workspace.package\]/,/^\[/s/^version = "\(.*\)"/\1/p' "$repo/Cargo.toml")
version=$(sed -n '0,/<artifactId>sunat-sdk<\/artifactId>/d; s:.*<version>\(.*\)</version>.*:\1:p' "$here/pom.xml" | head -1)
[ -n "$cargo_version" ] && [ "$cargo_version" = "$version" ] \
  || { echo "package version $version differs from Cargo's $cargo_version" >&2; exit 1; }

base="$artifacts/sunat-sdk-$version"
for file in "$base.jar" "$base-sources.jar" "$base-javadoc.jar" "$base.pom"; do
  [ -s "$file" ] || { echo "missing artifact: $file" >&2; exit 1; }
done
cmp -s "$base.pom" "$here/pom.xml" || { echo "$base.pom is not bindings/java/pom.xml" >&2; exit 1; }

entries=$(unzip -Z1 "$base.jar")
missing=0
for entry in \
  META-INF/MANIFEST.MF com/kembec/sunatsdk/SunatSdkClient.class \
  com/kembec/sunatsdk/native/win32-x86/sunat_sdk_ffi.dll com/kembec/sunatsdk/native/win32-x86-64/sunat_sdk_ffi.dll \
  com/kembec/sunatsdk/native/linux-x86-64/libsunat_sdk_ffi.so com/kembec/sunatsdk/native/linux-aarch64/libsunat_sdk_ffi.so \
  com/kembec/sunatsdk/native/darwin-x86-64/libsunat_sdk_ffi.dylib com/kembec/sunatsdk/native/darwin-aarch64/libsunat_sdk_ffi.dylib
do
  grep -qxF "$entry" <<<"$entries" || { echo "missing in the jar: $entry" >&2; missing=1; }
done
unzip -Z1 "$base-sources.jar" | grep -qxF com/kembec/sunatsdk/SunatSdkClient.java || { echo "sources jar without the client" >&2; missing=1; }
unzip -Z1 "$base-javadoc.jar" | grep -qxF index.html || { echo "javadoc jar without index.html" >&2; missing=1; }
[ "$missing" = 0 ] || exit 1

manifest=$(unzip -p "$base.jar" META-INF/MANIFEST.MF | tr -d '\r')
grep -qxF "Implementation-Version: $version" <<<"$manifest" || { echo "manifest without Implementation-Version: $version" >&2; exit 1; }
grep -qxF "Automatic-Module-Name: com.kembec.sunatsdk" <<<"$manifest" || { echo "manifest without Automatic-Module-Name" >&2; exit 1; }

# Class file major version 52 is Java 8: a newer one would not load where most billing systems still run.
for class in $(grep '\.class$' <<<"$entries"); do
  major=$(unzip -p "$base.jar" "$class" | od -An -tu1 -j6 -N2 | awk '{print $1 * 256 + $2}')
  [ "$major" = 52 ] || { echo "$class has class file version $major, not 52 (Java 8)" >&2; exit 1; }
done

work=$(mktemp -d)
trap 'rm -r "$work"' EXIT
unzip -q "$base.jar" 'com/kembec/sunatsdk/native/*' -d "$work"
for prefix in linux-x86-64 linux-aarch64; do
  glibc=$(readelf -V -W "$work/com/kembec/sunatsdk/native/$prefix/libsunat_sdk_ffi.so" | grep -o 'GLIBC_[0-9.]*' | sed 's/GLIBC_//' | sort -uV | tail -1)
  newest=$(printf '%s\n%s\n' "$glibc" "$max_glibc" | sort -V | tail -1)
  [ "$newest" = "$max_glibc" ] || { echo "$prefix needs glibc $glibc, above $max_glibc" >&2; exit 1; }
  echo "$prefix: needs glibc $glibc or later"
done
echo "ok: com.kembec:sunat-sdk:$version, $(wc -l <<<"$entries" | tr -d ' ') entries in the jar, $(du -k "$base.jar" | cut -f1) KB"
