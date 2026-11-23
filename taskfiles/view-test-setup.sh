#!/bin/sh
# Installs the page tests' tooling, view-tests/node_modules, only when it is not the install of
# view-tests/package-lock.json: task view-test-setup, which task view-test runs before its timed
# part, and the forge's check before task check.
#
# usage: sh taskfiles/view-test-setup.sh   (from the repository's root)
#
# The digest is the sha256 of view-tests/package-lock.json; the install keeps it in
# view-tests/node_modules/.surf-lock.sha256. When the two are equal it prints "view-tests:
# node_modules matches package-lock.json" and runs nothing else. Otherwise it runs npm
# --prefix view-tests ci --prefer-offline --no-audit --no-fund (npm ci removes node_modules
# first, the stored digest with it), then writes the digest and prints "view-tests:
# node_modules installed from package-lock.json". It exits 1 when npm fails, 2 with one line on
# standard error when the lock is missing. POSIX sh: the forge's sh is dash.

set -u

lock=view-tests/package-lock.json
stored=view-tests/node_modules/.surf-lock.sha256

if [ ! -f "$lock" ]; then
  printf 'view-test-setup: %s: no such file\n' "$lock" >&2
  exit 2
fi
digest=$(sha256sum < "$lock") || exit 2
digest=${digest%% *}

if [ -f "$stored" ] && [ "$(cat "$stored")" = "$digest" ]; then
  printf '%s\n' 'view-tests: node_modules matches package-lock.json'
  exit 0
fi

npm --prefix view-tests ci --prefer-offline --no-audit --no-fund || exit 1
printf '%s\n' "$digest" > "$stored"
printf '%s\n' 'view-tests: node_modules installed from package-lock.json'
