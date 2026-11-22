---
format: aep.planning-md/3
id: story:session-memory-bound
kind: story
status: draft
title: A session holds a bounded number of document trees and rows
summary: surf::Browser keeps the tree and the rows of every document a session parsed and nothing bounds the sum; the session keeps the current document's and drops or bounds the others
relations:
- decomposes: epic:agent-cli-target
scope:
- confidence: cited
  path: crates/ports/src/store.rs
- confidence: cited
  path: crates/session/src/lib.rs
- confidence: cited
  path: crates/session/tests
- confidence: cited
  path: crates/surf/src/lib.rs
revision: 5
---
## Why

A session process keeps, for the life of the session, something of every HTML document it
parses, and nothing bounds the sum over documents.

- `surf::Browser` keeps the node tree of each parsed document, and the node of each of its
  element rows, in `kept: BTreeMap<String, Kept>` (`crates/surf/src/lib.rs`, `struct Browser`).
  The parser step inserts into it when `FinishParsing` answers `parsed`; no code removes an
  entry (no `remove` or `clear` of `kept` in `crates/surf/src/lib.rs`).
- One tree is bounded by `surf-html`'s limits (`crates/html/src/limits.rs`): at the defaults
  at most 2 × 64 MiB + 3 × 136 × 1,048,576 bytes of heap, about 536 MiB, a figure the code
  documents as "inferred from the bound, not measured".
- The rows of the components stay too: per `go` one action, one navigation, one request, and
  for an HTML page one document, its title and one element row per actionable element
  (`crates/session/src/lib.rs`, "What it keeps": "no bound over all documents yet"). Since
  `story:agent-forms-spec` the `document` component also holds form, option and submission
  rows; the in-memory ports have `delete` (`crates/ports/src/store.rs`), and no command of the
  session calls it.
- The session's own record is emptied after each request: `Session::answer` calls
  `Browser::forget_delivered`, and `memory::bodies_of_answered_commands_are_not_kept`
  (`crates/session/tests/memory.rs`) asserts `held()` is `(0, 0, 0)` after each of 20 `go`s.
  That test counts the events, invocations and bodies of the record only, not the kept trees
  or the rows.
- How long a session lives: not known as a bound. `IDLE_EXIT` (600 s) is how long `surfd`
  runs "with no open session and no connection" (`crates/surfd/src/daemon.rs`); it does not
  end an open session.
- How much a session process holds after `n` pages, measured: not known. No test and no log
  in the round's scratch directory measures the heap of a session process.

The text cache of the fix round of round 1 (`crates/session/src/lib.rs`, "The page as text")
does not change this: it keeps the rendered lines of the current document only, at most one
rendering per form, and drops them with every request but `look` and when the current
document is another one (tests `core::a_page_read_to_its_end_is_rendered_once` and
`core::a_look_after_a_go_shows_the_new_page` in `crates/session/tests/core.rs`). It adds at
most two renderings of one document to what the session holds; it neither keeps nor frees
anything of an earlier document.

## Change

- `surf::Browser` keeps the tree of the current document of each tab, or of a bounded number
  of documents, and drops the others' trees. Which documents a session still needs a tree
  for (the current one only, or also the ones a later `back` story returns to): not known;
  the story decides it.
- The element, form, option and submission rows of a document whose tree is dropped are
  deleted through the storage traits, or kept with a stated bound per session. Which of the
  two: not known; the story decides it.
- `crates/session/src/lib.rs`, "What it keeps", states the bound over all documents.
- The text cache of round 1 keeps its rule (the current document only).

## Acceptance

- A count test, no time bound: a session that loads `n` and `2n` HTML pages in turn (each
  through a stub transport, no network) holds the same number of kept trees after both runs,
  the number the bound states, read through a counting hook of `surf::Browser` (for example
  the number of documents `document_tree` answers for).
- The same test holds the element rows the `document` component keeps to the bound after `n`
  and `2n` pages.
- `look --as tree` and `look --as markdown` on the current document after `2n` pages answer
  as before; a document whose tree was dropped answers `note text: no-document`.
- `memory::bodies_of_answered_commands_are_not_kept` still passes.
- No test of the story runs over 60 s; the step of `surf-session` within 120 s.
