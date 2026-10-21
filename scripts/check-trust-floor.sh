#!/bin/sh
# Fails when Cargo.lock resolves a root-store or certificate-verifier crate
# below the release that drops the compromised eMudhra roots:
#   webpki-roots / webpki-root-certs 1.x  >= 1.0.10
#   rustls-webpki 0.103.x                 >= 0.103.16
# Older lines (webpki-roots 0.x, rustls-webpki 0.101/0.102) only warn.
# Reads the lockfile only; no network.
set -eu

lock="${1:-$(dirname "$0")/../Cargo.lock}"

if [ ! -f "$lock" ]; then
  echo "check-trust-floor: $lock not found" >&2
  exit 2
fi

awk '
  # Compare dotted numeric versions; pre-release suffixes are ignored.
  function below(v, floor,    a, b, i, n) {
    sub(/[-+].*/, "", v)
    split(v, a, ".")
    n = split(floor, b, ".")
    for (i = 1; i <= n; i++) {
      if (a[i] + 0 < b[i] + 0) return 1
      if (a[i] + 0 > b[i] + 0) return 0
    }
    return 0
  }

  function check(name, version,    major, minor, p) {
    split(version, p, ".")
    major = p[1] + 0
    minor = p[2] + 0

    if (name == "webpki-roots" || name == "webpki-root-certs") {
      if (major == 1 && below(version, "1.0.10")) {
        printf "FAIL %s %s: below the 1.0.10 trust floor\n", name, version
        failed = 1
      } else if (major == 0) {
        printf "WARN %s %s: 0.x line predates the trust floor\n", name, version
      }
    } else if (name == "rustls-webpki") {
      if (major == 0 && minor == 103 && below(version, "0.103.16")) {
        printf "FAIL %s %s: below the 0.103.16 trust floor\n", name, version
        failed = 1
      } else if (major == 0 && minor < 103) {
        printf "WARN %s %s: 0.%d line predates the trust floor\n", name, version, minor
      }
    }
  }

  /^\[\[package\]\]/ { name = ""; next }
  /^name = / { name = $3; gsub(/"/, "", name); next }
  /^version = / {
    version = $3
    gsub(/"/, "", version)
    if (name != "") check(name, version)
    next
  }

  END {
    if (failed) exit 1
    print "check-trust-floor: ok"
  }
' "$lock"
