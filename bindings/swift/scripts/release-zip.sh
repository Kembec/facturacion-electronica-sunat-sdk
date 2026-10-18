# Sourced by pin-release.sh and publish-release.sh: the one place that knows where the released xcframework zip comes
# from (the `SunatSDKFFI.xcframework-release-zip` artifact of a swift.yml run), where it goes (the release of tag
# v<version>) and how its SHA-256 is checked. Both scripts check the same bytes the same way, so the zip a release
# attaches is the one whose checksum the tagged Package.swift names.
# shellcheck shell=bash

RELEASE_REPO="Kembec/facturacion-electronica-sunat-sdk"
RELEASE_ARTIFACT="SunatSDKFFI.xcframework-release-zip"
RELEASE_ZIP="SunatSDKFFI.xcframework.zip"
# shellcheck disable=SC2034  # read by the scripts that source this file
RELEASE_LINE_PATTERN='^let release: \(url: String, checksum: String\)\? = '

# The asset URL SwiftPM downloads for a version.
release_url() {
  printf 'https://github.com/%s/releases/download/v%s/%s\n' "$RELEASE_REPO" "$1" "$RELEASE_ZIP"
}

release_sha256() {
  if command -v sha256sum >/dev/null; then sha256sum "$1" | cut -d' ' -f1; else shasum -a 256 "$1" | cut -d' ' -f1; fi
}

# Downloads the artifact of run $1 into $2 and checks the zip against the .sha256 the CI wrote next to it; prints the
# SHA-256. The run must be a successful run of swift.yml in this repository.
fetch_release_zip() {
  local run_id=$1 dir=$2 meta path conclusion branch expected got
  meta=$(gh api "repos/$RELEASE_REPO/actions/runs/$run_id" --jq '[.path, .conclusion, .head_branch] | @tsv')
  IFS=$'\t' read -r path conclusion branch <<<"$meta"
  [ "$path" = ".github/workflows/swift.yml" ] || { echo "run $run_id is $path, not swift.yml" >&2; return 1; }
  [ "$conclusion" = "success" ] || { echo "run $run_id of swift.yml concluded '$conclusion', not success" >&2; return 1; }
  echo "run $run_id: swift.yml on $branch, success" >&2
  gh run download "$run_id" --repo "$RELEASE_REPO" -n "$RELEASE_ARTIFACT" -D "$dir" >&2
  [ -f "$dir/$RELEASE_ZIP" ] && [ -f "$dir/$RELEASE_ZIP.sha256" ] || {
    echo "the artifact of run $run_id has no $RELEASE_ZIP and its .sha256" >&2
    return 1
  }
  expected=$(tr -d '[:space:]' <"$dir/$RELEASE_ZIP.sha256")
  got=$(release_sha256 "$dir/$RELEASE_ZIP")
  [ "$got" = "$expected" ] || { echo "$RELEASE_ZIP: SHA-256 $got, its .sha256 says $expected" >&2; return 1; }
  printf '%s\n' "$got"
}
