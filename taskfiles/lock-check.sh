#!/bin/sh
# The lock check: nothing that ships uses a third-party crate but the packages of ring, the one
# source of cryptographic primitives, and those of the script engine swell, one crate of the
# organization taken by a git dependency on a release tag (README.md, principle 4); only the
# crates the owners file names depend on them.
#
# Usage: sh taskfiles/lock-check.sh <lock> <allow-list> <owners> <manifest>
#
# The allow-list holds one line per package from outside this repository that the lock may
# hold: `<name> <version> <source> <checksum>`, four fields separated by one space, as cargo
# writes them into the lock, with `none` for the checksum of a package that has none (cargo
# writes no `checksum` line for a package from git); sorted, no comment lines, no empty lines.
#
# The owners file holds one line `<package> <crate>` per crate of this repository that may name
# the package among its dependencies: a package of the allow-list, or surf-script, the one
# crate that runs swell, which no crate may depend on without a line of its own
# (docs/design/scripts.md, Q14 and Q15). Two fields separated by one space; sorted, no comment
# lines, no empty lines.
#
# The manifest is the root Cargo.toml: cargo reads profiles from the root workspace's manifest
# only, so a dependency's own `overflow-checks = true` is not read, and each package from git
# needs a table `[profile.release.package.<name>]` there holding `overflow-checks = true` (the
# dev and test profiles check overflow by cargo's default).
#
# The lock is accepted only in the form cargo writes it. cargo reads the lock as TOML and, under
# --locked, compares values, so it accepts the same lock in other spellings (keys without
# spaces, a comment after a header, a dependency array on one line, literal strings); this
# check refuses every such spelling. Each line of the lock must be one cargo writes: the two
# header comment lines, `version = <n>`, an empty line, `[[package]]`, `name = "…"`,
# `version = "…"`, `source = "…"`, `checksum = "<64 hex digits>"`, `dependencies = [`, an entry
# ` "<name>",`, ` "<name> <version>",` or ` "<name> <version> (<source>)",`, and `]`. Any other
# line (`[metadata]` and `[[patch.unused]]` among them) fails as "<lock>:<n>: not in the form
# cargo writes: <line>". The rules below still read such a lock as TOML would, as far as these
# spellings go, so that its packages are named too.
#
# cargo writes a `source` line for every package it takes from a registry or a git repository,
# and none for a crate it reaches by path. The check fails, one line per finding, when:
# - a package with a `source` line matches no line of the allow-list on name, version, source
#   and checksum (`none` for no checksum): "<lock>: <name> <version> from <source>: not on the
#   allow-list"; when the list holds that name with another version, source or checksum, the
#   line goes on with the package's checksum and the listed line;
# - a package without a `source` line (a crate of this repository) lists among its
#   dependencies a package of the allow-list or surf-script, and the owners file has no line
#   `<that package> <the crate>`: "<lock>: <crate> depends on <package>; lock-owners.txt does
#   not allow it". A package of the allow-list may depend on another (ring's on theirs, swell on
#   its five);
# - a package with a `source` line that starts with `git+` has no `overflow-checks = true` in
#   its table of the manifest's release profile: "<manifest>: <name> <version> from <source>: no
#   overflow-checks = true under [profile.release.package.<name>]";
# - a line of the allow-list matches no package of the lock: "<allow-list>: <name> <version> is
#   in no package of <lock>";
# - a line of the allow-list is not four fields, a line of the owners file not two, or the
#   lock, the list, the owners file or the manifest is missing.
# Else it prints "<lock>: <n> package(s), <m> from outside this repository, each on the
# allow-list", or "<lock>: <n> package(s), none from outside this repository" when <m> is 0.
#
# A path dependency on a directory outside this repository has no source line and is not
# caught. conformance/ has a lock of its own, which this does not read.

set -u

usage='usage: lock-check.sh <lock> <allow-list> <owners> <manifest>'
lock=${1:?$usage}
allow=${2:?$usage}
owners=${3:?$usage}
manifest=${4:?$usage}

test -f "$lock" || { echo "$lock: missing; the root workspace's lock is committed"; exit 1; }
test -f "$allow" || { echo "$allow: missing; it lists the packages from outside this repository the lock may hold"; exit 1; }
test -f "$owners" || { echo "$owners: missing; it lists the crates of this repository that may depend on a listed package or on surf-script"; exit 1; }
test -f "$manifest" || { echo "$manifest: missing; it holds the release profile of each package from git"; exit 1; }

exec awk -v lock="$lock" -v allow="$allow" -v owners="$owners" -v manifest="$manifest" '
  # Whether line l is one cargo writes into a lock.
  function cargo_form(l) {
    if (l == "" || l == "[[package]]" || l == "dependencies = [" || l == "]") return 1
    if (l == "# This file is automatically @generated by Cargo.") return 1
    if (l == "# It is not intended for manual editing.") return 1
    if (l ~ /^version = [0-9]+$/) return 1
    if (l ~ /^(name|version|source) = "[^"\\]+"$/) return 1
    if (l ~ /^checksum = "[0-9a-f]+"$/ && length(l) == 77) return 1
    if (l ~ /^ "[^" \\]+( [^" \\]+( \([^"\\]+\))?)?",$/) return 1
    return 0
  }
  function trim(s) { sub(/^[ \t]+/, "", s); sub(/[ \t\r]+$/, "", s); return s }
  # s without a comment: a # outside a basic or literal string ends the line.
  function uncomment(s,   i, c, q) {
    q = ""
    for (i = 1; i <= length(s); i++) {
      c = substr(s, i, 1)
      if (q == "\"" && c == "\\") { i++; continue }
      if (q != "") { if (c == q) q = ""; continue }
      if (c == "\"" || c == "\047") { q = c; continue }
      if (c == "#") return substr(s, 1, i - 1)
    }
    return s
  }
  # The strings of s, basic or literal, into out[1..n]; closed is set when a ] outside a
  # string ends the array.
  function strings(s, out,   i, c, q, v, n) {
    n = 0; q = ""; closed = 0
    for (i = 1; i <= length(s); i++) {
      c = substr(s, i, 1)
      if (q == "\"" && c == "\\") { v = v substr(s, i + 1, 1); i++; continue }
      if (q != "") { if (c == q) { out[++n] = v; q = "" } else v = v c; continue }
      if (c == "\"" || c == "\047") { q = c; v = ""; continue }
      if (c == "]") { closed = 1; break }
    }
    return n
  }
  # The dependency names in s, added to deps.
  function add_deps(s,   n, i, d, parts) {
    n = strings(s, parts)
    for (i = 1; i <= n; i++) { split(parts[i], d, " "); deps = deps " " d[1] }
  }
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
  FILENAME == ARGV[2] {
    if (NF != 2 || $0 != $1 " " $2) {
      print owners ": line " FNR " is not \"<package> <crate>\": " $0
      bad++
      next
    }
    owner[$0] = 1
    next
  }
  FILENAME == ARGV[3] {
    s = trim(uncomment($0))
    if (substr(s, 1, 1) == "[") {
      table = s
      gsub(/[ \t"\047]/, "", table)
      next
    }
    if (table !~ /^\[profile\.release\.package\.[^].]+\]$/) next
    eq = index(s, "=")
    if (!eq) next
    k = trim(substr(s, 1, eq - 1)); gsub(/["\047]/, "", k)
    if (k != "overflow-checks") next
    p = substr(table, 26, length(table) - 26)
    checked[p] = (trim(substr(s, eq + 1)) == "true")
    next
  }
  {
    if (!cargo_form($0)) {
      print lock ":" FNR ": not in the form cargo writes: " $0
      bad++
    }
    s = trim(uncomment($0))
    if (s == "") next
    if (substr(s, 1, 1) == "[") {
      header = s
      gsub(/[ \t"\047]/, "", header)
      finish()
      if (header == "[[package]]") {
        inpkg = 1; name = ""; version = ""; source = ""; checksum = ""; deps = ""
      }
      indeps = 0
      next
    }
    if (!inpkg) next
    if (indeps) { add_deps(s); if (closed) indeps = 0; next }
    eq = index(s, "=")
    if (!eq) next
    k = trim(substr(s, 1, eq - 1)); gsub(/["\047]/, "", k)
    rest = trim(substr(s, eq + 1))
    if (k == "dependencies") { add_deps(rest); indeps = !closed; next }
    if (strings(rest, value) == 0) value[1] = rest
    if (k == "name") name = value[1]
    else if (k == "version") version = value[1]
    else if (k == "source") source = value[1]
    else if (k == "checksum") checksum = value[1]
  }
  END {
    finish()
    for (p = 1; p <= packages; p++) {
      if (psource[p] != "") {
        outside++
        if (substr(psource[p], 1, 4) == "git+" && !checked[pname[p]]) {
          print manifest ": " pname[p] " " pversion[p] " from " psource[p] ": no overflow-checks = true under [profile.release.package." pname[p] "]"
          bad++
        }
        sum = pchecksum[p] == "" ? "none" : pchecksum[p]
        line = pname[p] " " pversion[p] " " psource[p] " " sum
        if (line in entry) { matched[entry[line]] = 1; continue }
        out = lock ": " pname[p] " " pversion[p] " from " psource[p] ": not on the allow-list"
        if (pname[p] in allowed) {
          out = out "; its checksum " sum
          for (l = 1; l <= listed; l++) if (lname[l] == pname[p]) out = out ", the list holds " key[l]
        }
        print out
        bad++
        continue
      }
      n = split(pdeps[p], ds, " ")
      for (i = 1; i <= n; i++) {
        if (ds[i] == pname[p]) continue
        if (!(ds[i] in allowed) && ds[i] != "surf-script") continue
        if ((ds[i] " " pname[p]) in owner) continue
        print lock ": " pname[p] " depends on " ds[i] "; lock-owners.txt does not allow it"
        bad++
      }
    }
    for (l = 1; l <= listed; l++) if (!(l in matched)) {
      print allow ": " lname[l] " " lversion[l] " is in no package of " lock
      bad++
    }
    if (bad) {
      print bad " finding(s); a package from outside this repository must be on " allow ", a crate may depend on one or on surf-script only by a line of " owners ", a package from git needs overflow-checks = true in the release profile of " manifest ", and the lock is read only in the form cargo writes"
      exit 1
    }
    if (outside) print lock ": " packages " package(s), " outside " from outside this repository, each on the allow-list"
    else print lock ": " packages " package(s), none from outside this repository"
  }
' "$allow" "$owners" "$manifest" "$lock"
