#!/bin/sh
# usage: tls-certs-check.sh [--write] [<crate dir>]
#
# The gate's check of the committed test certificates (story tls-test-certs): fast, no build, no
# openssl, POSIX sh. It recomputes the digest of the sources certs/ was made from and compares
# it with <crate dir>/certs/DIGEST (crate dir default: crates/tls-test-certs).
#
# DIGEST holds one line per source, `<sha256>  <source>`, for src/, keys/, openssl/ and
# Cargo.toml in that order, then `<sha256>  sources`, the sha256 of those four lines. The sha256
# of a directory is that of the lines `sha256sum` prints for its files, sorted by path in the C
# locale, each path relative to the crate directory; the sha256 of Cargo.toml is that of the
# file.
#
# First it refuses, naming each, every entry under the crate directory whose name holds a
# character outside A-Z a-z 0-9 . _ - (a space or a line feed among them), every symbolic link
# and every entry that is neither a regular file nor a directory: the digest could not hold
# them. Each is printed as `tls-certs: refused <path>: <why>`, then the line below, exit 1.
#
# On a match it prints `tls-certs: <n> cases, certs/ matches its sources` (n: the lines of
# certs/INDEX) and exits 0. A missing or empty certs/DIGEST or certs/INDEX prints `tls-certs:
# certs/<file> is missing or empty; run task tls-certs-slow` and exits 1. Otherwise it prints
# `tls-certs: the sources changed since certs/ was generated; run task tls-certs-slow` and one
# line per source whose sha256 differs, and exits 1. Every command that fails ends it non-zero.
# With --write it writes DIGEST instead (task tls-certs-slow WRITE=1 calls it so).

set -eu
set -f
LC_ALL=C
export LC_ALL

failed="tls-certs: the sources changed since certs/ was generated; run task tls-certs-slow"

write=0
if [ "${1:-}" = "--write" ]; then
  write=1
  shift
fi
crate=${1:-crates/tls-test-certs}
cd "$crate"

# The entries the digest could not hold.
names=$(find . -name '*[!A-Za-z0-9._-]*') || exit 1
links=$(find . -type l) || exit 1
others=$(find . ! -type f ! -type d ! -type l) || exit 1
if [ -n "$names$links$others" ]; then
  [ -z "$names" ] || printf 'tls-certs: refused %s: a name outside A-Z a-z 0-9 . _ -\n' "$names"
  [ -z "$links" ] || printf 'tls-certs: refused %s: a symbolic link\n' "$links"
  [ -z "$others" ] || printf 'tls-certs: refused %s: neither a file nor a directory\n' "$others"
  echo "$failed"
  exit 1
fi

# The sha256 of the lines sha256sum prints for <dir>'s files, in sorted order.
dir_sum() {
  files=$(find "$1" -type f) || exit 1
  if [ -z "$files" ]; then
    echo "tls-certs: $1/ holds no file"
    exit 1
  fi
  sorted=$(printf '%s\n' "$files" | sort) || exit 1
  # Names are checked above: no blank, no glob character, so the split is the list.
  # shellcheck disable=SC2086
  sums=$(sha256sum $sorted) || exit 1
  sum=$(printf '%s\n' "$sums" | sha256sum) || exit 1
  printf '%s\n' "${sum%% *}"
}

lines=""
for source in src keys openssl; do
  sum=$(dir_sum "$source") || { printf '%s\n' "$sum"; exit 1; }
  lines="$lines$sum  $source/
"
done
sum=$(sha256sum Cargo.toml) || exit 1
sources="$lines${sum%% *}  Cargo.toml"
total=$(printf '%s\n' "$sources" | sha256sum) || exit 1
total=${total%% *}
digest="$sources
$total  sources"

if [ "$write" = 1 ]; then
  printf '%s\n' "$digest" > certs/DIGEST
  echo "tls-certs: wrote certs/DIGEST, sources $total"
  exit 0
fi

for file in DIGEST INDEX; do
  if [ ! -s "certs/$file" ]; then
    echo "tls-certs: certs/$file is missing or empty; run task tls-certs-slow"
    exit 1
  fi
done

committed=$(cat certs/DIGEST) || exit 1
if [ "$committed" != "$digest" ]; then
  echo "$failed"
  printf '%s\n' "$sources" | while read -r sum source; do
    case "
$committed
" in
      *"
$sum  $source
"*) ;;
      *) echo "tls-certs: $source differs from certs/DIGEST" ;;
    esac
  done
  exit 1
fi

cases=$(wc -l < certs/INDEX) || exit 1
set -- $cases
if [ "$#" -ne 1 ] || [ "$1" -lt 1 ]; then
  echo "tls-certs: certs/INDEX is missing or empty; run task tls-certs-slow"
  exit 1
fi
echo "tls-certs: $1 cases, certs/ matches its sources"
