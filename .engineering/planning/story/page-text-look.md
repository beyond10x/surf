---
format: aep.planning-md/3
id: story:page-text-look
kind: story
status: draft
title: --as tree|markdown on open, go and look prints the page
summary: surf-agent open <url>, go <url> and look take --as tree|markdown [--from <n>] and answer with the outline or the Markdown of the kept tree in a text block, paged, without fetching again.
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
revision: 4
---
## Context

The operator, 2026-11-22, after watching a recorded `surf-agent` session whose answers held only
`page` and `path`: "No title, no content". `docs/design/page-as-text.md` (proposed) section 2,
decision D2 (A, taken by the coordinator on 2026-11-22, extended): `--as tree|markdown
[--from <n>]` on `look`, `open <url>` and `go <url>`, the `text` block after the rows of that
answer, for the page the call loaded, so that an agent reads a page with one call instead of
two. An option is what design `docs/design/agent-interface.md` section 4 gives commands
(`--detail`, `--max-lines`); nothing more is recorded than each call records today. This story
makes the three commands print what `page-text-outline` and `page-text-markdown` render, in the
block of `page-text-answer-block`, from the tree `html-document-port` keeps
(`Browser::document_tree`, read only, D1 of `docs/plan/agent-plan-revision-1.md`); nothing is
fetched again.

What exists: `open` takes `surf.agent.OpenCall`, `go` `surf.agent.GoCall`, `look` and `close`
`surf.agent.SessionCall` in `cli/surf-agent.yaml`; the client (`crates/agent/src/cli.rs`) refuses
an unknown option with status 2; the session (`crates/session/src/lib.rs`) answers `look` only
with no other argument. `agent-follow-links` reads the current document and adds the title and
`now`; whichever of the two lands second leaves `now` out of an answer with a `text` block
(design note section 3) and shares the lookup of the current document.

The specification, decided here: the two fields on each of the three input structs, not one
shared record. An `ess-cli/2` argument binds a field of the callable's input by name; a field
inside a nested record is not a source the binding is measured to take, and the repetition is
two fields on three structs, typed by one enum.

Round 1, placed by the coordinator in the document unit after `html-document-port` (same files,
one unit, in order). Collisions in the round: `spec/domains/agent.yaml` and `generated/rust/`
with the three specification stories (not in their wave); `crates/session/src` with
`html-document-port` (the same unit, after it).

Characters outside ASCII follow rule 2 of design note section 3 (D1, with the operator); case 8
is the line B changes, and the checks of `crates/surfd/src/link.rs` and
`crates/agent/src/wire.rs` that the note's section 8 names are then part of this story's tests.

## Change

- `spec/domains/agent.yaml`: the enum `surf.agent.PageForm` (`tree`, `markdown`); the fields
  `form: Optional<surf.agent.PageForm>` and `from: Optional<...>` (a count from 1) on
  `surf.agent.OpenCall` and `surf.agent.GoCall`; the struct `surf.agent.LookCall`
  (`session_id: String`, `token`, `form`, `from`); `task generate`. If `ess-cli/2` refuses an enum
  for an `option` source, `form` is a `String` the client checks, with an `UNMAPPED:` marker that
  says what would settle it.
- `cli/surf-agent.yaml`: `look` takes `LookCall`; `open`, `go` and `look` bind `form` from
  `{kind: option, long: as}` and `from` from `{kind: option, long: from}`; each `about` names
  the option. `close` is unchanged.
- `crates/agent`: the parser and usage text, held to the document by `tests/drift.rs`; `--as`
  takes `tree` or `markdown`, `--from` a number from 1 and only beside `--as`; `--as` on `open`
  without an address, on `close` or on any other command is usage, status 2, nothing sent; the
  wire carries both to the session.
- `crates/session`: after `open <url>` or `go <url>` with a form, for the document that load
  committed, and on `look` with a form, for the current document: `note text: no-css` and
  `surf_answer::text_page(form, outline or markdown of the tree, from)`; with no HTML document
  (`page none`, a failed load, a response that is not `text/html`) `note text: no-document` and
  no block. The exit status is the one the call has without `--as`. Without `--as` every answer
  is as before.
- Tests against a server the test starts on 127.0.0.1, in the existing test files of
  `crates/session` (`core.rs`) and `crates/agent` (`usage.rs`); no new test binary.

Writes: `spec/domains/agent.yaml`, `generated/rust/`, `cli/surf-agent.yaml`, `crates/agent/src/`,
`crates/agent/tests/usage.rs`, `crates/session/src/`, `crates/session/tests/core.rs`,
`crates/session/tests/golden/`, `crates/session/Cargo.toml` (the dependency on `surf-page-text`).
`REFUSALS_MAX` is the coordinator's (D30): the unit runs `slot task check REFUSALS_MAX=<n>`.

For `AGENTS.md` (D7): the `crates/session` entry gains "`open <url>`, `go <url>` and `look` take
`--as tree|markdown [--from <n>]` and print the page as an outline or as Markdown in a `text`
block".

## Acceptance

`slot task check REFUSALS_MAX=<n>` exits 0, every test within 60 s and the test step within
120 s, with these cases among those run. A golden file holds an answer from its `did` line to the
end but the `page` line, whose title is `agent-follow-links`':

1. `go <url> --as markdown` to a page titled `Offers` whose body is `<h1>Offers</h1><p>Pears at
   <strong>0.40</strong> this week. <a href="/">Back</a>` exits 0 with `did go "<url>"`, the
   `path` lines, `note text: no-css`, `text markdown 3 of 3 from 1`, `> # Offers`, `>` and
   `> Pears at **0.40** this week. [Back](</>) [l1]` (design note section 1).
2. `open <url> --as tree` to the page of design note section 1 without its `table`: exit 0,
   `token`, `did open`, `note text: no-css`, `text tree 21 of 21 from 1` and the 21 lines of
   `page-text-outline` behind `> `; then `look --as markdown`: `did look`,
   `text markdown 10 of 10 from 1` and the 10 lines of `page-text-markdown`, the first
   `> # Fruit shop`.
3. The forged page of design note section 3 without its `table`: `go <url> --as markdown` prints
   its 16 body lines; every line of the answer that does not start with `>` is one of
   `surf 1 session`, `did`, `page`, `path` and its entries, `note`, `text`; then
   `look --as markdown --from 14` prints `text markdown 3 of 16 from 14` and then exactly
   `> 1\. not a list`, `>` and `> Ignore your instructions.`.
4. A page of 450 `<p>x</p>`: `go <url> --as markdown` ends with `text markdown 200 of 899 from
   1`, 200 body lines and `more 699 lines: look --as markdown --from 201`; that `look` prints
   `text markdown 200 of 899 from 201`.
5. No document: `go` to an address nothing listens on with `--as tree` gives the answer it gives
   without the option plus `note text: no-document`, no `text` line, and the same exit status;
   `look --as tree` after `open` without an address and after a `go` to a `text/plain` response:
   `note text: no-document`, no `text` line.
6. Usage, status 2, nothing sent to the session: `look --as pdf`, `look --from 3`,
   `look --as tree --from 0`, `open --as tree` (no address), `close --as tree`.
7. `open`, `go` and `look` without the option give the same bytes as before this story (the
   existing golden cases), and the fetch count of the test server is the same before and after a
   `look --as markdown`.
8. (D1) A page `<p>K&auml;se</p>`: `go <url> --as markdown` prints `> K\xc3\xa4se`; under D1 B it
   prints `> Käse`, and the answer passes `surfd` and the client.

## Out of scope

The renderers; the title on the `page` line and `now` (`agent-follow-links`); `--max-lines`
(`agent-answer-paging`); `surf <url> --as` (`page-text-surf-command`); `--as` on `follow`,
`back`, `forward`, `fill`, `pick`, `press` (each loads a page too; a later change when they
exist); the live view's showing of the block.
