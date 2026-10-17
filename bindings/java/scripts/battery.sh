#!/usr/bin/env bash
# The Java battery against the jar in an artifacts directory, resolved as a consumer resolves it: the jar and its pom
# installed into a local repository of their own (tests/.m2), the battery compiled against them (tests/pom.xml) and
# run, with JNA from Maven Central as the pom declares it. What is published is what is tested, not ../src.
#
#   battery.sh build <artifacts-dir>     install the jar, compile the battery (Java 8 bytecode), copy its classpath
#   battery.sh run <java-home>...        run the battery on each JDK; fails if any check fails on any of them
set -euo pipefail

here=$(cd "$(dirname "$0")/.." && pwd)
repo=$(cd "$here/../.." && pwd)
version=$(sed -n '0,/<artifactId>sunat-sdk<\/artifactId>/d; s:.*<version>\(.*\)</version>.*:\1:p' "$here/pom.xml" | head -1)
[ -n "$version" ] || { echo "no version in $here/pom.xml" >&2; exit 1; }
native_path() { if command -v cygpath >/dev/null; then cygpath -m "$1"; else printf '%s' "$1"; fi; }

case ${1:-} in
  build)
    artifacts=$(cd "${2:?usage: battery.sh build <artifacts-dir>}" && pwd)
    m2=$(native_path "$here/tests/.m2")
    mvn -B -ntp -q -Dmaven.repo.local="$m2" org.apache.maven.plugins:maven-install-plugin:3.2.0:install-file \
      -Dfile="$(native_path "$artifacts/sunat-sdk-$version.jar")" -DpomFile="$(native_path "$artifacts/sunat-sdk-$version.pom")"
    mvn -B -ntp -q -f "$(native_path "$here/tests/pom.xml")" -Dmaven.repo.local="$m2" -Dsunat-sdk.version="$version" \
      clean compile dependency:copy-dependencies
    ls "$here/tests/target/dependency"
    ;;
  run)
    shift
    [ $# -gt 0 ] || { echo "usage: battery.sh run <java-home>..." >&2; exit 2; }
    sep=:
    if command -v cygpath >/dev/null; then sep=';'; fi
    classpath="$(native_path "$here/tests/target/classes")$sep$(native_path "$here/tests/target/dependency")/*"
    failed=0
    for home in "$@"; do
      echo "--- $home"
      "$home/bin/java" -version 2>&1 | head -1
      "$home/bin/java" -cp "$classpath" com.kembec.sunatsdk.vectors.Vectors "$(native_path "$repo")" || failed=1
    done
    exit $failed
    ;;
  *)
    echo "usage: battery.sh build <artifacts-dir> | run <java-home>..." >&2
    exit 2
    ;;
esac
