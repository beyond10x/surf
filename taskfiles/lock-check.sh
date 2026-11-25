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
# writes no `checksum` line for a package from git); sorted, no comment lines, no empty lines,
# no line twice.
#
# The owners file holds one line `<package> <crate>` per crate of this repository that may name
# the package among its dependencies: a package of the allow-list, or surf-script, the one
# crate that runs swell, which no crate may depend on without a line of its own
# (docs/design/scripts.md, Q14 and Q15). Two fields separated by one space; sorted, no comment
# lines, no empty lines, no line twice, and each line used: the crate depends on the package.
#
# The manifest is the root Cargo.toml. cargo reads no profile from a dependency's manifest, so
# swell's own `overflow-checks = true` is not read, and each package from git needs a table
# `[profile.release.package.<name>]` in the root manifest holding `overflow-checks = true` (the
# dev and test profiles check overflow by cargo's default). The root manifest holds no
# multi-line string (`"""` or `'''`), so that no table hides inside one; the lines inside one are
# not read as tables.
#
# A cargo config file overrides the root manifest's profiles and adds rustflags after the
# profile's flags, and rustc takes the last `-C overflow-checks`. The check reads the config
# files of the tree, `.cargo/config.toml` and `.cargo/config` beside the manifest, when they
# exist. Such a file holds no multi-line string and no backslash, so that each line is read as
# what it says: no table hides in a string and no name is spelled in escapes. It fails on a
# line in a `profile` table or key that names a package from git as a package of a profile
# (`[profile.<any>.package.<name>]`, also in dotted keys or an inline table), on a line outside
# a comment that holds `rustflags` in any letter case (under `[build]`, `[target.<any>]`,
# `[host]` or a profile, and `RUSTFLAGS`, `CARGO_ENCODED_RUSTFLAGS` or `CARGO_BUILD_RUSTFLAGS`
# under `[env]`), and on a line that holds `overflow` outside a comment and is not
# `overflow-checks = true`.
#
# The files that start builds are `Taskfile.yml` beside the manifest, every file under
# `taskfiles/` but this script, `lock-allow.txt` and `lock-owners.txt`, and every file under
# `.github/workflows/`. A line of one of them fails when it holds `overflow-checks` or
# `overflow_checks` in any letter case (the rustc flag in both spellings, the key in a
# `--config`, `CARGO_PROFILE_<NAME>_OVERFLOW_CHECKS`), or `rustflags` in any letter case with an
# `@` (an argument file).
#
# The check holds the plain forms: a key or a flag written in the tree's cargo config files or
# in the files that start builds. It does not hold a setting reached through another file (a
# config file named by `--config`, a script outside `taskfiles/`, a program a task downloads),
# a `rustc` or wrapper named in a config file, an ancestor's config file, or the environment of
# whoever starts the build. A change of these files is reviewed; that holds the rest. Some
# lines a reader may expect to pass fail by these rules and stay so: `"""` or a backslash in a
# comment of a config file, `overflow-checks=true` without spaces, and an `[env]` key that
# holds the word overflow.
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
# - a package without a `source` line bears the name of a package of the allow-list: "<lock>:
#   <name> <version> has no source but the name of a listed package";
# - a package with a `source` line that starts with `git+` has no `overflow-checks = true` in
#   its table of the manifest's release profile: "<manifest>: <name> <version> from <source>: no
#   overflow-checks = true under [profile.release.package.<name>]";
# - a line of the manifest holds `"""` or `'''`: "<manifest>:<n>: a multi-line string ...";
# - a line of a config file as above: "<config>:<n>: a multi-line string ...", "... a
#   backslash ...", "... names <name>, a package from git, in a profile ...", "... rustflags;
#   ..." or "... holds overflow but is not overflow-checks = true ...";
# - a line of a file that starts a build as above: "<file>:<n>: names overflow checks ..." or
#   "... rustflags with an argument file ...";
# - a line of the allow-list matches no package of the lock: "<allow-list>: <name> <version> is
#   in no package of <lock>";
# - a line of the owners file is used by no dependency: "<owners>: <package> <crate> is a
#   permission nobody uses: <crate> does not depend on <package>";
# - a line of the allow-list is not four fields, a line of the owners file not two, a line of
#   either is there twice, or the lock, the list, the owners file or the manifest is missing.
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

# The config files of the tree that cargo reads beside the manifest, and the files that start
# builds, those that exist: the lock comes last, every other file before it.
dir=$(dirname "$manifest")
case $dir in
  .) pre= ;;
  *) pre=$dir/ ;;
esac
config1=${pre}.cargo/config.toml
config2=${pre}.cargo/config
set -- "$allow" "$owners" "$manifest"
test -f "$config1" && set -- "$@" "$config1"
test -f "$config2" && set -- "$@" "$config2"
test -f "${pre}Taskfile.yml" && set -- "$@" "${pre}Taskfile.yml"
starts=$(find "${pre}taskfiles" "${pre}.github/workflows" -type f 2>/dev/null | LC_ALL=C sort)
ifs=$IFS
IFS='
'
set -f
for f in $starts; do
  case $f in
    "${pre}taskfiles/lock-check.sh" | "${pre}taskfiles/lock-allow.txt" | "${pre}taskfiles/lock-owners.txt") ;;
    *) set -- "$@" "$f" ;;
  esac
done
set +f
IFS=$ifs
set -- "$@" "$lock"

exec awk -v lock="$lock" -v allow="$allow" -v owners="$owners" -v manifest="$manifest" \
  -v config1="$config1" -v config2="$config2" '
  BEGIN { sq = sprintf("%c", 39); tsq = sq sq sq; tdq = "\"\"\"" }
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
  # How many times t occurs in s, not overlapping.
  function count(s, t,   n, i) {
    n = 0
    while ((i = index(s, t)) > 0) { n++; s = substr(s, i + length(t)) }
    return n
  }
  # Whether s names the package g: g not inside a longer name.
  function names(s, g,   i, before, after) {
    while ((i = index(s, g)) > 0) {
      before = i > 1 ? substr(s, i - 1, 1) : ""
      after = substr(s, i + length(g), 1)
      if (before !~ /[A-Za-z0-9_-]/ && after !~ /[A-Za-z0-9_-]/) return 1
      s = substr(s, i + length(g))
    }
    return 0
  }
  FILENAME == allow {
    if (NF != 4 || $0 != $1 " " $2 " " $3 " " $4) {
      print allow ": line " FNR " is not \"<name> <version> <source> <checksum>\": " $0
      bad++
      next
    }
    if ($0 in entry) {
      print allow ": line " FNR " is there twice: " $0
      bad++
      next
    }
    listed++
    key[listed] = $0; lname[listed] = $1; lversion[listed] = $2
    allowed[$1] = 1
    entry[$0] = listed
    next
  }
  FILENAME == owners {
    if (NF != 2 || $0 != $1 " " $2) {
      print owners ": line " FNR " is not \"<package> <crate>\": " $0
      bad++
      next
    }
    if ($0 in owner) {
      print owners ": line " FNR " is there twice: " $0
      bad++
      next
    }
    owner[$0] = 1
    owned[++owners_n] = $0
    next
  }
  FILENAME == manifest {
    # The root manifest holds no multi-line string, and a line inside one is no table.
    n = count($0, tdq) + count($0, tsq)
    if (n) {
      print manifest ":" FNR ": a multi-line string; the root manifest holds none, so that no table hides in one: " $0
      bad++
      if (n % 2) instring = !instring
      next
    }
    if (instring) next
    s = trim(uncomment($0))
    if (substr(s, 1, 1) == "[") {
      table = s
      gsub(/[ \t"]/, "", table)
      gsub(sq, "", table)
      next
    }
    if (table !~ /^\[profile\.release\.package\.[^].]+\]$/) next
    eq = index(s, "=")
    if (!eq) next
    k = trim(substr(s, 1, eq - 1)); gsub(/"/, "", k); gsub(sq, "", k)
    if (k != "overflow-checks") next
    p = substr(table, 26, length(table) - 26)
    checked[p] = (trim(substr(s, eq + 1)) == "true")
    next
  }
  FILENAME == config1 || FILENAME == config2 {
    if (FNR == 1) { ctable = ""; cinstring = 0 }
    # No multi-line string and no backslash, so that a line is read as what it says.
    n = count($0, tdq) + count($0, tsq)
    if (n) {
      print FILENAME ":" FNR ": a multi-line string; a cargo config file of the tree holds none, so that no table or key hides in one: " $0
      bad++
      if (n % 2) cinstring = !cinstring
      next
    }
    if (cinstring) next
    if (index($0, "\\")) {
      print FILENAME ":" FNR ": a backslash; a cargo config file of the tree holds none, so that no name is spelled in escapes: " $0
      bad++
      next
    }
    s = trim(uncomment($0))
    if (s == "") next
    # No rustflags at all; a line that holds overflow outside a comment, kept for the END rules.
    if (index(tolower(s), "rustflags")) {
      print FILENAME ":" FNR ": rustflags; a cargo config file of the tree holds none, so that no flag and no argument file undoes the release profile of " manifest ": " $0
      bad++
    } else if (index(tolower(s), "overflow") && s != "overflow-checks = true") {
      overs++
      ofile[overs] = FILENAME; oline[overs] = FNR; otext[overs] = $0
    }
    # Each line of a profile table or key, kept with its key path for the END rules.
    if (substr(s, 1, 1) == "[") {
      ctable = s
      gsub(/[][ \t"]/, "", ctable)
      gsub(sq, "", ctable)
      path = ctable
    } else {
      eq = index(s, "=")
      if (!eq) next
      k = trim(substr(s, 1, eq - 1)); gsub(/[ \t"]/, "", k); gsub(sq, "", k)
      path = ctable == "" ? k : ctable "." k
    }
    if (path != "profile" && substr(path, 1, 8) != "profile.") next
    configs++
    cfile[configs] = FILENAME; cline[configs] = FNR; cpath[configs] = path
    ctext[configs] = $0
    next
  }
  FILENAME != lock {
    # A file that starts a build names no overflow checks and no argument file in rustflags.
    low = tolower($0)
    if (index(low, "overflow-checks") || index(low, "overflow_checks")) {
      print FILENAME ":" FNR ": names overflow checks; a file that starts a build sets none, so that the release profile of " manifest " holds: " $0
      bad++
    }
    if (index(low, "rustflags") && index($0, "@")) {
      print FILENAME ":" FNR ": rustflags with an argument file; a file that starts a build names none, so that no flag read from a file undoes the release profile of " manifest ": " $0
      bad++
    }
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
        if (substr(psource[p], 1, 4) == "git+") {
          fromgit[pname[p]] = 1
          if (!checked[pname[p]]) {
            print manifest ": " pname[p] " " pversion[p] " from " psource[p] ": no overflow-checks = true under [profile.release.package." pname[p] "]"
            bad++
          }
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
      if (pname[p] in allowed) {
        print lock ": " pname[p] " " pversion[p] " has no source but the name of a listed package"
        bad++
        continue
      }
      n = split(pdeps[p], ds, " ")
      for (i = 1; i <= n; i++) {
        if (ds[i] == pname[p]) continue
        if (!(ds[i] in allowed) && ds[i] != "surf-script") continue
        if ((ds[i] " " pname[p]) in owner) { used[ds[i] " " pname[p]] = 1; continue }
        print lock ": " pname[p] " depends on " ds[i] "; lock-owners.txt does not allow it"
        bad++
      }
    }
    for (c = 1; c <= configs; c++) {
      n = split(cpath[c], seg, ".")
      hit = ""
      if (n >= 4 && seg[3] == "package" && (seg[4] in fromgit)) hit = seg[4]
      else if (index(ctext[c], "{")) for (g in fromgit) if (names(ctext[c], g)) { hit = g; break }
      if (hit != "") {
        print cfile[c] ":" cline[c] ": names " hit ", a package from git, in a profile; a cargo config file overrides the profiles of " manifest ": " ctext[c]
        bad++
        named[cfile[c] SUBSEP cline[c]] = 1
      }
    }
    for (o = 1; o <= overs; o++) {
      if ((ofile[o] SUBSEP oline[o]) in named) continue
      print ofile[o] ":" oline[o] ": holds overflow but is not overflow-checks = true; a cargo config file sets the profiles and the rustflags of each build of the tree, and rustc takes the last overflow-checks flag: " otext[o]
      bad++
    }
    for (l = 1; l <= listed; l++) if (!(l in matched)) {
      print allow ": " lname[l] " " lversion[l] " is in no package of " lock
      bad++
    }
    for (o = 1; o <= owners_n; o++) if (!(owned[o] in used)) {
      split(owned[o], pc, " ")
      print owners ": " owned[o] " is a permission nobody uses: " pc[2] " does not depend on " pc[1]
      bad++
    }
    if (bad) {
      print bad " finding(s); a package from outside this repository must be on " allow ", a crate may depend on one or on surf-script only by a line of " owners ", a package from git needs overflow-checks = true in the release profile of " manifest " that no cargo config file of the tree and no file that starts a build may undo, and the lock is read only in the form cargo writes"
      exit 1
    }
    if (outside) print lock ": " packages " package(s), " outside " from outside this repository, each on the allow-list"
    else print lock ": " packages " package(s), none from outside this repository"
  }
' "$@"
