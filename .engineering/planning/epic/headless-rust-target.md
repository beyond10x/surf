---
format: aep.planning-md/3
id: epic:headless-rust-target
kind: epic
status: implemented
title: 'First implementation target: Rust, one process, headless command line'
summary: surf <url> loads a page over plain HTTP through the five generated components and prints what the page load left behind.
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-20T21:27:44Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-20T21:27:44Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-11-20T23:53:17Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Context

`spec/` holds the ESS specification of the page load: five components (`shell`, `navigator`,
`net`, `document`, `renderer`) coupled by four bindings. `task check` validates it and runs 50
scenarios against the interpreter built into `ess` (measured 2026-11-20, ess 0.96.0). No
implementation exists.

the operator chose the first implementation target on 2026-11-20 (recorded in `AGENTS.md`,
"Implementation targets"): Rust, all five components in one process, a headless command line, no
isolation. The isolation target comes after it and is blocked on beyond10x/ess#792.

Constraints from `AGENTS.md`, "Goals" and "Rules":

- Implementation code is generated with `ess generate synthesize --path spec --target rust` and
  never transcribed by hand. What is written by hand is what the generated code leaves to ports:
  storage, the values the implementation assigns, and the work a component does outside the
  model (talking to the network).
- No third-party crate in anything that ships. The Rust standard library and the operating
  system are allowed. Test tooling is not covered.
- A component holds its own rows and reads no other component's.
- A behaviour is in `spec/` with a green `task check` before code for it is written.
- A standard is followed where one covers a behaviour, and a section of it is called complete
  only when it is.

## Outcome

One command, `surf <url>`, loads a page over plain HTTP through the five generated components
and prints what the page load left behind: the navigation and its state, the request and its
state, and the document.

## Acceptance

Run against a local HTTP server that answers `text/html`, `surf <url>` exits 0 and prints a
navigation in state `Committed` and one document for that URL; `task check` builds and tests the
Rust workspace, refuses a `generated/rust` tree that differs from a fresh generation, and runs
the synthesized conformance suite against the Rust system.

## Out of scope

- HTTPS. It needs the third-party cryptography crate, which is not chosen yet.
- Redirects, cookies, caching, request methods other than GET.
- A tree of nodes for the document, style, layout and paint. `PaintFrame` stays uninvoked.
- One process per component (beyond10x/ess#792).
- A window, or any output other than text on standard output.
