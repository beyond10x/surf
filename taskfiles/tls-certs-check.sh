#!/bin/sh
# usage: tls-certs-check.sh [--write] [<crate dir>]
#
# The gate's check of the committed test certificates (story tls-test-certs): fast, no build, no
# openssl. It recomputes the digest of the sources certs/ was made from and compares it with
# <crate dir>/certs/DIGEST (crate dir default: crates/tls-test-certs).
#
# DIGEST holds one line per source, `<sha256>  <source>`, for src/, keys/, openssl/ and
# Cargo.toml in that order, then `<sha256>  sources`, the sha256 of those four lines. The sha256
# of a directory is that of the lines `sha256sum` prints for its files, sorted by path in the C
# locale, each path relative to the crate directory; the sha256 of Cargo.toml is that of the
# file.
#
# On a match it prints `tls-certs: <n> cases, certs/ matches its sources` (n: the lines of
# certs/INDEX) and exits 0. Otherwise it prints `tls-certs: the sources changed since certs/ was
# generated; run task tls-certs-slow` and one line per source whose sha256 differs, and exits 1.
# With --write it writes DIGEST instead (task tls-certs-slow WRITE=1 calls it so).

set -eu

write=0
if [ "${1:-}" = "--write" ]; then
  write=1
  shift
fi
crate=${1:-crates/tls-test-certs}
cd "$crate"

lines() {
  for source in src keys openssl; do
    sum=$(find "$source" -type f | LC_ALL=C sort | xargs sha256sum | sha256sum | cut -d ' ' -f 1)
    printf '%s  %s/\n' "$sum" "$source"
  done
  printf '%s  Cargo.toml\n' "$(sha256sum < Cargo.toml | cut -d ' ' -f 1)"
}

sources=$(lines)
total=$(printf '%s\n' "$sources" | sha256sum | cut -d ' ' -f 1)
digest=$(printf '%s\n%s  sources' "$sources" "$total")

if [ "$write" = 1 ]; then
  printf '%s\n' "$digest" > certs/DIGEST
  echo "tls-certs: wrote certs/DIGEST, sources $total"
  exit 0
fi

if [ ! -f certs/DIGEST ] || [ "$(cat certs/DIGEST)" != "$digest" ]; then
  echo "tls-certs: the sources changed since certs/ was generated; run task tls-certs-slow"
  printf '%s\n' "$sources" | while read -r sum source; do
    if ! grep -qxF "$sum  $source" certs/DIGEST 2>/dev/null; then
      echo "tls-certs: $source differs from certs/DIGEST"
    fi
  done
  exit 1
fi

cases=$(wc -l < certs/INDEX | tr -d ' ')
echo "tls-certs: $cases cases, certs/ matches its sources"
