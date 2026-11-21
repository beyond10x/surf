---
format: aep.planning-md/3
id: story:runner-run
kind: story
status: active
title: 'surf run: run a scenario in process, replayed by default'
summary: One line per step, expected and actual lines on failure, exit 0 only when every step passed, same bytes every run.
relations:
- decomposes: epic:e2e-scenario-runner
- depends_on: story:runner-scenario-from
- depends_on: story:runner-replay-transport
- depends_on: story:runner-surf-cli-package
- depends_on: story:agent-change-and-log
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:13Z", actor: "human:timo", revision: 5}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:13Z", actor: "human:timo", revision: 6}
---
## Context

`surf run <scenario>` runs a scenario in one process, with no socket and no session process,
driving the same `agent` component; one line per step, pass or fail, and for a failure the
expected and the actual lines; exit 0 only when every step passed; replayed network by default,
live on request; the same result every time (epic, Outcome). `story:runner-decisions` fixes the
locator resolution (Q2), the unrecorded request (Q3), the variables (D6), stopping at the first
failure and the exit statuses (D8), and how expectations compare (D9). The formats and their
parser are `story:runner-formats`, the replay transport `story:runner-replay-transport`; the
subcommand dispatch is in `crates/surf-cli/src/main.rs` after `story:runner-scenario-from`. No
detailed line is compared: none exists in this epic (D13).

Stands on `epic:agent-cli-target` (through `agent-change-and-log`): the session core of
`crates/session` (`agent-session-process`), a library that opens a session over a
caller-supplied `Transport` with no socket and no process, and executes `Go`, `Follow`, `Back`,
`Forward`, `Fill`, `Pick`, `Press` and `CloseSession`, returning the typed answer
(`surf_answer::Answer`), whose `page`, `changed` and `now` blocks the run reads. On `epic:html-parser` (through `html-document-port`):
for each element in `now`, its node in the tree read through `Browser::document_tree`, for the
tiebreak by tree path.

## Change

- `crates/surf-cli`: the subcommand `surf run <scenario> [--live]`. It reads the scenario and,
  unless `--live`, its recording; reads every `${NAME}` from `SURF_VAR_<NAME>` before the first
  step and refuses a missing one (exit 2, naming it); builds the session core over the
  `ReplayTransport`, or over the `surf-fetch` transport with `--live`; then for each step resolves
  the locator against the current `now` (Q2), takes the action, and compares: the actual lines
  are `surf_scenario::expected_lines` (`runner-formats`) of the blocks of the core's typed answer,
  with the locator map of the current `now` and the elements' tree paths, the same function
  `surf scenario from` wrote the `expect` lines with; the step's `expect` lines must equal those
  lines with the lines equal to one of its `seen` lines left out, and `seen` lines are not
  checked (D9). An unrecorded request ends through `FailRequest` and
  `FailNavigation` and fails the step with its `unrecorded` line (Q3). The scenario and its
  recording are read by the reader of `surf-scenario`, which reads recorded answer lines with
  `surf_answer::parse_blocks` and quoted values with `surf_answer::unquote`
  (`docs/plan/agent-plan-revision-2.md`, D20). This story writes no parser of the answer grammar
  and no handle replacement of its own. The first failing step stops the run; every later step is
  printed `skip` (D8).
- Output, specified in `docs/formats/surf-run.md`: a first line naming the scenario and
  `replayed` or `live`; `pass <n> <command> <locator>`; for a failure `fail <n> ...` then
  indented `expected` and `actual` lines (for an unresolved locator: the locator, and the
  candidates or the elements of that kind; for an unrecorded request: `unrecorded <method>
  "<address>"`); `skip <n> ...` for every later step; a last line with the counts and the count of
  unused exchanges. Page-derived values quoted and escaped as in the answer. No time, no session
  identity, no absolute path not given on the command line.
- Exit statuses per D8. The usage text names the subcommand.
- `crates/surf-cli/tests/run.rs` over hand-written scenarios and recordings under
  `crates/surf-cli/tests/golden/run/`. In every case the standard output equals that case's
  golden output file under the same directory byte for byte, and the exit status is the one
  named. The nine cases:
  1. `every_step_passes_without_agent_or_socket`: a scenario whose recording holds a redirected
     `POST`; run with `PATH` emptied and `XDG_RUNTIME_DIR` unset, so none of `surf-agent`,
     `surfd` and `surf-session` can be started and no socket of theirs can exist; exit 0;
  2. `two_runs_print_the_same_bytes`: the scenario of case 1 run twice; both outputs equal the
     one golden file;
  3. `relabelled_button_fails_its_press_step`: the recorded body of the form page has the button
     labelled `"Log in"` where the scenario expects `"Sign in"`; the `press` step fails with its
     `expected` and `actual` lines, the later steps are skipped; exit 1;
  4. `same_kind_and_name_resolved_by_tree_path`: exit 0;
  5. `same_kind_and_name_with_no_matching_path_is_ambiguous`: exit 1, every candidate printed;
  6. `unrecorded_request_fails_its_step`: exit 1, the `unrecorded` line printed;
  7. `missing_variable_exits_2_before_any_step`: no step line printed;
  8. `malformed_scenario_exits_2_with_its_line`;
  9. `live_run_against_a_server_passes`: `--live` against a server the test starts; the golden
     output is compared after the test writes its port into the one place the golden file marks
     for it; exit 0.

Surfaces: `crates/surf-cli/src/` (a new module and `main.rs`), `crates/surf-cli/Cargo.toml`
(path dependencies on `surf-session`, `surf-scenario` and `surf-fetch`),
`crates/surf-cli/tests/run.rs`, `crates/surf-cli/tests/golden/run/`, `docs/formats/surf-run.md`.
It does not write `crates/scenario` or `crates/session`. Follows `runner-scenario-from` on
`crates/surf-cli/src/main.rs`; `runner-e2e` follows it on `crates/surf-cli/tests/`.

## For AGENTS.md and README.md

- `AGENTS.md`, "Commands": "`target/debug/surf run <scenario> [--live]`: run a scenario in one
  process, its network replayed from the recording unless `--live`. Exit 0 when every step
  passed, 1 when one failed, 2 usage or an unreadable file, 3 stopped. `docs/formats/surf-run.md`
  gives the output."
- `README.md`, section "Record and run a scenario", second half: "`surf run login.scenario`
  replays it without the network; `--live` runs it against the site. A password typed while
  recording is read from `SURF_VAR_PASSWORD_1` and never from an argument. The first failing step
  stops the run and prints what was expected and what was found."

## Acceptance

`cargo test -p surf-cli` passes with the nine named `run` cases, each printing exactly its golden
output and exiting with its named status, among them `every_step_passes_without_agent_or_socket`
run with `PATH` emptied and `XDG_RUNTIME_DIR` unset, and `cargo tree -p surf-cli --edges normal`
lists only crates of this repository.

## Out of scope

Recording (`runner-record-session`). The end-to-end case through `surf-agent` (`runner-e2e`). A
fixed clock and a seeded random source, which wait for scripts. Detailed lines
(`epic:detailed-mode`). HTTPS.

## Domain relations

A locator and an actionable element of `surf.document`, zero or one resolved at run time: rests
on the actionable elements owned by a document that `html-document-spec` declares, which no
`ess/1` document on `main` holds yet (`spec/domains/document.yaml`, the `UNMAPPED:` marker); the
resolution rule is `story:runner-decisions`, Q2.
