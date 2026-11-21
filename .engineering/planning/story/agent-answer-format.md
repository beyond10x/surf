---
format: aep.planning-md/3
id: story:agent-answer-format
kind: story
status: implemented
title: 'The agent answer format: its grammar, and a renderer page text cannot forge'
summary: docs/agent-answer.md states the line grammar of step 1; crates/answer renders it with every page, server and address value quoted and escaped.
relations:
- decomposes: epic:agent-cli-target
scope:
- confidence: cited
  path: crates/answer
- confidence: cited
  path: docs/agent-answer.md
revision: 9
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:08Z", actor: "human:timo", revision: 7}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:08Z", actor: "human:timo", revision: 8}
- {from: "active", to: "implemented", at: "2026-11-21T01:25:57Z", actor: "human:timo", revision: 9, decided_on: {"recorded":{"test_result":1,"verification":1}}}
---
## Context

Design section 5: one line-oriented format for every command, printable ASCII only; every value
that comes from a page, a server or an address is escaped and written inside double quotes, and
page text never starts a line or looks like a field. The reader is a language model and page text
is untrusted input to it. ESS has no text rendering of a view row (`docs/plan/agent-cli-draft-1.md`,
marker 11), so the grammar is written here with golden tests (design section 6).

`escape` in `crates/surf/src/lib.rs` writes printable ASCII other than the backslash as itself and
everything else as `\x` with two hexadecimal digits per UTF-8 byte. It lets `"` through, which
inside a quoted field would end it early.

The design example (section 5.1) writes addresses bare; the epic says every value from an address
is quoted. This follows the epic: addresses and media types are quoted; numbers and the words of
the format are not.

The renderer is a library so the session process and the scenario runner
(`epic:e2e-scenario-runner`, story `runner-formats`) print and read the same lines. It takes plain
Rust values, not generated types, so it depends on no other story.

## Change

- `docs/agent-answer.md`: the grammar, in prose and in a notation it names, for the blocks of step
  1: the head line `surf 1 session <id> <state>`, the state one of `open`, `closed`, `died` (a
  session whose process ended under `surfd` without `close`) and `unknown` (an identity `surfd`
  does not list); `token <64 hexadecimal digits>` in the answer of
  `open` only; `did <command> <argument>` or `did refused <reason>`, the reasons from a fixed list the
  document keeps (step 1: `token`, `closed`, `usage`, `no-such-session`, `session-died`,
  `session-timeout`); `page none`,
  `page <status> "<media type>" "<address>"` with an optional quoted title,
  `page failed <reason> "<address>"`; `path <shown> of <total>` and its indented lines
  `<n> <command> "<argument>"`; `note` lines from a fixed list the document keeps, which starts
  with `note parser: <parts>`: the parts of the tree builder not implemented yet, as the names in
  `surf_html::NOT_YET_PARSED` give them (`html-document-port` exposes the list as
  `Browser::parser_not_yet_parsed`), comma-separated, each a lower-case word checked against
  `[a-z-]+` and refused by the renderer otherwise. `now` and `changed` are
  named as defined by later stories. The exit statuses of design section 4 and the rule that a
  refusal still prints the answer.
- The quoting rule, in the document and in one function: a field from a page, a server or an
  address is written in double quotes; inside them `"`, `\` and every character outside U+0020 to
  U+007E become `\x` and two lower-case hexadecimal digits per UTF-8 byte; a field with a length
  limit (a label: 60 characters, design section 5.2) is cut before escaping on a character
  boundary and then ends in `...`; no other part of a line holds an outside value.
- `crates/answer` (package `surf-answer`, standard library only, `#![forbid(unsafe_code)]`): typed
  values for each block, one function that renders an answer to lines, and the reverse (decision
  D20 of `docs/plan/agent-plan-revision-2.md`): `unquote`, from one quoted field to the exact
  value, refusing anything the quoting function cannot have written; and `parse`, from lines to
  the typed answer, refusing a line outside the grammar. Parsing what `render` wrote gives the
  value back; a field cut on its length limit parses to its kept part and a mark that it was cut.
  `surfd` and the live view read an answer only through these two (D20, D26). Every later story
  that adds a block adds its parse and a round-trip case.
- `crates/answer/tests/`, the counted cases:
  - 11 golden answers, one per block form of step 1 (head, head of a died session, head of an
    unknown session, token, did, did refused, page, page none, page failed, path, `note parser`
    with two parts), each rendered and compared byte for byte with `crates/answer/tests/golden/`;
  - 11 round trips, one per golden file: `parse` of the file gives the typed value of its golden
    test, and `render` of that value gives the file byte for byte;
  - 6 `unquote` tests: `"a\x22b\x5cc\xe2\x80\xa8"` gives `a"b\c` and U+2028; refused: no
    closing quote, `\x` with one digit, upper-case hexadecimal digits, a byte outside U+0020 to
    U+007E between the quotes, a `\x` sequence that is not UTF-8;
  - 3 `parse` refusals: a line outside the grammar, a refusal reason not in the list, an answer
    without its head line;
  - 1 `note parser` test: a part outside `[a-z-]+` is refused by the renderer;
  - the forging matrix, 5 field tests: each of 10 forging values (a line feed, a carriage return,
    U+2028, U+2029, a bidirectional control, `"`, `\`, `" now 5`, a value beginning `page `,
    10,000 characters) is rendered into each of the 5 outside fields of step 1 (the `page`
    address, the `page` media type, the `page` title, the `page failed` address, the `path`
    argument); each of the 50 answers holds only bytes from U+0020 to U+007E and line feeds, has
    exactly the line count of the same answer with a plain value, and `parse` gives back exactly
    the values given.

Surfaces written: `docs/agent-answer.md`, `crates/answer/`.

## For AGENTS.md (handed over in the unit report, decision D7)

- "Layout": "`crates/answer` (`surf-answer`): renders an agent answer in the grammar of
  `docs/agent-answer.md`; every value from a page, a server or an address goes through one
  quoting function. No dependency."

## Acceptance

`cargo test -p surf-answer` passes with the 32 counted tests of Change, each named after its form,
field or input: 11 golden, 11 round trips, 6 `unquote`, 3 `parse` refusals, 1 `note parser`
refusal; and the 5 forging field tests reporting 50 round trips equal to the values given. `cargo
tree -p surf-answer` prints the crate alone.

## Out of scope

The `now` and `changed` blocks and the `log` answer (stories `agent-follow-links`,
`agent-fill-pick`, `agent-press-submit`, `agent-back-forward`, `agent-change-and-log`); JSON (design
section 10, question 5: no); the detailed mode (`epic:detailed-mode`, decision D3).

## Scope

Derived 2026-11-21 by `story-scoper`.

- **Primary surface:** `crates/answer/` — cited, the story's "Surfaces written" (`.engineering/planning/story/agent-answer-format.md:68`) names it. It is a new crate: `crates/` holds only `fetch`, `ports` and `surf`.
- **Files written:** `crates/answer/Cargo.toml` — cited, the story (line 55) gives the package `surf-answer` with no dependencies. Inferred from the template `crates/fetch/Cargo.toml`: edition 2021, `publish = false`, empty `[dependencies]`.
- **Files written:** `crates/answer/src/lib.rs`, plus further modules under `crates/answer/src/` — inferred, the story names the crate, the typed values, one render function and `#![forbid(unsafe_code)]` (lines 55-56) but no file names. `crates/fetch/src/lib.rs:75` is the pattern.
- **Files written:** `crates/answer/tests/` — cited (story lines 57-66), covering the 11 golden tests, the `note parser` refusal test and the forging matrix. The test file names are inferred.
- **Files written:** `crates/answer/tests/golden/` — cited (story line 60), the golden files compared byte for byte.
- **Files written:** `docs/agent-answer.md` — cited (story lines 34 and 68). New: `docs/` holds only `design/` and `plan/`.
- **Files read, not written:** `crates/surf/src/lib.rs:221` (`escape`) — cited, the story (lines 21-23) describes its behaviour. The new quoting function is its own, so `escape` stays unchanged. Whether it is a copy or a variant of `escape` is inferred.
- **Files read, not written:** `docs/design/agent-interface.md` sections 4, 5 and 5.1-5.2 — cited, the story gives them as the source of the grammar, the exit statuses, the example and the label limit.
- **Files read, not written:** `docs/plan/agent-cli-draft-1.md` marker 11 — cited, the story gives it as the reason the grammar is written here.
- **Files read, not written:** `surf_html::NOT_YET_PARSED` — cited, the story (lines 41-43) names it only as the source of the `note parser` part names. The renderer takes plain strings and checks them against `[a-z-]+`, so it has no code dependency. The crate does not exist yet.
- **Handed to the coordinator:** `AGENTS.md` "Layout" — cited, story lines 70-73 give the sentence: "`crates/answer` (`surf-answer`): renders an agent answer in the grammar of `docs/agent-answer.md`; every value from a page, a server or an address goes through one quoting function. No dependency."
- **Handed to the coordinator:** root `Cargo.lock` — cited, `AGENTS.md` Layout (a new crate fails `--locked` until `cargo generate-lockfile --offline` has run) and decision D7. It needs one new `surf-answer` package entry with no dependencies.
- **Handed to the coordinator:** root `Taskfile.yml` — none, cited. The story adds no task (D16 applies only to a story that does).
- **Handed to the coordinator:** `README.md` — none, inferred. The story does not mention it.
- **Symbols:** crate `surf-answer`, lib name `surf_answer` — cited (package name) / inferred (lib name).
- **Symbols:** typed value types for each block of step 1: head and state (`open`, `closed`, `died`, `unknown`), token, `did` and `did refused` with a fixed reason list, `page`, `page none`, `page failed`, `path`, `note parser` — cited that they exist (story line 55-56). Their names are inferred.
- **Symbols:** one render function from an answer to lines — cited ("one function that renders an answer to lines", story line 56). The name is inferred.
- **Symbols:** one quoting function, which applies the label cut at 60 characters, then `"` and `\` and `\x` escaping, then double quotes — cited (story lines 49-53, "in the document and in one function"). The name is inferred.
- **Symbols:** an error returned when a `note parser` part is outside `[a-z-]+` — cited that the renderer refuses it (story lines 41-45). The error type is inferred.
- **Would collide with:** `crates/answer/` and `docs/agent-answer.md`. Every later story that lists them under "Surfaces written" collides: `agent-follow-links`, `agent-fill-pick`, `agent-press-submit`, `agent-back-forward`, `agent-change-and-log` and `runner-detail-option`. Those bodies were read, and each adds blocks to the same two paths. Cited, by their "Surfaces written" lines and their `crates/answer` bullets.
- **Would collide with:** the other stories do not write either path. `agent-session-process`, `agent-surfd-supervisor`, `agent-session-events`, `view-surfd-serves` and `runner-formats` depend on or read `surf-answer` without writing it. Cited, by their dependency lines and their text.
- **Would collide with:** only `runner-formats` reads `docs/agent-answer.md`, as it stands after `agent-change-and-log` (line 29). The coordinator's files are written once per wave and are not collisions.
- **Not yet existing when it starts:** nothing. The story has no `depends_on` edge, only `decomposes epic:agent-cli-target`, and says it "depends on no other story" (story line 23). It must exist before `agent-session-process`, `agent-surfd-supervisor` and `runner-formats`, which depend on it.
- **Confidence:** high — the story states both written surfaces and their layout. The tree confirms the crate and the document are new and nothing on `main` shadows either name. The file and symbol names inside the crate are inferred and cannot collide outside it.
