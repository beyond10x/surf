---
format: aep.planning-md/3
id: story:agent-change-and-log
kind: story
status: active
title: changed in every answer, text changes, and log
summary: The session computes what the last action changed over actionable elements and text, prints it in every answer, and log prints the whole action path.
relations:
- decomposes: epic:agent-cli-target
- depends_on: story:agent-press-submit
- depends_on: story:agent-back-forward
- depends_on: story:html-document-port
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:08Z", actor: "human:timo", revision: 6}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:08Z", actor: "human:timo", revision: 7}
---
## Context

Step 5 of design section 9; design section 5.3 (`changed`), section 4 (`log [<n>]` over the view
`SessionLog`, `agent-page-path-spec`). The session process keeps the page state of its last answer
(design section 3); its core returns the two states it compared. This is the last story of the
epic and its walk test is the epic acceptance, which "goes back", so it stands on
`agent-back-forward`.

Decision D2: a document's tree does not change before scripts exist. What an agent changes on a
page is control state (value, checkedness, selectedness), which lives in the control rows of the
specification. So a same-document `changed` block holds control-state lines and nothing else: the
design kinds `+ <element>`, `- <handle>`, `~ label`, `~ target`, `~ visibility`, `~ enabled` and
text changes inside one document need a script to happen, and wait for it. Decision D3: the
detailed mode and `--detail` are `epic:detailed-mode`, after the script engine.

Needs from `epic:html-parser`: the tree of each document through `Browser::document_tree`, read
only (decision D1), for the texts of a new document (`html-document-port`).

Decided here, where design section 5.3 leaves it open: after a navigation to a new document the
block is `changed page "<old address>" -> "<new address>"` followed by `+ text "<text>"` lines for
the texts of the new document that the old one did not hold, so the message a submit produces
shows; at most 10, each cut at 120 characters, and `+ text more <n>` for the rest.

## Change

- `crates/session`: between the state of the previous answer and the state after the action. Same
  document: `~ <handle> value "<before>" -> "<after>"`, `~ <handle> checked <before> -> <after>`,
  `~ <handle> selected "<before>" -> "<after>"`; a password prints `(set)` or `(empty)`. New
  document, `back` and `forward` included: the `changed page` line and the text lines. Every
  answer has the block: `changed 0` when nothing changed. `log [<n>]`: every action of the session
  in order, or the last `n`.
- `crates/answer`, `docs/agent-answer.md`: the `changed` block and the `log` answer, each with its
  `parse` beside its rendering (decision D20 of `docs/plan/agent-plan-revision-2.md`).
- `cli/surf-agent.yaml`, `crates/agent`: `log`, together, so the drift test holds.
- `crates/session/tests/`, the counted cases, against a server the test starts:
  1. `fill` a field: `~ f1 value "" -> "ada"`; 2. `fill` a password: `(empty) -> (set)`;
  3. `pick` a checkbox: `~ f3 checked false -> true`; 4. `pick` a radio: one line per control
     of the group that changed; 5. `pick` a select option: `~ f5 selected`;
  6. `look` after `look`: `changed 0`;
  7. a new document: `changed page` and a `+ text` line per new text;
  8. 15 new texts: 10 lines and `+ text more 5`; 9. a text of 200 characters cut to 120 and `...`;
  forging, each quoted and escaped with no extra line, with a value holding a line feed, `"` and a
  bidirectional control: 10. a `+ text` line; 11. the before and after of a `~ value` line;
  12. both addresses of a `changed page` line;
  13. the same calls in two runs give the same output;
  14. `log` lists every action in order, and `log 2` the last two;
  15. the walk of the epic acceptance through the built `surf-agent`, `surfd` and `surf-session`
      only, `surfd` started by the `open` of the walk, against a server with a
      page of links and a login form: `open`, `go` to the form page, `follow` a link by its
      handle, `back` to the form page, `fill` the user, `fill -` the password, `press`; the answer
      holds `changed page` with the landing address and a `+ text` line with the message the server
      put on it, and `path` the last five actions with the total; `log` lists all seven actions,
      `back` among them; the token of a second session is refused on the first; `close`, after
      which `look` exits 1 with `did refused closed` (a built `surfd` is asserted only through
      what an answer prints, decision D29);
  16. round trip: `surf_answer::parse` of the answers of cases 10 to 12 and 14 and of the `press`
      answer of case 15 gives the typed answer, and rendering that gives the same lines byte for
      byte.

Surfaces written: `crates/session/`, `crates/answer/`, `docs/agent-answer.md`,
`cli/surf-agent.yaml`, `crates/agent/`.

## For AGENTS.md and README.md (handed over in the unit report, decision D7)

- `AGENTS.md` "Implementation targets": "The agent target has every command of the epic:
  `open`, `go`, `look`, `close`, `follow`, `back`, `forward`, `fill`, `pick`, `press`, `log`.
  Before scripts a same-document `changed` block holds control state only."
- `README.md` "Driving surf from an agent": "`changed` says what the last action changed: the
  value of a control, or the new page and its new texts. `log` lists every action."

## Acceptance

`cargo test -p surf-session -p surf-answer -p surf-agent` passes with the 16 numbered cases, each a
test named after its case, the walk of case 15 answering `press` with `changed page` and the
`+ text` line of the server message and listing 7 actions in `log`.

## Out of scope

The detailed mode and `--detail` (`epic:detailed-mode`, decision D3); changes that need a script
(elements appearing or going, labels, targets, visibility, text inside one document); computed
style.
