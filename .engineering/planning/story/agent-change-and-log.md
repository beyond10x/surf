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
scope:
- confidence: inferred
  path: AGENTS.md
- confidence: inferred
  path: README.md
- confidence: cited
  path: cli/surf-agent.yaml
- confidence: cited
  path: crates/agent/src/cli.rs
- confidence: cited
  path: crates/agent/src/wire.rs
- confidence: inferred
  path: crates/agent/tests
- confidence: cited
  path: crates/answer/src/lib.rs
- confidence: cited
  path: crates/answer/tests
- confidence: cited
  path: crates/session/src/lib.rs
- confidence: cited
  path: crates/session/tests
- confidence: inferred
  path: crates/surfd/tests
- confidence: cited
  path: docs/agent-answer.md
- confidence: inferred
  path: generated/rust
- confidence: inferred
  path: spec/domains/agent.yaml
revision: 18
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:08Z", actor: "human:timo", revision: 6}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:08Z", actor: "human:timo", revision: 7}
---
## Context

Step 5 of design section 9; design section 5.3 (`changed`), section 4 (`log [<n>]` over the view
`SessionLog`, `agent-page-path-spec`). The session process is to keep the page state of its last
answer (design section 3); today it keeps none between commands, only the components' rows and
the kept text of `look` (`crates/session/src/lib.rs:105-116`), so this story adds that state. Its
core returns the two states it compared. This is the last story of the
epic and its walk test is the epic acceptance, which "goes back", so it stands on
`agent-back-forward`.

Decision D2: a document's tree does not change before scripts exist. What an agent changes on a
page is control state (value, checkedness, selectedness), which lives in the control rows of the
specification. So a same-document `changed` block holds control-state lines and nothing else: the
design kinds `+ <element>`, `- <handle>`, `~ label`, `~ target`, `~ visibility`, `~ enabled` and
text changes inside one document need a script to happen, and wait for it. Decision D3: the
detailed mode and `--detail` are `epic:detailed-mode`, after the script engine.

Reads, on `main` since `story:html-document-port`: the tree of each document through
`Browser::document_tree`, read only (decision D1). The texts of a new document are the lines
`crates/page-text` (`surf-page-text`) renders from that tree, the renderer of `look --as`.

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
- `log` is a read: it records no action and its answer has no `did` line. The lines it lists are
  `path` entries in the form `story:agent-follow-links` gives them: `<n> go "<address>"` for an
  address, `<n> follow l4 "Sign in"`, `<n> fill f1`, `<n> pick f3`, `<n> press b1` for an action
  on an element (the handle bare, a `follow`'s label quoted), `<n> back`, `<n> forward`,
  `<n> open` and `<n> close` without argument.
- `cli/surf-agent.yaml`, `crates/agent`: `log`, together, so the drift test holds.
  `crates/agent/src/wire.rs`: its module documentation lists the commands of a request
  (`command <name>`, `:24`); `log` joins that list, and `log <n>` sends its number as an `arg`
  (`:31-34` lists the arguments; the name is this story's).
- `crates/session/tests/`, the counted cases, against a server the test starts:
  1. `fill` a field: `~ f1 value "" -> "ada"`; 2. `fill` a password: `(empty) -> (set)`;
  3. `pick` a checkbox: `~ f3 checked false -> true`; 4. `pick` a radio: one line per control
     of the group that changed; 5. `pick` a select option: `~ f5 selected`;
  6. `look` after `look`: `changed 0`;
  7. a new document: `changed page` and a `+ text` line per new text;
  8. 15 new texts: 10 lines and `+ text more 5`; 9. a text of 200 characters cut to 120 and `...`
  by `surf_answer::quote_cut` (`crates/answer/src/lib.rs:444`; `quote_text_cut`, `:404`, is the
  form of a `text` block and is not used here);
  forging, each quoted and escaped with no extra line, with a value holding a line feed, `"` and a
  bidirectional control: 10. a `+ text` line; 11. the before and after of a `~ value` line of a
  `textarea` (a single-line field removes line feeds on `fill`, `story:agent-fill-pick`,
  "As decided after the adversary's pass", decision 1);
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

- Every answer gains `changed 0` at least, so every golden file and every exact-text test of a
  whole answer in this story's base gains that line, including those the command stories before
  it add (`agent-follow-links` case 4, `agent-fill-pick` case 1). On `main`, as an example: the
  14 of `crates/answer/tests/golden/` (`did-refused`, `did`, `head-died`, `head`,
  `head-unknown`, `note-parser`, `page-failed`, `page-none`, `page`, `path`, `text-forged`,
  `text-markdown`, `text-tree`, `token`, each `.txt`, each a whole answer from its head line) and
  the 7 of `crates/session/tests/golden/` (`close`, `go-as-markdown`, `go`, `look-as-markdown`,
  `look`, `open-as-tree`, `open`, each `.txt`); and the tests that compare an answer's exact text:
  `crates/answer/tests/forging.rs`, `crates/surfd/tests/in_process.rs` and
  `crates/surfd/tests/common/mod.rs` (inferred, not read whether each asserts the whole text),
  `crates/agent/tests/stand_in.rs` (inferred).

Surfaces written: `crates/session/`, `crates/answer/`, `docs/agent-answer.md`,
`cli/surf-agent.yaml`, `crates/agent/` (`src/wire.rs` included), `crates/surfd/tests/`.

## For AGENTS.md and README.md (handed over in the unit report, decision D7)

- `AGENTS.md` "Implementation targets": "The agent target has every command of the epic:
  `open`, `go`, `look`, `close`, `follow`, `back`, `forward`, `fill`, `pick`, `press`, `log`.
  Before scripts a same-document `changed` block holds control state only."
- `README.md` "Driving surf from an agent": "`changed` says what the last action changed: the
  value of a control, or the new page and its new texts. `log` lists every action."

## Acceptance

`cargo nextest run -p surf-session -p surf-answer -p surf-agent` passes with the 16 numbered
cases, each a test named after its case, the walk of case 15 answering `press` with `changed
page` and the `+ text` line of the server message and listing 7 actions in `log`. No test runs
over 60 s, and the test step ends within 120 s.

## Out of scope

The detailed mode and `--detail` (`epic:detailed-mode`, decision D3); changes that need a script
(elements appearing or going, labels, targets, visibility, text inside one document); computed
style.

## As decided while building

The change is `cf9a6ba` on `impl/round2-commands`. Three places where the text above and the
tree did not agree, each decided:

1. Decision D290. The `+ text` lines put page text into the answer, and surfd's log keeps the
   answer as printed (`crates/surfd/src/link.rs`, `events::case_02`). The test
   `events::case_07_a_body_is_kept_as_its_length` forbade every kept entry to hold 8 bytes of
   the page in a row. Its claim is about what the session records of a response, so its check
   applies to action entries and event entries. The answer entry is the answer byte for byte
   and may hold `+ text` lines; the same test asserts that it holds one, so the test says where
   page text may be and where not.
2. Decision D291. "Every answer has the block" holds for an answer that has a page state. An
   answer without one carries no `changed` block: a bare refusal (the token refusal, `open`'s
   usage refusal, a malformed request), an answer with status 3, and surfd's own answers
   (closed, no such session, session died, timeout). `docs/agent-answer.md` states this rule in
   one place.
3. The 14 golden files of `crates/answer/tests/golden/` keep their text: they are answers
   rendered from typed values without a page state. Three golden files are added for the
   block: `changed-controls`, `changed-page`, `changed-none`.

## As decided after the adversary's pass

The adversary's pass on `cf9a6ba` and `dc86f24` added 22 tests
(`crates/session/tests/adversary_change_log.rs`, `crates/answer/tests/adversary_changed.rs`); 3
were red. Fixed in `d46a8d6`.

1. Decision D298. A `selected` line compares a select's selected options by their places in the
   select; each side lists the labels of all selected options in document order, joined by
   `, `, and a move between two options of one label gives a line whose two sides read the
   same. Before, only the first selected label was compared, and a `pick` that deselected an
   option of a `select multiple` answered `changed 0`.
2. Decision D299. A `+ text` line is at most 1,935 bytes and a `~` line under 32,820 bytes: a
   character of 4 bytes is quoted as 16. The largest answer is about 3,491,000 bytes, inside the
   answer limit of 4,194,304 bytes of the client and `surfd`. The module documentation of
   `crates/session` states these figures; no bound changed.
3. Decision D300. The lists of entries that follow the public rule on a traversal keep only
   entries whose row in the navigator is not dropped. No test: no answer shows the lists.
4. Decision D301. The test of a page with many texts runs with 20,000 texts instead of
   100,000: with 100,000 it took up to 26.6 s under load, past its own bound of 20 s.
5. The acceptance counts the 16 numbered cases, the adversary's 22 tests and the two cases of 1.
