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
scope:
- confidence: inferred
  path: .github/workflows/check.yml
- confidence: cited
  path: crates/html-conformance
- confidence: cited
  path: taskfiles/html-conformance.yml
revision: 9
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:11Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:11Z", actor: "human:timo", revision: 5}
---
## Context

After story `html-conformance-harness`, `cargo run -p surf-html-conformance` prints how many
cases of its two sources (html5lib-tests and web-platform-tests, `docs/design/html-parser.md` item
3) exist, run and pass, and nothing holds those numbers. Its lines
(`crates/html-conformance/src/runner.rs`, `Report::lines`; documented in `src/lib.rs`):

- one per data file, in the order of the inventory: `file <path> <pass>/<run>/<exist>`, followed
  by ` not-run:<reason>=<n>` for each reason cases of that file are not run;
- one per tag, in the order of the `tag` lines of `html-inventory`, counted over all three
  groups: `tag <name> <pass>/<run>/<exist>`;
- the total: `html-conformance: <exist> cases, <run> run, <pass> passed`.

Measured on 2026-11-21 (the crates `html` and `html-conformance` and the note copied to a scratch
workspace, `cargo run -p surf-html-conformance`): 93 lines, 81 `file`, 11 `tag`, and
`html-conformance: 9107 cases, 9095 run, 5 passed`. The five passing cases are all in
`html5lib-tests/tokenizer/test3.test` (`5/1802/1802`): one tagged `markup`
(`tag markup 1/1979/1979`) and four `text` (`tag text 4/328/328`). Every other `file` line is
`0/<n>/<n>`, or `0/0/<n>` with its `not-run:` fields for the seven files of not-run cases.

`AGENTS.md` ("Goals") says a section of a standard is complete only when every requirement in it
is specified and every test for it passes, and "Where the mark is recorded is not decided yet."
`docs/design/html-parser.md` decides it: one expected file per group of the inventory,
`crates/html-conformance/expected/<group>.txt` with `<group>` `tokenizer`, `tree-construction` or
`encoding` (item 9, the `file` record), and the fenced block `html-sections` (item 8) that puts
each data file in one section and lists, per section, the requirements the note leaves out
(`open`) and the web-platform-tests tests that do not run (`wpt`). `task check` (`Taskfile.yml`)
is the gate; it does not run the harness.

`html-sections` today: nine `section` lines, 81 `data` lines (one per `file` of
`html-inventory`), 10 `open` and 82 `wpt` lines. No section has data files in two groups:
`tokenizer` holds `tokenization`, `named-character-references` and
`coercing-an-html-dom-into-an-infoset`; `tree-construction` holds `tree-construction`,
`parsing-main-inforeign` and `parsing-html-fragments`; `encoding` holds `the-input-byte-stream`.
Two sections have no `data` line (`the-end`, `speculative-html-parsing`). The `wpt` lines naming
`html/syntax/parsing/html5lib_write.html` and `html5lib_write_single.html` stand for their
variants `?file=<f>`, one for each `.dat` file of that section; a `wpt` path ending in `/` stands
for every test under that directory. Every section has at least one `open` or `wpt` line, so
every section is `incomplete` today whatever its counts.

The tags are not per group: `not-run` has cases in all three groups (4 in `tokenizer`, the
`xmlViolation.test` file; 6 in `tree-construction`, the four `scripted_*.dat`; 2 in `encoding`),
and the harness prints tag lines only over all groups. A group's expected file therefore holds
tag counts over that group's cases, which the harness does not print yet.

Wave 2 routed one finding here: the test `every_case_has_exactly_one_tag`
(`crates/html-conformance/tests/inventory.rs:110`) can check only that each tag name is declared
and that the sums agree, because `Case::tag` (`src/cases.rs`) holds one `Tag`: "exactly one" is
the type's, not the test's. The per-file breakdown by tag is already compared with the inventory
by `the_case_count_per_tag_equals_the_inventory` in the same file.

This epic is judged by its conformance counts, so the check is built so that no later story can
meet its acceptance by editing a number: the section map comes from the note, not from a file a
parser story writes; every expected count is compared with its value at a base revision; and
`complete` is refused unless both halves of the rule in `AGENTS.md` hold.

Ordering: this story writes no line of the root `Taskfile.yml`. Its task goes to a file of its
own, `taskfiles/html-conformance.yml`, and the include line and the line in `check` are handed to
the coordinator (`docs/plan/agent-plan-revision-1.md`, D16; `docs/plan/agent-plan-revision-2.md`,
D30). Its edges to `story:agent-session-spec` and `story:agent-page-path-spec` stay as D8 set them.
`AGENTS.md` is the coordinator's in a wave of more than one unit (D7): this story hands over its
sentences below.

## Change

- `crates/html-conformance/expected/tokenizer.txt`, `tree-construction.txt`, `encoding.txt`, plain
  text, one record per line, the format documented at the top of the crate (`src/lib.rs`):
  - the `file` lines of that group's data files, exactly as the harness prints them;
  - one `tag <name> <pass>/<run>/<exist>` line per tag that has a case in that group, counted
    over that group's cases, in the order of the inventory's `tag` lines;
  - one `section <name> incomplete` line per section of `html-sections` whose `data` files are
    in that group, in the note's order.
  Their content today, from the measured output:
  - `tokenizer.txt`: 14 `file` lines, `test3.test` `5/1802/1802`, `xmlViolation.test`
    `0/0/4 not-run:xml-infoset=4`, the other twelve `0/<n>/<n>`; `tag markup 1/1979/1979`,
    `tag text 4/328/328`, `tag charref 0/4739/4739`, `tag not-run 0/0/4`; the sections
    `tokenization`, `named-character-references`, `coercing-an-html-dom-into-an-infoset`.
  - `tree-construction.txt`: 62 `file` lines, the four `scripted_*.dat` `0/0/<n>
    not-run:needs-script=<n>` (1, 1, 2, 2), the other 58 `0/<n>/<n>`; `tag body 0/476/476`,
    `tag formatting 0/22/22`, `tag tables 0/39/39`, `tag template 0/123/123`,
    `tag foreign 0/222/222`, `tag mixed 0/1085/1085`, `tag not-run 0/0/6`; the sections
    `tree-construction`, `parsing-main-inforeign`, `parsing-html-fragments`.
  - `encoding.txt`: `file html5lib-tests/encoding/chardet/test_big5.txt 0/0/1
    not-run:autodetect=1`, `file html5lib-tests/encoding/scripted/tests1.dat 0/0/1
    not-run:needs-script=1`, `file html5lib-tests/encoding/test-yahoo-jp.dat 0/1/1`,
    `file html5lib-tests/encoding/tests1.dat 0/59/59`, `file html5lib-tests/encoding/tests2.dat
    0/22/22`; `tag encoding 0/82/82`, `tag not-run 0/0/2`; the section `the-input-byte-stream`.
  The unit writes them from the harness's output on its own tree and reports any line that
  differs from this list as a finding, not as a number to reach.
- The harness: tag counts per group (the `Report` gains them); the `tag` lines it prints stay
  the totals over all groups, and the check compares them with the sum of the three files.
- `--check --base <revision>` in the harness binary, which fails, naming the file, tag or
  section and both numbers, when:
  - a case count per file or per tag differs from the `html-inventory` block (the data changed);
  - a run or pass count, per file or per tag, is below the expected one (a regression);
  - an expected count is below its value in the same file at `<revision>`, read with `git show
    <revision>:<path>` (an expected count lowered to hide a regression); a file that does not
    exist at the base is not compared; a revision git cannot resolve fails, naming it, and is
    never skipped;
  - the sections in an expected file differ from the sections of `html-sections` whose `data`
    files are in that group, or a data file or a case tagged `not-run` belongs to no section;
  - a section with no `data` line has neither an `open` nor a `wpt` line (nothing would test it
    and nothing would keep it `incomplete`);
  - a section marked `complete` has a case of its files that does not pass or does not run, or
    the note lists for it an `open` line or a `wpt` line.
  A count above the expected one passes and prints that the expected file can be raised.
- A section with no `data` line is in no expected file: it is printed from the note alone as
  `section <name> incomplete 0/0 <reason>`, the reason naming its `open` lines and its `wpt`
  tests. A `wpt` line for `html5lib_write.html` or `html5lib_write_single.html` counts as one
  test not run per `data` file of its section (its variants); a `wpt` path ending in `/` counts
  as one entry, written with the `/`.
- Output of `--check`: one line per section, `section <name> complete <n>/<n>` or `section <name>
  incomplete <pass>/<exist> <reason>`, where the reason names failing cases, cases not run, open
  requirements or tests not run; the 11 `tag` lines as the harness prints them; and the total
  line. A partial pass is printed as a count and never as complete. Today's sections, `<exist>`
  from the inventory: `the-input-byte-stream` 0/84, `tokenization` 5/2836,
  `named-character-references` 0/4210, `tree-construction` 0/1809, `parsing-main-inforeign`
  0/83, `parsing-html-fragments` 0/81, `the-end` 0/0, `speculative-html-parsing` 0/0,
  `coercing-an-html-dom-into-an-infoset` 0/4; their `<exist>` sum to 9107.
- `taskfiles/html-conformance.yml` (new): a task `html-conformance` that runs from the repository
  root as `taskfiles/cli.yml` does (`dir: '{{.TASKFILE_DIR}}/..'`), with a variable `HTML_BASE`
  (a git revision, default `main`, settable on the command line and from the environment; the
  unit measures both with the `task` installed, 3.54.0 here, and the 3.52.0 of CI), and runs
  `--check --base {{.HTML_BASE}}` into `.out/html-conformance.log`, printing every `section` and
  `tag` line and the total line. It runs the same whether the root `Taskfile.yml` includes it or
  it is called with `task --taskfile taskfiles/html-conformance.yml html-conformance`.
- `.github/workflows/check.yml`: the checkout fetches the history (`fetch-depth: 0`) and the job
  sets `HTML_BASE: origin/main`, because a checkout of depth 1 has no `main` on a pull request
  and the check refuses a base it cannot resolve.
- `crates/html-conformance/tests/inventory.rs`: `every_case_has_exactly_one_tag` is renamed
  `every_case_tag_is_declared_in_the_inventory`, which is what it checks; its body stays.
- Tests in `crates/html-conformance/tests/check.rs`, each over a small fixture of data, note
  blocks and expected files, the base cases in a git repository the test creates under
  `CARGO_TARGET_TMPDIR`: `a_pass_count_below_the_expected_count_fails`,
  `a_tag_pass_count_below_the_expected_count_fails`,
  `a_case_count_that_differs_from_the_inventory_fails`,
  `an_expected_count_below_its_value_at_the_base_fails`,
  `a_base_git_cannot_resolve_fails`,
  `a_section_map_that_differs_from_the_note_fails`, `a_data_file_in_no_section_fails`,
  `a_case_not_run_in_no_section_fails`, `a_section_marked_complete_with_a_failing_case_fails`,
  `a_section_marked_complete_with_a_case_not_run_fails`,
  `a_section_marked_complete_with_an_open_requirement_fails`,
  `a_section_marked_complete_with_a_web_platform_test_not_run_fails`,
  `a_count_above_the_expected_count_passes`,
  `a_section_without_a_data_file_is_printed_incomplete_with_its_reasons`,
  `a_section_without_a_data_file_and_without_open_or_wpt_lines_fails`,
  `a_write_wrapper_line_counts_one_test_per_data_file_of_its_section`,
  `the_tag_counts_of_the_groups_sum_to_the_tag_lines` (over the real data: `not-run` 4, 6 and 2).

Writes: `crates/html-conformance/` (the check, `src/`, `expected/`, `tests/check.rs`,
`tests/inventory.rs`), `taskfiles/html-conformance.yml` (new), `.github/workflows/check.yml`. Not
the root `Taskfile.yml`. No other wave 4 story writes any of these.

For the root `Taskfile.yml`, handed to the coordinator in the report (D16, D30):
- an `includes:` entry for `taskfiles/html-conformance.yml` under which the task is called
  `html-conformance`, written as the entries of `cli` and `site` are (`flatten: true`, `optional:
  true`);
- in `check`, the line `- task: html-conformance` after `- task: test`.

For `AGENTS.md`, handed over in the report (D7):
- "Goals", the sentence "Where the mark is recorded is not decided yet." becomes: "For the HTML
  parser the mark is recorded in `crates/html-conformance/expected/<group>.txt`, against the
  section map and the open requirements in `docs/design/html-parser.md`, and `task
  html-conformance` refuses a `complete` the rule does not allow. For the other standards where
  it is recorded is not decided yet."
- "Layout", the `crates/html-conformance` line gains: "the expected counts per group under
  `expected/`, and `--check`, which holds them."
- "Commands": "`task html-conformance`: the parser against its conformance data; prints one line
  per section and per tag and fails when a count drops below the expected file or below its value
  at `HTML_BASE` (default `main`; CI sets `origin/main`)."
- "The gate's numbers": "The parser's counts are in `crates/html-conformance/expected/` and
  printed by `task html-conformance`; they are not copied here."

## Acceptance

`cargo test -p surf-html-conformance` passes with the 17 tests of `tests/check.rs` named in
Change and the renamed test of `tests/inventory.rs`; `slot task --taskfile
taskfiles/html-conformance.yml html-conformance` exits 0 and prints the nine `section` lines of
Change, each `incomplete` with the counts given there, the 11 `tag` lines the harness prints
today (`tag markup 1/1979/1979`, `tag text 4/328/328`, the other nine `0/<run>/<exist>`, `tag
not-run 0/0/12`), and `html-conformance: 9107 cases, 9095 run, 5 passed`; the three expected
files hold 14, 62 and 5 `file` lines, 4, 7 and 2 `tag` lines and 3, 3 and 1 `section` lines
(`grep -c` on each); the report holds the include entry and the `check` line for the root
`Taskfile.yml`.

## Out of scope

Raising any count or marking any section complete: the parser stories do that, each in the
expected file of its group.

## Revised 2026-11-21, before wave 4

- The harness's output format stated from `src/runner.rs` (`file`, `tag`, total lines), and the
  expected files' content from its measured output (5 passes, all in `test3.test`), not from the
  design note.
- Tag counts per group are new: `not-run` spans all three groups (4, 6, 2) and the harness prints
  tags over all groups only; the old text asked for per-tag counts per file it could not write.
- The acceptance's total line is `5 passed`, not `<pass>`; section and file counts are given.
- A base git cannot resolve fails instead of passing unread, and `check.yml` fetches history with
  `HTML_BASE: origin/main`, because a depth-1 checkout has no `main` on a pull request: without
  this the gate would fail in CI or skip the floor.
- The wave 2 finding: `every_case_has_exactly_one_tag` is renamed to what it checks; the per-file
  tag breakdown is already compared by `the_case_count_per_tag_equals_the_inventory`.
- Tests: 17, adding `a_base_git_cannot_resolve_fails` and
  `the_tag_counts_of_the_groups_sum_to_the_tag_lines`.
