---
format: aep.planning-md/3
id: story:agent-back-forward
kind: story
status: active
title: back and forward move the session through its history
summary: The session core and surf-agent go back and forward, refusing at either end and for a POST entry before anything is issued; answer format, command-line document and parser gain both.
relations:
- decomposes: epic:agent-cli-target
- depends_on: story:agent-history-spec
- depends_on: story:agent-press-submit
- depends_on: story:agent-session-events
scope:
- confidence: inferred
  path: AGENTS.md
- confidence: inferred
  path: README.md
- confidence: cited
  path: cli/surf-agent.yaml
- confidence: cited
  path: crates/agent/src/cli.rs
- confidence: cited
  path: crates/agent/src/wire.rs
- confidence: inferred
  path: crates/agent/tests/drift.rs
- confidence: cited
  path: crates/answer/src/lib.rs
- confidence: inferred
  path: crates/answer/tests
- confidence: cited
  path: crates/session/src/entries.rs
- confidence: cited
  path: crates/session/src/lib.rs
- confidence: cited
  path: crates/session/tests
- confidence: cited
  path: crates/surfd/src/daemon.rs
- confidence: cited
  path: docs/agent-answer.md
- confidence: cited
  path: docs/view-stream.md
- confidence: inferred
  path: generated/rust
- confidence: inferred
  path: spec/domains/agent.yaml
revision: 15
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:08Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:08Z", actor: "human:timo", revision: 5}
---
## Context

Step 3 of design section 9 lists `back` and `forward`; design section 4 maps them to `GoBack` and
`GoForward`. Story `agent-history-spec` specifies them and the session history of a tab in the
navigator (decision in `decision-blocker:agent-session-history-home`, 2026-11-21, by the
coordinating agent; the operator can overrule it). Story `agent-follow-links` gives the session
pages to go back between; story `agent-press-submit` gives it `POST` results, whose entries are
refused.

The scenario runner executes `Back` and `Forward` through the in-process session core
(`epic:e2e-scenario-runner`, story `runner-run`), so both are commands of the core, not only of
the binary.

The `agent` component reads no navigator rows, so it cannot refuse a traversal the navigator
would refuse. The session process, which is the assembly and reads every view as `crates/surf`
does, reads the history of its tab before it issues `GoBack` or `GoForward` and refuses there,
so the agent gets the reason and status 1 instead of a navigation that never starts.

## Change

- `crates/session`: `back` and `forward` in the core and the binary. Before issuing, the history
  view of the tab of the session is read: no earlier entry is refused `did refused at-start`, no
  later entry `did refused at-end`, an entry made by `POST` `did refused post-entry`, all status
  1 with the current state. Otherwise `GoBack` or `GoForward`, then the answer of the page the
  traversal committed at, or `page failed` when it failed. Handles start at 1 again, as for any
  new document. `back` and `forward` go to an entry of the session's own history, so the three
  refusals of a target (`address-too-long`, `unsupported-scheme`, `forbidden-destination`,
  `story:agent-follow-links`, "As decided after the adversary's pass") do not apply to them: an
  entry whose address was refused never entered the history, since an entry is made only by a
  committed navigation of a push (`.engineering/planning/story/agent-history-spec.md:82-83`,
  `:97-98`)
  and a refused target commits none: a target refused by the session's checks starts no
  navigation, and one the fetch refuses at its first request (`forbidden-destination` by the
  address a host resolves to, `story:agent-press-submit`, "As decided after the adversary's
  pass", decision 1) ends a navigation that fails before any connection and commits nothing.
- `crates/answer`, `docs/agent-answer.md`: `did back`, `did forward`, the refusal reasons, the
  `path` lines `back` and `forward`, each with its `parse` beside its rendering (decision D20 of
  `docs/plan/agent-plan-revision-2.md`).
- `crates/session`: the event entries of the `GoBack` and `GoForward` events of
  `agent-history-spec`, with their fields, in the mapping function `entries::event_line`
  (`crates/session/src/entries.rs:141`, whose last arm, `:287`, writes `unmapped`) and in the
  mapping list `covered()` of `crates/session/tests/events.rs:858` (decision D27). Each new
  event name goes into the "Event names" block of `docs/view-stream.md` (`:219-239`), which
  `task view-stream-check` reads (`docs/view-stream.md:215-217`: "A story that maps a new event
  adds its name here"); no other part of that document is written.
- `crates/agent/src/wire.rs`: its module documentation lists the commands of a request
  (`command <name>`, `:24`, "`open`, `go`, `look`, `close`"); `back` and `forward` join that list.
  They take no `arg`.
- The `did` and `path` lines of `back` and `forward` take no argument, as `docs/agent-answer.md`
  and `crates/answer` write a command without one: `did back`, `did forward`, `<n> back`,
  `<n> forward`. The reasons `at-start`, `at-end` and `post-entry` go into the reason table with
  exit status 1, into `RefusalReason` and its `RefusalReason::ALL` (`crates/answer/src/lib.rs:150`,
  an array of fixed length, 6 on `main`, which each reason-adding story before this one grows),
  into the exhaustive `match` of `crates/surfd/src/daemon.rs:815-821`, and into a test.
- `cli/surf-agent.yaml`, `crates/agent`: `back` and `forward`, written together so the drift test
  holds.
- `crates/session/tests/`, the counted cases, against a server the test starts:
  1. `go` A, `follow` to B, `back`: the page line of A; 2. then `forward`: the page line of B;
  3. `back` at the first page: status 1, `at-start`; 4. `forward` at the last: status 1, `at-end`;
  5. `go` C after `back` drops B: `forward` refused `at-end`;
  6. after a form `POST` answered 200, `back` and then `forward` to the `POST` entry: status 1,
     `post-entry`;
  7. the server down when the agent goes back: `page failed`; with the server up again, `back`
     lands on A (a failed traversal leaves the current entry where it was,
     `story:agent-history-spec`);
  8. handles start at 1 on the page `back` lands on;
  9. the calls of cases 1 and 2 through the core with a transport the test supplies give the same
     answers;
  10. `path` lists `back` and `forward` as actions;
  11. round trip: `surf_answer::parse` of the answers of cases 1, 3 and 10 gives the typed answer,
      and rendering that gives the same lines byte for byte;
  12. `history_events_map_with_their_fields`: after cases 1 and 2, with `surfd` run in the test
      process (decision D29), the session log holds the `GoBack` and the `GoForward` event entries
      with every field of the specification and no `unmapped` mark, each between its action entry
      and its answer entry.

  The cases stand on commands built before this story in the commands unit: cases 1, 2, 5, 7, 8,
  9, 10, 11 and 12 reach a second page with `follow` (`story:agent-follow-links`); case 6 makes
  its `POST` entry with `press` (`story:agent-press-submit`, and `fill`,
  `story:agent-fill-pick`, where its form has a field); cases 3 and 4 need neither.

Surfaces written: `crates/session/`, `crates/answer/`, `docs/agent-answer.md`,
`cli/surf-agent.yaml`, `crates/agent/` (`src/wire.rs` included), `crates/surfd/src/daemon.rs`,
`docs/view-stream.md` ("Event names" block only).

## For README.md (handed over in the unit report, decision D7)

- "Driving surf from an agent": "`back` and `forward` move through the pages of the session. A page
  reached by a form `POST` cannot be gone back or forward to until surf has a cache."

## Acceptance

`cargo nextest run -p surf-session -p surf-answer -p surf-agent` passes with the 12 numbered
cases, each a test named after its case, case 1 answering the page line of A and case 2 that of
B, and the drift test holding with `back` and `forward` in both the document and the parser. No
test runs over 60 s, and the test step ends within 120 s.

## Out of scope

The history model (`agent-history-spec`); resubmitting a `POST`; a delta other than one step;
`changed` (`agent-change-and-log`).
