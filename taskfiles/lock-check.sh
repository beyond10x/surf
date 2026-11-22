#!/bin/sh
# The lock check: nothing that ships uses a third-party crate but the packages of ring, the one
# source of cryptographic primitives (README.md, principle 4), and only surf-tls depends on them.
#
# Usage: sh taskfiles/lock-check.sh <lock> <allow-list>
#
# The allow-list holds one line per package from outside this repository that the lock may
# hold: `<name> <version> <source> <checksum>`, four fields separated by one space, as cargo
# writes them into the lock; sorted, no comment lines, no empty lines.
#
# cargo writes a `source` line for every package it takes from a registry or a git repository,
# and none for a crate it reaches by path. The check fails, one line per finding, when:
# - a package with a `source` line matches no line of the allow-list on name, version, source
#   and checksum: "<lock>: <name> <version> from <source>: not on the allow-list"; when the list
#   holds that name with another version, source or checksum, the line goes on with the
#   package's checksum and the listed line;
# - a package without a `source` line (a crate of this repository) other than surf-tls lists an
#   allowed package among its dependencies: "<lock>: <package> depends on <allowed>; only
#   surf-tls may". An allowed package may depend on another allowed package;
# - a line of the allow-list matches no package of the lock: "<allow-list>: <name> <version> is
#   in no package of <lock>";
# - a line of the allow-list is not four fields, or the lock or the list is missing.
# Else it prints "<lock>: <n> package(s), <m> from outside this repository, each on the
# allow-list", or "<lock>: <n> package(s), none from outside this repository" when <m> is 0.
#
# A path dependency on a directory outside this repository has no source line and is not
# caught. conformance/ has a lock of its own, which this does not read.

set -u

lock=${1:?usage: lock-check.sh <lock> <allow-list>}
allow=${2:?usage: lock-check.sh <lock> <allow-list>}

test -f "$lock" || { echo "$lock: missing; the root workspace's lock is committed"; exit 1; }
test -f "$allow" || { echo "$allow: missing; it lists the packages from outside this repository the lock may hold"; exit 1; }

exec awk -v lock="$lock" -v allow="$allow" '
  function unquote(s) { gsub(/"/, "", s); return s }
  function finish() {
    if (!inpkg) return
    inpkg = 0
    packages++
    pname[packages] = name; pversion[packages] = version
    psource[packages] = source; pchecksum[packages] = checksum
    pdeps[packages] = deps
  }
  FILENAME == ARGV[1] {
    if (NF != 4 || $0 != $1 " " $2 " " $3 " " $4) {
      print allow ": line " FNR " is not \"<name> <version> <source> <checksum>\": " $0
      bad++
      next
    }
    listed++
    key[listed] = $0; lname[listed] = $1; lversion[listed] = $2
    allowed[$1] = 1
    entry[$0] = listed
    next
  }
  /^\[\[package\]\]$/ { finish(); inpkg = 1; name = ""; version = ""; source = ""; checksum = ""; deps = ""; indeps = 0; next }
  /^\[/ { finish(); indeps = 0; next }
  !inpkg { next }
  indeps && /^\]/ { indeps = 0; next }
  indeps { d = $1; gsub(/[",]/, "", d); deps = deps " " d; next }
  /^name = / { name = unquote($3); next }
  /^version = / { version = unquote($3); next }
  /^source = / { source = unquote($3); next }
  /^checksum = / { checksum = unquote($3); next }
  /^dependencies = \[\]$/ { next }
  /^dependencies = \[$/ { indeps = 1; next }
  END {
    finish()
    for (p = 1; p <= packages; p++) {
      if (psource[p] != "") {
        outside++
        line = pname[p] " " pversion[p] " " psource[p] " " pchecksum[p]
        if (line in entry) { matched[entry[line]] = 1; continue }
        out = lock ": " pname[p] " " pversion[p] " from " psource[p] ": not on the allow-list"
        if (pname[p] in allowed) {
          out = out "; its checksum " (pchecksum[p] == "" ? "none" : pchecksum[p])
          for (l = 1; l <= listed; l++) if (lname[l] == pname[p]) out = out ", the list holds " key[l]
        }
        print out
        bad++
        continue
      }
      if (pname[p] == "surf-tls") continue
      n = split(pdeps[p], ds, " ")
      for (i = 1; i <= n; i++) if (ds[i] in allowed) {
        print lock ": " pname[p] " depends on " ds[i] "; only surf-tls may"
        bad++
      }
    }
    for (l = 1; l <= listed; l++) if (!(l in matched)) {
      print allow ": " lname[l] " " lversion[l] " is in no package of " lock
      bad++
    }
    if (bad) {
      print bad " finding(s); a package from outside this repository must be on " allow ", and only surf-tls may depend on one"
      exit 1
    }
    if (outside) print lock ": " packages " package(s), " outside " from outside this repository, each on the allow-list"
    else print lock ": " packages " package(s), none from outside this repository"
  }
' "$allow" "$lock"
