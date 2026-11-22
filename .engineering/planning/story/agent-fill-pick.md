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
revision: 9
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
  `surf_page_text::handles` (`story:page-text-outline`) gives them, by the rules links follow;
  kind; visibility as for links; label; state: the value quoted, `(set)` or
  `(empty)` for a password and never its value; `disabled`, `checked`, `required`; for a select the
  label of its selected option, quoted. `fill <handle> <text>` and `fill <handle> -`;
  `pick <handle> <option>` for a select (by option label, then value), `pick <handle> on|off` for a
  checkbox, `pick <handle> on` for a radio. Refused, status 1: an unknown handle, a hidden or
  disabled control, a control of another kind, an unknown option, a refusal of the document command.
- `crates/answer`, `docs/agent-answer.md`: the control and button lines, each with its `parse`
  beside its rendering (decision D20 of `docs/plan/agent-plan-revision-2.md`).
- `crates/session`: the event entries of `agent-session-events` for the events this story adds,
  in its mapping function and its mapping list (decision D27). Decision D22: the value of a
  password control is written as `<n> characters`, never the value; every other control value is
  kept as the answer prints it, quoted by `surf-answer`.
- The `did` and `path` lines of `fill` and `pick`, as `docs/agent-answer.md` and
  `crates/answer` write them: `did fill "<handle>"`, `did pick "<handle>"`, and `<n> fill
  "<handle>"`, `<n> pick "<handle>"`, the handle a quoted field and never the value typed or
  picked. The reasons `hidden` (if `agent-follow-links` has not added it), `disabled`,
  `wrong-kind` and `unknown-option` go into the reason table with exit status 1, into
  `RefusalReason` and into a test.
- `cli/surf-agent.yaml`, `crates/agent`: `fill` and `pick`, together, so the drift test holds.
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
      after case 3 no entry of the session log holds the password and the `Fill` event entry has
      its value as `<n> characters`; after case 2 the `Fill` event entry has the value filled,
      quoted; neither is `unmapped`;
  18. round trip: `surf_answer::parse` of the answer of case 1 and of each answer of cases 14 to
      16 gives the typed answer, and rendering that gives the same lines byte for byte.

Surfaces written: `crates/session/`, `crates/answer/`, `docs/agent-answer.md`,
`cli/surf-agent.yaml`, `crates/agent/`.

## For README.md (handed over in the unit report, decision D7)

- "Driving surf from an agent": "`fill f1 ada` types into a field; for a secret use
  `fill f2 -` and write the value to standard input: an argument word is readable by other users.
  `pick` chooses an option or checks a box. A password is never printed: `(set)` or `(empty)`."

## Acceptance

`cargo test -p surf-session -p surf-answer -p surf-agent` passes with the 18 numbered cases, each a
test named after its case, case 1 equal to its golden text and no answer of case 3 containing the
password.

## Out of scope

`press` and submission (`agent-press-submit`); file upload; `changed` (`agent-change-and-log`).
