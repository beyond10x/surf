---
format: aep.planning-md/3
id: story:values-and-labels-before-fill
kind: story
status: draft
title: No password value is printed and a recorded label is cut, before the first fill command
summary: 'Four things a review of round 1 named become defects with the first command that fills a control or records a label: a stored value printed as it is, a label recorded uncut, inputs with no effect, the kept text''s drop untested'
relations:
- decomposes: epic:agent-cli-target
scope:
- confidence: cited
  path: crates/page/src/label.rs
- confidence: cited
  path: crates/session/src/entries.rs
- confidence: cited
  path: crates/session/src/lib.rs
- confidence: cited
  path: crates/surf/src/lib.rs
- confidence: cited
  path: docs/surf-command.md
- confidence: cited
  path: spec/domains/document.yaml
revision: 7
---
## Why

Round 1 put links and forms into the specification (`story:agent-links-spec`,
`story:agent-forms-spec`). No session command fills a control or records a label yet; the
stories that add those commands (`story:agent-fill-pick`, `story:agent-press-submit`,
`story:agent-follow-links`) will. A review of round 1 named four things that are harmless
today and become defects with the first such command:

1. A stored value is printed as it is. The `Controls` view and the `Elements` view
   (`spec/domains/document.yaml`) carry a control's `value`, and `surf::Report::lines`
   (`crates/surf/src/lib.rs`) prints it. After a fill of a control of `type=password` the
   password would be printed into the answer an agent reads and into anything that keeps
   answers.
2. `ActionRecorded.label` is written uncut (`crates/session/src/entries.rs`), against the
   sentence in `crates/session/src/lib.rs` that the one field holding text from a page is cut.
   A label comes from the page, so its bytes are hostile.
3. `handle` and `label` of `Follow`, and the `handle` of `Fill`, `Pick` and `Press`, are inputs
   with no effect: a run with two of them swapped passes the conformance suite.
4. No test observes that the kept text of a document (the rendered lines a `look` reads) is
   dropped on a request other than `look`; a change that never drops it passes every test,
   because the check of the current document hides it.

Also open from the same review: a page's own `value` attribute on a password input can become
part of the label of an element that contains it (`crates/page/src/label.rs`), and
`docs/surf-command.md` does not mention the line `note text: deep <n>`.

## Change

- The value of a control of `type=password` is never printed: every view text, `Report` line
  and page text shows that the control is filled, not with what and not how long. The stored
  value stays what a submission sends.
- `ActionRecorded.label` is cut and escaped by the rule the session's other page text follows,
  and the sentence in `crates/session/src/lib.rs` names the rule and the bound.
- Each of the inputs of point 3 either has an effect that a scenario observes (a refusal when
  the handle does not name the element, the label recorded) or leaves the command.
- A test observes the kept text being dropped on a request other than `look`.
- `crates/page/src/label.rs` takes no `value` of a password input into a label.
- `docs/surf-command.md` names `note text: deep <n>`.

This story is implemented before the first of `story:agent-fill-pick`,
`story:agent-press-submit` and `story:agent-follow-links`, or as the first commit of that
one.

## Acceptance

When the story is implemented:

1. A marker string set as the value of a `type=password` control through `SetValue` appears in
   no line of `Report::lines`, in no row text of `Controls` or `Elements` as the session
   prints them, and in neither text form of `look`; the same marker in a `type=text` control
   appears (a test for each).
2. A label of 10,000 characters with control bytes and bidi controls is recorded cut to the
   stated bound and escaped; the entry read back holds no control byte.
3. For each input of point 3 a scenario or test fails when two values of it are swapped, or
   the input is gone from the specification and the suite's counts are restated.
4. A change that never drops the kept text fails a test.
5. The suite's numbers after the change are stated beside the numbers before (scenarios,
   authored, refused), and the gate holds them.
