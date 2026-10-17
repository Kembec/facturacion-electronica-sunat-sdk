#!/usr/bin/env bash
# Maven $MAVEN_VERSION from Maven Central into $RUNNER_TEMP, checked against $MAVEN_SHA512, and put on the PATH of the
# following steps. The runner images do not all carry Maven, nor the same version.
set -euo pipefail
: "${MAVEN_VERSION:?}" "${MAVEN_SHA512:?}" "${RUNNER_TEMP:?}" "${GITHUB_PATH:?}"
archive="$RUNNER_TEMP/apache-maven-$MAVEN_VERSION-bin.tar.gz"
curl -fsSL --retry 3 -o "$archive" \
  "https://repo.maven.apache.org/maven2/org/apache/maven/apache-maven/$MAVEN_VERSION/apache-maven-$MAVEN_VERSION-bin.tar.gz"
# From stdin: with a path holding backslashes (Windows' RUNNER_TEMP) sha512sum prefixes its line with one.
if command -v sha512sum >/dev/null; then sum=$(sha512sum <"$archive"); else sum=$(shasum -a 512 <"$archive"); fi
[ "${sum%% *}" = "$MAVEN_SHA512" ] || { echo "SHA-512 of Maven $MAVEN_VERSION does not match" >&2; exit 1; }
tar -xzf "$archive" -C "$RUNNER_TEMP"
bin="$RUNNER_TEMP/apache-maven-$MAVEN_VERSION/bin"
if command -v cygpath >/dev/null; then bin=$(cygpath -w "$bin"); fi
echo "$bin" >> "$GITHUB_PATH"
"$RUNNER_TEMP/apache-maven-$MAVEN_VERSION/bin/mvn" -v
