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
- confidence: inferred
  path: crates/agent-tasks/src/agent.rs
- confidence: inferred
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
revision: 13
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
  `href` value as written, the whole accessible name, and `hidden` when the element or an ancestor
  has the `hidden` attribute;
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
  `document_base_url(tree, document_address)`.
- `crates/session`: after each command the current document is read; the `page` line gains the
  title; `note parser: <parts>` when `Browser::parser_not_yet_parsed` is not empty; a `now` block
  lists each link: handle `l<n>` as `surf_page::handles` (`crates/page`, added by `story:page-text-outline`) gives
  it, in document order, kept while the document lives, starting at 1 again for a new document,
  nothing random; kind
  `link`; visibility `hid` when `hidden` applies, otherwise `?`, with
  `note visibility: hidden attribute only`; the label quoted and cut at 60 characters; the target
  resolved with `surf-url` against the base URL and written as its path when it has the origin of
  the page, quoted; a target that does not resolve is printed as written, quoted, with the word
  `unresolved`. `follow <handle>`: an unknown handle, a hidden link and an unresolved target are
  refused (status 1, `did refused unknown-handle`, `hidden`, `unresolved`); otherwise `Follow`.
- `crates/answer`, `docs/agent-answer.md`: the title on the `page` line, the `now` block, the link
  line, `note visibility: hidden attribute only`, each with its `parse` beside its rendering, so
  parsing what `render` wrote gives the value back (decision D20 of
  `docs/plan/agent-plan-revision-2.md`).
- `crates/session`: the event entry of the `Follow` event of `agent-links-spec`, with its fields,
  in the mapping function of `agent-session-events`, and the variant added to its mapping list
  (decision D27).
- Every expected `note parser` line in a test of this story is built from
  `surf_html::NOT_YET_PARSED` joined by `,`; a golden file holds the placeholder line
  `note parser: {NOT_YET_PARSED}`, which the test replaces before comparing, or no such line when
  the constant is empty; no test names a literal list (decision D31).
- The `did` and `path` lines of `follow`, as `docs/agent-answer.md` and `crates/answer` write
  them: `did follow "<handle>"` and `<n> follow "<handle>"`, the handle a quoted field. The
  reasons `unknown-handle`, `hidden` and `unresolved` go into the reason table of
  `docs/agent-answer.md` with exit status 1, into `RefusalReason` and into a test.
- `cli/surf-agent.yaml` and `crates/agent`: `follow <handle>`, together, so the drift test holds.
- The counted cases, in `crates/page/tests/base_url.rs` and `crates/session/tests/`, against a
  server the test starts:
  1. no `base`: the document address; 2. a relative `base href`; 3. two `base` elements: the first
  with `href`; 4. the `now` block of a page with a relative link, a link to another origin, a link
  inside a `hidden` element and an `a` without `href` equals its golden text;
  5. `follow` of the relative link answers the target page; 6. `follow l9`: status 1,
  `unknown-handle`; 7. `follow` of the hidden link: status 1, `hidden`; 8. `follow` of an
  unresolved target: status 1, `unresolved`; 9. two runs over the same pages give the same handles;
  10. a label of 200 characters is cut to 60 and `...`;
  forging, each rendered quoted and escaped with no extra line, for every page-derived field this
  story prints: 11. the title; 12. the label; 13. an unresolved target as written; 14. a resolved
  target whose path holds `"` and a line feed (percent-encoded by the parser, so printed as is);
  each case with a value holding a line feed, `"` and a bidirectional control;
  15. the answer of a `text/html` page holds `note parser: ` and the entries of
  `surf_html::NOT_YET_PARSED` joined by `,`, built from the constant, when it is not empty, and no
  such line when it is empty;
  16. round trip: `surf_answer::parse` of the answer of case 4 and of each answer of cases 11 to 14
  gives the typed answer, and rendering that gives the same lines byte for byte;
  17. `follow_event_maps_with_its_fields`: after `follow` of the relative link, with `surfd` run in
  the test process (decision D29), the session log holds the `Follow` event entry with every field
  of the specification and no `unmapped` mark, between the `follow` action entry and its answer
  entry.

Surfaces written: `crates/page/src/base_url.rs`, `crates/page/src/lib.rs` (one line),
`crates/page/tests/base_url.rs`, `crates/page/tests/main.rs` (one `mod` line), `crates/session/`, `crates/answer/`, `docs/agent-answer.md`,
`cli/surf-agent.yaml`, `crates/agent/`.

## For README.md (handed over in the unit report, decision D7)

- "Driving surf from an agent": "`now` lists every link with a handle such as `l3`; `follow l3`
  goes there. `?` means visibility is not computed; only the `hidden` attribute is known."

## Acceptance

`cargo test -p surf-page -p surf-session -p surf-answer -p surf-agent` passes with the 17 numbered
cases, each a test named after its case, case 4 equal to its golden text, and the drift test
holding with `follow` in both the document and the parser.

## Out of scope

`back` and `forward` (`agent-history-spec`, `agent-back-forward`); controls; the `changed` block;
visibility from CSS; paging a long `now` block (backlog `story:agent-answer-paging`).
