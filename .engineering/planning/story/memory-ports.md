---
format: aep.planning-md/3
id: story:memory-ports
kind: story
status: implemented
title: In-memory ports for the generated components, one store per component
summary: surf-ports implements the six storage traits and identity assignment and builds each component port.
relations:
- decomposes: epic:headless-rust-target
- depends_on: story:generated-rust-workspace
scope:
- confidence: cited
  path: crates/ports
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-11-20T21:27:44Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "proposed", to: "active", at: "2026-11-20T21:27:44Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "active", to: "implemented", at: "2026-11-20T21:58:07Z", actor: "human:timo", revision: 8, decided_on: {"recorded":{"test_result":1,"review_outcome":1,"verification":1}}}
---
## Context

The generated crate `surf-types` implements every command and view over ports the implementor
supplies (`generated/rust/crates/surf-types/src/behaviour.rs`): one storage trait per entity
(`DocumentStorage`, `NavigationStorage`, `RequestStorage`, `FrameStorage`, `TabStorage`,
`WindowStorage`, each `get`, `put`, `delete`, `list` by identity) and a `Context` that assigns the
five kinds of identity the model leaves to the implementation. `Generated<P>` is the bundle a
component port is built from. No implementation of the ports exists.

`AGENTS.md`, "Rules": a component holds its own rows and reads no other component's.

The crate joins the root Cargo workspace of story `generated-rust-workspace`. The root
`Cargo.lock` gains an entry for it; in a wave that file is the coordinator's and this unit's
change to it is not merged.

## Change

A library crate `crates/ports` (package `surf-ports`), standard library only, depending on
`surf-types`:

- in-memory storage for the six entities, keeping insertion order for `list`;
- one ports type per component, holding only that component's entities: shell (`Window`, `Tab`),
  navigator (`Navigation`), net (`Request`), document (`Document`), renderer (`Frame`). No type
  in the crate gives one component's behaviours access to another component's rows;
- identity assignment as version 4 UUIDs (RFC 9562, named from memory) from a random source the
  ports type is built with. The default source is the operating system's, read with the standard
  library and no third-party crate. A source that cannot be read is reported through the
  generated `TryContext` as an unmet obligation, never replaced by a predictable value;
- constructors that give each of the five generated component ports
  (`shell::Shell`, `navigator::Navigator`, `net::Net`, `document::Document`,
  `renderer::Renderer`) its `Generated<…>` bundle.

The tests:

1. each of the six storages through `put`, `get`, `list` in insertion order, `put` replacing a
   row in place, and `delete`;
2. 1,000 assigned identities are distinct and each has the version and variant bits of a
   version 4 UUID;
3. a ports type built with a random source that fails answers an identity request with an unmet
   obligation, and `OpenWindow` through it returns that obligation as its error;
4. all five generated component ports are built from this crate, and `surf.shell.OpenWindow`
   answers `opened` followed by one row from the `surf.shell.Windows` view;
5. a `compile_fail` documentation test in which the navigator's component port is built from
   the shell's ports type, with a comment naming the storage trait the compiler misses.

## Acceptance

`cargo test -p surf-ports` passes, documentation tests included, with the five numbered tests
above each present under a name that says its case.

## Out of scope

Durable storage. The system assembly and the binding pump (story `headless-cli`).

## Scope

Derived 2026-11-20 by `story-scoper`. Every line is **cited** (read from the story or the tree) or
**inferred** (a reading that could be wrong).

- **Primary surface:** `crates/ports` (package `surf-ports`), a new directory; no file in it exists yet — cited
- **Files:** `Cargo.lock` at the repository root gains the `surf-ports` entry; the story calls it the coordinator's file in a wave, and this unit's change to it is not merged — cited
- **Files:** `crates/ports/Cargo.toml`, `crates/ports/src/lib.rs` (carries the `compile_fail` documentation test), and tests 1 to 4 in `crates/ports/src/` or `crates/ports/tests/` — inferred, Cargo's layout; the story names only the directory
- **Reads, does not change:** `generated/rust/crates/surf-types/src/behaviour.rs` (storage traits `:25`–`:128`, `Context` `:134`, `TryContext` `:158`, `unmet_context` `:180`, `Generated<P>` `:187`) — cited
- **Reads, does not change:** root `Cargo.toml`; its members are the glob `crates/*`, so adding the crate edits nothing there (story `generated-rust-workspace`, "Change") — cited
- **Symbols:** `DocumentStorage`, `NavigationStorage`, `RequestStorage`, `FrameStorage`, `TabStorage`, `WindowStorage`, `Context`, `TryContext`, `Generated<P>`, `shell::Shell`, `navigator::Navigator`, `net::Net`, `document::Document`, `renderer::Renderer` — cited
- **Symbols:** bounds each ports type must meet: shell `TryContext + TabStorage + WindowStorage` (`behaviour.rs:434`, `:463`, `:495`, `:586`, `:604`); navigator `TryContext + NavigationStorage` (`:221`–`:308`, `:530`); net `RequestStorage` only (`:328`, `:357`, `:386`, `:549`); document `TryContext + DocumentStorage` (`:202`, `:513`); renderer `TryContext + FrameStorage` (`:405`, `:567`) — cited
- **Symbols:** `surf_types::primitives::Uuid` is `pub struct Uuid(pub String)` (`generated/rust/crates/surf-types/src/primitives.rs:32`), so an assigned identity is the UUID's text form — cited
- **Also likely:** path dependencies from `crates/ports/Cargo.toml` on the generated crates `shell`, `navigator`, `net`, `document`, `renderer` besides `surf-types`; the story says "depending on `surf-types`", but its constructors and test 4 name the five port types, which live in those crates — inferred
- **Documents:** none; the Change and Acceptance name no document — cited
- **Confidence:** high — the story names the crate directory, the package and the trait file, and the sample tree holds every symbol it names; only the file layout inside the crate is guessed — cited
- **Would collide with:** any unit that creates or edits files under `crates/ports`; any unit that adds a crate to the root workspace (root `Cargo.lock`); any unit that changes `spec/` or regenerates `generated/rust`, because the trait signatures and identity kinds this crate implements move with it — inferred
- **Safety fact:** the change is additive: it creates one directory and edits no existing file except `Cargo.lock`, because the root members are the glob `crates/*`, and nothing in the tree depends on `surf-ports` (it is mentioned only in the planning store) — step 1 for the glob (stated in story `generated-rust-workspace`, the manifest does not exist yet), step 2 for the absence of dependents, unproven — cited

Coordinator's note, 2026-11-20: `Cargo.lock` is not written as a typed scope entry, for the
reason given in the Scope of `story:http-fetch-library`. Two hazards the scoper read and did not
compile go into the unit brief: `Navigator::new` carries no bound (the bound is on the handler
block), so the `compile_fail` test must call a handler or a `surf-ports` constructor that carries
the bound, or it passes for the wrong reason; and the blanket `impl<T: Context> TryContext for T`
means a ports type implements `TryContext` directly and never `Context` as well.

Confirmed by the implementor, wave 3, unit `b7686c3`, gated at `aa52c47` (2026-11-20):

- Landed on `crates/ports/Cargo.toml`, `crates/ports/src/{lib,ports,random,store}.rs` and `crates/ports/tests/ports.rs`. Nothing else.
- Inferred file layout: confirmed. Tests 1 to 4 are in `tests/ports.rs`; test 5 and its passing twin are documentation tests in `src/lib.rs`.
- Inferred path dependencies on `shell`, `navigator`, `net`, `document`, `renderer`: confirmed; `cargo tree` lists those five and `surf-types`.
- Both hazards in the coordinator note held as read: no ports type implements `Context`, and the `compile_fail` test goes through `navigator_port`, which carries the bound, and is pinned to `E0277`.
- Departure from the body: `NetPorts` takes no random source, because the net behaviours assign no identity.
