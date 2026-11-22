---
format: aep.planning-md/3
id: story:agent-links-spec
kind: story
status: active
title: Specify following a link, and redirects inside one request
summary: surf.agent gains Follow, mapped to Navigate; surf.net delivers the response URL after redirects and the navigation commits at it.
relations:
- decomposes: epic:agent-cli-target
- depends_on: story:agent-page-path-spec
- depends_on: story:agent-session-process
- depends_on: story:html-document-port
- depends_on: story:agent-fetch-post
scope:
- confidence: cited
  path: conformance/src
- confidence: cited
  path: crates/session/src/lib.rs
- confidence: cited
  path: crates/surf/src/lib.rs
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
revision: 12
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:09Z", actor: "human:timo", revision: 5}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:09Z", actor: "human:timo", revision: 6}
---
## Context

Step 3 of design section 9; design section 4 maps `follow <handle>` to the agent command
`Follow`, and that to `surf.navigation.Navigate` to the link target. The redirect model that this
story held moved to `agent-redirect-spec` (decision D4); this story keeps `Follow`.

Handles are not modelled. Design section 5.5 makes a handle a name inside one session, assigned
in document order by the session; the session process holds the map from handle to element and
resolves the target with `surf-url` (`agent-follow-links`). So `Follow` takes the resolved
address, the handle and the link label for its `path` line, and reads nothing of the node tree.
Decided here, reversible: modelling handles would put a presentation choice into the model. The
story comes after `html-document-port` because both write `crates/surf/src/lib.rs` and
`conformance/src/target.rs`, not because it reads the tree.

## Domain relations

- `surf.agent.Session → surf.agent.Action`: as `agent-page-path-spec` writes it; `Follow` adds a
  command to the enumeration.
- `surf.navigation.Navigation → surf.net.Request`: one to one (`spec/domains/navigation.yaml`,
  identity `navigation_id: surf.net.RequestId`); a follow is a new navigation with `Get` and no
  body (`agent-request-body-spec`).

## Change

- `spec/domains/agent.yaml`: `Follow(session_id, url, handle, label)` with the refusals of `Go`
  (not the holder, no such session, closed) and an accepted outcome that emits
  `NavigationRequested` as `Go` does and records an `Action` with command `Follow`, the handle and
  the label.
- Authored scenarios: `spec/scenarios/agent-follow-navigates.yaml` (the navigation starts at the
  given address with `Get`) and `agent-follow-records-handle-and-label.yaml`.
- `task generate`; `conformance/src/`; `crates/surf/src/lib.rs` only where the regenerated
  interface forces it. `REFUSALS_MAX` is not written here (decision D30): the unit runs the tests of the packages it touched, `<n>` is the measured `refused:` count (`.out/synthesize.log` after `slot task suite REFUSALS_MAX=<n>`), and its report gives `<n>` and each new refusal with its cause; the coordinator writes the last `<n>` of the round on the wave branch and runs the gate once after the round's units, as `slot --timeout 25m task check-runner`.

Surfaces written: `spec/domains/agent.yaml`, `spec/components.yaml`, `spec/scenarios/`,
`spec/ess-inputs.yaml`, `generated/rust/`, `crates/surf/src/lib.rs` (only if forced),
`conformance/src/`, `crates/session/src/lib.rs` (`command_word` matches every `ActionCommand`).

Bounds for every command of this story. Every build and test runs through the build pool:
`cargo` is wrapped by the pool's shim, every other command that builds or tests gets the `slot`
prefix. The pool ends a job after 10 minutes; a command that needs longer names its time with
`slot --timeout <t>` or `SLOT_TIMEOUT=<t>`, at most 30 minutes; a `timeout` wrapped around the
command extends nothing. A script that measures or waits starts without the `slot` prefix; the
builds and test runs inside it carry it. No test runs over 60 s and the test step ends within
120 s. No Python.

## For AGENTS.md (handed over in the unit report, decision D7)

- "Components": the `agent` row gains `Follow`.
- "The gate's numbers": the new totals and each new refusal with its cause.

## Acceptance

On the unit's branch after this story, `slot task conform REFUSALS_MAX=<n>` prints a scenario
total at least 5 above the total recorded before it with `0 failed, 0 error, 0 unsupported`, and
`.out/report.json` (the report `task conform` writes with `--report-out`) lists the two authored
files and the synthesized `Follow/outcome/requested`, `Follow/outcome/not-holder`,
`Follow/outcome/no-such-session` and `Follow/outcome/closed` as passed;
`slot task conform-rust REFUSALS_MAX=<n>` and then `task conform-totals` show the same total on
both runners; and `.out/synthesize.log` holds `<n>` `refused:` lines, at most 4 above the count
recorded before it, the unit report giving `<n>` and each new refusal with its cause (decision
D30).

## Out of scope

The link lines and handles (`agent-follow-links`); redirects (`agent-redirect-spec`,
`agent-fetch-redirects`); session history, `back`, `forward` (`agent-history-spec`,
`agent-back-forward`).
