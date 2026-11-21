---
format: aep.planning-md/3
id: story:html-decisions
kind: story
status: implemented
title: Settle how the HTML parser meets the specification, its test data and the label
summary: A design note answers the epic's open questions, fixes the case tags and inventories the html5lib test data.
relations:
- decomposes: epic:html-parser
scope:
- confidence: cited
  path: docs/design/html-parser.md
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:11Z", actor: "human:timo", revision: 5}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:11Z", actor: "human:timo", revision: 6}
- {from: "active", to: "implemented", at: "2026-11-21T01:25:58Z", actor: "human:timo", revision: 7, decided_on: {"recorded":{"test_result":1,"verification":1}}}
---
## Context

`epic:html-parser` (step 2 of section 9 of `docs/design/agent-interface.md`, accepted as the
basis for the epics on 2026-11-21) leaves four questions "to be settled by this epic's first
story". Every later story of the epic reads the answers, and the conformance gate reads parts of
them by program, so they are written down once, in a design note, before any of them starts.
This story is a decision story: its outcome is that note, and no code.

Decisions already taken across the three epics bind this note (`docs/plan/agent-plan-revision-1.md`):
D1 (the session process reads the tree through `Browser`; no port receives it), D2 (the tree is
not changed before scripts exist; control state lives in rows), D10 (the document base URL is a
function in `crates/page` written by `story:agent-follow-links`), D11 (a `select` element stores
no value), D12 (pages are parsed by an incomplete tree builder until the tree chain ends, and the
answer says so in a `note` line).

What exists: `spec/domains/document.yaml` holds a `Document` as an identity and an address, with
an `UNMAPPED:` marker for the node tree, parse errors and the character encoding.
`surf.document.ParseDocument` is generated in full (`generated/rust/crates/surf-types/src/
behaviour.rs`, `impl ParseDocumentBehavior for Generated`). The `net` component already shows
the pattern for work the model does not compute: `surf.net.Fetch` starts it, the transport in
`crates/surf/src/lib.rs` (`Browser::run`) does it, and `DeliverResponse` or `FailRequest` ends
it; the suite drives those ending commands itself (`AGENTS.md`, "Layout", `conformance/`).
`surf-fetch` reports a media type without parameters (`spec/domains/net.yaml`,
`surf.net.MediaType`), so a `charset` parameter of `Content-Type` does not reach the document.
`AGENTS.md` ("Goals") says where a completeness mark is recorded "is not decided yet".

Standards named below are named from memory, to be confirmed by this story: the WHATWG HTML
Standard (its parsing section: the input byte stream, tokenization, tree construction, the end,
the fragment parsing algorithm), the WHATWG Encoding Standard, the html5lib-tests repository
(`github.com/html5lib/html5lib-tests`), the W3C Accessible Name and Description Computation and
HTML Accessibility API Mappings, and web-platform-tests (`html/syntax/parsing/`).

## Change

A new design note `docs/design/html-parser.md`, one section per item below, each with the
answer, what it rests on (a measurement, a file read, a sentence of the standard) and what it
leaves open. The answers this decomposition proposes are given with each item; the story
confirms or replaces each one and says which.

1. **Node tree in ESS or behind the port, and who reads it.** Proposed: behind the parser port.
   The specification holds the document (its title, the encoding used, how parsing ended, the
   navigation it belongs to) and its actionable elements as rows. The tree is held by the
   session assembly (`crates/surf`, `Browser`) per document. The session process reads it
   through `Browser::document_tree`, read only; no component's port receives the tree, the
   `agent` component's included (D1). This narrows the epic's Outcome sentence "the node tree is
   reachable by the `agent` component's port", which the coordinator rewrites to match; the note
   says so. The tree is built by parsing and not changed afterwards before scripts exist; a
   control's value, checkedness and selectedness are rows, as the HTML Standard keeps them apart
   from attributes (D2).
2. **The command shape.** Proposed, after the `net` precedent: `ParseDocument` starts parsing
   and creates the document; the parser step in `crates/surf`, a parameter of `Browser` like the
   transport, parses and issues the ending command or commands that carry the title and the
   actionable elements; the suite issues those ending commands itself, so the interpreter and the
   Rust system both run every scenario. The note records, measured on a scratch copy of `spec/`
   under `.out/` with `ess specify validate` and `ess generate synthesize` (ess 0.96.0), whether
   one ending command can create many element rows from a list input or one command per element
   is needed, and the refusal count either way.
3. **How the conformance data enters the repository.** Proposed: html5lib-tests (MIT licence,
   from memory) vendored at one pinned commit under `crates/html-conformance/data/
   html5lib-tests/`, only the directories used (`tokenizer/`, `tree-construction/`,
   `encoding/`), with its `LICENSE` and a `SOURCE` file naming the address, the commit and the
   date. Read only by the crate `surf-html-conformance`, standard library only, `publish =
   false`, on which nothing depends; its JSON and `.dat` readers are written in Rust, nothing
   converts the data, no Python. The WHATWG `entities.json` for the named character reference
   table: licence (CC BY 4.0, from memory) and place confirmed here, vendored by story
   `html-character-references`. Also fixed here: what one case is (proposed: a tokenizer test
   object in one initial state; a tree-construction `#data` block in one scripting mode, run
   with scripting off unless it says `#script-on`); what is compared (proposed: tokens and parse
   error codes in order for the tokenizer, the tree for tree construction, and the reason the
   tree-construction `#errors` are not compared); which cases are not run and why (proposed:
   `scripted/` directories, which need `document.write`, and any case needing a feature this
   note excludes, such as declarative shadow roots); whether any other directory of the data
   (`serializer/`, others) holds cases a section depends on; and which web-platform-tests parser
   tests are not generated from html5lib-tests, with what each needs.
4. **Which encodings first.** Proposed: the whole label table of the Encoding Standard, so
   sniffing and the prescan name every encoding; decoders for UTF-8, UTF-16LE, UTF-16BE,
   windows-1252 and replacement; windows-1252 as the default when nothing else decides; an
   encoding without a decoder is decoded as windows-1252 and the document records the name it
   could not honour; the transport-layer step is skipped while `surf.net` drops the `charset`
   parameter, an open requirement of its section; a `meta` beyond the prescan window does not
   change the encoding.
5. **What "label" is before CSS.** Proposed: the accessible name computation and the HTML
   mappings, over the tree, with "hidden" read as the `hidden` attribute or
   `aria-hidden="true"` and no generated content; the steps that need rendering are listed as
   not taken; the row holds the whole name, and cutting it to 60 characters is the answer
   format's job.
6. **What the rows hold that the epic leaves open.** Proposed: a link's target is stored as its
   `href` value as written; resolving it against the document base URL is done with `surf-url`
   (`story:agent-url-parser`) by `story:agent-follow-links`, which owns the base URL function
   (D10). A document carries the `surf.net.RequestId` of its navigation, so the agent can find
   the document of a tab. The kinds are the nine of design section 5.2, with the mapping from
   element and `input` type state to kind. A `select` element row holds no value; what is stored
   is each option's selectedness, in rows `story:agent-forms-spec` adds (D11).
7. **Limits.** Proposed defaults, stated like those of `crates/fetch`: input 16,777,216 bytes
   (16 MiB, the fetch body limit), tree depth 512, nodes 1,048,576, attributes per element
   1,024, stored bytes (text and attribute values held by the tree, clones included)
   67,108,864 (64 MiB). Reaching one stops parsing; the document holds what was built and names
   the limit. The depth, node, attribute and stored-byte limits are not in the standard and are
   a deliberate deviation.
8. **The case tags and the section map.** Every case has exactly one tag, written in a fenced
   block `html-inventory` of the note that the harness reads. Tokenizer tags, by the case:
   `markup` (initial state Data, no `&` in the input), `text` (another initial state, no `&`),
   `charref` (the input holds `&`). Tree-construction tags apply to the files the note lists as
   thematic, by the case input read case-insensitively, the latest that applies: `body`,
   `formatting` (a formatting element tag: `a b big code em font i nobr s small strike strong tt
   u`), `tables` (`table caption colgroup col tbody thead tfoot tr td th select`), `template`,
   `foreign` (`svg`, `math`, or a `#document-fragment` case). Every case of a file the note lists
   as mixed (from memory `tests1.dat` to `tests26.dat`, `webkit01.dat`, `webkit02.dat`,
   `tricky01.dat`, `html5test-com.dat`, `pending-spec-changes.dat`) is tagged `mixed`. Encoding
   cases are tagged `encoding`. A case the note does not run is tagged `not-run` with its reason,
   so every other tag has as many cases run as exist. The block gives, per tag, the case count.
   A second fenced block `html-sections` fixes the map from sections of the standard to data
   files (every data file in exactly one section), and per section the requirements the note
   leaves out (for example the insertion point and `document.write`, the transport-layer
   encoding) and the web-platform-tests tests for it that do not run. The gate compares its
   expected files with this block and refuses `complete` for a section while either list is not
   empty.
9. **The inventory and the script hooks.** The data is fetched at the commit chosen, into an
   empty directory under the session scratch directory, and read there; the `html-inventory`
   block lists every data file with its case count and its count per tag. For each place where
   the parsing algorithm reaches a script (the `script` end tag in the text insertion mode, the
   insertion point, `document.write`, the SVG `script` end tag in foreign content, the end of
   parsing with its deferred scripts and events, custom element creation), the note says what
   surf does instead and which section lists it as left out.

Writes: `docs/design/html-parser.md` (new). Nothing else.

## Acceptance

`docs/design/html-parser.md` exists with one section for each of the nine numbered items above,
each stating an answer and what it rests on, and with the fenced blocks `html-inventory` (every
file under `tokenizer/`, `tree-construction/` and `encoding/` of html5lib-tests at the pinned
commit, with its case count and a count per tag that sums to it) and `html-sections` (every one
of those files in exactly one section).

## Out of scope

Any change to `spec/`, any code, vendoring the data into the repository (story
`html-conformance-harness`). A question this note cannot settle is written in it as open, with
the answer proposed here, and reported; it is not settled by assumption.

## Scope

Derived 2026-11-21 by `story-scoper`.

- **Primary surface:** `docs/design/html-parser.md` — cited, the story says "Writes: `docs/design/html-parser.md` (new). Nothing else." (`.engineering/planning/story/html-decisions.md:137`). The file does not exist on the branch; `docs/design/` holds only `agent-interface.md`.
- **Files written:** `docs/design/html-parser.md` (new) — cited, same line. It is a document only: the story says "no code" and "Any change to `spec/`, any code, vendoring the data" are out of scope (`html-decisions.md`, Context and Out of scope).
- **Files read, not written:** `docs/design/agent-interface.md` (section 5.2 kinds, section 9 step 2) — cited, Context and item 6.
- **Files read, not written:** `docs/plan/agent-plan-revision-1.md` (D1, D2, D10, D11, D12) — cited, Context.
- **Files read, not written:** `spec/domains/document.yaml` and `spec/domains/net.yaml` (`surf.net.MediaType`) — cited, Context.
- **Files read, not written:** `generated/rust/crates/surf-types/src/behaviour.rs` (`impl ParseDocumentBehavior for Generated`) — cited, Context.
- **Files read, not written:** `crates/surf/src/lib.rs` (`Browser::run`) — cited, Context.
- **Files read, not written:** `crates/fetch` (limits style) — cited, item 7.
- **Files read, not written:** `AGENTS.md` ("Goals", "Layout") and `conformance/` — cited, Context.
- **Files read, not written:** a scratch copy of `spec/` under `.out/`, run with `ess specify validate` and `ess generate synthesize` — cited, item 2. This is a measurement and nothing from it is committed.
- **Files read, not written:** html5lib-tests, fetched at one commit into an empty directory under the session scratch directory, and the Encoding and HTML Standards — cited, item 9. Nothing is vendored.
- **Handed to the coordinator:** the rewrite of the `epic:html-parser` Outcome sentence "the node tree is reachable by the `agent` component's port" to match item 1, which the note must say it narrows. This is a store write, not a file. — cited, item 1: "which the coordinator rewrites to match".
- **Handed to the coordinator:** `AGENTS.md`, `README.md`, the root `Taskfile.yml` and `Cargo.lock`: nothing needed. — inferred, because the story writes one design note and names none of them. `AGENTS.md` "Goals" ("where a completeness mark is recorded is not decided yet") is another story's edit: `story:html-conformance-gate` hands it over.
- **Symbols:** none; no crate is touched — cited, "a decision story: its outcome is that note, and no code".
- **Would collide with:** the file `docs/design/html-parser.md`. `story:html-tree-complete` adds "one section ... outside the fenced blocks" (`html-tree-complete.md:34-38`) — cited. It comes later, so this is an ordering dependency, not a parallel conflict. `story:html-conformance-gate` reads the `html-inventory` and `html-sections` blocks and `story:html-conformance-harness` reads the inventory total, but neither writes the note — cited. Nothing else writes it. No code unit can collide, because only `docs/design/html-parser.md` is written.
- **Not yet existing when it starts:** nothing from a dependency; the story has no `depends_on`, only `decomposes epic:html-parser`. It reads the file `docs/design/agent-interface.md` and decisions D1 to D12, which already exist on the branch. It needs network access to fetch html5lib-tests and the standards, which is an environment need and no story delivers it.
- **Confidence:** high — the story names its single written file, and the tree confirms the file is absent and the directory exists.

