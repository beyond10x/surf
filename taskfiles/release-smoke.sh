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
#    `fetch-failed <id> Unreachable http://127.0.0.1:1/`; `surf https://127.0.0.1:1/` exits 1
#    and prints `fetch-failed <id> Unreachable https://127.0.0.1:1/`, so the scheme is accepted
#    (a binary without `https:` ends `UnsupportedScheme`); `surf --anchors` exits 0 and prints
#    `anchors <n> bundled <date> sha256:<hex>`, the date and sha256 those of
#    crates/tls/src/anchors/SOURCE of the checkout and <n> the count of SOURCE less the roots of
#    NO_ANCHOR in crates/tls/tests/tls/anchors.rs (a root that gives no anchor), so the binary
#    finds its anchors; and `surf --ca-file <root.pem> https://127.0.0.1:<port>/` makes one TLS
#    1.3 handshake with `openssl s_server` serving the committed test certificate
#    crates/tls-test-certs/certs/ok-ecdsa-p256, exits 0 and prints `navigation <id> Committed
#    https://127.0.0.1:<port>/`, so ring's primitives run in the shipped binary. The smoke test
#    needs `openssl` (OpenSSL 3 preferred; on macOS Homebrew's openssl@3, as /usr/bin/openssl
#    is LibreSSL) and fails without one; the port is the first of 48443, 48543, 48643, 48743
#    and 48843 that s_server can listen on.
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

# 2. surf: a usage error, then an unreachable address, the anchors and one TLS 1.3 handshake.
status=0
"$dir/surf" >/dev/null 2>&1 || status=$?
[ "$status" = 2 ] || fail "surf without an argument exited $status, not 2"
id='[0-9a-f][0-9a-f-]*'
status=0
out=$("$dir/surf" http://127.0.0.1:1/ 2>/dev/null) || status=$?
[ "$status" = 1 ] || fail "surf http://127.0.0.1:1/ exited $status, not 1"
line=$(printf '%s\n' "$out" | grep '^fetch-failed ') || line=$out
[ "$(printf '%s\n' "$line" | wc -l | tr -d ' ')" = 1 ] ||
  fail "surf http://127.0.0.1:1/ printed more than one fetch-failed line: $line"
printf '%s\n' "$line" | grep -q "^fetch-failed $id Unreachable http://127.0.0.1:1/\$" ||
  fail "surf http://127.0.0.1:1/ printed no fetch-failed line: $out"
echo "release-smoke: surf exits 2 without an argument and 1 on http://127.0.0.1:1/"

status=0
out=$("$dir/surf" https://127.0.0.1:1/ 2>/dev/null) || status=$?
line=$(printf '%s\n' "$out" | grep '^fetch-failed ') || line=$out
[ "$(printf '%s\n' "$line" | wc -l | tr -d ' ')" = 1 ] ||
  fail "surf https://127.0.0.1:1/ printed more than one fetch-failed line: $line"
[ "$status" = 1 ] || fail "surf https://127.0.0.1:1/ exited $status, not 1: $line"
printf '%s\n' "$line" | grep -q "^fetch-failed $id Unreachable https://127.0.0.1:1/\$" ||
  fail "surf https://127.0.0.1:1/ did not end Unreachable: $line"
echo "release-smoke: surf https://127.0.0.1:1/ exits 1: $line"

source=crates/tls/src/anchors/SOURCE
[ -f "$source" ] || fail "$source is missing: run the script from a checkout of the commit built"
date=$(sed -n 's/^date: //p' "$source")
sha256=$(sed -n 's/^sha256: //p' "$source")
count=$(sed -n 's/^count: //p' "$source")
case "$count" in
  '' | *[!0-9]*) fail "$source has no count that is a number: $count" ;;
esac
[ -n "$date" ] && [ -n "$sha256" ] || fail "$source has no date or no sha256"
status=0
out=$("$dir/surf" --anchors 2>/dev/null) || status=$?
[ "$status" = 0 ] || fail "surf --anchors exited $status, not 0: $out"
n=$(printf '%s\n' "$out" | sed -n "s/^anchors \([1-9][0-9]*\) bundled $date sha256:$sha256\$/\1/p")
[ -n "$n" ] && [ "$out" = "anchors $n bundled $date sha256:$sha256" ] ||
  fail "surf --anchors printed no line anchors <n> bundled $date sha256:$sha256: $out"
anchors_rs=crates/tls/tests/tls/anchors.rs
no_anchor=$(sed -n 's/^const NO_ANCHOR: \[(&str, &str); \([0-9][0-9]*\)\] = .*/\1/p' "$anchors_rs")
case "$no_anchor" in
  '' | *[!0-9]*) fail "$anchors_rs has no NO_ANCHOR count: $no_anchor" ;;
esac
[ "$n" = "$((count - no_anchor))" ] ||
  fail "surf --anchors counts $n anchors, not $((count - no_anchor))" \
    "($count of $source less $no_anchor of NO_ANCHOR): $out"
echo "release-smoke: surf --anchors exits 0: $out ($count of $source less $no_anchor of NO_ANCHOR)"

# One TLS 1.3 handshake of the directory's surf against `openssl s_server`, a TLS implementation
# other than surf's own, with the committed test certificate ok-ecdsa-p256 (127.0.0.1 in its
# subjectAltName) and its root given with --ca-file. The OpenSSL 3 of the usual paths is taken
# first (macOS has LibreSSL at /usr/bin/openssl), else any whose s_server takes -tls1_3 and
# -cert_chain; none is a failure. The key and the server's log live in a directory of mktemp -d;
# the trap kills the server and removes the directory on every exit.
hs_dir=""
hs_server=""
hs_clean() {
  if [ -n "$hs_server" ]; then
    kill "$hs_server" 2>/dev/null || true
    wait "$hs_server" 2>/dev/null || true
    hs_server=""
  fi
  if [ -n "$hs_dir" ]; then
    rm -rf "$hs_dir"
    hs_dir=""
  fi
}
trap hs_clean EXIT
trap 'exit 1' INT TERM HUP
openssl=""
looked=""
for candidate in "$(command -v openssl 2>/dev/null || true)" \
  /opt/homebrew/opt/openssl@3/bin/openssl /usr/local/opt/openssl@3/bin/openssl; do
  [ -n "$candidate" ] && [ -x "$candidate" ] || continue
  looked="$looked $candidate"
  case "$("$candidate" version 2>/dev/null)" in
    "OpenSSL 3"*) openssl=$candidate; break ;;
  esac
done
if [ -z "$openssl" ]; then
  for candidate in $looked; do
    help=$("$candidate" s_server -help 2>&1 || true)
    case "$help" in
      *-tls1_3*) case "$help" in *-cert_chain*) openssl=$candidate; break ;; esac ;;
    esac
  done
fi
[ -n "$openssl" ] ||
  fail "the handshake needs openssl with s_server -tls1_3 and -cert_chain; none found" \
    "(looked at:${looked:- none on PATH, in /opt/homebrew/opt/openssl@3, /usr/local/opt/openssl@3})"
certs=crates/tls-test-certs/certs/ok-ecdsa-p256
hs_dir=$(mktemp -d "${TMPDIR:-/tmp}/surf-smoke.XXXXXX")
"$openssl" pkey -inform DER -in "$certs/leaf.pk8" -out "$hs_dir/leaf.key" 2>"$hs_dir/pkey.err" ||
  fail "$openssl pkey cannot read $certs/leaf.pk8: $(cat "$hs_dir/pkey.err")"
port=""
ports="48443 48543 48643 48743 48843"
for try in $ports; do
  "$openssl" s_server -accept "127.0.0.1:$try" -cert "$certs/chain.pem" \
    -cert_chain "$certs/chain.pem" -key "$hs_dir/leaf.key" -tls1_3 -www \
    >"$hs_dir/server.log" 2>&1 &
  hs_server=$!
  waited=0
  while [ "$waited" -lt 50 ]; do
    if grep -q '^ACCEPT' "$hs_dir/server.log"; then
      port=$try
      break
    fi
    kill -0 "$hs_server" 2>/dev/null || break
    sleep 0.1
    waited=$((waited + 1))
  done
  [ -z "$port" ] || break
  if kill -0 "$hs_server" 2>/dev/null; then
    fail "$openssl s_server on 127.0.0.1:$try printed no ACCEPT within 5 s:" \
      "$(tail -n 3 "$hs_dir/server.log")"
  fi
  wait "$hs_server" 2>/dev/null || true
  hs_server=""
  why=$(grep -m 1 -i 'error' "$hs_dir/server.log" || tail -n 1 "$hs_dir/server.log")
  echo "release-smoke: $openssl s_server cannot listen on 127.0.0.1:$try: $why"
done
[ -n "$port" ] || fail "$openssl s_server could listen on none of the ports $ports of 127.0.0.1"
address=https://127.0.0.1:$port/
status=0
out=$("$dir/surf" --ca-file "$certs/root.pem" "$address" 2>&1) || status=$?
line=$(printf '%s\n' "$out" | grep "^navigation $id Committed $address\$") ||
  fail "surf --ca-file $certs/root.pem $address made no TLS 1.3 handshake with" \
    "$openssl s_server (exit $status): $out"
[ "$status" = 0 ] || fail "surf $address exited $status, not 0: $out"
echo "release-smoke: surf loads $address from $openssl s_server ($("$openssl" version)): $line"
hs_clean
trap - EXIT INT TERM HUP

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
