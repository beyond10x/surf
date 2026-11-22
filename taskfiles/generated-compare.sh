#!/bin/sh
# Compare generated/rust with the tree task generated-fresh wrote: steps 1 and 2 of task
# generated-check without the generation. task generated-compare runs it in the repository; the
# tests of crates/surf/tests/*generated_check*.rs run it with sh in their copies, so no case
# starts task.
#
# usage: sh taskfiles/generated-compare.sh [<fresh tree>]
#
# Run in the directory that holds spec/ and generated/rust. The fresh tree (default
# .out/generated-check/fresh) is what ess generated from spec/, and <fresh tree>.log the output
# ess printed for it. Its own files go to .out/generated-check/compare, which it removes first.
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
# sets pipefail where the shell has it, as the Taskfile does for its tasks.

set -u
if (set -o pipefail) 2> /dev/null; then
  set -o pipefail
fi

fresh=${1:-.out/generated-check/fresh}
dir=.out/generated-check/compare
rm -rf "$dir"
mkdir -p "$dir"

test -d "$fresh" && test -f "$fresh.log" || {
  printf '%s: no generation; run task generated-fresh\n' "$fresh"
  exit 1
}
export LC_ALL=C
(cd "$fresh" && find . -path ./.ess-output -prune -o -type f -print) | sed 's|^\./||' | sort > "$dir/fresh.list"
reported=$(sed -n 's/^\([0-9][0-9]*\) artifact(s), written to .*/\1/p' "$fresh.log")
found=$(wc -l < "$dir/fresh.list")
if [ "$found" -ne "${reported:-0}" ]; then
  printf 'ess reports %s artifact(s) and wrote %s file(s) besides .ess-output; the check would not cover what ess reports\n' "${reported:-no}" "$found"
  exit 1
fi
: > "$dir/drift.txt"
if [ -L generated/rust ]; then
  printf '%s\n' "generated/rust: a symbolic link, not a directory" >> "$dir/drift.txt"
fi
(cd "$fresh" && find . -type f -print) | sed 's|^\./||' | sort > "$dir/fresh.all"
(cd "$fresh" && find . -type f -perm /111 -print) | sed 's|^\./||' | sort > "$dir/fresh.executable"
if [ -d generated/rust ]; then
  (cd generated/rust && find . \( -path ./.ess-output -o -path ./target -o -path ./Cargo.lock \) -prune -o -type f -print) | sed 's|^\./||' | sort > "$dir/committed.list"
  (cd generated/rust && find . \( -path ./target -type d \) -prune -o ! -type f ! -type d -print) | sed 's|^\./||' | sort | sed 's|^|generated/rust/|; s|$|: not a regular file|' >> "$dir/drift.txt"
  (cd generated/rust && find . \( -path ./target -o -path ./Cargo.lock \) -prune -o -type f -print) | sed 's|^\./||' | sort > "$dir/committed.all"
  (cd generated/rust && find . \( -path ./target -o -path ./Cargo.lock \) -prune -o -type f -perm /111 -print) | sed 's|^\./||' | sort > "$dir/committed.executable"
else
  : > "$dir/committed.list"
  : > "$dir/committed.all"
  : > "$dir/committed.executable"
fi
comm -23 "$dir/fresh.list" "$dir/committed.list" | sed 's|^|generated/rust/|; s|$|: missing|' >> "$dir/drift.txt"
comm -13 "$dir/fresh.list" "$dir/committed.list" | sed 's|^|generated/rust/|; s|$|: not written by ess|' >> "$dir/drift.txt"
comm -12 "$dir/fresh.list" "$dir/committed.list" | while read -r path; do
  cmp -s "$fresh/$path" "generated/rust/$path" || printf 'generated/rust/%s: differs from what ess generates\n' "$path"
done >> "$dir/drift.txt"
comm -12 "$dir/fresh.all" "$dir/committed.all" > "$dir/both.all"
comm -13 "$dir/fresh.executable" "$dir/committed.executable" | comm -12 - "$dir/both.all" | sed 's|^|generated/rust/|; s|$|: executable, and ess writes it not executable|' >> "$dir/drift.txt"
comm -23 "$dir/fresh.executable" "$dir/committed.executable" | comm -12 - "$dir/both.all" | sed 's|^|generated/rust/|; s|$|: not executable, and ess writes it executable|' >> "$dir/drift.txt"
if [ -s "$dir/drift.txt" ]; then
  cat "$dir/drift.txt"
  printf '%s path(s) differ from spec/; run task generate\n' "$(wc -l < "$dir/drift.txt" | tr -d ' ')"
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
printf 'generated/rust matches spec/ (%s artifact(s) and .ess-output)\n' "$(wc -l < "$dir/fresh.list" | tr -d ' ')"
