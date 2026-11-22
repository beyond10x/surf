#!/bin/sh
# Fail when the Rust run and the interpreter's run are of different suites or totals. task
# conform-totals runs it in the repository; the tests of crates/surf/tests/conform_totals.rs run
# it with sh in their copies, so no case starts task.
#
# usage: sh taskfiles/conform-totals.sh
#
# Reads .out/report.json (task conform) and .out/conform-rust.json (task conform-rust) of the
# directory it runs in. It prints one line and exits 0 when both name the same suite by its
# digest and the same counts.total; else it prints both and exits 1, as it does when either
# report is missing or holds no digest or total. Values are printed with printf: the forge's sh
# is dash, whose echo expands backslashes.

set -u

for report in .out/report.json .out/conform-rust.json; do
  test -f "$report" || {
    printf '%s: missing; run task conform and task conform-rust first\n' "$report"
    exit 1
  }
done
interpreter=$(tr -d ' \n' < .out/report.json)
rust=$(tr -d ' \n' < .out/conform-rust.json)
interpreter_total=$(printf '%s\n' "$interpreter" | sed -n 's/.*"counts":{[^}]*"total":\([0-9][0-9]*\).*/\1/p')
rust_total=$(printf '%s\n' "$rust" | sed -n 's/.*"counts":{[^}]*"total":\([0-9][0-9]*\).*/\1/p')
interpreter_suite=$(printf '%s\n' "$interpreter" | sed -n 's/.*"suite":{"digest":"\([^"]*\)".*/\1/p')
rust_suite=$(printf '%s\n' "$rust" | sed -n 's/.*"suite":{"digest":"\([^"]*\)".*/\1/p')
if [ -n "$interpreter_total" ] && [ -n "$interpreter_suite" ] && [ "$interpreter_total" = "$rust_total" ] && [ "$interpreter_suite" = "$rust_suite" ]; then
  printf 'the interpreter and the Rust system ran %s scenario(s) each, of suite %s\n' "$rust_total" "$rust_suite"
  exit 0
fi
printf 'interpreter: %s scenario(s) of suite %s (.out/report.json)\n' "${interpreter_total:-no}" "${interpreter_suite:-unknown}"
printf 'rust: %s scenario(s) of suite %s (.out/conform-rust.json)\n' "${rust_total:-no}" "${rust_suite:-unknown}"
printf '%s\n' "the two runs differ; each must run every scenario of the same suite"
exit 1
