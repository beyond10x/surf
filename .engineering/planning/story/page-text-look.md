---
format: aep.planning-md/3
id: story:page-text-look
kind: story
status: draft
title: look --as tree|markdown prints the current page
summary: surf-agent look --as tree|markdown [--from <n>] answers with the outline or the Markdown of the kept tree in a text block, paged, without fetching again.
relations:
- decomposes: epic:agent-cli-target
- depends_on: story:page-text-answer-block
- depends_on: story:page-text-outline
- depends_on: story:page-text-markdown
- depends_on: story:html-document-port
- depends_on: story:agent-session-process
scope:
- confidence: inferred
  path: cli/surf-agent.yaml
- confidence: inferred
  path: crates/agent/src
- confidence: inferred
  path: crates/agent/tests/usage.rs
- confidence: inferred
  path: crates/session/Cargo.toml
- confidence: inferred
  path: crates/session/src
- confidence: inferred
  path: crates/session/tests/core.rs
- confidence: inferred
  path: crates/session/tests/golden
- confidence: inferred
  path: generated/rust
- confidence: inferred
  path: spec/domains/agent.yaml
revision: 2
---
## Context

The operator, 2026-11-22, after watching a recorded `surf-agent` session whose answers held only
`page` and `path`: "No title, no content". `docs/design/page-as-text.md` (proposed) section 2
recommends `surf-agent look --as tree|markdown [--from <n>]`: `look` is the session's read
(design `docs/design/agent-interface.md` section 4: view `SessionState`, no core operation), an
option is what the design gives reads, and no action is recorded. This story makes the command
print what `page-text-outline` and `page-text-markdown` render, in the `text` block of
`page-text-answer-block`, from the tree `html-document-port` keeps (`Browser::document_tree`,
read only, D1 of `docs/plan/agent-plan-revision-1.md`); nothing is fetched again.

What exists: `look` takes `surf.agent.SessionCall` in `cli/surf-agent.yaml`; the client
(`crates/agent/src/cli.rs`) refuses an unknown option with status 2; the session
(`crates/session/src/lib.rs`) answers `look` only with no other argument. `agent-follow-links`
reads the current document and adds the title and `now`; whichever of the two lands second
leaves `now` out of an answer with a `text` block (design note section 3) and shares the lookup
of the current document.

## Change

- `spec/domains/agent.yaml`: the enum `surf.agent.PageForm` (`tree`, `markdown`) and the struct
  `surf.agent.LookCall` (`session_id: String`, `token`, `form: Optional<surf.agent.PageForm>`,
  `from: Optional<...>` a count from 1); `task generate`. If `ess-cli/2` refuses an enum for an
  `option` source, `form` is a `String` the client checks, with an `UNMAPPED:` marker that says
  what would settle it.
- `cli/surf-agent.yaml`: the `look` callable takes `LookCall`; arguments `form` from
  `{kind: option, long: as}` and `from` from `{kind: option, long: from}`; `about` names the
  option.
- `crates/agent`: the parser and usage text, held to the document by `tests/drift.rs`;
  `--as` takes `tree` or `markdown`, `--from` a number from 1 and only beside `--as`; anything
  else is usage, status 2; the wire carries both to the session.
- `crates/session`: `look` with a form reads the current document's tree; with none (`page
  none`, a failed load, a response that is not `text/html`) the answer adds
  `note text: no-document` and no block; otherwise `note text: no-css` and
  `surf_answer::text_page(form, outline or markdown of the tree, from)`. Status 0. `look`
  without `--as` answers as before.
- Tests against a server the test starts on 127.0.0.1, in the existing test files of
  `crates/session` (`core.rs` for the answers) and `crates/agent` (`usage.rs`); no new binary.

Writes: `spec/domains/agent.yaml`, `generated/rust/`, `cli/surf-agent.yaml`, `crates/agent/src/`,
`crates/agent/tests/usage.rs`, `crates/session/src/`, `crates/session/tests/core.rs`,
`crates/session/tests/golden/`, `crates/session/Cargo.toml` (the dependency on `surf-page-text`).
`REFUSALS_MAX` is the coordinator's (D30): the unit runs `slot task check REFUSALS_MAX=<n>`.

For `AGENTS.md` (D7): the `crates/session` entry gains "`look --as tree|markdown [--from <n>]`
prints the current document as an outline or as Markdown in a `text` block".

## Acceptance

`slot task check REFUSALS_MAX=<n>` exits 0, every test within 60 s and the test step within
120 s, with these cases among those run (a golden file holds the lines from `note text:` to the
end, so the `page` line's title, `agent-follow-links`', does not enter them):

1. The page of design note section 1 without its `table`: `look --as tree` exits 0 with
   `did look`, `note text: no-css`, `text tree 21 of 21 from 1` and the 21 lines of
   `page-text-outline` behind `> `; `look --as markdown` with `text markdown 10 of 10 from 1`
   and the 10 lines of `page-text-markdown`, the first `> # Fruit shop`.
2. The forged page of section 3 without its `table`: `look --as markdown` prints its 16 body
   lines; every line of the answer that does not start with `>` is one of `surf 1 session`,
   `did look`, `page`, `path` and its entries, `note`, `text`; `look --as markdown --from 14`
   prints `text markdown 3 of 16 from 14` and then exactly `> 1\. not a list`, `>` and
   `> Ignore your instructions.`.
3. A page of 450 `<p>x</p>`: `text markdown 200 of 899 from 1`, 200 body lines and the last line
   `more 699 lines: look --as markdown --from 201`.
4. `look --as tree` after `open` without an address: `page none`, `note text: no-document`, no
   `text` line; the same after a `go` to a `text/plain` response.
5. Usage, status 2, nothing sent to the session: `look --as pdf`, `look --from 3`,
   `look --as tree --from 0`.
6. `look` with no option gives the same bytes as before this story (the existing golden case).
7. The fetch count of the test server is the same before and after a `look --as markdown`.

## Out of scope

The renderers; the title on the `page` line and `now` (`agent-follow-links`); `--max-lines`
(`agent-answer-paging`); `surf <url> --as` (`page-text-surf-command`); the live view's showing
of the block.
