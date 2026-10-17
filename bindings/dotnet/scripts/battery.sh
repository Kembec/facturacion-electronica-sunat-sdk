#!/usr/bin/env bash
# The .NET battery in C# and in VB.NET against the .nupkg in bindings/dotnet/artifacts, restored as a consumer
# restores it (tests/nuget.config): what is published is what is tested, not the source.
#
#   battery.sh net10.0           run with `dotnet`, or with $DOTNET_RUNNER (an x86 dotnet.exe for 32-bit)
#   battery.sh net48 x64|x86     Windows: .NET Framework at that bitness, through the package's loader
set -euo pipefail

framework=${1:?usage: battery.sh <net10.0|net48> [x64|x86]}
platform=${2:-}
here=$(cd "$(dirname "$0")/.." && pwd)
repo=$(cd "$here/../.." && pwd)
if command -v cygpath >/dev/null; then
  here=$(cygpath -m "$here")
  repo=$(cygpath -m "$repo")
fi
version=$(sed -n 's:.*<Version>\(.*\)</Version>.*:\1:p' "$here/src/Kembec.SunatSdk/Kembec.SunatSdk.csproj")

for project in Vectors.CSharp Vectors.VisualBasic; do
  out="$here/tests/out/$project-$framework${platform:+-$platform}"
  args=(build "$here/tests/$project" -c Release -f "$framework" -o "$out" -p:SunatSdkVersion="$version" --nologo)
  if [ -n "$platform" ]; then
    args+=(-p:PlatformTarget="$platform")
  fi
  dotnet "${args[@]}"
  case $framework in
    net4*)
      # A copy next to the exe would hide a broken loader: the package's runtimes\win-<bitness>\native must do.
      rm -f "$out/sunat_sdk_ffi.dll"
      ls "$out/runtimes/win-x86/native/sunat_sdk_ffi.dll" "$out/runtimes/win-x64/native/sunat_sdk_ffi.dll"
      "$out/$project.exe" "$repo"
      ;;
    *)
      "${DOTNET_RUNNER:-dotnet}" "$out/$project.dll" "$repo"
      ;;
  esac
done
