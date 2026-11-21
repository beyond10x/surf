---
format: aep.planning-md/3
id: story:agent-page-path-spec
kind: story
status: active
title: Specify the action log and the fields of the page line
summary: Action rows owned by a session give path and log their lines; the session shows the status, media type and failure reason of its page.
relations:
- decomposes: epic:agent-cli-target
- depends_on: story:agent-session-spec
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:10Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:10Z", actor: "human:timo", revision: 5}
---
## Context

Step 1 of design section 9 puts the `page` and `path` blocks in the first slice. Design section
5.1: `page <status> <media type> <address> <title>`, `page none`, `page failed <reason>
<address>`; `path` lists the last actions, oldest first, with the total. Design section 6 puts
the log in the specification: `Action` rows owned by a session (sequence number, command,
input, outcome), and the views `SessionState` and `SessionLog`.

After `agent-session-spec` a `Session` holds its navigation, address and navigation state.
`surf.navigation.NavigationCommitted` carries `status` and `media_type`;
`surf.navigation.NavigationFailed` carries no reason, which only `surf.net.FetchFailed` does.

`Action` exists in no `ess/1` document; this story introduces it, and `agent-links-spec`,
`agent-forms-spec` and `agent-history-spec` add their commands to its enumeration.

The design field "outcome" is not stored, decided here (scope critic, round 1, finding 1): a
refused command records no action, so every stored action was accepted; what came of it (the
navigation committed or failed) is the state of the navigation, which the session already shows
on the `page` line, and a second copy would have a second writer.

## Domain relations

- `surf.agent.Session → surf.agent.Action`: one to many (zero before the first action); the
  session owns its actions; an action never outlives its session. In no `ess/1` document yet;
  stated by the operator-accepted design, section 6
  (`approval-record:agent-design-accepted-2026-11-21`). Written here as a `relations:` entry on
  `Session` (`owns`, `many`).
- `surf.navigation.Navigation → surf.net.Request`: one to one through one identity
  (`spec/domains/navigation.yaml`, `Navigation` identity `navigation_id: surf.net.RequestId`;
  binding `fail-on-fetch-failed`, `navigation_id: event.request_id`): the path a failure reason
  takes to the navigation.

## Change

- `spec/domains/agent.yaml`: entity `Action`, its identity chosen by the caller (the session
  process draws the next sequence number: deterministic, nothing random), with the session, the
  sequence number, the command (`Open`, `Go`, `Close` for now) and an optional argument
  (`Optional<String>`): the address for `go` and for `open` with an address, none for `open`
  without one and for `close`. The `path` line writes `<n> <command>`, and the argument as a
  quoted field only when there is one, as `docs/agent-answer.md` and `crates/answer` do (`2 go
  "http://example.org/"`, `3 close`); the `did` line the same (`did go "<address>"`, `did
  close`).
  An action never stores a value the agent typed (design section 8). Only an accepted
  command is recorded. If one outcome cannot both act and create the action, a binding from the
  agent event to a recording command does. View `SessionLog(session_id)`: the actions of a
  session in sequence order.
- The session gains optional `status`, `media_type` and `failure_reason`, set by the note
  commands; `surf.navigation.FailNavigation` and `NavigationFailed` gain
  `reason: surf.net.FailureReason`, mapped from `FetchFailed` in `fail-on-fetch-failed`;
  `SessionState` publishes the new fields.
- Authored scenarios: `spec/scenarios/agent-action-recorded-for-go.yaml`,
  `agent-action-none-for-refused-go.yaml`, `agent-session-shows-committed-status.yaml`,
  `agent-session-shows-failure-reason.yaml`, `agent-action-without-argument-for-close.yaml` (an
  `open` without an address and a `close` record actions with no argument).
- `task generate`; `crates/ports` stores `Action`; `conformance/src/`; `crates/surf/src/lib.rs`
  only where the regenerated interface forces it. `REFUSALS_MAX` is not written here (decision D30): the unit runs its gate as `slot task check REFUSALS_MAX=<n>`, `<n>` the measured `refused:` count, and its report gives `<n>` and each new refusal with its cause; the coordinator writes the number on the wave branch.

Surfaces written: `spec/domains/agent.yaml`, `spec/domains/navigation.yaml`,
`spec/components.yaml`, `spec/scenarios/`, `spec/ess-inputs.yaml`, `generated/rust/`,
`crates/ports/`, `crates/surf/src/lib.rs` (only if forced), `conformance/src/`.

## For AGENTS.md (handed over in the unit report, decision D7)

- "Components": the `agent` row gains `Action`; the sentence on `FetchFailed` says the reason
  reaches the navigation.
- "The gate's numbers": the new totals and each new refusal with its cause.

## Acceptance

`slot task check REFUSALS_MAX=<n>` exits 0 with the scenario total at least 9 above the total
`agent-session-spec` recorded in `AGENTS.md` and equal on both runners with `0 failed, 0 error,
0 unsupported`, the five authored files of Change among the passed, and the `refused:` count at
most 8 above the count `agent-session-spec` recorded, `<n>` equal to it, the unit report giving `<n>` and each new refusal with its cause (decision
D30).

## Out of scope

Rendering the lines (`agent-answer-format`, `agent-session-process`); the title
(`agent-follow-links`); the `log` command (`agent-change-and-log`).
