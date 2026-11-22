---
format: aep.planning-md/3
id: story:values-and-labels-before-fill
kind: story
status: draft
title: No password value is printed and a recorded label is cut, before the first fill command
summary: 'Four things the As built sections of round 1 left open become defects with the first command that fills a control or records a label: a password value printed, a recorded label uncut, five inputs with no effect, the kept text''s drop untested'
relations:
- decomposes: epic:agent-cli-target
scope:
- confidence: inferred
  path: AGENTS.md
- confidence: inferred
  path: README.md
- confidence: cited
  path: conformance/src/target.rs
- confidence: cited
  path: crates/page/src/label.rs
- confidence: inferred
  path: crates/page/tests
- confidence: cited
  path: crates/session/src/entries.rs
- confidence: cited
  path: crates/session/src/lib.rs
- confidence: inferred
  path: crates/session/tests
- confidence: cited
  path: crates/surf/src/lib.rs
- confidence: inferred
  path: crates/surf/tests
- confidence: inferred
  path: docs/badges/conformance.svg
- confidence: inferred
  path: docs/badges/make.sh
- confidence: cited
  path: docs/surf-command.md
- confidence: inferred
  path: generated/rust
- confidence: cited
  path: spec/domains/agent.yaml
- confidence: inferred
  path: spec/ess-inputs.yaml
- confidence: cited
  path: spec/scenarios
- confidence: inferred
  path: website/docs/how-it-is-built.md
- confidence: inferred
  path: website/docs/principles.md
- confidence: inferred
  path: website/docs/status.md
- confidence: inferred
  path: website/docs/what-works.md
revision: 11
---
## Why

Round 1 put links and forms into the specification. Its As built sections left four things open
that are harmless while no session command fills a control or records a label, and become
defects with the first that does (`story:agent-follow-links`, `story:agent-fill-pick`,
`story:agent-press-submit`):

1. A stored value is printed as it is. `story:agent-forms-spec`, As built, line 210:
   `surf::Report::lines` prints a control's value. Today that is the `element` line
   (`element_line`, `crates/surf/src/lib.rs:478-505`, the value at line 493), and a password's
   own `value` attribute is that value (`crates/page/src/lib.rs:705-707`). The second place a
   password value is printed today: the label rule takes the `value` attribute of a password
   input embedded in another element's name (`crates/page/src/label.rs:620-637`); from there it
   reaches the `ActionRecorded.label` entry and, through that entry, the view stream.
2. `ActionRecorded.label` is written uncut. `story:agent-links-spec`, As built, line 118: the entry
   writes the label with `optional` (`crates/session/src/entries.rs:151`), against the sentence
   of `crates/session/src/lib.rs:142-146` ("the one field that holds text from a page, the
   title", "An element's label, target and value are in no entry") and of
   `crates/session/src/entries.rs:19-22`. A label comes from the page, so its bytes are hostile.
   Its escaping already holds (`surf_answer::quote`); only the cut is missing.
3. `Follow.handle`, `Follow.label`, `Fill.handle`, `Pick.handle` and `Press.handle` are inputs
   with no effect. `story:agent-links-spec`, As built, line 121, and `story:agent-forms-spec`,
   As built, line 213: no outcome of `Follow`, `Fill`, `Pick` or `Press` uses them
   (`spec/domains/agent.yaml:476-704`). The record is a separate command, `RecordAction`
   (`spec/domains/agent.yaml:883-941`), whose `handle` and `label` the session process
   supplies; nothing in the specification connects them to the four commands' inputs, so
   `conformance/src/target.rs:1477-1485` passes any value and the suite stays green.
4. No test observes that the kept text of a document is dropped on a request other than
   `look`. `story:page-text-look`, As built, line 252. The drop is the line
   `self.rendered = None;` (`crates/session/src/lib.rs:457`); a change that removes it passes
   every test, because the kept text is also filtered by the current document
   (`crates/session/src/lib.rs:581-585`).

`story:agent-forms-spec`, As built, line 215, leaves open three more: `readonly`, a `select`
with `multiple` and the radio group of a button with no form owner or no name
(`spec/domains/document.yaml:735`, `306`, `798`). This story does not take them; see "Left
open".

## Owned surfaces

This story owns `surf::Report::lines`, the label rule of `crates/page/src/label.rs` and the
`ActionRecorded.label` entry of `crates/session/src/entries.rs`. It does not own the `now` block
(`story:agent-fill-pick`), the `changed` block (`story:agent-change-and-log`) or the live view
(`story:view-controls-and-changes`).

Password display: the answer's `now` block and `Report::lines` show `(set)` or `(empty)` for a
control of `type=password`, never its value, as design section 5.2 says
(`docs/design/agent-interface.md:257-258`: "A password's value is never printed: `(set)` or
`(empty)`."). The session log and the view keep the value's length as decision D22 says
(`docs/view-stream.md:207-210`, `docs/design/live-view.md:328-331`,
`crates/session/src/entries.rs:245`); this story does not overrule D22.

The derived `Debug` form of `Report` (`crates/surf/src/lib.rs:361`) and of a row of `Elements` or
`Controls` (`generated/rust/crates/surf-types/src/document.rs:1734,1790`) prints a value; it is
not printed outside tests. Checked by a stated reading in the unit report, not by a test: the
implementor lists every `{:?}`, `{:#?}` and `dbg!` under `crates/*/src` and says what each
formats outside `#[cfg(test)]`. On `main` at `c5c2895` none formats `Report` or a row: the hits
under `crates/session` and `crates/surf-cli` format an enum variant
(`crates/session/src/entries.rs:125`), a failure reason (`crates/session/src/lib.rs:1006`) and
an error (`crates/surf-cli/src/main.rs:153`); those of `crates/surfd/src/view.rs:518,547` are
under `#[cfg(test)]` (line 448).

## Change

1. `surf::Report::lines`: the last field of the `element` line of a row of kind `password` is
   `(set)` when its value is not empty and `(empty)` when it is, unquoted, never the value. Every
   other kind is printed as today. The stored value stays what a submission sends.
   `docs/surf-command.md:44` says so.
2. `crates/page/src/label.rs:620-637`: an embedded `input` of `type=password` adds nothing to
   another element's name; the other text states keep their value.
3. `crates/session/src/entries.rs:151`: `ActionRecorded.label` is written with
   `entries::Line::cut` (line 78) at `TITLE_ENTRY_BYTES` (1,024 bytes, line 45), followed by
   `label_bytes="<n>"` when cut. The sentences of `crates/session/src/lib.rs:142-146` and
   `crates/session/src/entries.rs:19-22` are rewritten to name the page texts an entry holds
   (the title of `DocumentParsed`, the label of `ActionRecorded`, and the `label` this story adds
   to `NavigationRequested`), their bound (`TITLE_ENTRY_BYTES`, 1,024 bytes) and their fields
   (`title_bytes`, `label_bytes`).
4. `spec/domains/agent.yaml`: the five inputs stay. The story adds what carries them toward the
   record: the request event each `requested` outcome emits holds the input, `NavigationRequested`
   (from `Follow`) gains `handle` and `label` as `Optional<String>`, `FillRequested`,
   `PickRequested` and `PressRequested` gain `handle`; the session stories then record the action
   with the values of that event. `generated/rust/` is regenerated; `conformance/src/target.rs`
   and `crates/session/src/entries.rs` map the new fields (a label cut as in 3). The refusal
   `unknown-handle` stays in the session stories (`story:agent-follow-links` case 6,
   `story:agent-fill-pick` case 13): handles are not modelled (`spec/domains/agent.yaml:476-479`).
   The existing scenarios observe the values: `agent-follow-navigates` (`handle: l1`,
   `label: Offers`), `agent-fill-sets-value` (`handle: f1`), `agent-pick-selects-option`
   (`handle: f3`) and `agent-press-submits-post` (`handle: b1`) each expect the input in the
   payload of the event they already check.
5. A test observes the kept text being dropped: `kept_text_is_dropped_on_a_request_other_than_look`
   in `crates/session/tests/memory.rs`, through a read-only `Session::kept_text_lines` (the
   number of lines kept, 0 when none), as `Session::held` is read there.

This story is implemented before `story:agent-follow-links` (edge `depends_on`), so before
`story:agent-fill-pick` and `story:agent-press-submit`, which come after that one.

## Acceptance

When the story is implemented:

1. Change 1. A test in `crates/surf/tests/` loads a page with `<input type=password
   value="pw-marker">`, `<input type=password>` and `<input type=text value="tx-marker">`:
   `Report::lines` has an `element` line of kind `password` ending in `(set)`, one ending in
   `(empty)`, no line containing `pw-marker`, and a `text` line ending in `"tx-marker"`. Today the
   first password line ends in `"pw-marker"`.
2. Change 2. A test in `crates/page/tests/label.rs`: the name of
   `<button>Code <input type=password value="pw-marker"></button>` holds `Code` and no
   `pw-marker`; with `type=text` it holds `pw-marker`. Today both hold `pw-marker`.
3. Change 3. A test in `crates/session/tests/events.rs`: `entries::event_line` of a constructed
   `ActionRecorded` (as `covered()` builds one, `crates/session/tests/events.rs:861-868`) with a
   label of 10,000 bytes, a line feed, a `"` and a bidirectional control among its first 100,
   gives a line whose `label` field quotes the first 1,024 bytes (cut at a character boundary)
   and is followed by `label_bytes="10000"`; a label of 1,024 bytes has no `label_bytes`. Today
   the whole label is written and no `label_bytes` exists. The two rewritten sentences name the
   label, `TITLE_ENTRY_BYTES` and `label_bytes` (read in the diff).
4. Change 4. For each of the five inputs, its named scenario fails when
   `conformance/src/target.rs` passes another value of it to the command
   (`agent-follow-navigates` for `Follow.handle` and `Follow.label`, `agent-fill-sets-value`,
   `agent-pick-selects-option`, `agent-press-submits-post`); the unit report shows each failing
   run with the value changed and the passing run without. Today each passes with any value.
5. Change 5. `kept_text_is_dropped_on_a_request_other_than_look`: after a `look` of a page
   `kept_text_lines` is above 0; after a following `go` it is 0. The test fails with
   `self.rendered = None;` (`crates/session/src/lib.rs:457`) removed; the unit report shows that
   run. Today no test fails with it removed.
6. Numbers. Before (`AGENTS.md`, "The gate's numbers", after round 1): `169 scenario(s) (24
   authored), 48 refusal(s)`. After, as change 4 adds no scenario file: 169 scenarios, 24
   authored, 48 refused; `REFUSALS_MAX` (`Taskfile.yml:56`) is 48, the refused count after. A
   different reading of `slot task suite` is restated in every file that states the counts and
   named in the unit report with its cause. The gate holds the refused count as an upper bound
   only (`Taskfile.yml:74`); nothing checks the scenario counts but these files.

## Left open

- `readonly`, `select multiple` and the radio group of a button with no form owner or no name
  (`spec/domains/document.yaml:735`, `306`, `798`): `story:agent-fill-pick` refuses `fill` of a
  `readonly` control and names the other two as open.

## Scope

Surfaces written: `crates/surf/src/lib.rs`, `crates/page/src/label.rs`,
`crates/session/src/entries.rs`, `crates/session/src/lib.rs`, `docs/surf-command.md`,
`spec/domains/agent.yaml`, `spec/scenarios/` (the four scenarios above),
`spec/ess-inputs.yaml` (only if a scenario file is added), `generated/rust/`,
`conformance/src/target.rs`, the test files `crates/surf/tests/`, `crates/page/tests/`,
`crates/session/tests/`, and, if the counts change, the files that state them: `AGENTS.md`
("The gate's numbers"), `README.md`, `docs/badges/make.sh` and `docs/badges/conformance.svg`,
`website/docs/status.md`, `website/docs/what-works.md`, `website/docs/principles.md`,
`website/docs/how-it-is-built.md`. `spec/domains/document.yaml` is not written.
