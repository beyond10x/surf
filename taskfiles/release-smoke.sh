#!/bin/sh
# The smoke test of one unpacked release archive (docs/releasing.md). The release workflow runs
# it on every target, and it runs unchanged on a workstation against binaries built with the
# workflow's flags:
#
#   sh taskfiles/release-smoke.sh <dir> [<target>]
#
# <dir> is the unpacked directory `surf-<version>-<target>/`. <target> is the cargo target the
# end-to-end case is built for; the host's when it is absent. cargo runs from the repository root
# with the caller's RUSTFLAGS, so the case reuses what the build of the binaries compiled.
#
# On Linux the directory holds `surf`, `surf-agent`, `surfd`, `surf-session`, `README.md`,
# `LICENSE` and `NOTICE`, and nothing else. On another system it holds `surf` and the three files:
# the agent binaries read the caller's user id from /proc/self and need XDG_RUNTIME_DIR, so they
# run on Linux only.
#
# 1. The file list of the directory, and every binary in it executable.
# 2. `surf` without an argument exits 2; `surf http://127.0.0.1:1/` exits 1 and prints
#    `fetch-failed <id> Unreachable http://127.0.0.1:1/`.
# 3. Linux only: the end-to-end case of the session process (crates/session/tests/e2e.rs) on the
#    directory's `surf-agent`, `surfd` and `surf-session`, in a private XDG_RUNTIME_DIR; then no
#    process whose arguments start with the directory may still run.

set -eu

fail() {
  echo "release-smoke: $*" >&2
  exit 1
}

[ "$#" -ge 1 ] && [ "$#" -le 2 ] || fail "usage: sh taskfiles/release-smoke.sh <dir> [<target>]"
[ -d "$1" ] || fail "$1 is not a directory"
dir=$(cd "$1" && pwd -P)
target=${2:-}
cd "$(dirname "$0")/.."

case "$(uname -s)" in
  Linux)
    binaries="surf surf-agent surf-session surfd"
    agent=yes
    ;;
  *)
    binaries="surf"
    agent=no
    ;;
esac

# 1. What the directory holds.
expected=$(printf '%s\n' LICENSE NOTICE README.md $binaries | LC_ALL=C sort)
found=$(cd "$dir" && LC_ALL=C ls -A)
if [ "$found" != "$expected" ]; then
  echo "release-smoke: $dir holds:" >&2
  printf '%s\n' "$found" >&2
  echo "release-smoke: and should hold exactly:" >&2
  printf '%s\n' "$expected" >&2
  exit 1
fi
for binary in $binaries; do
  [ -f "$dir/$binary" ] && [ -x "$dir/$binary" ] || fail "$dir/$binary is not an executable file"
done
echo "release-smoke: $dir holds" $(printf '%s\n' "$found")

# 2. surf: a usage error, then an unreachable address.
status=0
"$dir/surf" >/dev/null 2>&1 || status=$?
[ "$status" = 2 ] || fail "surf without an argument exited $status, not 2"
status=0
out=$("$dir/surf" http://127.0.0.1:1/ 2>/dev/null) || status=$?
[ "$status" = 1 ] || fail "surf http://127.0.0.1:1/ exited $status, not 1"
printf '%s\n' "$out" | grep -q '^fetch-failed [0-9a-f-]* Unreachable http://127.0.0.1:1/$' ||
  fail "surf http://127.0.0.1:1/ printed no fetch-failed line: $out"
echo "release-smoke: surf exits 2 without an argument and 1 on http://127.0.0.1:1/"

[ "$agent" = yes ] || {
  echo "release-smoke: ok, surf only on $(uname -s)"
  exit 0
}

# 3. The agent binaries, end to end. A private XDG_RUNTIME_DIR as .github/workflows/check.yml
# exports it: the caller's own when it is an absolute directory the caller owns, otherwise a new
# one of mode 0700 under TMPDIR or /tmp, removed at the end. Its path must leave room for the
# case's socket paths (107 bytes at most); the case says so when it does not.
made=""
case "${XDG_RUNTIME_DIR:-}" in
  /*) [ -d "$XDG_RUNTIME_DIR" ] && [ -O "$XDG_RUNTIME_DIR" ] || XDG_RUNTIME_DIR="" ;;
  *) XDG_RUNTIME_DIR="" ;;
esac
if [ -z "$XDG_RUNTIME_DIR" ]; then
  made=$(mktemp -d "${TMPDIR:-/tmp}/xdg.XXXXXX")
  chmod 0700 "$made"
  XDG_RUNTIME_DIR=$made
  trap 'rm -rf "$made"' EXIT
fi
export XDG_RUNTIME_DIR

set -- --locked --release -p surf-session --test main e2e::
[ -z "$target" ] || set -- "$@" --target "$target"
SURF_BIN_DIR=$dir cargo test "$@"

left=$(ps -eo args= | while IFS= read -r args; do
  case "$args" in
    "$dir"/*) printf '%s\n' "$args" ;;
  esac
done)
if [ -n "$left" ]; then
  echo "release-smoke: processes from $dir still run:" >&2
  printf '%s\n' "$left" >&2
  exit 1
fi
echo "release-smoke: no process from $dir runs"
echo "release-smoke: ok, surf, surf-agent, surfd and surf-session on Linux"
