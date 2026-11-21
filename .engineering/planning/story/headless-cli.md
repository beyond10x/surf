---
format: aep.planning-md/3
id: story:headless-cli
kind: story
status: implemented
title: 'surf <url>: assemble the system, run the page load, print the result'
summary: The binary wires ports, system and the net transport, and prints the navigation, request and document rows.
relations:
- decomposes: epic:headless-rust-target
- depends_on: story:generated-rust-workspace
- depends_on: story:memory-ports
- depends_on: story:http-fetch-library
- depends_on: story:net-fetch-failure-reasons
scope:
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: README.md
- confidence: cited
  path: crates/surf
revision: 12
transitions:
- {from: "draft", to: "proposed", at: "2026-11-20T21:27:44Z", actor: "human:timo", revision: 8, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "proposed", to: "active", at: "2026-11-20T21:27:44Z", actor: "human:timo", revision: 9, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "active", to: "implemented", at: "2026-11-20T22:18:28Z", actor: "human:timo", revision: 12, decided_on: {"recorded":{"test_result":1,"review_outcome":1,"verification":1}}}
---
## Context

After stories `generated-rust-workspace`, `memory-ports` and `http-fetch-library` the repository
has the generated components, ports for them and a function that fetches over HTTP. Nothing
assembles them. The generated `surf-system` crate holds the five component ports, delivers the
four bindings in its `pump`, and keeps a log of published events. The specification leaves one
thing to the `net` component's own transport: after `surf.net.Fetch` made a request `Pending`,
the transport issues `surf.net.DeliverResponse` or `surf.net.FailRequest`
(`spec/domains/net.yaml`, the comment above `DeliverResponse`).

## Change

A crate `crates/surf` (package `surf`) with a library and a binary, standard library only:

- the library builds the generated `System` from the five component ports of `surf-ports`, and
  is what story `rust-conformance` builds its target from. The transport is a parameter of the
  assembly (a trait with one method: fetch this address), so a test or a conformance run can
  replace the network;
- the transport step: for every `surf.net.RequestStarted` the system published, the library
  asks the transport for the request's address and issues `DeliverResponse` (status, media type,
  body) or `FailRequest` with the reason that matches the failure (`Unreachable`,
  `UnsupportedScheme`, `MalformedResponse`), then pumps again. It issues commands on the `net`
  component's port and on no other;
- the binary, `surf <url>`: uses `surf-fetch` as the transport, issues `OpenWindow`, `OpenTab`,
  `Navigate`, runs the system until no event is undelivered and no request is `Pending`, then
  prints one line per row of the `Navigations` and `Requests` views with identity, address and
  state, one line per row of the `Documents` view with identity and address (a document has one
  state and the view publishes none), and one line per published `surf.net.FetchFailed` with its
  reason. The
  output format is stable and documented in `README.md`;
- exit status 0 when the navigation is `Committed`, 1 when it is `Failed`, 2 for a usage error.
  An unmet obligation from the generated code is printed on standard error and exits 3; it is
  never swallowed.

The tests. Cases 1 to 6 start their own server on `127.0.0.1` and run the binary; each asserts
the exit status and the printed states:

1. an HTML response: exit 0, navigation `Committed`, request `Responded`, one document with the
   address;
2. a `text/plain` response: exit 0, `Committed`, no document;
3. a closed port: exit 1, navigation `Failed`, request `Failed`, reason `Unreachable`;
4. a reply that is not HTTP: exit 1, `Failed`, reason `MalformedResponse`;
5. an `ftp:` address: exit 1, `Failed`, reason `UnsupportedScheme`;
6. no argument: exit 2 and a usage line on standard error;
7. in the library, with ports whose random source fails: the run returns the unmet obligation,
   and the function that turns a run's result into an exit status maps it to 3.

`README.md` and `AGENTS.md` say how to run it.

## Acceptance

`cargo test -p surf` passes with the seven numbered cases above, each under a name that says its
case, and `cargo tree -p surf --edges normal` lists only crates of this repository.

## Out of scope

Rendering, a second navigation in one run, cancelling, concurrency: one request is fetched at a
time on the calling thread.

## Scope

Derived 2026-11-20 by `story-scoper`. Every line is **cited** (read from the story or the tree) or
**inferred** (a reading that could be wrong).

- **Primary surface:** `crates/surf` (package `surf`, library and binary), a new directory that no other story names as its own — cited
- **Files:** `crates/surf/Cargo.toml`, `crates/surf/src/lib.rs`, `crates/surf/src/main.rs`, `crates/surf/tests/` for cases 1 to 6, which run the binary — inferred, the story names the crate and not its files; this is the Cargo layout for a library plus a binary
- **Symbols:** `System`, `System::pump`, `System::published`, `SystemEvent::RequestStarted`, `SystemEvent::FetchFailed`, the `pub net` field (`generated/rust/crates/surf-system/src/lib.rs:200`, `:286`, `:232`, `:36`, `:34`, `:206`); `Net::deliver_response`, `Net::fail_request`, `Net::requests` (`generated/rust/crates/net/src/lib.rs`); `FailureReason` (`generated/rust/crates/surf-types/src/net.rs:15`); `UnmetObligation` (`generated/rust/crates/surf-types/src/obligation.rs`) — cited, read in the generation sample; all are used, none is changed
- **Reads, does not write:** `generated/rust/crates/surf-system`, `crates/ports` (`surf-ports`), `crates/fetch` (`surf-fetch`), as path dependencies — cited
- **Also likely:** `Cargo.lock` gains the `surf` entry; it is the coordinator's file and the unit's change to it is not merged (story `generated-rust-workspace`, Change) — cited
- **Also likely:** root `Cargo.toml` is not edited, because its members are the glob `crates/*` (story `generated-rust-workspace`, Change) — cited
- **Documents:** `README.md` (how to run it, the output format, line 10 "There is no implementation yet") and `AGENTS.md` (how to run it) — cited
- **Documents:** in `AGENTS.md` the edit lands in "Implementation targets" (line 91, "None is built yet") and "Commands" (line 126) — inferred, the story names the file and not the section
- **Confidence:** high — the story names the crate directory and both documents; only the file names inside the crate are inferred — cited
- **Would collide with:** any unit that edits `README.md` or `AGENTS.md`, any unit that creates or edits `crates/surf`, and any unit that changes the public API of `crates/ports`, `crates/fetch` or the generated `surf-system` and `net` crates while this one builds against them — inferred
- **Safety fact:** the transport step needs no edit to generated code: `System.net` is a public field (`surf-system/src/lib.rs:206`), `published()` exposes the log (`:232`), and `RequestStarted` is delivered to no binding (`:365`), so the library can read started requests and issue `DeliverResponse` or `FailRequest` on the `net` port from outside — step 2, unproven — cited

Coordinator's note, 2026-11-20: the scoper found that the `Documents` view row has only
`document_id` and `url`, so the body's "identity, address and state" for a document was wrong.
The Change now prints identity and address for a document. Case 7 needs `surf-ports` to accept a
failing random source from outside the crate; `story:memory-ports` states that its ports types
are built with a random source, and its brief makes that constructor public.

Confirmed by the implementor, wave 4, unit `dd32b9c`, merge `8867e2a` (2026-11-20):

- Landed on `crates/surf/Cargo.toml`, `crates/surf/src/lib.rs` (553 lines), `crates/surf/src/main.rs` (53), `crates/surf/tests/cli.rs` (253), `crates/surf/tests/library.rs` (162), the root `Cargo.lock` (assigned to the unit in this wave), `README.md` and `AGENTS.md`.
- Inferred file layout: correct; the tests are `tests/cli.rs` and `tests/library.rs`.
- Inferred `AGENTS.md` lines: wrong on lines and text. The section "Implementation targets" was at line 107 and said "Nothing assembles them into a running program yet"; "Commands" was at line 164; the "Layout" list and the measured test count also needed edits.
- Cited `README.md` line 10: wrong. The sentence that became untrue was line 13, "Nothing runs them together yet."
- Safety fact (the transport step needs no edit to generated code): held. Nothing under `generated/` changed.
- `crates/ports` and `crates/fetch` were not changed.
