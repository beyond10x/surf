#!/bin/sh
# Fail when a doc comment outside crates/ports holds a code block that rustdoc would run, or a doc
# attribute: task test runs the doc tests of surf-ports only, so a doc test elsewhere never runs.
#
# Usage: sh taskfiles/doc-tests-check.sh [<root>]   (default: .)
#
# - It reads every *.rs under <root>/crates and <root>/generated but those under crates/ports/;
#   a directory named target and one whose name starts with a dot are not entered. A root that
#   holds neither crates/ nor generated/ exits 2.
# - In the /// and //! lines of a file (//// is no doc comment) it follows the fences: a line
#   whose text after the comment mark and blanks starts with three backticks or three tildes
#   opens a block when none is open and closes the open one otherwise. An opening fence whose
#   mark, the rest of the line without blanks around it, is not text fails.
# - A line that holds #[doc = or #![doc = (blanks around doc and = allowed) fails.
# - Not seen: a code block written by indentation, one in a block doc comment (/** or /*!), and
#   a doc attribute written through cfg_attr or a macro.
#
# It prints one line per failing line, "<file>:<line>" relative to <root>, and exits 1; else it
# prints "doc-tests-check: no doc test outside crates/ports". It calls no cargo.

set -u

[ "$#" -le 1 ] || {
  echo 'usage: sh taskfiles/doc-tests-check.sh [<root>]' >&2
  exit 2
}
root=${1:-.}
cd "$root" || exit 2

dirs=
for dir in crates generated; do
  if [ -d "$dir" ]; then dirs="$dirs $dir"; fi
done
[ -n "$dirs" ] || {
  printf '%s\n' "doc-tests-check: $root holds neither crates/ nor generated/"
  exit 2
}

status=0
# shellcheck disable=SC2086
found=$(find $dirs \( -name target -o -name '.*' -o -path crates/ports \) -prune \
  -o -type f -name '*.rs' -exec awk '
  FNR == 1 { open = 0 }
  /#!?\[[ \t]*doc[ \t]*=/ {
    printf "doc-tests-check: %s:%d sets a doc attribute; the gate runs the doc tests of surf-ports only\n", FILENAME, FNR
    bad = 1
  }
  {
    text = $0
    sub(/^[ \t]+/, "", text)
    if (text !~ /^\/\/[\/!]/ || text ~ /^\/\/\/\//) next
    text = substr(text, 4)
    sub(/^[ \t]+/, "", text)
    if (text !~ /^(```|~~~)/) next
    if (open) { open = 0; next }
    open = 1
    mark = text
    sub(/^[`~]+/, "", mark)
    gsub(/^[ \t]+|[ \t]+$/, "", mark)
    if (mark != "text") {
      printf "doc-tests-check: %s:%d opens a code block that rustdoc would run; mark it text or move the doc test to crates/ports\n", FILENAME, FNR
      bad = 1
    }
  }
  END { exit bad }
' {} +) || status=$?

if [ -n "$found" ]; then
  printf '%s\n' "$found" | sort -t: -k1,1 -k2,2n
fi
if [ "$status" -ne 0 ]; then
  exit 1
fi
echo 'doc-tests-check: no doc test outside crates/ports'
