#!/bin/sh
# Every crate's integration tests are one test binary (story `one-test-binary-per-crate`).
#
# Usage: sh taskfiles/test-layout.sh <root>
#
# It visits every directory under <root> that holds a Cargo.toml and a tests/ directory
# (target, node_modules, .out, .git and every directory whose name starts with a dot are not
# entered, and a Cargo.toml below a tests/ directory is a fixture, not a crate). In a file of a
# test binary it reads a line without its string and char literals and its comments. For each
# directory it fails, naming the path or the module, when:
# - tests/main.rs is missing;
# - the Cargo.toml has no line `autotests = false` in its [package] table, or not exactly one
#   [[test]] entry, or that entry holds other than exactly `name = "main"` and
#   `path = "tests/main.rs"`, or an [[example]] or [[bench]] entry holds `test = true`; a table
#   header is compared with its spaces and quotes removed, so `[[ test ]]` and `[["test"]]` are
#   [[test]];
# - a line of main.rs is not blank, a `//` comment, an inner attribute `#![...]` without `cfg`,
#   a `use ...;` line, `mod <name>;`, or `#[path = "<stem>.rs"]` or
#   `#[path = "<dir>/<stem>.rs"]` followed by `mod <name>;` on the same or the next line;
# - a *.rs file directly in tests/, other than main.rs, is not declared in main.rs;
# - a *.rs file one directory down (common/ and data/ skipped) is not declared by
#   `#[path = "<dir>/<stem>.rs"]` and `mod <stem>;`;
# - a file of the binary has `#![cfg` or `#![cfg_attr` anywhere in a line, which can drop its
#   module, or an inner attribute `#![` that does not close on its line. The files of the
#   binary are those main.rs declares and, from each, every child module declared out of line
#   (`mod <name>;`, with or without `#[path]`), followed to its file; a child module whose file
#   this check cannot read fails, and so does an out-of-line `mod <name>;` inside an inline
#   module (indented, or after a `{` on its line), which it cannot follow;
# - a module would change a selection: SELECTIONS below lists, for each test selection of the
#   repository, the crate directory, the module it selects and the modules it skips. cargo
#   matches a filter as a substring of a test's path, so `--test main cli::` also runs a module
#   whose name ends in `cli`, and `--skip review1_cli::` also drops one whose name ends in
#   `review1_cli`. In such a crate a module of main.rs whose name ends with a selected or skipped
#   name fails unless it is that module, and so does every module declared inside a file of the
#   binary (inline or out of line, with any visibility or attribute before it on its line),
#   whose name ends with one of them, the selected module's own file included. A stem added to
#   the skipped list gets its `--skip <stem>::` in the selection itself.
#
# Not seen by the check: a module that a macro expands to, a file pulled in by `include!`,
# `autotests = false` inside a multi-line string of [package], and a crate reached through a
# symbolic link.
#
# It calls no cargo. Else it prints one line with the counts and exits 0.

set -u

# The selections: crate directory, selected module, skipped modules.
#   .github/workflows/release.yml       cargo test -p surf-cli --test main cli:: -- --skip review1_cli::
#   taskfiles/release-smoke.sh          cargo test -p surf-session --test main e2e::
#   crates/view/tests/prototype.rs      cargo test -p surf-view --test main prototype::
#   taskfiles/agent-tasks.yml           cargo nextest run -p surf-agent-tasks -E 'test(=tasks::every_task)'
SELECTIONS='crates/surf-cli cli review1_cli
crates/session e2e
crates/view prototype
crates/agent-tasks tasks'

root=${1:?usage: test-layout.sh <root>}
cd "$root" || exit 1

# The test directories, and their manifests and files. A path is of word characters, `-`, `.`
# and `/`, so it holds no white space and no `=`.
dirs=$(find . \( -name target -o -name node_modules -o -name '.?*' \) -prune -o -type d -name tests -print |
  sed 's|^\./||' | LC_ALL=C sort)
nl='
'
set --
files=
testdirs=
for tests in $dirs; do
  # A Cargo.toml below a tests/ directory is a fixture of a test, not a crate of the workspace.
  case "$tests" in tests/* | */tests/*) continue ;; esac
  [ -f "${tests%/tests}/Cargo.toml" ] || continue
  testdirs="$testdirs $tests"
  set -- "$@" "${tests%/tests}/Cargo.toml"
  if [ -f "$tests/main.rs" ]; then set -- "$@" "$tests/main.rs"; fi
  for file in "$tests"/*.rs "$tests"/*/*.rs; do
    if [ -f "$file" ]; then files="$files$file$nl"; fi
  done
done

# The lines of the test files that hold `mod`, `path`, `cfg` or `#!`, for the rules on the files
# of the binary; /dev/null so that grep never reads its standard input.
# shellcheck disable=SC2086
grep -n -H -E 'mod|path|cfg|#[[:space:]]*!' /dev/null $files |
  FILES=$files SELECTIONS=$SELECTIONS awk -v dirs="$testdirs" '
  function problem(text) { print "test-layout: " text; problems++ }
  function module_of(s) { sub(/^.*mod /, "", s); sub(/;$/, "", s); return s }
  function path_of(s) { sub(/^#\[path = "/, "", s); sub(/"\].*$/, "", s); return s }
  function dir_of(s) { if (s !~ /\//) return "."; sub(/\/[^\/]*$/, "", s); return s }
  function ends_with(s, t) { return length(s) >= length(t) && substr(s, length(s) - length(t) + 1) == t }
  function declare(main, rel, module) { decls[main] = decls[main] " " rel "=" module }
  function close_main() {
    if (current != "" && pending != "")
      problem(current ":" pending_line ": a #[path] line not followed by mod <name>;")
    current = ""
    pending = ""
  }
  # A name of a module of a selected crate, at any depth below the modules of main.rs.
  function nested(crate, file, name,   n, k, names) {
    if (!(crate in selected)) return
    n = split(watched[crate], names, " ")
    for (k = 1; k <= n; k++)
      if (ends_with(name, names[k]))
        problem(file ": module " name " ends with " names[k] ": the selection `" skipped_text[crate] "` of " crate " would match its tests")
  }
  # The line without its char and string literals, its block comments closed on the line and
  # its line comment: what is left is code.
  function code_of(s) {
    gsub("\047(\\\\.|[^\047\\\\])\047", "\047 \047", s)
    gsub(/"(\\.|[^"\\])*"/, "\"\"", s)
    gsub(/\/\*([^*]|\*+[^*\/])*\*+\//, " ", s)
    sub(/\/\/.*$/, "", s)
    return s
  }
  # Whether the inner attribute that starts at the first `#![` of `s` closes on the line.
  function attribute_closes(s,   at, depth, k, c) {
    match(s, /#[ \t]*![ \t]*\[/)
    at = RSTART + RLENGTH
    depth = 1
    for (k = at; k <= length(s); k++) {
      c = substr(s, k, 1)
      if (c == "[") depth++
      else if (c == "]" && --depth == 0) return 1
    }
    return 0
  }
  # The lines of a file that may matter (they hold `mod`, `path`, `cfg` or `#!`), as
  # kept[file, i] with their numbers at[file, i]: from grep for the files of tests/, read here
  # for any other file. Returns 0 when the file cannot be read.
  function load(file,   line, number, status) {
    if (file in kept_count) return 1
    if (file in exists) { kept_count[file] = 0; return 1 }
    number = 0
    kept_count[file] = 0
    while ((status = (getline line < file)) > 0) {
      number++
      if (index(line, "mod") || index(line, "path") || index(line, "cfg") || line ~ /#[ \t]*!/) {
        kept_count[file]++
        kept[file, kept_count[file]] = line
        at[file, kept_count[file]] = number
      }
    }
    close(file)
    if (status < 0) { delete kept_count[file]; return 0 }
    return 1
  }
  # Read one file of the binary: its inner attributes, and every module it declares; follow each
  # child module declared out of line to its file.
  function visit(crate, file,   i, line, code, number, attr, attr_at, path_here, name, rest, end, child, dir, moddir, base, before) {
    if ((crate SUBSEP file) in visited) return
    visited[crate SUBSEP file] = 1
    dir = dir_of(file)
    base = file
    sub(/^.*\//, "", base)
    moddir = (base == "mod.rs") ? dir : dir "/" substr(base, 1, length(base) - 3)
    attr = ""
    attr_at = -1
    for (i = 1; i <= kept_count[file]; i++) {
      line = kept[file, i]
      number = at[file, i]
      # The value of a #[path] attribute is read before the strings are removed.
      path_here = ""
      if (match(line, /#\[[ \t]*path[ \t]*=[ \t]*"[^"]*"[ \t]*\]/)) {
        path_here = substr(line, RSTART, RLENGTH)
        sub(/^[^"]*"/, "", path_here)
        sub(/".*$/, "", path_here)
      }
      code = code_of(line)
      if (code ~ /#[ \t]*![ \t]*\[/) {
        if (code ~ /#[ \t]*![ \t]*\[[ \t]*cfg/)
          problem(file ":" number ": " line ": #![cfg can drop this module from the binary")
        else if (!attribute_closes(code))
          problem(file ":" number ": " line ": an inner attribute that does not close on its line, which this check cannot read")
      }
      if (path_here != "" && code ~ /^[ \t]*#\[[ \t]*path[ \t]*=[ \t]*""[ \t]*\][ \t]*$/) {
        attr = path_here
        attr_at = number
        continue
      }
      if (match(code, /(^|[^A-Za-z0-9_])mod[ \t]+[A-Za-z_][A-Za-z0-9_]*[ \t]*[;{]/)) {
        before = substr(code, 1, RSTART)
        rest = substr(code, RSTART, RLENGTH)
        end = substr(rest, length(rest), 1)
        name = rest
        sub(/^.*mod[ \t]+/, "", name)
        sub(/[^A-Za-z0-9_].*$/, "", name)
        nested(crate, file, name)
        if (end == "{" && match(substr(code, RSTART + RLENGTH), /(^|[^A-Za-z0-9_])mod[ \t]+[A-Za-z_][A-Za-z0-9_]*[ \t]*;/)) {
          problem(file ":" number ": " line ": an out-of-line module inside an inline module, which this check cannot follow")
        } else if (end == ";") {
          if (code ~ /^[ \t]/ || index(before, "{"))
            problem(file ":" number ": " line ": an out-of-line module inside an inline module, which this check cannot follow")
          else {
            if (attr_at != number - 1) attr = ""
            if (path_here != "") attr = path_here
            if (attr != "") child = dir "/" attr
            else if (load(moddir "/" name ".rs")) child = moddir "/" name ".rs"
            else child = moddir "/" name "/mod.rs"
            if (load(child)) visit(crate, child)
            else problem(file ":" number ": mod " name "; names " child ", which this check cannot read")
          }
        }
      }
      attr = ""
    }
  }
  BEGIN {
    nfiles = split(ENVIRON["FILES"], all, "\n")
    for (i = 1; i <= nfiles; i++) if (all[i] != "") exists[all[i]] = 1
  }
  FNR == 1 {
    close_main()
    if (FILENAME ~ /\/Cargo\.toml$/) kind = "manifest"
    else if (FILENAME ~ /\/tests\/main\.rs$/) kind = "main"
    else kind = "grep"
    if (kind == "main") current = FILENAME
    table = ""
    if (kind != "grep") exists[FILENAME] = 1
  }
  kind == "grep" {
    # A line of grep: <file>:<number>:<line>.
    colon = index($0, ":")
    file = substr($0, 1, colon - 1)
    rest = substr($0, colon + 1)
    colon = index(rest, ":")
    kept_count[file]++
    at[file, kept_count[file]] = substr(rest, 1, colon - 1) + 0
    kept[file, kept_count[file]] = substr(rest, colon + 1)
    next
  }
  kind == "manifest" {
    line = $0
    sub(/[ \t]*#.*$/, "", line)
    if (line ~ /^[ \t]*\[/) {
      table = line
      gsub(/[ \t"\047]/, "", table)
      if (table == "[[test]]") entries[FILENAME]++
      next
    }
    gsub(/[ \t]/, "", line)
    gsub(/\047/, "\"", line)
    if (line == "") next
    if (table == "[package]" && line == "autotests=false") autotests[FILENAME] = 1
    if ((table == "[[example]]" || table == "[[bench]]") && line == "test=true")
      problem(FILENAME ":" FNR ": test = true in an entry " table ", which cargo builds as a test binary of its own")
    if (table == "[[test]]") {
      if (line == "name=\"main\"") names[FILENAME]++
      else if (line == "path=\"tests/main.rs\"") paths[FILENAME]++
      else others[FILENAME] = others[FILENAME] " `" $0 "`"
    }
    next
  }
  kind == "main" {
    line = $0
    if (pending != "") {
      if (line ~ /^mod [A-Za-z_][A-Za-z0-9_]*;$/) {
        declare(FILENAME, pending, module_of(line))
        pending = ""
        next
      }
      problem(FILENAME ":" pending_line ": a #[path] line not followed by mod <name>;")
      pending = ""
    }
    if (line ~ /^[ \t]*$/ || line ~ /^[ \t]*\/\//) next
    if (line ~ /^#!\[.*\]$/) {
      if (line ~ /cfg/) problem(FILENAME ":" FNR ": " line ": an inner attribute with cfg")
      next
    }
    if (line ~ /^use [^;]*;$/) next
    if (line ~ /^mod [A-Za-z_][A-Za-z0-9_]*;$/) {
      declare(FILENAME, module_of(line) ".rs", module_of(line))
      next
    }
    if (line ~ /^#\[path = "([A-Za-z0-9_]+\/)?[A-Za-z0-9_]+\.rs"\]$/) {
      pending = path_of(line)
      pending_line = FNR
      next
    }
    if (line ~ /^#\[path = "([A-Za-z0-9_]+\/)?[A-Za-z0-9_]+\.rs"\] mod [A-Za-z_][A-Za-z0-9_]*;$/) {
      declare(FILENAME, path_of(line), module_of(line))
      next
    }
    problem(FILENAME ":" FNR ": " line ": not a blank line, a comment, an inner attribute without cfg, a use line or a module declaration")
    next
  }
  END {
    close_main()
    count = split(ENVIRON["SELECTIONS"], rows, "\n")
    for (r = 1; r <= count; r++) {
      n = split(rows[r], words, " ")
      if (n < 2) continue
      selected[words[1]] = words[2]
      watched[words[1]] = words[2]
      skipped_text[words[1]] = "--test main " words[2] "::"
      for (w = 3; w <= n; w++) {
        watched[words[1]] = watched[words[1]] " " words[w]
        skipped_text[words[1]] = skipped_text[words[1]] " --skip " words[w] "::"
      }
    }
    ndirs = split(dirs, list, " ")
    for (d = 1; d <= ndirs; d++) {
      tests = list[d]
      crate = tests
      sub(/\/tests$/, "", crate)
      manifest = crate "/Cargo.toml"
      main = tests "/main.rs"
      if (!(manifest in autotests)) problem(manifest ": no line autotests = false in its [package] table")
      if (entries[manifest] != 1)
        problem(manifest ": " entries[manifest] + 0 " [[test]] entries, expected exactly one")
      else if (names[manifest] != 1 || paths[manifest] != 1 || others[manifest] != "")
        problem(manifest ": its [[test]] entry must hold exactly name = \"main\" and path = \"tests/main.rs\"" (others[manifest] != "" ? ", it holds" others[manifest] : ""))
      if (!(main in exists)) problem(main ": missing; the tests of " crate " are one binary over tests/main.rs")

      # The declarations of main.rs: rel=module, rel under tests/.
      delete declared
      nd = split(decls[main], pairs, " ")
      for (p = 1; p <= nd; p++) {
        eq = index(pairs[p], "=")
        rel = substr(pairs[p], 1, eq - 1)
        module = substr(pairs[p], eq + 1)
        declared[rel "=" module] = 1
        declared[rel] = 1
        file = tests "/" rel
        if (!(file in exists) && rel !~ /\//) {
          alt = tests "/" substr(rel, 1, length(rel) - 3) "/mod.rs"
          if (alt in exists) file = alt
        }
        if ((crate in selected) && index(" " watched[crate] " ", " " module " ") == 0) {
          n = split(watched[crate], names_watched, " ")
          for (k = 1; k <= n; k++)
            if (ends_with(module, names_watched[k]))
              problem(main ": module " module " ends with " names_watched[k] ": the selection `" skipped_text[crate] "` of " crate " would match its tests")
        }
        if (!(file in exists)) {
          problem(main ": mod " module "; names " tests "/" rel ", which does not exist")
          continue
        }
        visit(crate, file)
      }

      for (i = 1; i <= nfiles; i++) {
        file = all[i]
        if (file == "" || index(file, tests "/") != 1) continue
        rel = substr(file, length(tests) + 2)
        if (rel == "main.rs") continue
        if (rel !~ /\//) {
          files++
          if ((main in exists) && !(rel in declared))
            problem(file ": not declared in " main " (mod " substr(rel, 1, length(rel) - 3) ";)")
        } else if (rel !~ /^(common|data)\//) {
          stem = rel
          sub(/^.*\//, "", stem)
          sub(/\.rs$/, "", stem)
          if ((main in exists) && !((rel "=" stem) in declared))
            problem(file ": not declared in " main " (#[path = \"" rel "\"] and mod " stem ";)")
        }
      }
    }
    if (problems) {
      print "test-layout: the tests of a crate are one binary over tests/main.rs; see AGENTS.md, Layout"
      exit 1
    }
    print "test-layout: " ndirs " test directories, " files + 0 " test files, one test binary each"
  }
' - "$@"
