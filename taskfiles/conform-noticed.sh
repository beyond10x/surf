#!/bin/sh
# The list of scenarios each planted defect of the conformance crate runs over,
# conformance/tests/noticed.txt, and the digest of its sources.
#
# usage: conform-noticed.sh digest
#        conform-noticed.sh check [list]
#        conform-noticed.sh assemble <directory>
#        conform-noticed.sh counts <directory>
#
# Run from the repository's root. The sources, read in this order: .out/suite.json (task suite
# writes it); every file under conformance/src and conformance/tests but
# conformance/tests/noticed.txt itself, a list read from the directories and sorted, so a new
# file counts; conformance/Cargo.toml; conformance/Cargo.lock, whole. The digest is the sha256
# of one line per source, "<sha256 of its bytes>  <name>". Not covered: the surf crates the
# crate is built against (crates/, generated/), and the list's own lines after its first; a
# planted-defect test whose listed scenarios all pass says the list may be stale.
#
# digest prints it. check compares it with the first line of the list (default
# conformance/tests/noticed.txt), "sources <digest>", prints "conform-noticed: <list> matches
# its sources" and exits 0, or prints "conform-rust: the sources of
# conformance/tests/noticed.txt changed; run task conform-slow" and exits 1. assemble prints a
# new list: the first line, then the lines the planted-defect tests wrote into <directory>, one
# file <name>.line each, sorted. counts prints the count lines they wrote there, one file
# <name>.count each, sorted. It exits 2 with one line on standard error on a missing source or
# a bad argument. POSIX sh: the forge's sh is dash.

set -u
if (set -o pipefail) 2> /dev/null; then
  set -o pipefail
fi

list_file=conformance/tests/noticed.txt

fail() {
  printf 'conform-noticed: %s\n' "$1" >&2
  exit 2
}

hash() {
  sha256sum | awk '{ print $1 }'
}

# Every source, one name per line, in the order the digest reads them.
sources() {
  printf '%s\n' .out/suite.json
  find conformance/src conformance/tests -path "$list_file" -prune -o -type f -print | LC_ALL=C sort
  printf '%s\n' conformance/Cargo.toml conformance/Cargo.lock
}

digest() {
  for dir in conformance/src conformance/tests; do
    [ -d "$dir" ] || fail "$dir: no such directory"
  done
  names=$(sources) || fail 'the sources could not be listed'
  # Checked before the pipeline: dash has no pipefail, so a failure inside it would be lost.
  missing=$(printf '%s\n' "$names" | while IFS= read -r file; do
    [ -f "$file" ] || printf '%s\n' "$file"
  done)
  [ -z "$missing" ] || fail "$(printf '%s' "$missing" | head -n 1): no such file"
  printf '%s\n' "$names" | while IFS= read -r file; do
    printf '%s  %s\n' "$(hash < "$file")" "$file"
  done | hash
}

# The files <directory>/*.<suffix>, sorted; fails when there is none.
written() {
  dir=$1
  suffix=$2
  [ -n "$dir" ] && [ -d "$dir" ] || fail "${dir:-<directory>}: no such directory"
  set -- "$dir"/*."$suffix"
  [ -f "$1" ] || fail "$dir: no .$suffix file written"
  cat "$@" | LC_ALL=C sort
}

case ${1:-} in
  digest)
    digest
    ;;
  check)
    list=${2:-$list_file}
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
    sources=$(digest) || exit 2
    lines=$(written "${2:-}" line) || exit 2
    printf 'sources %s\n' "$sources"
    printf '%s\n' "$lines"
    ;;
  counts)
    written "${2:-}" count
    ;;
  *)
    fail "usage: conform-noticed.sh digest | check [list] | assemble <directory> | counts <directory>"
    ;;
esac
