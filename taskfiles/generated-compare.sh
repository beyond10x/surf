#!/bin/sh
# Compare generated/rust with the tree task generated-fresh wrote: steps 1 and 2 of task
# generated-check without the generation. task generated-compare runs it in the repository; the
# tests of crates/surf/tests/*generated_check*.rs run it with sh in their copies, so no case
# starts task.
#
# usage: sh taskfiles/generated-compare.sh [<fresh tree>]
#        sh taskfiles/generated-compare.sh --lists <fresh tree>
#
# Run in the directory that holds spec/ and generated/rust. The fresh tree (default
# .out/generated-check/fresh) is what ess generated from spec/, and <fresh tree>.log the output
# ess printed for it. Its own files go to .out/generated-check/compare, which it removes first.
#
# The listings of the fresh tree: <fresh tree>.list (every file but those under .ess-output),
# <fresh tree>.all (every file) and <fresh tree>.executable (every file with an executable bit),
# paths relative to the tree, sorted. --lists writes the three beside the tree and nothing else;
# task generated-fresh runs it once after ess wrote the tree, so the many runs of the tests read
# them and do not list the same tree again. A comparison uses them only when none is older than
# the tree, any file or directory in it, or its log: a file written or removed in the tree after
# the listings changes the time of the file or of its directory. Else it refuses them, naming
# the listing and task generated-fresh, and exits 1. When the three are not all beside the tree,
# it lists the tree itself into its own directory, never beside the tree.
#
# - Step 1: every file ess wrote, .ess-output left out, against generated/rust: a file missing,
#   one ess does not write, one whose bytes differ, an entry that is not a regular file, and an
#   executable bit other than the one ess writes, each one line "generated/rust/<path>: <what>";
#   then the count, and exit 1.
# - Step 2, when step 1 passed: ess over a copy of the committed generated/rust with its
#   .ess-output; exit 1 when ess refuses or when the regeneration rewrites .ess-output.
#
# ess is the one of PATH, run as task generate runs it (ESS_TOOLCHAIN chooses its release).
# Values are printed with printf: the forge's sh is dash, whose echo expands backslashes. It
# sets pipefail where the shell has it, as the Taskfile does for its tasks. find -printf and
# diff --no-dereference are GNU, as find -perm /111 is.

set -u
if (set -o pipefail) 2> /dev/null; then
  set -o pipefail
fi
export LC_ALL=C
tab=$(printf '\t')

# The three listings of the tree $1 as $2.list, $2.all and $2.executable: one find, one sort,
# one awk. A line of find is <path> TAB <mode>; sort orders on the path alone.
fresh_lists() {
  : > "$2.list"
  : > "$2.all"
  : > "$2.executable"
  find "$1/." -type f -printf "%P$tab%m\n" | sort -t "$tab" -k 1,1 | OUT=$2 awk -F "$tab" '
    {
      mode = $NF
      path = substr($0, 1, length($0) - length(mode) - 1)
      print path > (ENVIRON["OUT"] ".all")
      if (path != ".ess-output" && substr(path, 1, 12) != ".ess-output/") print path > (ENVIRON["OUT"] ".list")
      bits = substr(mode, length(mode) - 2)
      if (substr(bits, 1, 1) % 2 || substr(bits, 2, 1) % 2 || substr(bits, 3, 1) % 2)
        print path > (ENVIRON["OUT"] ".executable")
    }'
}

if [ "${1:-}" = --lists ]; then
  fresh=${2:?usage: generated-compare.sh --lists <fresh tree>}
  test -d "$fresh" || {
    printf '%s: no generation; run task generated-fresh\n' "$fresh"
    exit 1
  }
  fresh_lists "$fresh" "$fresh"
  exit
fi

fresh=${1:-.out/generated-check/fresh}
dir=.out/generated-check/compare
rm -rf "$dir"
mkdir -p "$dir"

test -d "$fresh" && test -f "$fresh.log" || {
  printf '%s: no generation; run task generated-fresh\n' "$fresh"
  exit 1
}
if test -f "$fresh.list" && test -f "$fresh.all" && test -f "$fresh.executable"; then
  stale=$(find "$fresh" "$fresh.log" \
    \( -newer "$fresh.list" -printf 'list %p\n' -quit \) -o \
    \( -newer "$fresh.all" -printf 'all %p\n' -quit \) -o \
    \( -newer "$fresh.executable" -printf 'executable %p\n' -quit \))
  if [ -n "$stale" ]; then
    printf '%s.%s: older than %s; run task generated-fresh\n' "$fresh" "${stale%% *}" "${stale#* }"
    exit 1
  fi
  lists=$fresh
else
  fresh_lists "$fresh" "$dir/fresh"
  lists=$dir/fresh
fi
reported=$(sed -n 's/^\([0-9][0-9]*\) artifact(s), written to .*/\1/p' "$fresh.log")
found=$(wc -l < "$lists.list")
if [ "$found" -ne "${reported:-0}" ]; then
  printf 'ess reports %s artifact(s) and wrote %s file(s) besides .ess-output; the check would not cover what ess reports\n' "${reported:-no}" "$found"
  exit 1
fi
: > "$dir/drift.txt"
if [ -L generated/rust ]; then
  printf '%s\n' "generated/rust: a symbolic link, not a directory" >> "$dir/drift.txt"
fi
: > "$dir/committed.list"
: > "$dir/committed.all"
: > "$dir/committed.executable"
if [ -d generated/rust ]; then
  # One find of the committed tree, the contents of a directory target left out: its regular
  # files into the three listings (target and Cargo.lock left out, and .ess-output from
  # committed.list), every other entry but a directory as "not a regular file".
  find generated/rust/. \( -path generated/rust/./target -type d \) -prune -o ! -type d -printf "%P$tab%y$tab%m\n" |
    sort -t "$tab" -k 1,1 | DIR=$dir awk -F "$tab" '
    {
      mode = $NF
      type = $(NF - 1)
      path = substr($0, 1, length($0) - length(mode) - length(type) - 2)
      top = path
      sub(/\/.*$/, "", top)
      if (type != "f") {
        print "generated/rust/" path ": not a regular file" >> (ENVIRON["DIR"] "/drift.txt")
        next
      }
      if (top == "target" || top == "Cargo.lock") next
      if (top != ".ess-output") print path > (ENVIRON["DIR"] "/committed.list")
      print path > (ENVIRON["DIR"] "/committed.all")
      bits = substr(mode, length(mode) - 2)
      if (substr(bits, 1, 1) % 2 || substr(bits, 2, 1) % 2 || substr(bits, 3, 1) % 2)
        print path > (ENVIRON["DIR"] "/committed.executable")
    }'
fi
comm -23 "$lists.list" "$dir/committed.list" | sed 's|^|generated/rust/|; s|$|: missing|' >> "$dir/drift.txt"
comm -13 "$lists.list" "$dir/committed.list" | sed 's|^|generated/rust/|; s|$|: not written by ess|' >> "$dir/drift.txt"
if [ -d generated/rust ]; then
  # The bytes of every path in both listings: one diff over the two trees. A path it reports as
  # differing, or names in an error line (a file it cannot read, or every file below a directory
  # it cannot read), differs; the lines come in the order of the fresh listing.
  diff -rq --no-dereference "$fresh" generated/rust/. 2>&1 |
    FRESH=$fresh LIST=$lists.list COMMITTED=$dir/committed.list awk '
    BEGIN {
      fp = ENVIRON["FRESH"]
      if (substr(fp, length(fp)) != "/") fp = fp "/"
      cp = "generated/rust/./"
      middle = " and " cp
      errors = 0
    }
    substr($0, 1, 6) == "Files " && substr($0, length($0) - 6) == " differ" {
      if (substr($0, 7, length(fp)) != fp) next
      rest = substr($0, 7 + length(fp))
      n = (length(rest) - length(middle) - 7) / 2
      differs[substr(rest, 1, n)] = 1
      next
    }
    substr($0, 1, 6) == "diff: " {
      rest = substr($0, 7)
      sub(/: [^:]*$/, "", rest)
      if (rest == ENVIRON["FRESH"] || rest == "generated/rust/." || rest == "generated/rust") rest = ""
      else if (substr(rest, 1, length(fp)) == fp) rest = substr(rest, length(fp) + 1)
      else if (substr(rest, 1, length(cp)) == cp) rest = substr(rest, length(cp) + 1)
      else next
      unread[++errors] = rest
    }
    END {
      while ((getline path < ENVIRON["COMMITTED"]) > 0) both[path] = 1
      while ((getline path < ENVIRON["LIST"]) > 0) {
        if (!(path in both)) continue
        hit = (path in differs)
        for (k = 1; !hit && k <= errors; k++)
          if (unread[k] == "" || path == unread[k] || substr(path, 1, length(unread[k]) + 1) == unread[k] "/") hit = 1
        if (hit) printf "generated/rust/%s: differs from what ess generates\n", path
      }
    }' >> "$dir/drift.txt"
fi
comm -12 "$lists.all" "$dir/committed.all" > "$dir/both.all"
comm -13 "$lists.executable" "$dir/committed.executable" | comm -12 - "$dir/both.all" | sed 's|^|generated/rust/|; s|$|: executable, and ess writes it not executable|' >> "$dir/drift.txt"
comm -23 "$lists.executable" "$dir/committed.executable" | comm -12 - "$dir/both.all" | sed 's|^|generated/rust/|; s|$|: not executable, and ess writes it executable|' >> "$dir/drift.txt"
if [ -s "$dir/drift.txt" ]; then
  awk '{ print } END { printf "%d path(s) differ from spec/; run task generate\n", NR }' "$dir/drift.txt"
  exit 1
fi

test -f generated/rust/.ess-output/state.json || {
  printf '%s\n' "generated/rust/.ess-output/state.json: missing; restore it from git, or remove generated/rust and run task generate"
  exit 1
}
mkdir -p "$dir/in-place"
{ printf '%s\n' .ess-output; cat "$dir/committed.list"; } > "$dir/copy.list"
tar -cf - -C generated/rust -T "$dir/copy.list" | tar -xf - -C "$dir/in-place"
ess generate synthesize --strict-requires --path spec --target rust --out "$dir/in-place" > "$dir/in-place.log" 2>&1 || {
  cat "$dir/in-place.log"
  printf '%s\n' "generated/rust/.ess-output/state.json: ess refuses to regenerate over this record; restore it from git, or remove generated/rust and run task generate"
  exit 1
}
diff -rq generated/rust/.ess-output "$dir/in-place/.ess-output" || {
  printf '%s\n' "generated/rust/.ess-output: regeneration rewrites it; run task generate"
  exit 1
}
printf 'generated/rust matches spec/ (%s artifact(s) and .ess-output)\n' "$(wc -l < "$lists.list" | tr -d ' ')"
