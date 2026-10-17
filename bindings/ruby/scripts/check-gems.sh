#!/usr/bin/env bash
# What a consumer gets from each .gem: the version of the Rust workspace, the platform in the file name and in the
# spec, every Ruby file, the README and the LICENSE, exactly one native library of that platform's architecture, and
# for Linux the oldest glibc it loads on.
#
#   check-gems.sh <pkg>
set -euo pipefail

pkg=${1:?usage: check-gems.sh <pkg>}
here=$(cd "$(dirname "$0")/.." && pwd)
repo=$(cd "$here/../.." && pwd)
# The Linux builds come from manylinux2014 (.github/workflows/ruby.yml): no symbol newer than this.
max_glibc=2.17

cargo_version=$(sed -n '/^\[workspace.package\]/,/^\[/s/^version = "\(.*\)"/\1/p' "$repo/Cargo.toml")
sdk_version=$(sed -n 's/^ *SDK_VERSION = "\(.*\)"/\1/p' "$here/lib/kembec/sunat_sdk/version.rb")
[ -n "$cargo_version" ] && [ "$cargo_version" = "$sdk_version" ] \
  || { echo "gem SDK_VERSION $sdk_version differs from Cargo's $cargo_version" >&2; exit 1; }
gem_version=${sdk_version//-/.}

# platform, native file, and an extended regex over what `file` says about it (its architecture)
expected=(
  "x86_64-linux-gnu libsunat_sdk_ffi.so ^ELF 64-bit .*x86-64"
  "aarch64-linux-gnu libsunat_sdk_ffi.so ^ELF 64-bit .*aarch64"
  "x86_64-darwin libsunat_sdk_ffi.dylib ^Mach-O 64-bit .*x86_64"
  "arm64-darwin libsunat_sdk_ffi.dylib ^Mach-O 64-bit .*arm64"
  "x64-mingw-ucrt sunat_sdk_ffi.dll ^PE32\\+ .*x86-64"
  "x86-mingw32 sunat_sdk_ffi.dll ^PE32 .*80386"
)
ruby_files=$(cd "$here" && find lib -name '*.rb' | sort)
work=$(mktemp -d)
trap 'rm -r "$work"' EXIT

count=$(find "$pkg" -name '*.gem' | wc -l | tr -d ' ')
[ "$count" = "${#expected[@]}" ] || { echo "$count gems in $pkg, expected ${#expected[@]}" >&2; exit 1; }
for row in "${expected[@]}"; do
  read -r platform file pattern <<<"$row"
  gem="$pkg/kembec-sunat_sdk-$gem_version-$platform.gem"
  [ -f "$gem" ] || { echo "missing $gem" >&2; exit 1; }
  dir="$work/$platform"
  mkdir -p "$dir"
  tar -xf "$gem" -C "$dir" data.tar.gz metadata.gz
  mkdir "$dir/data"
  tar -xzf "$dir/data.tar.gz" -C "$dir/data"
  spec=$(gunzip -c "$dir/metadata.gz")
  grep -q "^platform: $platform\$" <<<"$spec" || { echo "$gem: the spec's platform is not $platform" >&2; exit 1; }
  grep -q "^  version: $gem_version\$" <<<"$spec" || { echo "$gem: the spec's version is not $gem_version" >&2; exit 1; }
  for entry in $ruby_files README.md LICENSE; do
    [ -f "$dir/data/$entry" ] || { echo "$gem: missing $entry" >&2; exit 1; }
  done
  natives=$(ls "$dir/data/lib/kembec/sunat_sdk/native")
  [ "$natives" = "$file" ] || { echo "$gem: native libraries [$natives], expected $file" >&2; exit 1; }
  library="$dir/data/lib/kembec/sunat_sdk/native/$file"
  kind=$(file -b "$library")
  grep -Eq "$pattern" <<<"$kind" || { echo "$gem: $file is «${kind}», expected «${pattern}»" >&2; exit 1; }
  case $platform in
    *-linux-gnu)
      glibc=$(readelf -V -W "$library" | grep -o 'GLIBC_[0-9.]*' | sed 's/GLIBC_//' | sort -uV | tail -1)
      newest=$(printf '%s\n%s\n' "$glibc" "$max_glibc" | sort -V | tail -1)
      [ "$newest" = "$max_glibc" ] || { echo "$platform needs glibc $glibc, above $max_glibc" >&2; exit 1; }
      echo "$platform: $file needs glibc $glibc or later"
      ;;
  esac
  echo "ok $(basename "$gem"): $(find "$dir/data" -type f | wc -l | tr -d ' ') files, $kind"
done
