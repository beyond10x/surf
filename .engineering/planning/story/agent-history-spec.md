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
revision: 5
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
`POST` entry is recognised by. Since `agent-links-spec` a committed navigation carries the
response URL, which is the address of its entry.

Place in the chain of specification stories: after `agent-form-port`, the last story that writes
`crates/surf` and `conformance/src/target.rs` before it, so forms are not held back; story
`agent-back-forward` and then `agent-change-and-log` stand on it.

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
    fails or is cancelled leaves the current entry where it was.
- `spec/domains/agent.yaml`: `GoBack(session_id)` and `GoForward(session_id)` with the refusals of
  `Go`; their events invoke the traversal for the tab of the session; `Action` gains `Back` and
  `Forward`.
- `spec/components.yaml`: the `navigator` component accepts and publishes the new commands and
  events; bindings from the agent events to the traversal; whatever binding the entry bookkeeping
  needs inside the navigator.
- Authored scenarios, each named after its case: `navigation-push-after-back-drops-forward.yaml`,
  `navigation-traverse-refused-at-start.yaml`, `navigation-traverse-refused-at-end.yaml`,
  `navigation-traverse-refused-for-post-entry.yaml`, `navigation-traverse-commits-without-entry.yaml`,
  `navigation-traverse-failed-keeps-current.yaml`, `agent-go-back-traverses.yaml`,
  `agent-go-forward-traverses.yaml`.
- `task generate`; `crates/ports` stores the entries; `conformance/src/`; `crates/surf/src/lib.rs`
  only where the regenerated interface forces it, keeping every signature `crates/session` and
  `crates/surf-cli` call (`Browser::start` pushes by default). `REFUSALS_MAX` is not written here (decision D30): the unit runs its gate as `slot task check REFUSALS_MAX=<n>`, `<n>` the measured `refused:` count, and its report gives `<n>` and each new refusal with its cause; the coordinator writes the number on the wave branch.

Surfaces written: `spec/domains/navigation.yaml`, `spec/domains/agent.yaml`,
`spec/components.yaml`, `spec/scenarios/`, `spec/ess-inputs.yaml`, `generated/rust/`,
`crates/ports/`, `crates/surf/src/lib.rs` (only if forced), `conformance/src/`.

## For AGENTS.md (handed over in the unit report, decision D7)

- "Components": the `navigator` row gains the session history entries and the traversal; the
  `agent` row gains `GoBack` and `GoForward`.
- "The gate's numbers": the new totals and each new refusal with its cause.

## Acceptance

`slot task check REFUSALS_MAX=<n>` exits 0 with the scenario total at least 12 above the total recorded before it
and equal on both runners with `0 failed, 0 error, 0 unsupported`, the eight authored files of
Change among the passed, and the `refused:` count at most 8 above the count recorded before it,
`<n>` equal to it, the unit report giving `<n>` and each new refusal with its cause (decision
D30).

## Out of scope

`back` and `forward` on the command line (`agent-back-forward`); a cache and resubmitting a
`POST`; `replace` navigations; fragment navigations; `history.pushState` and anything else a
script does; a delta other than -1 and +1.
