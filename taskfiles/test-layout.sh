#!/bin/sh
# Every crate's integration tests are one test binary (story `one-test-binary-per-crate`).
#
# Usage: sh taskfiles/test-layout.sh <root>
#
# For each directory crates/*/tests and conformance/tests under <root> it fails, naming the path
# or the module, when:
# - tests/main.rs is missing;
# - the Cargo.toml beside the directory has no line `autotests = false`, or not exactly one
#   [[test]] entry, or that entry holds other than exactly `name = "main"` and
#   `path = "tests/main.rs"`;
# - a line of main.rs is not blank, a `//` comment, an inner attribute `#![...]` without `cfg`,
#   a `use ...;` line, `mod <name>;`, or `#[path = "<stem>.rs"]` or
#   `#[path = "<dir>/<stem>.rs"]` followed by `mod <name>;` on the same or the next line;
# - a *.rs file directly in the directory, other than main.rs, is not declared in main.rs;
# - a file main.rs declares has a line that starts with `#![cfg`;
# - a *.rs file one directory down (common/ and data/ skipped) is not declared by
#   `#[path = "<dir>/<stem>.rs"]` and `mod <stem>;`;
# - a module would join a selection by accident: SELECTIONS below lists, for each test
#   selection of the repository, the crate directory, the module it selects and the modules it
#   skips. cargo matches a filter as a substring, so `--test main cli::` also runs a module
#   whose name ends in `cli`, and an inline `mod cli {` inside another module. A module name on
#   a `mod` line of that crate's main.rs, or an inline `mod <name> {` of a file it declares,
#   that ends with the selected name fails unless it is the selected module or a skipped one.
#   A stem added to the skipped list gets its `--skip <stem>::` in the selection itself.
#
# It calls no cargo and reads every file in one awk run. Else it prints one line with the
# counts and exits 0.

set -u

# The selections: crate directory, selected module, skipped modules.
#   .github/workflows/release.yml       cargo test -p surf-cli --test main cli:: -- --skip review1_cli::
#   taskfiles/release-smoke.sh          cargo test -p surf-session --test main e2e::
#   crates/view/tests/prototype.rs      cargo test -p surf-view --test main prototype::
SELECTIONS='crates/surf-cli cli review1_cli
crates/session e2e
crates/view prototype'

root=${1:?usage: test-layout.sh <root>}
cd "$root" || exit 1

# The test directories and their files: the manifests and the main.rs files are read whole,
# the other files only for their lines that start `#![cfg` or an inline `mod <name> {`, which
# one grep picks out. A path is <crate>/tests/... of word characters and `-`, so it holds no
# white space and no `=`.
nl='
'
dirs=
for tests in crates/*/tests conformance/tests; do
  if [ -d "$tests" ]; then dirs="$dirs $tests"; fi
done
set --
files=
others=
for tests in $dirs; do
  for file in "${tests%/tests}/Cargo.toml" "$tests"/*.rs "$tests"/*/*.rs; do
    [ -f "$file" ] || continue
    files="$files$file$nl"
    case "$file" in
      */Cargo.toml | */tests/main.rs) set -- "$@" "$file" ;;
      *) others="$others $file" ;;
    esac
  done
done

grep -H -E '^#!\[cfg|^[[:blank:]]*(pub )?mod [A-Za-z_][A-Za-z0-9_]* *\{' /dev/null $others |
  FILES=$files SELECTIONS=$SELECTIONS awk -v dirs="$dirs" '
  function problem(text) { print "test-layout: " text; problems++ }
  function module_of(s) { sub(/^.*mod /, "", s); sub(/;$/, "", s); return s }
  function path_of(s) { sub(/^#\[path = "/, "", s); sub(/"\].*$/, "", s); return s }
  function declare(main, rel, module) {
    decls[main] = decls[main] " " rel "=" module
  }
  function close_main() {
    if (current != "" && pending != "")
      problem(current ":" pending_line ": a #[path] line not followed by mod <name>;")
    current = ""
    pending = ""
  }
  BEGIN {
    nfiles = split(ENVIRON["FILES"], all, "\n")
    for (i = 1; i <= nfiles; i++) if (all[i] != "") exists[all[i]] = 1
  }
  FNR == 1 {
    close_main()
    kind = "grep"
    if (FILENAME ~ /\/Cargo\.toml$/) kind = "manifest"
    else if (FILENAME ~ /\/tests\/main\.rs$/) { kind = "main"; current = FILENAME }
    entry = 0
  }
  kind == "manifest" {
    if ($0 == "autotests = false") autotests[FILENAME] = 1
    if ($0 ~ /^[ \t]*\[/) {
      entry = ($0 ~ /^[ \t]*\[\[test\]\][ \t]*(#.*)?$/)
      if (entry) entries[FILENAME]++
      next
    }
    if (entry && $0 !~ /^[ \t]*(#.*)?$/) {
      line = $0
      sub(/^[ \t]+/, "", line)
      sub(/[ \t]+$/, "", line)
      if (line == "name = \"main\"") names[FILENAME]++
      else if (line == "path = \"tests/main.rs\"") paths[FILENAME]++
      else others[FILENAME] = others[FILENAME] " `" line "`"
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
  {
    # A line of grep: <file>:<line>.
    colon = index($0, ":")
    file = substr($0, 1, colon - 1)
    line = substr($0, colon + 1)
    if (line ~ /^#!\[cfg/) cfg[file] = 1
    if (line ~ /^[ \t]*(pub )?mod [A-Za-z_][A-Za-z0-9_]* *\{/) {
      name = line
      sub(/^[ \t]*(pub )?mod /, "", name)
      sub(/[^A-Za-z0-9_].*$/, "", name)
      inline[file] = inline[file] " " name
    }
  }
  END {
    close_main()
    count = split(ENVIRON["SELECTIONS"], rows, "\n")
    for (r = 1; r <= count; r++) {
      n = split(rows[r], words, " ")
      selected[words[1]] = words[2]
      skipped[words[1]] = " " words[2]
      for (w = 3; w <= n; w++) skipped[words[1]] = skipped[words[1]] " " words[w]
      skipped[words[1]] = skipped[words[1]] " "
    }
    ndirs = split(dirs, list, " ")
    for (d = 1; d <= ndirs; d++) {
      tests = list[d]
      crate = tests
      sub(/\/tests$/, "", crate)
      manifest = crate "/Cargo.toml"
      main = tests "/main.rs"
      if (!(manifest in exists)) problem(manifest ": missing")
      else {
        if (!(manifest in autotests)) problem(manifest ": no line autotests = false")
        if (entries[manifest] != 1)
          problem(manifest ": " entries[manifest] + 0 " [[test]] entries, expected exactly one")
        else if (names[manifest] != 1 || paths[manifest] != 1 || others[manifest] != "")
          problem(manifest ": its [[test]] entry must hold exactly name = \"main\" and path = \"tests/main.rs\"" (others[manifest] != "" ? ", it holds" others[manifest] : ""))
      }
      if (!(main in exists)) problem(main ": missing; the tests of " crate " are one binary over tests/main.rs")

      # The declarations: rel=module, rel under tests/.
      delete declared
      delete module_file
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
        if (!(file in exists)) {
          problem(main ": mod " module "; names " tests "/" rel ", which does not exist")
          continue
        }
        module_file[module] = file
        if (file in cfg) problem(file ": a line starts with #![cfg, which can drop the module from the binary")
      }

      for (i = 1; i <= nfiles; i++) {
        file = all[i]
        if (index(file, tests "/") != 1) continue
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

      if (!(crate in selected) || !(main in exists)) continue
      sel = selected[crate]
      for (p = 1; p <= nd; p++) {
        eq = index(pairs[p], "=")
        module = substr(pairs[p], eq + 1)
        if (index(skipped[crate], " " module " ")) continue
        if (length(module) > length(sel) && substr(module, length(module) - length(sel) + 1) == sel)
          problem(main ": module " module " ends with " sel ", so the selection " sel ":: of " crate " runs it")
        if (!(module in module_file)) continue
        file = module_file[module]
        ni = split(inline[file], names_in, " ")
        for (k = 1; k <= ni; k++) {
          name = names_in[k]
          if (length(name) >= length(sel) && substr(name, length(name) - length(sel) + 1) == sel)
            problem(file ": inline module " name " ends with " sel ", so the selection " sel ":: of " crate " runs it")
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
