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
revision: 5
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

The grammars embed lines of the agent answer and do not redefine them. They refer to
`docs/agent-answer.md` as it stands after story `agent-change-and-log`: the `page` line, the
lines of the `changed` block and the element line are written into a recording exactly as that
document defines them, each on its own line after a fixed indentation, and read back by its
rules. A scenario's `expect` and `seen` lines are those same lines with every handle replaced by
its locator; this story defines that substitution and the locator, not the lines.

Reading those lines back is `surf-answer`'s (`docs/plan/agent-plan-revision-2.md`, D20):
`surf_answer::parse` turns answer lines into the typed answer and `surf_answer::unquote` reads one
quoted value; `agent-answer-format` writes both and every story that adds a block extends them.
The reader of `surf-scenario` calls them for every answer line of a recording and for every
quoted value of both formats (a locator's accessible name, an address, a body line), and writes
no parser of the answer grammar of its own. A line `surf_answer::parse` refuses is refused with
its line number in the recording. The writer writes answer lines as the session rendered them and
quotes with `surf-answer`'s quoting function.

The `exchange` block needs what the transport carries after story `agent-request-body-spec`: the
method and request body, and the address the response was served from after redirects inside
one request (`docs/plan/agent-plan-revision-1.md`, D14). Both stories are edges of this one.

## Change

- `docs/formats/surf-recording.md`: the grammar of `surf-recording 1`. First line `surf-recording
  1`; a header (`session <id>`, informational and never compared, D7); then, in the order they
  happened:
  - `action <seq> <command> ...` blocks, one per row of the action log, `open` and `close`
    included: the command, the handle and locator (kind, quoted accessible name, tree path) of the
    element it acted on, its input with secrets as references, the answer's `page` line and its
    `changed` lines;
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
- A new library crate `crates/scenario` (package `surf-scenario`), standard library only,
  `#![forbid(unsafe_code)]`, depending on `surf-answer` only: typed values for a recording, a scenario, a
  locator and a variable reference; `parse` and `write` for both formats. A malformed file is
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

Surfaces: `docs/formats/` (new), `crates/scenario/` (new). The root `Cargo.lock` gains
`surf-scenario` (the coordinator's file in a wave). Later writers of `crates/scenario`:
`runner-replay-transport` and `runner-record-session`, both ordered after this story.

## For AGENTS.md

"Layout": "`crates/scenario` (`surf-scenario`): reads and writes the recording and scenario
formats of `docs/formats/`. Standard library only."

## Acceptance

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
