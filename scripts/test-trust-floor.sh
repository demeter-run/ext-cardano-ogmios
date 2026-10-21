#!/bin/sh
# Runs check-trust-floor.sh against the fixture lockfiles and checks each
# exit code and the line it must print. No network.
set -eu

here="$(dirname "$0")"
check="$here/check-trust-floor.sh"
fixtures="$here/trust-floor-fixtures"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

# Generated inputs: line endings and empty files do not survive git reliably.
awk '{ printf "%s\r\n", $0 }' "$fixtures/fail-webpki-roots.lock" > "$tmp/fail-crlf.lock"
awk '{ printf "%s\r\n", $0 }' "$fixtures/pass-floor.lock" > "$tmp/pass-crlf.lock"
: > "$tmp/empty.lock"

failures=0

# expect <lockfile> <exit code> <line that must appear> [line that must not]
expect() {
  out="$(sh "$check" "$1" 2>&1)" && code=0 || code=$?
  if [ "$code" -ne "$2" ]; then
    echo "not ok $(basename "$1"): exit $code, want $2"
    failures=$((failures + 1))
  elif ! printf '%s\n' "$out" | grep -qF -- "$3"; then
    echo "not ok $(basename "$1"): missing \"$3\""
    failures=$((failures + 1))
  elif [ -n "${4:-}" ] && printf '%s\n' "$out" | grep -qF -- "$4"; then
    echo "not ok $(basename "$1"): unexpected \"$4\""
    failures=$((failures + 1))
  else
    echo "ok $(basename "$1")"
    return 0
  fi
  printf '%s\n' "$out" | sed 's/^/    /'
}

expect "$fixtures/pass-floor.lock" 0 "check-trust-floor: ok" "WARN"
expect "$fixtures/pass-above.lock" 0 "check-trust-floor: ok" "WARN"
expect "$fixtures/pass-patch-unused.lock" 0 "check-trust-floor: ok" "FAIL"
expect "$tmp/pass-crlf.lock" 0 "check-trust-floor: ok" "WARN"

expect "$fixtures/fail-webpki-roots.lock" 1 "FAIL webpki-roots 1.0.9:" "check-trust-floor: ok"
expect "$fixtures/fail-webpki-root-certs.lock" 1 "FAIL webpki-root-certs 1.0.2:"
expect "$fixtures/fail-rustls-webpki.lock" 1 "FAIL rustls-webpki 0.103.15:"
expect "$fixtures/fail-rustls-webpki-old-patch.lock" 1 "FAIL rustls-webpki 0.103.4:"
expect "$fixtures/fail-webpki-roots-prerelease.lock" 1 "FAIL webpki-roots 1.0.10-rc.1:"
expect "$fixtures/fail-rustls-webpki-prerelease.lock" 1 "FAIL rustls-webpki 0.103.16-alpha.1:"
expect "$tmp/fail-crlf.lock" 1 "FAIL webpki-roots 1.0.9:"

expect "$fixtures/warn-old-lines.lock" 0 "WARN webpki-roots 0.26.11:" "FAIL"
expect "$fixtures/warn-old-lines.lock" 0 "WARN rustls-webpki 0.101.7:"
expect "$fixtures/warn-old-lines.lock" 0 "WARN rustls-webpki 0.102.8:"

expect "$tmp/empty.lock" 2 "not a Cargo.lock"
expect "$fixtures/not-a-lockfile.txt" 2 "not a Cargo.lock"
expect "$tmp/missing.lock" 2 "not found"

if [ "$failures" -ne 0 ]; then
  echo "test-trust-floor: $failures failed" >&2
  exit 1
fi
echo "test-trust-floor: all passed"
