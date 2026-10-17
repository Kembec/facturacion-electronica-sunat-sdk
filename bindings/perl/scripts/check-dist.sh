#!/usr/bin/env bash
# What a consumer gets from the CPAN distribution: the version of the Rust workspace (as a CPAN trial while it is a
# beta), every module, the README, the LICENSE and the tests, exactly one native library per supported platform with
# that platform's architecture, for Linux the oldest glibc it loads on, and no key or certificate.
#
#   check-dist.sh <dist>
set -euo pipefail

dist=$(cd "${1:?usage: check-dist.sh <dist>}" && pwd)
here=$(cd "$(dirname "$0")/.." && pwd)
repo=$(cd "$here/../.." && pwd)
# The Linux builds come from manylinux2014 (.github/workflows/perl.yml): no symbol newer than this.
max_glibc=2.17

cargo_version=$(sed -n '/^\[workspace.package\]/,/^\[/s/^version = "\(.*\)"/\1/p' "$repo/Cargo.toml")
sdk_version=$(sed -n "s/^use constant SDK_VERSION => '\(.*\)';/\1/p" "$here/lib/Kembec/SunatSdk.pm")
[ -n "$cargo_version" ] && [ "$cargo_version" = "$sdk_version" ] \
  || { echo "module SDK_VERSION $sdk_version differs from Cargo's $cargo_version" >&2; exit 1; }
# X.Y.Z is X.00Y00Z; beta N of X.Y.Z is the decimal just below it with _N (lib/Kembec/SunatSdk.pm).
cpan_version=$(perl -e '
  my ($x, $y, $z, $beta) = $ARGV[0] =~ /\A(\d+)\.(\d+)\.(\d+)(?:-beta\.(\d+))?\z/ or die "version $ARGV[0]\n";
  my $micro = $x * 1_000_000 + $y * 1_000 + $z;
  $micro -= 1 if defined $beta;
  printf "%d.%06d%s", int($micro / 1_000_000), $micro % 1_000_000, defined $beta ? sprintf("_%03d", $beta) : "";
' "$sdk_version")
module_version=$(sed -n "s/^our \$VERSION = '\(.*\)';/\1/p" "$here/lib/Kembec/SunatSdk.pm")
[ "$module_version" = "$cpan_version" ] \
  || { echo "module \$VERSION $module_version, expected $cpan_version for SDK $sdk_version" >&2; exit 1; }

count=$(find "$dist" -name '*.tar.gz' | wc -l | tr -d ' ')
[ "$count" = 1 ] || { echo "$count distributions in $dist, expected 1" >&2; exit 1; }
tarball="$dist/Kembec-SunatSdk-$cpan_version.tar.gz"
[ -f "$tarball" ] || { echo "missing $tarball" >&2; exit 1; }
work=$(mktemp -d)
trap 'rm -r "$work"' EXIT
tar -xzf "$tarball" -C "$work"
root="$work/Kembec-SunatSdk-$cpan_version"

perl -MJSON::PP -e '
  local $/; open my $h, "<", $ARGV[0] or die; my $meta = JSON::PP->new->decode(<$h>);
  $meta->{version} eq $ARGV[1] or die "META.json version $meta->{version}, expected $ARGV[1]\n";
  my $status = $ARGV[1] =~ /_/ ? "testing" : "stable";
  $meta->{release_status} eq $status or die "META.json release_status $meta->{release_status}, expected $status\n";
  $meta->{prereqs}{runtime}{requires}{"FFI::Platypus"} or die "META.json does not require FFI::Platypus\n";
' "$root/META.json" "$cpan_version"

for entry in $(cd "$here" && find lib -name '*.pm' | sort) README.md LICENSE Changes Makefile.PL MANIFEST META.json \
  t/00-load.t t/10-build.t t/data/01-invoice-taxed-cash.json; do
  [ -f "$root/$entry" ] || { echo "the distribution misses $entry" >&2; exit 1; }
done
secrets=$(find "$root" \( -name '*.pem' -o -name '*.key' -o -name '*.p12' -o -name '*.pfx' \) | wc -l | tr -d ' ')
[ "$secrets" = 0 ] || { echo "the distribution carries $secrets key or certificate files" >&2; exit 1; }

# rid, native file, and an extended regex over what `file` says about it (its architecture)
expected=(
  "linux-x64 libsunat_sdk_ffi.so ^ELF 64-bit .*x86-64"
  "linux-arm64 libsunat_sdk_ffi.so ^ELF 64-bit .*aarch64"
  "osx-x64 libsunat_sdk_ffi.dylib ^Mach-O 64-bit .*x86_64"
  "osx-arm64 libsunat_sdk_ffi.dylib ^Mach-O 64-bit .*arm64"
  "win-x64 sunat_sdk_ffi.dll ^PE32\\+ .*x86-64"
)
natives="$root/lib/Kembec/SunatSdk/native"
found=$(find "$natives" -type f | wc -l | tr -d ' ')
[ "$found" = "${#expected[@]}" ] || { echo "$found native libraries, expected ${#expected[@]}" >&2; exit 1; }
for row in "${expected[@]}"; do
  read -r rid file pattern <<<"$row"
  library="$natives/$rid/$file"
  [ -f "$library" ] || { echo "missing $rid/$file" >&2; exit 1; }
  grep -qx "lib/Kembec/SunatSdk/native/$rid/$file" "$root/MANIFEST" || { echo "MANIFEST misses $rid/$file" >&2; exit 1; }
  kind=$(file -b "$library")
  grep -Eq "$pattern" <<<"$kind" || { echo "$rid/$file is «${kind}», expected «${pattern}»" >&2; exit 1; }
  case $rid in
    linux-*)
      glibc=$(readelf -V -W "$library" | grep -o 'GLIBC_[0-9.]*' | sed 's/GLIBC_//' | sort -uV | tail -1)
      newest=$(printf '%s\n%s\n' "$glibc" "$max_glibc" | sort -V | tail -1)
      [ "$newest" = "$max_glibc" ] || { echo "$rid needs glibc $glibc, above $max_glibc" >&2; exit 1; }
      echo "$rid: $file needs glibc $glibc or later"
      ;;
  esac
  echo "ok $rid/$file: $kind"
done
echo "ok $(basename "$tarball"): $(find "$root" -type f | wc -l | tr -d ' ') files, $(du -k "$tarball" | cut -f1) KB"
