---
format: aep.planning-md/3
id: story:agent-fill-pick
kind: story
status: active
title: now lists form controls, and fill and pick change them
summary: Control lines with handles, kinds, flags and values, passwords never printed; fill (also from standard input) and pick, refusing hidden, disabled and wrong-kind controls.
relations:
- decomposes: epic:agent-cli-target
- depends_on: story:agent-follow-links
- depends_on: story:agent-form-port
- depends_on: story:html-document-port
- depends_on: story:agent-session-events
- depends_on: story:page-text-outline
scope:
- confidence: inferred
  path: AGENTS.md
- confidence: inferred
  path: CHANGELOG.md
- confidence: inferred
  path: README.md
- confidence: inferred
  path: Taskfile.yml
- confidence: cited
  path: cli/surf-agent.yaml
- confidence: inferred
  path: conformance/src/target.rs
- confidence: inferred
  path: crates/agent-tasks/status.txt
- confidence: cited
  path: crates/agent/src/cli.rs
- confidence: cited
  path: crates/agent/src/client.rs
- confidence: cited
  path: crates/agent/src/wire.rs
- confidence: inferred
  path: crates/agent/tests/drift.rs
- confidence: cited
  path: crates/answer/src/lib.rs
- confidence: inferred
  path: crates/answer/tests
- confidence: cited
  path: crates/session/src/lib.rs
- confidence: cited
  path: crates/session/tests
- confidence: cited
  path: crates/surfd/src/daemon.rs
- confidence: cited
  path: docs/agent-answer.md
- confidence: inferred
  path: docs/badges/conformance.svg
- confidence: inferred
  path: docs/badges/make.sh
- confidence: cited
  path: docs/design/agent-interface.md
- confidence: cited
  path: generated/rust
- confidence: inferred
  path: spec/domains/agent.yaml
- confidence: cited
  path: spec/domains/document.yaml
- confidence: inferred
  path: spec/ess-inputs.yaml
- confidence: inferred
  path: spec/scenarios
- confidence: inferred
  path: website/docs/how-it-is-built.md
- confidence: inferred
  path: website/docs/principles.md
- confidence: inferred
  path: website/docs/status.md
- confidence: inferred
  path: website/docs/what-works.md
revision: 18
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:09Z", actor: "human:timo", revision: 5}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:09Z", actor: "human:timo", revision: 6}
---
## Context

Step 4 of design section 9; design section 5.2 (control kinds, `(set)` and `(empty)` for a
password, the flags), section 4 (refusals), section 8 (typed values). `agent-forms-spec` specifies
`Fill` and `Pick`; `agent-form-port` fills the control rows; `agent-follow-links` prints the `now`
block and assigns handles.

Needs from `epic:html-parser`: the control rows of `html-document-spec` (kind, label, value, the
flags `hidden`, `disabled`, `checked`, `required`), with an element identity that stays the same
while the document lives (`html-document-port`).

Decision D11: the value printed for a select is derived from the selectedness of its options; no
stored select value exists.

Decision D13, recorded here as a reversible decision the operator can overrule (taken by the
coordinating agent on 2026-11-21, scope critic, round 1, finding 2): `fill` also reads the value
from standard input (`fill <handle> -`). A value given as an argument word is readable by other
local users, as the token would be; the epic gains the constraint "a secret value is never an
argument word", and `README.md` says to use `fill <handle> -` for one.

## Change

- `crates/session`: control lines in `now`: handle `f<n>` for a control, `b<n>` for a button, as
  `surf_page::handles` (`crates/page`, added by `story:page-text-outline`) gives them, by the rules links follow;
  kind; visibility as for links; label; state: the value quoted, `(set)` or
  `(empty)` for a password and never its value; `disabled`, `checked`, `required`; for a select the
  label of its selected option, quoted. `fill <handle> <text>` and `fill <handle> -`;
  `pick <handle> <option>` for a select (by option label, then value), `pick <handle> on|off` for a
  checkbox, `pick <handle> on` for a radio. Refused, status 1: an unknown handle, a hidden or
  disabled control, a control of another kind, an unknown option, a refusal of the document command.
- `crates/answer`, `docs/agent-answer.md`: the control and button lines, each with its `parse`
  beside its rendering (decision D20 of `docs/plan/agent-plan-revision-2.md`).
- The event entries exist already: `FillRequested`, `PickRequested`, `ValueSet`,
  `CheckednessSet` and `OptionSelected` are mapped (`crates/session/src/entries.rs:242-267`) and
  covered (`crates/session/tests/events.rs:858`). This story adds no mapping; case 17 tests them.
  Decision D22, corrected to what the code decided: the `value` of `FillRequested` is written as
  its length in UTF-8 bytes, `<n> bytes`, for every control, never the value
  (`crates/session/src/entries.rs:29-30`, `:245`; `crates/session/tests/events.rs:1044` expects
  `6 bytes`). The earlier sentence of this story (`<n> characters` for a password, every other
  value quoted) is replaced by it.
- The `did` and `path` lines of `fill` and `pick`. `did fill "<handle>"` and `did pick
  "<handle>"`: the handle quoted, as the caller typed it, the one quoted argument
  `docs/agent-answer.md` and `crates/answer` build for a `did` line. The design's example
  `did fill f1` (`docs/design/agent-interface.md:218`) is corrected by this story to
  `did fill "f1"`. The `path` entry takes the form
  `story:agent-follow-links` gives an action on an element (`spec/domains/agent.yaml:302-304`):
  `<n> fill f1` as the design writes it (`docs/design/agent-interface.md:223`) and `<n> pick f3`
  by the same rule, the handle bare, never the value typed or the option picked. The reasons
  `hidden` (if `agent-follow-links` has not added it), `disabled`, `wrong-kind` and
  `unknown-option` go into the reason table with exit status 1, into `RefusalReason` and its
  `RefusalReason::ALL` (`crates/answer/src/lib.rs:150`), into the exhaustive `match` of
  `crates/surfd/src/daemon.rs:815-821`, and into a test.
- `fill` of a `readonly` control is refused, status 1, `did refused readonly` (the reason
  `readonly` into the reason table, `RefusalReason`, `RefusalReason::ALL`, the `match` of
  `crates/surfd/src/daemon.rs` and a test), counted as case 19 below. It needs the element's
  `readonly`, which `spec/domains/document.yaml:735` does not store yet ("What would settle it: a
  field of the element when `fill` must refuse it"): the story adds that field, so
  `spec/domains/document.yaml`, its scenarios, `spec/ess-inputs.yaml`, `generated/rust/`,
  `conformance/src/target.rs` and, when the suite's counts change, the files that state them
  (`AGENTS.md` "The gate's numbers", `README.md`, `docs/badges/make.sh` and its svg,
  `website/docs/{status,what-works,principles,how-it-is-built}.md`, `Taskfile.yml`
  `REFUSALS_MAX`) join the surfaces written. A `select` with `multiple` and the radio group of
  a button with no form owner or no name stay open (`spec/domains/document.yaml:306`, `798`;
  `story:agent-forms-spec`, As built, line 215).
- `cli/surf-agent.yaml`, `crates/agent`: `fill` and `pick`, together, so the drift test holds.
  `fill <handle> -` reads the value from standard input, which the command line has no source
  for today (`crates/agent/src/cli.rs` declares standard input for the token only,
  `--token-stdin` at `:149`): `crates/agent/src/cli.rs` and `crates/agent/src/client.rs` read
  it, and `crates/agent/src/wire.rs` and its module documentation (the wire format, `arg url`,
  `arg form`, `arg line` at `:31-34`) gain the argument that carries it.
- `crates/session/tests/`, the counted cases, against a server the test starts:
  1. the `now` block of a form with a text field, a password, a checkbox, a radio group, a select,
     a textarea, a file control, a disabled field and a required field equals its golden text;
  2. `fill` a text field: its line shows the value; 3. `fill <h> -` a password from standard
     input: no answer, no `path` line and no action row holds it; 4. `fill` a password as a word:
     its line shows `(set)`; 5. `pick` a select option by label; 6. by value; 7. `pick` a checkbox
     `on` then `off`; 8. `pick` a radio: the others of its group show unchecked;
  9. `fill` a hidden control: `hidden`; 10. a disabled control: `disabled`; 11. `fill` a checkbox:
     `wrong-kind`; 12. `pick` an unknown option: `unknown-option`; 13. an unknown handle;
  forging, each quoted and escaped with no extra line: 14. a value filled with a line feed, `"` and
  a bidirectional control; 15. a label; 16. an option label from the page; each with the same
  three characters;
  17. `fill_events_map_with_their_fields`, with `surfd` run in the test process (decision D29):
      after case 3 no entry of the session log holds the password and the `FillRequested` entry
      has its value as `<n> bytes`; after case 2 the `FillRequested` entry has the value filled as
      `<n> bytes` too; neither is `unmapped`;
  18. round trip: `surf_answer::parse` of the answer of case 1 and of each answer of cases 14 to
      16 gives the typed answer, and rendering that gives the same lines byte for byte;
  19. `fill` a `readonly` text field: status 1, `readonly`.

Surfaces written: `crates/session/`, `crates/answer/`, `docs/agent-answer.md`,
`cli/surf-agent.yaml`, `crates/agent/`, `crates/surfd/src/daemon.rs`,
`docs/design/agent-interface.md` (line 218), and for `readonly` the specification files and count
files named above.

## For README.md (handed over in the unit report, decision D7)

- "Driving surf from an agent": "`fill f1 ada` types into a field; for a secret use
  `fill f2 -` and write the value to standard input: an argument word is readable by other users.
  `pick` chooses an option or checks a box. A password is never printed: `(set)` or `(empty)`."

## Acceptance

`cargo nextest run -p surf-session -p surf-answer -p surf-agent` passes with the 19 numbered
cases, each a test named after its case, case 1 equal to its golden text and no answer of case 3
containing the password. No test runs over 60 s, and the test step ends within 120 s.

## Out of scope

`press` and submission (`agent-press-submit`); file upload; `changed` (`agent-change-and-log`).
