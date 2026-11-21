---
format: aep.planning-md/3
id: story:agent-session-spec
kind: story
status: active
title: Land the agent session domain on main with a green gate
summary: 'The agent component and its session: surf.agent moved from net to it, the sixth port from park/agent-sixth-port (D32), and the surf.agent draft of spec/agent-session revised to caller-chosen identity, a token the session process issues, and open/go/look/close; generated, assembled, conformance-tested.'
relations:
- decomposes: epic:agent-cli-target
- depends_on: story:agent-cli-spec
scope:
- confidence: cited
  path: conformance/Cargo.lock
- confidence: cited
  path: conformance/src
- confidence: cited
  path: conformance/tests/planted_defect.rs
- confidence: cited
  path: conformance/tests/review2_planted_defects.rs
- confidence: cited
  path: conformance/tests/two_of_each_entity.rs
- confidence: cited
  path: crates/ports/Cargo.toml
- confidence: cited
  path: crates/ports/src
- confidence: cited
  path: crates/ports/tests
- confidence: inferred
  path: crates/surf/Cargo.toml
- confidence: cited
  path: crates/surf/src/lib.rs
- confidence: cited
  path: crates/surf/tests/library.rs
- confidence: cited
  path: crates/surf/tests/review1_library.rs
- confidence: cited
  path: generated/rust
- confidence: cited
  path: spec/components.yaml
- confidence: cited
  path: spec/domains/agent.yaml
- confidence: cited
  path: spec/ess-inputs.yaml
- confidence: cited
  path: spec/scenarios
revision: 15
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:10Z", actor: "human:timo", revision: 5}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:10Z", actor: "human:timo", revision: 6}
---
## Context

Step 1 of `docs/design/agent-interface.md` section 9. Story `agent-cli-spec` lands the domain
`surf.agent` with the types of the command line, owned by the component `net`; this story adds
the component `agent`, moves the domain to it, gives it its session and assembles the sixth port.

Decision D32 of `docs/plan/agent-plan-revision-2.md` (found in wave 1): ess 0.96.0 generates for
every component a port with a field `behaviors` that only command and view handlers read
(`ess-synth/src/rust/port.rs:158`), so a component with no command and no view does not build
with warnings denied (`error: field 'behaviors' is never read`,
`generated/rust/crates/agent/src/lib.rs:28`). The component `agent` therefore arrives with its
commands, here, and the sixth port with it. The hand-written assembly was written once, for
`agent-cli-spec`, and is parked on branch `park/agent-sixth-port` (`9e43dcf`): `AgentPorts`,
`agent_port` and `ComponentPorts.agent` in `crates/ports`, `Browser` and `MemoryBrowser` over six
components in `crates/surf`, the six-port system in `conformance/`. With dead code allowed, as a
measurement and not as a gate, it ran green: root workspace 45 test suites, 140 passed;
conformance crate 6 suites, 24 passed; 51 of 51 scenarios against the Rust system; the planted
defect fails 22 of 51 (commit message of `9e43dcf`). This story starts from it.

The first draft of the session is on branch `spec/agent-session`, commit `b48bc87` (base
`3aaa3aa`, older than review 1). It validates and fails the gate three ways
(`docs/plan/agent-cli-draft-1.md`, part 4): the Rust target refuses the whole model because
`OpenSession` marks its token `one_time_response`; 12 of 148 synthesized scenarios fail against
the interpreter, because the suite fixes the caller session identity when it is written and the
draft mints it at run time; `task suite` counts 33 `refused:` lines against `REFUSALS_MAX: 10`
(26 without the one-time marker, measured by the author on a scratch copy).

This story applies the decisions of `docs/plan/agent-cli-draft-1.md` (section "Decisions the
coordinating agent took", by marker number):

- marker 7: the session identity is chosen by the caller (drawn by `surfd` on `open`, story
  `agent-surfd-supervisor`), as
  `surf.net.Fetch` takes `request_id`, with a refusal for an identity that exists. 5 failures
  remained on the scratch copy with this change; they are read and resolved in the model;
- marker 4: the token is not marked `one_time_response` and is in no response; the session
  process issues and checks it, and an `UNMAPPED:` marker at `surf.agent.SessionToken` says so;
- marker 1: 32 bytes from the operating system random source, 64 lower-case hexadecimal digits;
- marker 6: the session process issues `OpenWindow`, `OpenTab` and `OpenSession` itself;
- marker 8: one session process answers only for its own session;
- markers 2, 5, 9 and 10 stand; 3 is answered by `epic:html-parser` (a document carries its
  navigation, story `html-document-spec`); 12 stays open.

Size (decision D5 asks): what is here is the component, the session model, its generation, the
sixth port, its ports obligations and its adapter lines. They are not split further: a change
to `spec/` regenerates `generated/rust` and breaks every caller of the regenerated interface, so
the model, `crates/ports`, `crates/surf/src/lib.rs` and `conformance/` must land in one change for
the gate to stay green, and by D32 the component cannot land before its commands. The parked
assembly makes the sixth port the smaller part.

## Domain relations

Cited from the draft `ess/1` document `spec/agent-session:spec/domains/agent.yaml` (validated
with ess 0.96.0), entity `surf.agent.Session`:

- `Session → surf.shell.Window`: relation `window`, `references`, `one`, via `window_id`. The
  shell owns the window; a closed session leaves it Open (marker 10; backlog
  `story:agent-close-window`).
- `Session → surf.shell.Tab`: relation `tab`, `references`, `one`, via `tab_id`; binding
  `close-tab-on-session-closed` closes it.
- `Session → surf.navigation.Navigation`: relation `navigation`, `references`, `one`, via the
  optional `navigation_id`.

## Change

1. The component `agent` (D32). `spec/components.yaml`: a component `agent`, `reached_by:
   in_process`, owning `surf.agent`, accepting the commands and publishing the events of item 2;
   `surf.agent` leaves the `owns: domains:` of `net`, where `agent-cli-spec` put it. The parked
   entry (`git show 9e43dcf -- spec/components.yaml`) is the starting point, without its
   "accepts and publishes nothing yet" comment and its `UNMAPPED:` marker, which this story
   answers.
2. Take from `b48bc87` (read with `git show`, no merge or cherry-pick) the entity `Session`,
   the actors, errors, commands, events, views, the six bindings and the two authored scenarios
   `agent-session-reads-committed-navigation.yaml` and `agent-session-reads-started-navigation.yaml`,
   and revise them into the `spec/domains/agent.yaml` that `agent-cli-spec` landed:
   `OpenSession(session_id, window_id, tab_id)` refusing an existing identity and answering no
   token; the markers rewritten to their decisions; the `SessionReadout` state typed
   `Session.State`; `surf.agent.OpenCall` kept under that name.
3. An authored scenario for each case the 5 remaining failures showed synthesis cannot arrange,
   named in the file after the case.
4. `task generate`: `generated/rust/` gains the crate `agent` and `System::new` takes six ports.
5. The sixth port, from `park/agent-sixth-port` (`9e43dcf`), read with `git show 9e43dcf --
   <path>` and applied to the current tree, no merge or cherry-pick (the commit also carries its
   own `spec/`, `cli/`, `taskfiles/`, `generated/rust/` and lock files, which `agent-cli-spec` and
   `task generate` write in their own form):
   - `crates/ports`: `AgentPorts`, `AgentPort`, `agent_port` and `ComponentPorts.agent`, and the
     generated `agent` crate as a path dependency in `crates/ports/Cargo.toml`;
   - `crates/surf/src/lib.rs`: `Browser` and `MemoryBrowser` over six components, `from_ports`
     passing `ports.agent`;
   - `conformance/src`: the six-port system (`MemoryTarget`, `ConformanceBrowser`, `SurfTarget`);
   - the tests that call `System::new` or name every field of `ComponentPorts`:
     `crates/ports/tests/ports.rs`, `crates/surf/tests/library.rs`,
     `crates/surf/tests/review1_library.rs`, `conformance/tests/planted_defect.rs`,
     `conformance/tests/review2_planted_defects.rs`, `conformance/tests/two_of_each_entity.rs`.

   Where the parked code says the component "accepts nothing yet" (`agent_port` without bounds,
   the unbounded first parameter of `Browser`), it takes the bounds the generated port now
   requires, as the five others do.

   `crates/surf/Cargo.toml` is not written unless the build needs the generated `agent` crate
   there: the parked attempt added it and nothing used it. `story:runner-surf-cli-package` edits
   that file in the same wave (wave 2 of `docs/plan/agent-waves.md`). If this unit adds the
   dependency, its report says so and why, and the coordinator orders the two. One likely need,
   inferred and not measured: `Browser` keeps its `System` private, so the adapter issues the
   agent commands through an `agent_mut` beside `document_mut` to `shell_mut`
   (`crates/surf/src/lib.rs:408-431`), and that method names `agent::Agent`.
6. `crates/ports`: the obligations the generation leaves for `surf.agent` (the draft measured
   the `instances:` selections of `NoteNavigationStarted` and `NoteNavigationEnded`), with tests
   in `crates/ports/tests/`; `conformance/src/`: the agent commands, views, events and binding
   invocations.
7. `REFUSALS_MAX` is not written here (decision D30): the unit runs its gate as `slot task check
   REFUSALS_MAX=<n>`, `<n>` the measured `refused:` count, and its report gives `<n>` and each
   new refusal with its cause; the coordinator writes the number on the wave branch. The root
   `Cargo.lock` gains the generated package `agent`; it is the coordinator's (D7).

Surfaces written: `spec/domains/agent.yaml`, `spec/components.yaml`, `spec/scenarios/agent-*.yaml`,
`spec/ess-inputs.yaml`, `generated/rust/`, `crates/ports/`, `crates/surf/src/lib.rs`,
`crates/surf/tests/library.rs`, `crates/surf/tests/review1_library.rs`, `conformance/src/`,
`conformance/tests/`, `conformance/Cargo.lock`; `crates/surf/Cargo.toml` only as item 5 says.

## For AGENTS.md (handed over in the unit report, decision D7)

- "Components": a row `agent | surf.agent | Session | OpenSession, Go, CloseSession,
  NoteNavigationStarted, NoteNavigationEnded`; the `net` row's domain column back to `surf.net`,
  `surf.url`; the page-load paragraph gains the six bindings.
- "Layout", six components: `spec/components.yaml` holds "the six components, the four bindings
  of the page load and the six of the agent session"; `generated/rust/` has "Eight crates under
  `crates/`", `agent` among them; `crates/surf` "builds the generated `System` from six component
  ports"; `spec/domains/agent.yaml` is "the `agent` component's domain".
- "Implementation targets": "Rust, all six components in one process". `README.md:14` says "all
  five components" too; it is the coordinator's.
- "The gate's numbers": the new scenario totals, the root workspace test count, and every
  refusal by cause, as the ten of today are listed, each with why it cannot go and which authored
  scenario covers the flow.

## Acceptance

`slot task check REFUSALS_MAX=<n>` exits 0 with at least 120 scenarios passed against the
interpreter and the same number against the Rust system (`0 failed, 0 error, 0 unsupported`),
among them `OpenSession/outcome/opened`, `Go/outcome/requested`, `Go/outcome/not-holder`,
`CloseSession/outcome/closed` and the two authored files named in Change, with at most 26
`refused:` lines and `<n>` equal to that count, the unit report giving `<n>` and each new refusal
with its cause (decision D30); and `RUSTFLAGS="-D warnings" cargo build --locked -p agent` exits
0, the generated crate `agent` building with warnings denied and without `field 'behaviors' is
never read` (decision D32).

## Out of scope

The session process and the client; the action log and page line fields
(`agent-page-path-spec`); a caller-chosen `tab_id` on `OpenTab` (backlog
`story:agent-open-tab-caller-id`); closing a window (backlog `story:agent-close-window`); filing
the ESS issues of the draft report (waits for the operator); `cli/surf-agent.yaml` and
`taskfiles/cli.yml` (`agent-cli-spec`); moving the `surf` binary (`runner-surf-cli-package`).

## Scope

Derived 2026-11-21 by `story-scoper`; revised after D32 on 2026-11-21 by `decomposer`. The
component `agent` and the six-component assembly moved here from `story:agent-cli-spec`.

- **Primary surface:** `spec/domains/agent.yaml` and `spec/components.yaml` — cited (Change items 1 and 2). Nearly everything else is generated from them or is the assembly of the port they add.
- **Files written:**
  - `spec/domains/agent.yaml` — cited. It exists after `agent-cli-spec` (types only); this story adds `Session`, the actors, errors, commands, events and views.
  - `spec/components.yaml` — cited (Change item 1; D32). The component `agent` with its `accepts:` and `publishes:`, `surf.agent` out of `net`'s `owns:` (`spec/components.yaml:46-51`), and the six bindings.
  - `spec/scenarios/agent-*.yaml` — cited. The two authored files of `b48bc87` and one per remaining failure, named after its case (Change items 2 and 3).
  - `spec/ess-inputs.yaml` — cited. Each new scenario file is listed there (`AGENTS.md`, "Layout").
  - `generated/rust/` through `task generate` only, `crates/agent` (new), `crates/surf-system` and `crates/surf-types` among it — cited (Change item 4; the parked diff `git show 9e43dcf --stat`).
  - `crates/ports/Cargo.toml`, `crates/ports/src/lib.rs`, `crates/ports/src/ports.rs` — cited (Change item 5; the parked diff). `AgentPorts`, `AgentPort`, `agent_port`, `ComponentPorts.agent`, and the obligations of Change item 6.
  - `crates/ports/tests/ports.rs` (destructures `ComponentPorts` at `:397`) and a new test file for the agent obligations, its name inferred — cited (Change items 5 and 6; the parked diff).
  - `crates/surf/src/lib.rs` — cited (Change item 5; the parked diff). `Browser`, `MemoryBrowser`, `from_ports`, the impl bounds; an `agent_mut` beside `document_mut` to `shell_mut` (`:408-431`) is inferred.
  - `crates/surf/tests/library.rs` (`System::new` at `:60`, `:83`) and `crates/surf/tests/review1_library.rs` (`:36`) — cited (Change item 5; the parked diff).
  - `conformance/src/lib.rs` and `conformance/src/target.rs` — cited (Change items 5 and 6; the parked diff). `values.rs` may gain agent values — inferred.
  - `conformance/tests/planted_defect.rs` (`System::new` at `:71`), `conformance/tests/review2_planted_defects.rs` (`:175`, `:202`, `:228`, `:254`), `conformance/tests/two_of_each_entity.rs` (`:163`) — cited (Change item 5; the parked diff).
  - `conformance/Cargo.lock`: gains the generated path package `agent` — cited (the parked diff). It is not the root `Cargo.lock`.
  - `crates/surf/Cargo.toml`: the generated `agent` crate as a path dependency, only if the build needs it — inferred (Change item 5: `agent_mut` would name `agent::Agent`). `story:runner-surf-cli-package` writes this file in the same wave.
- **Files read, not written:**
  - `docs/plan/agent-cli-draft-1.md`, the "Decisions the coordinating agent took" markers — cited.
  - Commit `b48bc87` on `spec/agent-session` and commit `9e43dcf` on `park/agent-sixth-port`, read with `git show`, no merge or cherry-pick — cited (Change items 1, 2 and 5; D32).
  - `docs/design/agent-interface.md` section 9 — cited.
  - `docs/plan/agent-plan-revision-2.md` D30 and D32 — cited.
  - `spec/domains/shell.yaml`, `spec/domains/navigation.yaml`, `spec/domains/net.yaml` — inferred. They define what `Session` references.
  - `Taskfile.yml` — cited. `REFUSALS_MAX` is on line 6.
  - `crates/surf/src/main.rs:48`: `MemoryBrowser::from_ports(ComponentPorts::new(), transport)`, whose signatures do not change — cited. `runner-surf-cli-package` moves this file.
  - `conformance/Cargo.toml` — cited. The parked diff does not change it: the adapter reaches the ports through `surf` and `surf-ports`.
- **Handed to the coordinator:**
  - Root `Taskfile.yml` line 6: `REFUSALS_MAX` becomes the `<n>` the unit reports — cited (D30; Change item 7).
  - Root `Cargo.lock`: gains the generated package `agent` — cited (D7; the parked diff).
  - `AGENTS.md`, "Components", "Layout", "Implementation targets" and "The gate's numbers" — cited, the lines of the story's "For AGENTS.md".
  - `README.md:14` ("all five components in one process") — cited (story, For AGENTS.md).
- **Symbols:**
  - `surf.agent.Session`, `OpenSession`, `Go`, `CloseSession`, `NoteNavigationStarted`, `NoteNavigationEnded` (spec, ess), the views and errors of the draft, and the component `agent` — cited.
  - `surf.agent.SessionReadout.state` typed `Session.State`; `surf.agent.OpenCall` unchanged — cited.
  - The generated crate `agent` and `surf_system::System::new` over six ports, `agent` first — cited (the parked diff), inferred for the names of the new command and view types.
  - `surf_ports::AgentPorts`, `AgentPort`, `agent_port`, `ComponentPorts.agent` — cited (D32; the parked diff); the obligation impls on `AgentPorts` — inferred as to which traits.
  - `surf::Browser<A, D, N, Ne, R, S, T>` and `surf::MemoryBrowser` — cited (the parked diff); `Browser::agent_mut` and a bound trait for `A` — inferred.
  - `surf_conformance::MemoryTarget`, `ConformanceBrowser`, `SurfTarget` over six ports, and the agent commands in `SurfTarget` — cited.
- **Would collide with:**
  - `story:agent-cli-spec` writes `spec/domains/agent.yaml`, `spec/components.yaml`, `spec/ess-inputs.yaml` and `generated/rust/` — cited. This story depends on it, so the order is fixed.
  - `story:runner-surf-cli-package`, same wave: writes `crates/surf/Cargo.toml`, removes `crates/surf/src/main.rs` and moves five binary tests out of `crates/surf/tests/` (not `library.rs` or `review1_library.rs`) — cited (its typed scope). The one shared file is `crates/surf/Cargo.toml`, and only if this story needs the `agent` dependency there.
  - `story:agent-cli-client`, same wave: writes `crates/agent/` (package `surf-agent`), a different path from `generated/rust/crates/agent` (package `agent`) — cited (its typed scope).
  - `story:agent-page-path-spec` and `story:agent-redirect-spec` write `spec/components.yaml`, `spec/domains/agent.yaml`, `generated/rust/`, `crates/ports` and `conformance/src/` — inferred. Their edges put them after this story.
  - `story:html-document-spec`, `agent-links-spec`, `agent-forms-spec`, `agent-request-body-spec`, `agent-history-spec` write the same spec and generated surfaces — inferred. Each sits behind this story through a chain.
  - `story:runner-record-option` writes `spec/domains/agent.yaml` and `generated/rust/` — inferred. It comes later by its edges.
  - `story:random-source-failure-cause` (draft) edits `crates/ports/src/random.rs` — cited. This story does not.
- **Not yet existing when it starts:**
  - From `story:agent-cli-spec`: the domain `surf.agent` with its types, owned by `net`, `cli/surf-agent.yaml` and `taskfiles/cli.yml`.
  - From `b48bc87` and `9e43dcf`, not on `main`: the session model to revise and the assembly to apply.
- **Confidence:** medium. The spec files, the assembly files and the test call sites are cited from the story, the parked diff and the tree. Whether `crates/surf/Cargo.toml` is needed, the agent bounds of `Browser`, and the obligation files depend on what `ess generate` writes for the component with commands, which cannot be read until it runs.
