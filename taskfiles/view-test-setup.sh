#!/bin/sh
# Installs the page tests' tooling, view-tests/node_modules, only when it is not the install of
# view-tests/package.json and view-tests/package-lock.json: task view-test-setup, which task
# view-test runs before its timed part, and the forge's check before task check.
#
# usage: sh taskfiles/view-test-setup.sh [digest]   (from the repository's root)
#
# With digest it prints the digest below and does nothing else.
#
# The digest is the sha256 of one line per file, "<sha256 of its bytes>  <name>", for
# view-tests/package.json and view-tests/package-lock.json: a package.json that left its lock
# makes npm ci fail here as on the forge. The install keeps the digest in
# view-tests/node_modules/.surf-lock.sha256. When the two are equal and every package the lock
# names, each key "node_modules/<name>" of its "packages" (read with jq, as task forge-watch-test
# reads JSON), has its directory under view-tests/, it prints "view-tests: node_modules matches
# package-lock.json" and runs nothing else. Otherwise it runs npm --prefix view-tests ci
# --prefer-offline --no-audit --no-fund (npm ci removes node_modules first, the stored digest
# with it), then writes the digest and prints "view-tests: node_modules installed from
# package-lock.json". It exits 1 when npm fails, 2 with one line on standard error when a file
# is missing or the lock cannot be read. POSIX sh: the forge's sh is dash.

set -u

package=view-tests/package.json
lock=view-tests/package-lock.json
stored=view-tests/node_modules/.surf-lock.sha256

fail() {
  printf 'view-test-setup: %s\n' "$1" >&2
  exit 2
}

hash() {
  sha256sum < "$1" | awk '{ print $1 }'
}

for file in "$package" "$lock"; do
  [ -f "$file" ] || fail "$file: no such file"
done
digest=$(printf '%s  %s\n%s  %s\n' "$(hash "$package")" "$package" "$(hash "$lock")" "$lock" | sha256sum)
digest=${digest%% *}
if [ "${1:-}" = digest ]; then
  printf '%s\n' "$digest"
  exit 0
fi
keys=$(jq -r '.packages | keys[] | select(startswith("node_modules/"))' "$lock") ||
  fail "$lock: its packages could not be read"

installed() {
  [ -f "$stored" ] && [ "$(cat "$stored")" = "$digest" ] || return 1
  missing=$(printf '%s\n' "$keys" | while IFS= read -r key; do
    [ -z "$key" ] || [ -d "view-tests/$key" ] || printf '%s\n' "$key"
  done)
  [ -z "$missing" ]
}

if installed; then
  printf '%s\n' 'view-tests: node_modules matches package-lock.json'
  exit 0
fi

npm --prefix view-tests ci --prefer-offline --no-audit --no-fund || exit 1
printf '%s\n' "$digest" > "$stored"
printf '%s\n' 'view-tests: node_modules installed from package-lock.json'
