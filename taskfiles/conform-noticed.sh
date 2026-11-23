#!/bin/sh
# The list of scenarios each planted defect of the conformance crate runs over,
# conformance/tests/noticed.txt, and the digest of its sources.
#
# usage: conform-noticed.sh digest
#        conform-noticed.sh check [list]
#        conform-noticed.sh assemble <directory>
#
# Run from the repository's root. The sources, read in this order: .out/suite.json (task suite
# writes it); conformance/tests/planted_defect.rs, review2_planted_defects.rs and
# two_of_each_entity.rs, the planted defects; conformance/src/target.rs and
# conformance/src/values.rs, the adapter; the [[package]] entry of ess-conformance in
# conformance/Cargo.lock, the runner. The digest is the sha256 of one line per source,
# "<sha256 of its bytes>  <name>".
#
# digest prints it. check compares it with the first line of the list (default
# conformance/tests/noticed.txt), "sources <digest>", prints "conform-noticed: <list> matches
# its sources" and exits 0, or prints "conform-rust: the sources of
# conformance/tests/noticed.txt changed; run task conform-slow" and exits 1. assemble prints a
# new list: the first line, then the lines the planted-defect tests wrote into <directory>, one
# file each, sorted. It exits 2 with one line on standard error on a missing source or a bad
# argument. POSIX sh: the forge's sh is dash.

set -u
if (set -o pipefail) 2> /dev/null; then
  set -o pipefail
fi

lock=conformance/Cargo.lock
files='.out/suite.json conformance/tests/planted_defect.rs conformance/tests/review2_planted_defects.rs conformance/tests/two_of_each_entity.rs conformance/src/target.rs conformance/src/values.rs'

fail() {
  printf 'conform-noticed: %s\n' "$1" >&2
  exit 2
}

hash() {
  sha256sum | awk '{ print $1 }'
}

# The [[package]] entry of ess-conformance in the lock, from its [[package]] line to the line
# before the next blank one.
lock_entry() {
  awk '
    $0 == "[[package]]" { if (found) exit; head = $0; next }
    head != "" && $0 == "name = \"ess-conformance\"" { found = 1; print head }
    $0 == "" { if (found) exit; head = ""; next }
    found { print }
  ' "$lock"
}

digest() {
  for file in $files; do
    [ -f "$file" ] || fail "$file: no such file"
  done
  [ -f "$lock" ] || fail "$lock: no such file"
  entry=$(lock_entry)
  [ -n "$entry" ] || fail "$lock: no [[package]] entry of ess-conformance"
  {
    for file in $files; do
      printf '%s  %s\n' "$(hash < "$file")" "$file"
    done
    printf '%s  %s\n' "$(printf '%s\n' "$entry" | hash)" "$lock ess-conformance"
  } | hash
}

case ${1:-} in
  digest)
    digest
    ;;
  check)
    list=${2:-conformance/tests/noticed.txt}
    [ -f "$list" ] || fail "$list: no such file"
    sources=$(digest) || exit 2
    if [ "$(sed -n 1p "$list")" = "sources $sources" ]; then
      printf 'conform-noticed: %s matches its sources\n' "$list"
      exit 0
    fi
    printf '%s\n' 'conform-rust: the sources of conformance/tests/noticed.txt changed; run task conform-slow'
    exit 1
    ;;
  assemble)
    dir=${2:-}
    [ -n "$dir" ] && [ -d "$dir" ] || fail "${dir:-<directory>}: no such directory"
    sources=$(digest) || exit 2
    set -- "$dir"/*
    [ -f "$1" ] || fail "$dir: no line written"
    printf 'sources %s\n' "$sources"
    cat "$@" | LC_ALL=C sort
    ;;
  *)
    fail "usage: conform-noticed.sh digest | check [list] | assemble <directory>"
    ;;
esac
