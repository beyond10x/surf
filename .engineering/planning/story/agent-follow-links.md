---
format: aep.planning-md/3
id: story:agent-follow-links
kind: story
status: active
title: now lists links, and follow <handle> navigates to one
summary: The session lists the links of the current document with deterministic handles, labels, hidden flags and resolved targets, and follows one by its handle.
relations:
- decomposes: epic:agent-cli-target
- depends_on: story:agent-session-process
- depends_on: story:agent-url-parser
- depends_on: story:agent-links-spec
- depends_on: story:html-document-port
- depends_on: story:html-accessible-name
- depends_on: story:agent-session-events
- depends_on: story:page-text-outline
- depends_on: story:values-and-labels-before-fill
scope:
- confidence: cited
  path: AGENTS.md
- confidence: inferred
  path: CHANGELOG.md
- confidence: inferred
  path: Cargo.lock
- confidence: cited
  path: README.md
- confidence: cited
  path: cli/surf-agent.yaml
- confidence: cited
  path: crates/agent-tasks/src/agent.rs
- confidence: cited
  path: crates/agent-tasks/status.txt
- confidence: cited
  path: crates/agent/src/cli.rs
- confidence: inferred
  path: crates/agent/src/wire.rs
- confidence: inferred
  path: crates/agent/tests/drift.rs
- confidence: cited
  path: crates/answer/src/lib.rs
- confidence: inferred
  path: crates/answer/tests
- confidence: cited
  path: crates/page/Cargo.toml
- confidence: cited
  path: crates/page/src/base_url.rs
- confidence: cited
  path: crates/page/src/lib.rs
- confidence: cited
  path: crates/page/tests/base_url.rs
- confidence: cited
  path: crates/page/tests/main.rs
- confidence: inferred
  path: crates/session/Cargo.toml
- confidence: cited
  path: crates/session/src/lib.rs
- confidence: cited
  path: crates/session/tests
- confidence: inferred
  path: crates/surfd/src/daemon.rs
- confidence: cited
  path: docs/agent-answer.md
- confidence: inferred
  path: generated/rust
- confidence: inferred
  path: spec/domains/agent.yaml
- confidence: inferred
  path: website/docs/drive-from-an-agent.md
revision: 19
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:09Z", actor: "human:timo", revision: 5}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:09Z", actor: "human:timo", revision: 6}
---
## Context

Step 3 of design section 9; design sections 5.2 and 5.5. `agent-links-spec` specifies `Follow`;
`agent-session-process` answers `open`, `go`, `look`, `close`; `agent-url-parser` resolves
addresses.

Needs from `epic:html-parser` (`html-document-spec`, `html-actionable-elements`,
`html-accessible-name`, `html-document-port`):
- the document row of the current navigation of the session, found by the navigation identity the
  document carries (draft marker 3, answered by `html-document-spec`);
- the document title;
- the link rows in document order: an identity that stays the same while the document lives, the
  `href` value as written, the whole accessible name, and whether the element or an ancestor
  has the `hidden` attribute (computed from the tree, see Change: the row's `hidden` flag is
  wider);
- the tree through `Browser::document_tree`, read only (decision D1), for the document base URL;
- `Browser::parser_not_yet_parsed`, the parts of the tree builder not implemented yet (decision
  D12): until the list is empty, real pages are parsed by an incomplete tree builder and the
  answer says so.

Decision D10: the document base URL has one owner, a function in `crates/page` written here
(HTML Standard "document base URL": the `href` of the first `base` element that has one, parsed
against the document address; from memory, to be confirmed); `agent-form-port` calls it. This
story writes new files of `crates/page` and one module line in its `lib.rs`; `html-tree-tables`
and `html-tree-template` add test files there and touch neither.

## Domain relations

- `surf.navigation.Navigation → surf.document.Document`: one navigation gives at most one document
  (binding `parse-on-navigation-committed` in `spec/components.yaml`, `delivery: at_most_once`,
  only for `text/html`); the `document` component owns it; the linking field is
  `html-document-spec`'s.
- Handle to element: memory of the session process, not the specification (`agent-links-spec`);
  design section 5.5 states the rule.

## Change

- `crates/page/src/base_url.rs` (new) and one `pub mod` line in `crates/page/src/lib.rs`:
  `document_base_url(tree, document_address)`. It parses with `surf-url`, a crate of this
  repository: `crates/page/Cargo.toml` gains that dependency, and the sentence of `AGENTS.md:580`
  ("Standard library and `surf-html` only") is corrected to name `surf-url`.
- `crates/session`: after each command the current document is read. The session sets both
  titles, which are `None` today: the one of its reply to `surfd` (`link::Answer.title`,
  `crates/session/src/lib.rs:258`), which `surfd` keeps in the status record and the view's
  session list shows (`crates/surfd/src/link.rs:502-503`), and the one on the `page` line
  (`page()`, `crates/session/src/lib.rs:984`, `title: None` at line 999). `story:view-now-panel`
  writes no file of `crates/session`. `crates/answer` (`Page::Loaded.title`, `crates/answer/src/lib.rs:186`) and the grammar
  (`page-loaded`, `docs/agent-answer.md:304`) carry it already. `docs/agent-answer.md`, "What an
  answer holds today" ("A loaded page shows no title", line 45), and `README.md:110` ("and no
  title") are corrected. `note parser: <parts>` when `Browser::parser_not_yet_parsed` is not
  empty (`Note::Parser` and `crates/answer/tests/golden/note-parser.txt` exist). A `now` block,
  in an answer that carries no page text; an answer with text (`--as tree|markdown`) has none
  (`docs/design/page-as-text.md:120`), so `no_now` (`crates/session/tests/core.rs:560`) and
  `an_answer_with_text_has_no_now_block` (`:855`) stay true. The block lists each link: handle
  `l<n>` as `surf_page::handles` (`crates/page/src/lib.rs:368`) gives it, in document order,
  kept while the document lives, starting at 1 again for a new document, nothing random; kind
  `link`; visibility `hid` when the element or an ancestor has the `hidden` attribute, otherwise
  `?`, with `note visibility: hidden attribute only`, as the epic allows ("an element line says
  `?` or what the `hidden` attribute gives", `.engineering/planning/epic/agent-cli-target.md:95-97`)
  and design section 5.2 says (`docs/design/agent-interface.md:249-251`, which agrees with the
  epic and is not changed). `aria-hidden`, a closed `dialog` or `details` and a popover do not
  make a link `hid`. The row's `hidden` flag is wider (`surf-page`'s `is_hidden`,
  `crates/page/src/lib.rs:257-258`, `:417-421`), so the session computes the narrow rule from the
  tree (`Browser::document_tree`) and does not read that flag. The label quoted and cut at 60
  characters; the target resolved with `surf-url` against the base URL and written as its path
  when it has the origin of the page, quoted; a target that does not resolve is printed as
  written, quoted, with the word `unresolved`. `follow <handle>`: an unknown handle, a hidden link
  (by the same narrow test) and an unresolved target are refused (status 1,
  `did refused unknown-handle`, `hidden`, `unresolved`); otherwise `Follow`.
- `crates/agent-tasks`, the round's measure: `now_handle`
  (`crates/agent-tasks/src/agent.rs:102-105`, which returns `None` today) returns the handle of the `now` line whose label equals the one
  asked, read through `surf_answer::parse`; `crates/agent-tasks/status.txt` sets `link pass`
  (line 3 reads `link pending story:agent-follow-links`).
- `crates/answer`, `docs/agent-answer.md`: the title on the `page` line, the `now` block, the link
  line, `note visibility: hidden attribute only`, each with its `parse` beside its rendering, so
  parsing what `render` wrote gives the value back (decision D20 of
  `docs/plan/agent-plan-revision-2.md`).
- No `Follow` event exists: `follow` publishes `NavigationRequested` and, through `RecordAction`,
  `ActionRecorded`, both mapped already (`crates/session/src/entries.rs:144-155`). This story adds
  no mapping; case 17 observes those entries, with the `handle` and `label` fields
  `story:values-and-labels-before-fill` adds to `NavigationRequested` and the label cut it
  gives `ActionRecorded`.
- Every expected `note parser` line in a test of this story is built from
  `surf_html::NOT_YET_PARSED` joined by `,`; a golden file holds the placeholder line
  `note parser: {NOT_YET_PARSED}`, which the test replaces before comparing, or no such line when
  the constant is empty; no test names a literal list (decision D31).
- The `did` and `path` lines of `follow`. `did follow "<handle>"`: the handle quoted, as the
  caller typed it. The `path` entry of an action on an element takes the form of the
  specification and the design (`spec/domains/agent.yaml:302-304`,
  `docs/design/agent-interface.md:222`): `<n> follow l4 "Sign in"`, the handle bare (the session
  made it) and the label quoted and cut at 60 characters as in the `now` block. `docs/agent-answer.md`
  (section "Path" and the grammar's `path-entry`, line 311, which allows one quoted field today)
  and `crates/answer` (`PathEntry`, `crates/answer/src/lib.rs:206-214`) gain that form:
  `PathEntry` gains `handle: Option<String>`, and the label stays in `argument`. So
  `crates/surfd/src/view.rs:205-209`, which reads `sequence`, `command` and `argument` by field,
  compiles unchanged and shows the label without the handle; `story:view-now-panel` adds the
  handle to the view's path entry. `path()` (`crates/session/src/lib.rs:885`, which prints `row.url` at line 900) prints it from
  the `SessionLog` row. The reasons `unknown-handle`, `hidden` and `unresolved` go into the
  reason table of `docs/agent-answer.md` with exit status 1, into `RefusalReason` and its
  `RefusalReason::ALL` (`crates/answer/src/lib.rs:150`, `[RefusalReason; 6]` today), into the
  exhaustive `match` of `crates/surfd/src/daemon.rs:815-821`, and into a test.
- `cli/surf-agent.yaml` and `crates/agent`: `follow <handle>`, together, so the drift test holds.
- The counted cases, in `crates/page/tests/base_url.rs` and `crates/session/tests/`, against a
  server the test starts:
  1. no `base`: the document address; 2. a relative `base href`; 3. two `base` elements: the first
  with `href`; 4. the `now` block of a page with a relative link, a link to another origin, a link
  inside a `hidden` element (`hid`), a link inside an element with `aria-hidden="true"` (`?`:
  only the attribute counts) and an `a` without `href` equals its golden text. The golden file
  holds the whole answer of `look`, as the files of `crates/session/tests/golden/` do (`go.txt`:
  head line to `path` block, with `<id>` and `<port>` placeholders, compared through `golden()`,
  `crates/session/tests/core.rs:240`);
  5. `follow` of the relative link answers the target page, and its `path` line reads
  `<n> follow l1 "<label>"`; 6. `follow l9`: status 1, `unknown-handle`; 7. `follow` of the
  hidden link: status 1, `hidden`; 8. `follow` of an unresolved target: status 1,
  `unresolved`;
  9. two runs over the same pages give the same handles; 10. a label of 200 characters is cut to
  60 and `...`;
  forging, each rendered quoted and escaped with no extra line, for every page-derived field this
  story prints: 11. the title; 12. the label, on the link line and on the `path` line; 13. an
  unresolved target as written; 14. an `href` holding `"`, a line feed and a bidirectional
  control resolves to a path printed with `%22`, no line feed (the URL parser removes tabs and
  newlines, `crates/url/src/parser.rs:119-124`) and the control percent-encoded (`%E2%80%AE` for
  U+202E, the path percent-encode set, `crates/url/src/percent.rs:27-39`); cases 11 to 13 each
  with a value holding a line feed, `"` and a bidirectional control;
  15. the answer of a `text/html` page holds `note parser: ` and the entries of
  `surf_html::NOT_YET_PARSED` joined by `,`, built from the constant, when it is not empty, and no
  such line when it is empty. The constant is empty today (`crates/html/src/tree_builder.rs:59`),
  so only the "no such line" branch runs;
  16. round trip: `surf_answer::parse` of the answer of case 4 and of each answer of cases 11 to 14
  gives the typed answer, and rendering that gives the same lines byte for byte;
  17. `follow_entries_hold_handle_and_label`: after `follow` of the relative link, with `surfd`
  run in the test process (decision D29), the session log holds, between the `follow` action
  entry and its answer entry, the `ActionRecorded` entry with `handle` and the label cut by the
  rule of `story:values-and-labels-before-fill`, and the `NavigationRequested` entry with its
  `handle` and `label` fields, neither `unmapped`;
  18. `session_list_title`: after `go` to a page whose title is `Opening hours`, the reply's
  `link::Answer.title` holds it quoted, and with `surfd` run in the test process (decision D29)
  its status record (`crates/surfd/src/link.rs:561-562`, `title "<the title, quoted>"`) holds
  that title.

Surfaces written: `crates/page/src/base_url.rs`, `crates/page/src/lib.rs` (one line),
`crates/page/Cargo.toml`, `crates/page/tests/base_url.rs`, `crates/page/tests/main.rs` (one `mod`
line), `crates/session/`, `crates/answer/`, `docs/agent-answer.md`, `cli/surf-agent.yaml`,
`crates/agent/`, `crates/surfd/src/daemon.rs`, `crates/agent-tasks/src/agent.rs`,
`crates/agent-tasks/status.txt`, `AGENTS.md`, `README.md`.

## For README.md (handed over in the unit report, decision D7)

- "Driving surf from an agent": "`now` lists every link with a handle such as `l3`; `follow l3`
  goes there. `?` means visibility is not computed; only the `hidden` attribute is known."

## Acceptance

`cargo nextest run -p surf-page -p surf-session -p surf-answer -p surf-agent` passes with the 18
numbered cases, each a test named after its case, case 4 equal to its golden text, and the drift
test holding with `follow` in both the document and the parser. The round's measure:
`now_handle` reads the handle from the `now` line with the label asked,
`crates/agent-tasks/status.txt` reads `link pass`, `tasks::every_task`
(`crates/agent-tasks/tests/tasks.rs:74`) passes, and `slot task agent-tasks-report` prints
`task link pass` (with `2 connections`, `crates/agent-tasks/tasks.txt`) and no `raise:` line;
today `link` fails with `no now line labelled "Opening hours"`
(`crates/agent-tasks/src/agent.rs:84`, `crates/agent-tasks/src/judge.rs:69`). No test runs over 60 s, and the test step ends
within 120 s.

## Out of scope

`back` and `forward` (`agent-history-spec`, `agent-back-forward`); controls; the `changed` block;
visibility from CSS; paging a long `now` block (backlog `story:agent-answer-paging`).
