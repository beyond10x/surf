#!/bin/sh
# usage: tls-flights-check.sh [--self-test] [<flights dir>]
#
# The gate's check of the recorded flights (story tls-handshake): fast, no build, no openssl, no
# network, POSIX sh. Run from the repository root. <flights dir> defaults to
# crates/tls/tests/data/flights.
#
# It requires that every *.bin of the directory has a line `<sha256>  <file>` in its SOURCE and
# that the sha256 matches, that every such line names a file that exists, and that DIGEST, one
# line `<sha256>  crates/tls/examples/record-flights.rs`, matches the recorder. On a match it
# prints `tls-flights: <n> recordings match SOURCE, the recorder matches DIGEST` and exits 0.
# Otherwise it prints one line per file that differs, is missing or is not pinned, naming it,
# then `tls-flights: run task tls-flights-slow`, and exits 1.
#
# With --self-test it copies the directory into .out/tls-flights-check-test, changes one byte of
# one recording, and requires that the check fails naming that file.

set -eu
set -f
LC_ALL=C
export LC_ALL

recorder=crates/tls/examples/record-flights.rs

check() {
  dir=$1
  failed=0
  if [ ! -s "$dir/SOURCE" ]; then
    echo "tls-flights: $dir/SOURCE is missing or empty"
    echo "tls-flights: run task tls-flights-slow"
    return 1
  fi
  pins=$(grep -E '^[0-9a-f]{64}  [A-Za-z0-9._-]+$' "$dir/SOURCE") || pins=
  count=0
  for file in $(ls "$dir" | grep -E '^[A-Za-z0-9._-]+\.bin$'); do
    path=$dir/$file
    count=$((count + 1))
    pin=$(printf '%s\n' "$pins" | awk -v f="$file" '$2 == f { print $1 }')
    if [ -z "$pin" ]; then
      echo "tls-flights: $file is not in SOURCE"
      failed=1
      continue
    fi
    sum=$(sha256sum "$path" | awk '{ print $1 }')
    if [ "$sum" != "$pin" ]; then
      echo "tls-flights: $file differs from its sha256 in SOURCE"
      failed=1
    fi
  done
  if [ -n "$pins" ]; then
    for file in $(printf '%s\n' "$pins" | awk '{ print $2 }'); do
      if [ ! -f "$dir/$file" ]; then
        echo "tls-flights: $file is in SOURCE but missing"
        failed=1
      fi
    done
  fi
  if [ "$count" = 0 ]; then
    echo "tls-flights: $dir holds no recording"
    failed=1
  fi
  digest="$(sha256sum "$recorder" | awk '{ print $1 }')  $recorder"
  if [ ! -f "$dir/DIGEST" ] || [ "$(cat "$dir/DIGEST")" != "$digest" ]; then
    echo "tls-flights: $recorder changed since the flights were recorded (DIGEST)"
    failed=1
  fi
  if [ "$failed" = 1 ]; then
    echo "tls-flights: run task tls-flights-slow"
    return 1
  fi
  echo "tls-flights: $count recordings match SOURCE, the recorder matches DIGEST"
}

if [ "${1:-}" = "--self-test" ]; then
  dir=${2:-crates/tls/tests/data/flights}
  copy=.out/tls-flights-check-test
  rm -rf "$copy"
  mkdir -p "$copy"
  cp -R "$dir" "$copy/flights"
  first=$(ls "$copy/flights" | grep '\.bin$' | head -n 1)
  byte=$(od -An -tu1 -j8 -N1 "$copy/flights/$first" | tr -d ' ')
  new=X
  [ "$byte" != 88 ] || new=Y
  printf '%s' "$new" | dd of="$copy/flights/$first" bs=1 seek=8 conv=notrunc 2>/dev/null
  if output=$(check "$copy/flights"); then
    echo "tls-flights: self-test failed: a changed copy of $first passed"
    exit 1
  fi
  case $output in
    *"$first differs from its sha256 in SOURCE"*) ;;
    *)
      echo "tls-flights: self-test failed: the check did not name $first:"
      echo "$output"
      exit 1
      ;;
  esac
  rm -rf "$copy"
  echo "tls-flights: self-test: a changed copy of $first fails naming it"
  exit 0
fi

check "${1:-crates/tls/tests/data/flights}"
