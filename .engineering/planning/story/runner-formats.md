---
format: aep.planning-md/3
id: story:runner-formats
kind: story
status: active
title: Specify the recording and scenario formats, with golden files and one parser
summary: docs/formats holds both grammars; surf-scenario reads and writes them, held to golden files.
relations:
- decomposes: epic:e2e-scenario-runner
- depends_on: story:runner-decisions
- depends_on: story:agent-answer-format
- depends_on: story:html-document-port
- depends_on: story:agent-request-body-spec
- depends_on: story:agent-change-and-log
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:13Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:13Z", actor: "human:timo", revision: 5}
---
## Context

`story:runner-decisions` fixes that recordings and scenarios are surf's own text formats,
`surf-recording 1` and `surf-scenario 1` (Q1), with locators (Q2), exchanges matched by key and
answered with the recorded response address (D4), a scenario naming its recording by path (D5),
variable references for secrets (D6), `expect` and `seen` lines (D9), a recording that mirrors
the action log, `open` and `close` included (D9), and a reserved `detail` block kind with no
detailed line defined (D13). The epic requires both formats to be specified before the code that
writes and reads them, with golden files. This story is that specification and the one library
that reads and writes both, so every later story uses one parser and one writer.

**How a recording holds an answer, decided here once** (the note, Q1: answer lines are "the
answer's own lines, read with `surf_answer::parse`", and D20 of
`docs/plan/agent-plan-revision-2.md`). `surf_answer::parse` (`crates/answer/src/lib.rs`) reads a
whole answer: it refuses a text that does not start with the head line `surf 1 session <id>
<state>`. A recording cannot keep whole answers: the head line repeats the session identity, which
the note (D7) allows once, in the recording's header, and the answer of `open` holds the `token`
line, which a recording never holds (D7). So each `action` block keeps the answer as the session
rendered it **without its head line and its token line**, and this story adds to `crates/answer`
the entry point that reads such lines back:

- `surf_answer::parse_blocks(text) -> Result<Blocks, ParseError>`: the lines that follow the head
  and token lines of an answer, every block optional and in the grammar's order, read by the same
  per-block functions `parse` uses (the list `BLOCKS` without `parse_token`), so a block a later
  story adds to `parse` is read here too; a line outside the grammar is refused with its number.
  `Blocks` holds the fields of `Answer` other than `head` and `token`.
- `surf_answer::render_blocks(&Blocks) -> Result<String, RenderError>`, the inverse, by the same
  per-block render functions.
- A round-trip case in `crates/answer`: for every golden answer under
  `crates/answer/tests/golden/`, its lines after the head and token lines parse with
  `parse_blocks` to the blocks of `parse` of the whole file, and `render_blocks` of them gives the
  same lines back.

The reader of `surf-scenario` calls `parse_blocks` for the answer lines of a recording and
`surf_answer::unquote` for every other quoted value of both formats (a locator's accessible name,
an address, a body line), and writes no parser of the answer grammar of its own. The writer
writes the lines `render_blocks` gives and quotes with `surf-answer`'s quoting function.

A scenario's `expect` and `seen` lines are answer lines with every handle replaced by its
locator. This story defines that replacement once, as one function of `surf-scenario`,
`expected_lines(blocks, locators)`: it renders the `page` line and the `changed` lines of a
`Blocks` with `render_blocks` and writes each handle as the locator the map gives it.
`runner-scenario-from` calls it on the recorded blocks to write `expect` lines, and `runner-run`
calls it on the blocks of the actual answer before it compares, so the two sides of a comparison
are made by one function. Scenario lines are kept and compared as text; nothing parses them as
answer lines.

The `exchange` block needs what the transport carries after story `agent-request-body-spec`: the
method and request body, and the address the response was served from after redirects inside
one request (`docs/plan/agent-plan-revision-1.md`, D14). Both stories are edges of this one.

## Change

- `docs/formats/surf-recording.md`: the grammar of `surf-recording 1`. First line `surf-recording
  1`; a header (`session <id>`, informational and never compared, D7); then, in the order they
  happened:
  - `action <seq> <command> ...` blocks, one per row of the action log, `open` and `close`
    included: the command, the handle and locator (kind, quoted accessible name, tree path) of the
    element it acted on, its input with secrets as references, the answer without its head and
    token lines as `render_blocks` writes it, and one line `at <handle> <tree path>` per handle of
    that answer's `now` block, so that every handle the action's lines name has a locator (kind
    and name from the `now` line, tree path from the `at` line; a handle that left the document is
    found in the latest earlier answer that listed it). When one command adds two rows (`open <url>`:
    `open`, then `go`), its answer is in the block of the last of them and the other block holds no
    answer lines;
  - `exchange <seq> during <action-seq> <method> "<request address>"` blocks: the request body
    if any, then either `response <status> <media type> "<response address>"` with the body, or
    `failed <reason>`, or `unanswered`;
  - the reserved `detail` block kind (D13): a version 1 reader keeps its lines as opaque quoted
    lines and writes them back unchanged; no detailed line is defined here.

  Bodies are written as escaped quoted lines, so any byte sequence round-trips and the file stays
  printable ASCII; a person can edit a body with a text editor. Limits (line length, body size,
  number of blocks) are stated and a file past them is refused.
- `docs/formats/surf-scenario.md`: the grammar of `surf-scenario 1`. First line `surf-scenario
  1`; `recording "<relative path>"`; then `step <n> <command> <locator> [<input>]` blocks with
  `expect` and `seen` lines (D9). A locator is `<kind> "<accessible name>" at <tree path>`, the
  tree path written as element names with a 1-based index among siblings of the same name
  (`/html/body/form[1]/button[1]`; a notation of this repository, modelled on XPath from memory,
  not a claim of following it). A variable reference is `${NAME}`, `NAME` upper-case letters,
  digits and `_`.
- `crates/answer/src/lib.rs`: `Blocks`, `parse_blocks` and `render_blocks` as decided above.
  `crates/answer/tests/blocks.rs`: the round-trip case, as
  `parse_blocks_round_trips_every_golden_answer`. `docs/agent-answer.md`: one paragraph saying
  that the lines after the head and token lines are read and written alone by these two
  functions, for recordings.
- A new library crate `crates/scenario` (package `surf-scenario`), standard library only,
  `#![forbid(unsafe_code)]`, depending on `surf-answer` only: typed values for a recording, a scenario, a
  locator and a variable reference; `parse` and `write` for both formats; `expected_lines` as
  decided above. A malformed file is
  refused with its line number and what was expected, never a panic. `write(parse(x)) == x` for
  every golden file.
- Golden files under `crates/scenario/tests/golden/`: one recording and one scenario that each
  contain every block kind and line kind their grammar document defines (the recording a
  redirected `POST` exchange with a response address other than its request address, a body with
  bytes outside printable ASCII, a `${PASSWORD_1}` reference, an `open` and a `close` action, a
  failed and an unanswered exchange, and one `detail` block with an opaque line; the scenario an
  `expect` and a `seen` line, and a locator for each element kind), and one malformed file per
  refusal the grammars name.
- `crates/scenario/tests/formats.rs`: the round trips, the refusals, and a test that reads each
  grammar document's list of block and line kinds and fails when the matching golden file lacks
  one of them.

Surfaces: `docs/formats/` (new), `crates/scenario/` (new; `autotests = false` and `[[test]] name = "main"` in its `Cargo.toml`, `tests/main.rs` with one `mod` line per test file, the form of `story:one-test-binary-per-crate`), `crates/answer/src/lib.rs`,
`crates/answer/tests/blocks.rs` (new), `crates/answer/tests/main.rs` (one `mod` line), `docs/agent-answer.md` (one paragraph). Every earlier writer
of `crates/answer` and `docs/agent-answer.md` (`agent-answer-format`, `agent-cli-client`,
`agent-follow-links`, `agent-fill-pick`, `agent-press-submit`, `agent-back-forward`,
`agent-change-and-log`) is an ancestor of this story; no later runner story writes them. The root `Cargo.lock` gains
`surf-scenario` (the coordinator's file in a wave). Later writers of `crates/scenario`:
`runner-replay-transport` and `runner-record-session`, both ordered after this story.

## For AGENTS.md

"Layout": "`crates/scenario` (`surf-scenario`): reads and writes the recording and scenario
formats of `docs/formats/`. Standard library only; reads and writes answer lines through
`surf-answer`."

## Acceptance

`cargo test -p surf-answer` passes with `parse_blocks_round_trips_every_golden_answer`, and
`cargo test -p surf-scenario` passes with one round-trip test per golden file, one refusal test
per malformed golden file naming the line it refuses, and the coverage test showing that the
golden recording contains every block and line kind `docs/formats/surf-recording.md` defines and
the golden scenario every kind `docs/formats/surf-scenario.md` defines, and `cargo tree -p
surf-scenario -e normal` lists `surf-scenario` and `surf-answer` and no other package.

## Out of scope

Producing a recording from a session (`runner-record-session`), converting one into a scenario
(`runner-scenario-from`), answering requests from one (`runner-replay-transport`), running a
scenario (`runner-run`), the output of `surf run` (specified by `runner-run`), and any detailed
line (`epic:detailed-mode`).

## Domain relations

None rests on an `ess/1` document (`story:runner-decisions`, D10: no entity). The structure the
grammars fix comes from `story:runner-decisions`: recording to action, one to one with the
action log (D9); recording to exchange, one to many, owned by the file (D4); exchange to action,
many to one by sequence number (D4); scenario to recording, many to one by path (D5); scenario to
step, one to many, ordered (Q1).
