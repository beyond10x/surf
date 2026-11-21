---
format: aep.planning-md/3
id: story:html-conformance-gate
kind: story
status: active
title: Hold the parser's conformance counts in the gate
summary: task check prints the counts per section and fails when one drops or a section marked complete has a case not passing.
relations:
- decomposes: epic:html-parser
- depends_on: story:html-conformance-harness
- depends_on: story:agent-session-spec
- depends_on: story:agent-page-path-spec
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:11Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:11Z", actor: "human:timo", revision: 5}
---
## Context

After story `html-conformance-harness`, `cargo run -p surf-html-conformance` prints how many
cases of its two sources (html5lib-tests and web-platform-tests, `docs/design/html-parser.md` item
3) exist, run and pass, per file, per tag and in total, and nothing holds
those numbers. `AGENTS.md` ("Goals") says a section of a standard is complete only when every
requirement in it is specified and every test for it passes, and that where the mark is recorded
is not decided yet. `docs/design/html-parser.md` (story `html-decisions`) decides it: one expected
file per group of the inventory, `crates/html-conformance/expected/<group>.txt` with `<group>`
`tokenizer`, `tree-construction` or `encoding`, and a fenced block `html-sections` in the note that fixes which data files belong to which section and lists, per
section, the requirements the note leaves out and the web-platform-tests tests that do not run.
`task check` (`Taskfile.yml`) is the gate; it does not run the harness.

Two things in `html-sections` are not a plain list of files (the note, item 8, the block's
format): two of its nine sections have no `data` line at all (`the-end` and
`speculative-html-parsing`), and the `wpt` lines naming `html/syntax/parsing/html5lib_write.html`
and `html/syntax/parsing/html5lib_write_single.html` stand for their variants `?file=<f>`, one for
each `.dat` file of that section; a `wpt` path ending in `/` stands for every test under that
directory.

This epic is judged by its conformance counts, so the check is built so that no later story can
meet its acceptance by editing a number: the section map comes from the note, not from a file a
parser story writes; every expected count is compared with its value at the story's base; and
`complete` is refused unless both halves of the rule in `AGENTS.md` hold.

Ordering: this story writes no line of the root `Taskfile.yml`. Its task goes to a file of its
own, `taskfiles/html-conformance.yml`, and the include line and the line in `check` are handed to
the coordinator (`docs/plan/agent-plan-revision-1.md`, D16; `docs/plan/agent-plan-revision-2.md`,
D30). Its edges to `story:agent-session-spec` and `story:agent-page-path-spec` stay as D8 set them.
`AGENTS.md` is the coordinator's in a wave of more than one unit (D7): this story hands over its
sentences below.

## Change

- `crates/html-conformance/expected/tokenizer.txt`, `tree-construction.txt`, `encoding.txt`:
  per data file and per tag the counts this story measures (cases that exist, run, pass), and
  per section of `html-sections` whose `data` files are in that group its mark, every one
  `incomplete`. Plain text a person reads,
  one record per line, the format documented at the top of the crate.
- `--check` in the harness binary, which fails, naming the file, tag or section and both
  numbers, when:
  - a case count per file or per tag differs from the `html-inventory` block (the data changed);
  - a run or pass count, per file or per tag, is below the expected one (a regression);
  - an expected count is below its value in the same file at the revision `--base` names, read
    with `git show <base>:<path>` (an expected count lowered to hide a regression); a file that
    does not exist at the base is not compared;
  - the sections in an expected file differ from the sections of `html-sections` whose `data`
    files are in that group, or a data file or a case tagged `not-run` belongs to no section;
  - a section with no `data` line has neither an `open` nor a `wpt` line (nothing would test it
    and nothing would keep it `incomplete`);
  - a section marked `complete` has a case of its files that does not pass or does not run, or
    the note lists for it an open requirement or a web-platform-tests test that does not run.
  A count above the expected one passes and prints that the expected file can be raised.
- A section with no `data` line (`the-end`, `speculative-html-parsing`) is in no expected file:
  it is printed from the note alone as `section <name> incomplete 0/0 <reason>`, the reason
  naming its `open` lines and its `wpt` tests not run. A `wpt` line for `html5lib_write.html` or
  `html5lib_write_single.html` counts as one test not run per `data` file of its section (its
  variants); a `wpt` path ending in `/` counts as one entry, written with the `/`.
- Output of `--check`: one line per section, `section <name> complete <n>/<n>` or
  `section <name> incomplete <pass>/<exist> <reason>`, where the reason names failing cases,
  cases not run, open requirements or tests not run; one line per tag,
  `tag <name> <pass>/<run>/<exist>`; and the total line. A partial pass is printed as a count and
  never as complete.
- `taskfiles/html-conformance.yml` (new): a task `html-conformance` with a variable `HTML_BASE` (a
  git revision, default `main`) that runs `--check --base {{.HTML_BASE}}` from the repository root
  into `.out/html-conformance.log` and prints every `section` and `tag` line and the total line.
  It runs the same whether the root `Taskfile.yml` includes it or it is called with
  `task --taskfile taskfiles/html-conformance.yml html-conformance`.
- Tests in `crates/html-conformance/tests/check.rs`, each over a small fixture of data, note
  blocks and expected files: `a_pass_count_below_the_expected_count_fails`,
  `a_tag_pass_count_below_the_expected_count_fails`,
  `a_case_count_that_differs_from_the_inventory_fails`,
  `an_expected_count_below_its_value_at_the_base_fails`,
  `a_section_map_that_differs_from_the_note_fails`, `a_data_file_in_no_section_fails`,
  `a_case_not_run_in_no_section_fails`, `a_section_marked_complete_with_a_failing_case_fails`,
  `a_section_marked_complete_with_a_case_not_run_fails`,
  `a_section_marked_complete_with_an_open_requirement_fails`,
  `a_section_marked_complete_with_a_web_platform_test_not_run_fails`,
  `a_count_above_the_expected_count_passes`,
  `a_section_without_a_data_file_is_printed_incomplete_with_its_reasons`,
  `a_section_without_a_data_file_and_without_open_or_wpt_lines_fails`,
  `a_write_wrapper_line_counts_one_test_per_data_file_of_its_section`.

Writes: `crates/html-conformance/` (the check, `expected/`, `tests/check.rs`),
`taskfiles/html-conformance.yml` (new). Not the root `Taskfile.yml`.

For the root `Taskfile.yml`, handed to the coordinator in the report (D16, D30):
- an `includes:` entry for `taskfiles/html-conformance.yml` under which the task is called
  `html-conformance`, written for the `task` version installed (the unit measures `task
  --version` and gives the exact lines);
- in `check`, the line `- task: html-conformance` after `- task: test`.

For `AGENTS.md`, handed over in the report (D7):
- "Goals", the sentence "Where the mark is recorded is not decided yet." becomes: "For the HTML
  parser the mark is recorded in `crates/html-conformance/expected/<group>.txt`, against the
  section map and the open requirements in `docs/design/html-parser.md`, and `task
  html-conformance` refuses a `complete` the rule does not allow. For the other standards where
  it is recorded is not decided yet."
- "Layout": "`crates/html` (`surf-html`): the HTML parser, standard library only.
  `crates/html-conformance` (`surf-html-conformance`, not shipped): the parser's conformance data,
  html5lib-tests (tokenizer and encoding, MIT) and web-platform-tests (tree construction, 3-Clause
  BSD) at the commits `docs/design/html-parser.md` pins, each with its licence, the harness and the check of the expected counts."
- "Commands": "`task html-conformance`: the parser against its conformance data; prints one line per
  section and per tag and fails when a count drops below the expected file or below its value at
  `HTML_BASE` (default `main`)."
- "The gate's numbers": "The parser's counts are in `crates/html-conformance/expected/` and
  printed by `task html-conformance`; they are not copied here."

## Acceptance

`cargo test -p surf-html-conformance` passes with the fifteen tests of `tests/check.rs` named in
Change, and `slot task --taskfile taskfiles/html-conformance.yml html-conformance` exits 0 printing
nine `section` lines, one per section of `html-sections`, each `incomplete` (`the-end` and
`speculative-html-parsing` as `incomplete 0/0`), eleven `tag` lines, one per tag of
`html-inventory`, and the total line `html-conformance: 9107 cases, 9095 run, <pass> passed`; the report holds the include entry and the `check` line
for the root `Taskfile.yml`.

## Out of scope

Raising any count or marking any section complete: the parser stories do that, each in the
expected file of its group.
