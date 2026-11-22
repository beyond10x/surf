---
format: aep.planning-md/3
id: story:html-document-port
kind: story
status: implemented
title: Parse every HTML response into the document's rows
summary: Browser parses each committed HTML body, keeps the tree per document and writes the title and element rows; surf <url> prints them.
relations:
- decomposes: epic:html-parser
- depends_on: story:html-document-spec
- depends_on: story:html-accessible-name
- depends_on: story:html-parser-limits
- depends_on: story:agent-session-process
- depends_on: story:runner-surf-cli-package
- depends_on: story:agent-fetch-post
- depends_on: story:one-test-binary-per-crate
scope:
- confidence: cited
  path: conformance/src/target.rs
- confidence: cited
  path: crates/session
- confidence: cited
  path: crates/surf-cli/src/main.rs
- confidence: cited
  path: crates/surf-cli/tests/main.rs
- confidence: cited
  path: crates/surf-cli/tests/page.rs
- confidence: cited
  path: crates/surf/Cargo.toml
- confidence: cited
  path: crates/surf/src/lib.rs
- confidence: cited
  path: crates/surf/tests/main.rs
- confidence: cited
  path: crates/surf/tests/parse.rs
revision: 13
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:11Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:11Z", actor: "human:timo", revision: 5}
- {from: "active", to: "implemented", at: "2026-11-22T16:29:18Z", actor: "human:operator", revision: 13, decided_on: {"recorded":{"test_result":1,"verification":1}}}
---
## Context

After story `html-document-spec` the specification has the rows of a parsed page and the
commands that write them, and the Rust system leaves them empty: nothing parses. `crates/surf`
(`Browser::run`) already does the analogous step for `net`: it hands each started request to a
`Transport`, a parameter of `Browser`, and issues the ending command on the `net` port only.
`docs/design/html-parser.md` (story `html-decisions`, items 1 and 2) puts the parser step there
too, as a parameter, so that `conformance/` can supply one that never answers, as it does for
the transport. The parser (`surf-html`: decoding, story `html-input-stream`; tree, through story
`html-tree-formatting`; limits, story `html-parser-limits`) and the row values (`surf-page`,
stories `html-actionable-elements` and `html-accessible-name`) exist.

Decisions that bind this story (`docs/plan/agent-plan-revision-1.md`):
- D1: the session process reads the tree through `Browser::document_tree`, read only. No
  component's port receives the tree, the `agent` component's included; the tree is the `document`
  component's, held by the assembly.
- D2: the tree is built by parsing and not changed afterwards before scripts exist. What an agent
  changes on a page is control state (a control's value, checkedness, selectedness), and that
  lives in the specification's rows, not in the tree. So there is no mutation path and none is
  added here; an element row keeps its identity, and its node, for the life of its document.
- D6: the `surf` binary is in `crates/surf-cli` (`story:runner-surf-cli-package`, on which this
  story depends); tests that run the binary are there.
- D12: this story lands before the tree chain ends: until `story:html-tree-complete`, real pages
  are parsed by an incomplete tree builder. `surf_html::NOT_YET_PARSED` (story `html-tree-body`)
  names what is not yet parsed as the standard says, `Browser` exposes it, and the output says
  so in a `note` line; the gate's counts (`task html-conformance`) say how incomplete.
- D7: `AGENTS.md` and `README.md` are the coordinator's in a wave of more than one unit; this
  story hands over its sentences below.

Ordering: this story depends on `story:agent-session-process`, which builds `crates/session` on
`Browser`; the new `Browser` parameter changes the interface that crate uses, and this story
adapts that use if needed.

What the other epics need from this one and this story delivers: the node tree reachable inside
the session process (through `Browser`), the actionable elements in document order with an
identity that stays the same for as long as the document lives, and the specification's rows
for them.

## Change

- `crates/surf/src/lib.rs`: a parser step, a trait parameter of `Browser` like `Transport`,
  whose default decodes the body, parses it with the default limits, computes title and
  elements with labels, and issues, on the `document` port only, the commands of
  `docs/design/html-parser.md` item 2 as `story:html-document-spec` names them: one per
  actionable element in document order, then the ending command. `Browser` keeps the tree of each document, keyed by the document's identity, with
  read-only access `Browser::document_tree`, the map from each element row's identity to its
  node, and `Browser::parser_not_yet_parsed` returning `surf_html::NOT_YET_PARSED`.
- `Report`: one `title` line per document, one `element` line per actionable element in
  document order, and while `NOT_YET_PARSED` is not empty one line
  `note parser: <its entries joined by ",">` (the form of the answer's `note parser` line; each
  entry matches `[a-z-]+`, `docs/plan/agent-plan-revision-2.md`, D31); every value from the page
  written by `escape`.
- `crates/surf-cli/src/main.rs` only if the output needs it.
- `conformance/src/target.rs`: the parser step that never answers, so the suite ends each parse
  itself.
- `crates/session/` only where the new `Browser` parameter needs the default supplied.
- Tests, each against a server the test starts on `127.0.0.1`:
  - `crates/surf/tests/parse.rs`: `element_identities_and_nodes_are_the_same_on_every_read`,
    `a_parser_step_that_never_answers_leaves_the_parse_open`, `a_non_html_response_is_not_parsed`.
  - `crates/surf-cli/tests/page.rs`:
    `a_page_with_a_title_three_links_and_a_form_prints_the_title_and_one_row_per_element`,
    `a_windows_1252_page_with_a_meta_charset_is_decoded`,
    `a_page_over_the_depth_limit_names_the_limit`,
    `page_text_that_looks_like_a_line_is_escaped`,
    `the_parser_note_lists_what_is_not_yet_parsed` (the line present exactly when the list is
    not empty; the expected line is built from `surf_html::NOT_YET_PARSED`, never written as a
    literal, so a tree story that shortens the list breaks no test here, D31).

Writes: `crates/surf/src/lib.rs`, `crates/surf/tests/parse.rs` (new), `crates/surf/tests/main.rs` (one `mod` line), `crates/surf-cli/src/main.rs`,
`crates/surf-cli/tests/page.rs` (new), `crates/surf-cli/tests/main.rs` (one `mod` line), `conformance/src/target.rs`, `crates/session/` (the
`Browser` construction only), `crates/surf/Cargo.toml` (the dependencies on `surf-html` and
`surf-page`). The root `Cargo.lock` and `conformance/Cargo.lock` change with it: the
coordinator's.

For `AGENTS.md`, handed over in the report (D7):
- "Layout", the `crates/surf` entry gains: "The library also runs the `document` component's
  parser step, a parameter of `Browser` like the transport: it decodes and parses each committed
  HTML body with `surf-html`, computes the title and the actionable elements with `surf-page`,
  and issues one command per actionable element and then the ending command on the `document`
  port. `Browser` keeps each document's tree,
  read only through `Browser::document_tree`; no component's port receives it."
- "Layout", a new entry: "`crates/page` (`surf-page`): the title, the actionable elements and
  their labels, computed from a `surf-html` tree. Standard library and `surf-html` only."

For `README.md`, handed over in the report (D7): the output format of `surf <url>` gains
`title <document-id> <title>`, `element <document-id> <position> <kind> <flags> <label>
<target or value>` and `note parser: <parts>`, with one sentence: "Until the parser is complete,
a `note parser:` line names what it does not yet parse as the HTML Standard says."

Bounds for every command of this story. Every build and test runs through the build pool:
`cargo` is wrapped by the pool's shim, every other command that builds or tests gets the `slot`
prefix. The pool ends a job after 10 minutes; a command that needs longer names its time with
`slot --timeout <t>` or `SLOT_TIMEOUT=<t>`, at most 30 minutes; a `timeout` wrapped around the
command extends nothing. A script that measures or waits starts without the `slot` prefix; the
builds and test runs inside it carry it. No test runs over 60 s and the test step ends within
120 s. No Python.

## Acceptance

After `slot task generated-fresh`, `SLOT_TIMEOUT=30m cargo test -p surf` passes with the three
tests of `tests/parse.rs` and `cargo test -p surf-cli`
with the five tests of `tests/page.rs` named in Change, among them a page with a title, three
links and a form printing the title line and one element line per actionable element, and
the gate exits 0, run once by the coordinator after the round's units as
`slot --timeout 25m task check-runner`.

## Out of scope

The session process, handles and the answer format (`epic:agent-cli-target`); printing the
parser `note` in an agent answer (the answer format's fixed list of notes); what changed between
two states of a page; resolving link targets (`story:agent-follow-links`). Scripts: a `script`
element is in the tree and never run; a page that builds its content with script shows only
what its markup holds.

## As built

What differs from the text above, as of 2418d48:

- Change item 1, "a trait parameter of `Browser`": the parser step is the type parameter `P` of
  `Browser`, default `HtmlParser`, replaced with `Browser::with_parser`; the default's work is the
  function `surf::parse_page` (`crates/surf/src/lib.rs`).
- Change item 1, "the map from each element row's identity": it is `Browser::element_nodes`,
  which answers the pairs of one document.
- Change item 2, the `Report` lines: a title, label, target and value are quoted fields written
  like an address, `"` as `\x22`: `title <document-id> "<title>"` and
  `element <document-id> <position> <kind> <flags> "<label>" "<target or value>"`. `<flags>` is
  the set flags among `hidden,disabled,checked,required` joined by `,`, or `-`; the last field is
  `-` for a row with neither target nor value (`Report::lines`, `crates/surf/src/lib.rs`).
- Change item 2 names no line for a parse a limit stopped: `Report` adds
  `note limit <document-id> <limit>`, the `surf.document.ParseLimit` that stopped it (test
  `a_page_over_the_depth_limit_names_the_limit` in `crates/surf-cli/tests/page.rs`).
- Change item 1, "parses it with the default limits": the limits can be set.
  `HtmlParser::with_options` and `HtmlParser::with_limits` take `surf-html`'s options or limits,
  `surf::parse_page_with` parses under them, and `crates/surf/tests/parse.rs` has one test per
  limit, `a_page_over_the_input_limit_is_parsed_up_to_it` to
  `a_page_over_the_step_limit_is_parsed_up_to_it`.
- Change, the tests of `page.rs`: it also has `a_response_without_a_content_type_is_not_parsed`,
  `an_error_status_with_an_html_body_is_parsed` and
  `an_html_body_that_is_not_utf_8_is_decoded_with_replacement_characters`.
- Not in the story: a review after the first implementation found that a long title took the
  entries of one request past `surfd::link::ENTRIES_MAX`, so `go` answered status 3 and the
  session ended. The `DocumentParsed` entry holds at most `TITLE_ENTRY_BYTES` (1,024) bytes of the
  title, with the field `title_bytes` when it was cut (`crates/session/src/entries.rs`; test
  `a_page_with_a_long_title_leaves_go_answering_and_the_session_open`).
- Not in the story: one request carries at most `ELEMENT_ENTRIES_MAX` (256) `ElementAdded`
  entries, and its action and event entries at most `ACTION_AND_EVENT_BYTES_MAX` bytes; one line
  `event surf.agent.EventsLeftOut count="<n>"` counts what was left out
  (`crates/session/src/lib.rs`, tests in `crates/session/tests/adversary_parse.rs`).
- Not in the story: the in-memory store keeps an index from identity to row (`Rows`,
  `crates/ports/src/store.rs`). Before it, one `go` to a page of 10,001 links took 7.8 to 12.9 s,
  after it 0.22 to 0.48 s (message of commit f9f111b; test
  `a_page_with_more_links_than_a_request_carries_answers_and_keeps_the_session_open`).
- Writes, "`crates/session/` (the `Browser` construction only)": `crates/session/src/lib.rs` and
  `crates/session/src/entries.rs` also hold the bounds above.
- Writes: files outside the list changed. `conformance/src/lib.rs` exports `Unparsed`;
  `crates/surf/tests/held.rs`, `crates/surf-cli/tests/cli.rs`, `crates/surf-cli/tests/redirects.rs`
  and `crates/session/tests/events.rs` count the new `title` and `note` lines and the new events;
  `crates/ports/src/store.rs`, `crates/ports/src/ports.rs` and
  `crates/session/tests/adversary_parse.rs` carry the index and the bounds above.
- Left open: `Browser` keeps the tree of every parsed document, and the components keep its
  element rows, for the life of the session, with no bound over all documents
  (`crates/session/src/lib.rs`, "What it keeps"; a draft story on the session's memory).
