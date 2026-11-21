---
format: aep.planning-md/3
id: story:runner-scenario-from
kind: story
status: active
title: 'surf scenario from: turn a recording into an editable scenario'
summary: Steps with locators in place of handles, structural lines expected, detailed lines kept unasserted.
relations:
- decomposes: epic:e2e-scenario-runner
- depends_on: story:runner-formats
- depends_on: story:runner-surf-cli-package
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:14Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:14Z", actor: "human:timo", revision: 5}
---
## Context

A recording is not edited into a test by hand: `surf scenario from <recording>` writes a scenario
a person or an agent can edit (epic, Outcome). The step keeps the action with a locator in place
of the handle, and the expectations after it: the page's status and address and the `changed`
lines, marked `expect` (`story:runner-decisions`, D9). `open` and `close` become no step, because
`surf run` opens a fresh session before the first step and closes it after the last (D9). A
recording holds no detailed line in this epic (D13); a `detail` block, if a later recording has
one, is not copied. The conversion is a function from one parsed recording to one scenario, both
in the formats of `story:runner-formats`, so it is tested on golden recordings and needs no
session. The answer lines a recording holds are read with `surf_answer::parse` and quoted values
with `surf_answer::unquote` (`docs/plan/agent-plan-revision-2.md`, D20: written by
`agent-answer-format` and extended by each story that adds a block), through the reader of
`surf-scenario`; this story writes no parser of the answer grammar of its own.

The `surf` binary is in `crates/surf-cli` (`story:runner-surf-cli-package`).

## Change

- `crates/surf-cli`: the subcommand `surf scenario from <recording> [--out <file>]`. Default
  output path: the recording's path with its extension replaced by `.scenario`. The scenario
  names the recording by a path relative to the scenario's directory (D5). Every handle in a
  copied line is replaced by the locator of the element it named in that action's answer; a
  handle the recording cannot map is refused, naming the action. Variable references are copied
  as they are. The file is created with mode 0600 and never overwrites (D7).
- Exit 0 written; 2 usage, an unreadable or malformed recording (the parser's line number in the
  message) or an existing output file; 3 a write that failed. The usage text of `surf` names the
  subcommand.
- `crates/surf-cli/tests/scenario_from.rs`, thirteen named tests. Golden pairs under
  `crates/surf-cli/tests/golden/scenario-from/`, a recording and the scenario it converts into;
  in each pair test the written scenario equals the golden one byte for byte and the exit status
  is 0:
  1. `login_session_open_and_close_make_no_step`: `open`, `go`, `follow`, `fill`, `press`,
     `close` become the four steps `go`, `follow`, `fill`, `press`;
  2. `same_kind_and_name_carry_their_tree_paths`: two elements of one kind and name, both
     locators with their tree paths;
  3. `password_reference_is_copied`: a `${PASSWORD_1}` reference copied as it is;
  4. `redirected_post_keeps_the_response_address`: the step after a `POST` answered by 303
     expects the page at the response address;
  5. `failed_exchange_converts`: a step whose page line is `page failed`;
  6. `detail_block_is_not_carried`: a recording with a `detail` block; the scenario has none of
     its lines.

  Refusals, each asserting the exit status, a reason on standard error and that no scenario file
  was written:
  7. `no_recording_argument_exits_2`;
  8. `unreadable_recording_exits_2`: a path that does not exist;
  9. `malformed_recording_exits_2_naming_its_line`: the reason holds the line number the parser
     gives;
  10. `unmappable_handle_exits_2_naming_the_action`: a `changed` line naming a handle no answer of
      the recording showed;
  11. `existing_output_exits_2_and_is_unchanged`: the existing file's bytes are the same after;
  12. `unwritable_output_directory_exits_3`: an output directory of mode 0500.

  File mode:
  13. `scenario_is_created_user_only`: the written file has mode 0600.

Surfaces: `crates/surf-cli/src/` (a new module and the subcommand dispatch in `main.rs`),
`crates/surf-cli/Cargo.toml` (path dependency on `surf-scenario`),
`crates/surf-cli/tests/scenario_from.rs`, `crates/surf-cli/tests/golden/scenario-from/`. It does
not write `crates/scenario`. On `crates/surf-cli` it follows `runner-surf-cli-package` and every
agent and html story that writes the binary (`agent-fetch-redirects`, `agent-request-body-spec`,
`agent-fetch-post`, `agent-form-port`, `html-document-port`, all ancestors through
`runner-formats`); `runner-run` follows it.

## For AGENTS.md and README.md

- `AGENTS.md`, "Commands": "`target/debug/surf scenario from <recording> [--out <file>]`: write
  the scenario of a recording, `docs/formats/surf-scenario.md`."
- `README.md`, a section "Record and run a scenario", first half: "`surf scenario from
  login.recording` writes `login.scenario`: one step per action, each with the element it acts on
  named by kind and label, and the lines expected after it. Edit it freely; a line marked `seen`
  is kept and not checked."

## Acceptance

`cargo test -p surf-cli` passes with the thirteen named tests of `scenario_from.rs`: the six golden
pairs, each written scenario equal to its golden file byte for byte, the six refusals 7 to 12
with their exit statuses, and `scenario_is_created_user_only`.

## Out of scope

Running the scenario (`runner-run`). Choosing smarter expectations than D9 names.

## Domain relations

Scenario to recording, many to one by relative path: `story:runner-decisions`, D5; no `ess/1`
document.
