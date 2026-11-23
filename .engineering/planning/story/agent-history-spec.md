---
format: aep.planning-md/3
id: story:agent-history-spec
kind: story
status: active
title: Specify session history per tab in the navigator, and GoBack and GoForward
summary: surf.navigation holds the session history entries of each tab and its current entry; a push drops the entries after it; traversal by one step, refused at either end and for a POST entry; agent GoBack and GoForward map to it.
relations:
- decomposes: epic:agent-cli-target
- depends_on: story:agent-form-port
- informed_by: decision-blocker:agent-session-history-home
- depends_on: story:agent-press-submit
scope:
- confidence: inferred
  path: AGENTS.md
- confidence: inferred
  path: Taskfile.yml
- confidence: cited
  path: conformance/src
- confidence: cited
  path: crates/ports
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
  path: spec/domains/navigation.yaml
- confidence: cited
  path: spec/ess-inputs.yaml
- confidence: cited
  path: spec/scenarios
revision: 13
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:09Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:09Z", actor: "human:timo", revision: 5}
---
## Context

`epic:agent-cli-target` lists `back` and `forward` among its commands and "goes back" in its
acceptance; design section 4 maps them to agent commands `GoBack` and `GoForward` and to the core
operation "`Navigate` to a history entry". Decided on 2026-11-21 by the coordinating agent, the
operator told and able to overrule (`decision-blocker:agent-session-history-home`, section
"Decision"): session history lives in the navigator, per tab, as the HTML Standard gives a
navigable its session history; traversing to an entry made by `POST` is refused until a cache
exists.

Standard, from memory, to be confirmed by this story: HTML Standard, the session history section
(session history entries, a traversable navigable and its session history, its current session
history entry, the history handling behaviours "push" and "replace", traversing the history by a
delta, and that a push removes the entries after the current one). Where this story differs (no
cache, so a `POST` entry is refused rather than resubmitted; no `replace` until something needs
it), a comment beside the construct says so.

Today `spec/domains/navigation.yaml` holds a `Navigation` per load and nothing that remembers the
loads of a tab. Since story `agent-request-body-spec` a navigation carries its method, which a
`POST` entry is recognised by. Since commit `c50ff20` ("a response names its address") a
committed navigation carries the response URL (`spec/domains/navigation.yaml:138-139`, `:157`),
which is the address of its entry.

Place in the chain: after `agent-press-submit` (edge `depends_on`), the last story of the
commands unit before it. The stories before it write the same files: `agent-form-port` writes
`crates/surf` and `conformance/src/target.rs`, `story:values-and-labels-before-fill` and
`agent-fill-pick` (its `readonly` field) write `spec/`, `generated/rust/` and
`conformance/src/target.rs`. Story `agent-back-forward` and then `agent-change-and-log` stand on
it.

## Domain relations

- `surf.shell.Tab → surf.navigation` session history entry: one tab has many entries (zero before
  its first committed navigation), ordered, one of them current. The navigator holds them by
  `tab_id`, as `Navigation` holds its tab (`spec/domains/navigation.yaml`, relation `tab`,
  `references`, `one`); the shell does not own them. Declared by no `ess/1` document yet; decided
  in `decision-blocker:agent-session-history-home`. This story writes it as a typed entry. A
  closed tab keeps its entries, as it keeps its navigations today (the `UNMAPPED:` marker on
  `TabClosed` in `navigation.yaml` stays).
- Entry → `surf.navigation.Navigation`: each entry is made by one committed navigation of a
  push, a `references` relation with cardinality `one`; the navigation is not owned by the entry.
  Same source; this story writes it.
- `surf.navigation.Navigation → surf.net.Request`: one to one (`spec/domains/navigation.yaml`,
  identity `navigation_id: surf.net.RequestId`); a traversal is a new navigation with a new
  request.

## Change

- `spec/domains/navigation.yaml`:
  - the session history entry: its tab, its position, the address (the response URL of the
    navigation that made it), its method, and that navigation; and the current entry of each
    tab. Whether the current entry is a row per tab identified by its `TabId` or a flag on the
    entries is measured with `ess specify validate` and `ess generate synthesize` on a scratch
    copy under `.out/`, and the story says which and why;
  - `Navigate` and the navigation gain the history handling, `Push` by default; a committed
    `Push` removes the entries after the current one, adds one and makes it current;
  - a command that traverses the history of a tab by a delta of -1 or +1: refused with no entry
    at that delta (at the start, at the end), refused for an entry made by `POST`, refused for a
    tab with no history; accepted, it starts a navigation to the entry address with `Get` and the
    traverse handling, whose commit makes that entry current and adds none; a traversal that
    fails or is cancelled leaves the current entry where it was;
  - after the adversary's pass on `1539aa2` (decisions D267 and D268):
    - an entry made by `POST` is one whose committing request was a `POST`. `CommitNavigation`
      carries the method of the request that the committed response answered, and the entry
      keeps that method, not the method its `Navigate` was sent with. A `POST` answered by a
      301, 302 or 303, which `surf-fetch` continues with `Get`, makes an entry of `Get`, and a
      traversal reaches it. The HTML Standard, from memory, drops the entry's POST resource
      when a redirect changes the method; a comment beside the construct says where this is
      confirmed or differs;
    - a traversal whose entry is `dropped` when its navigation would commit (a push committed
      first) commits nothing: no entry becomes current, no standing moves, and the tab keeps
      the entries the push left, one of them `back`. The comment that marks this order
      `UNMAPPED` is replaced by the rule. Nothing reaches the order today (the session runs one
      command at a time); the rule keeps a later caller from a tab that cannot step back.
      As built (commit `4c867db`, decision D275): a push that commits cancels every traversal of
      its tab that is still started, also one whose entry the push kept; the response of a
      cancelled traversal answers `wrong-state` and commits nothing. `ess` refuses the guard
      on the dropped entry alone (ESS-COMMAND-009, a row-set guard beside the `when_subject`
      guard of the traversal), and the row-set guard alone lost 11 synthesized scenarios and
      added 5 refusals in a probe. A traversal that commits after another traversal stays
      `UNMAPPED`, with a comment that says so.
    - As built for D267: `DeliverResponse` and `ResponseReceived` of `spec/domains/net.yaml`
      carry `method`, the method of the request the response answers, and the binding
      `commit-on-response` passes it to `CommitNavigation`. The entry line of
      `ResponseReceived` in the session log gains `method`; the examples of
      `docs/view-stream.md` and the test of `crates/view` that pins them are brought to it on
      the wave branch, where the view unit's changes of that test are merged.
- `spec/domains/agent.yaml`: `GoBack(session_id)` and `GoForward(session_id)` with the refusals of
  `Go`; their events invoke the traversal for the tab of the session; the enum
  `surf.agent.ActionCommand` (`spec/domains/agent.yaml:31`) gains the variants `back` and
  `forward`, lower case as its other variants.
- Every exhaustive `match` on `ActionCommand` at its base gains `back` and `forward`, which the
  regenerated enum forces, written in the path form `agent-back-forward` gives (`<n> back`,
  `<n> forward`, no argument). On `main` they are `command_word`
  (`crates/session/src/lib.rs:968`) and `action_command` and `action_command_named` of
  `conformance/src/target.rs` (`:512`, `:524`); the stories before it may add more.
- `spec/components.yaml`: the `navigator` component accepts and publishes the new commands and
  events; bindings from the agent events to the traversal; whatever binding the entry bookkeeping
  needs inside the navigator.
- Authored scenarios, each named after its case: `navigation-push-after-back-drops-forward.yaml`,
  `navigation-traverse-refused-at-start.yaml`, `navigation-traverse-refused-at-end.yaml`,
  `navigation-traverse-refused-for-post-entry.yaml`, `navigation-traverse-commits-without-entry.yaml`,
  `navigation-traverse-failed-keeps-current.yaml`, `agent-go-back-traverses.yaml`,
  `agent-go-forward-traverses.yaml`.
- The adversary's cases (branch `adv/history-spec`, commit `9ccf798`): five scenarios
  `adversary-history-*.yaml` and the module `adversary_history` of the test binary of
  `crates/ports`. `adversary-history-post-redirected-to-get` and
  `adversary-history-push-during-traversal` are red at `9ccf798` and green by the two rules
  above; where a rule turns a step of a scenario into a refusal, the scenario's step is changed
  to expect it and its closing assertions stay. The other three hold the standings `earlier`
  and `later`, which no scenario asserted (two mutants of those effects passed all 191
  scenarios), and two tabs.
- `task generate`; `crates/ports` stores the entries; `conformance/src/`; `crates/surf/src/lib.rs`
  only where the regenerated interface forces it, keeping every signature `crates/session` and
  `crates/surf-cli` call (`Browser::start` pushes by default). `REFUSALS_MAX` is not written here (decision D30): the unit runs its gate as `slot task check REFUSALS_MAX=<n>`, `<n>` the measured `refused:` count, and its report gives `<n>` and each new refusal with its cause; the coordinator writes the number on the wave branch.

Surfaces written: `spec/domains/navigation.yaml`, `spec/domains/agent.yaml`,
`spec/components.yaml`, `spec/scenarios/`, `spec/ess-inputs.yaml`, `generated/rust/`,
`crates/ports/`, `crates/surf/src/lib.rs` (only if forced), `conformance/src/`,
`crates/session/src/lib.rs` (its exhaustive `match` on `ActionCommand` only).

Baseline the gate's numbers are measured against: the counts at its base, which include the
scenarios `agent-fill-pick`'s `readonly` field adds (on `main` after round 1: 169 scenarios, 24
authored, 48 refusals, `AGENTS.md`, "The gate's numbers").

## For AGENTS.md (handed over in the unit report, decision D7)

- "Components": the `navigator` row gains the session history entries and the traversal; the
  `agent` row gains `GoBack` and `GoForward`.
- "The gate's numbers": the new totals and each new refusal with its cause.

## Acceptance

`slot task check REFUSALS_MAX=<n>` exits 0 with the scenario total at least 12 above the total recorded before it
and equal on both runners with `0 failed, 0 error, 0 unsupported`, the eight authored files of
Change among the passed, and the `refused:` count at most 8 above the count recorded before it,
`<n>` equal to it, the unit report giving `<n>` and each new refusal with its cause (decision
D30). No test runs over 60 s, and the test step ends within 120 s.

## Out of scope

`back` and `forward` on the command line (`agent-back-forward`); a cache and resubmitting a
`POST`; `replace` navigations; fragment navigations; `history.pushState` and anything else a
script does; a delta other than -1 and +1.
