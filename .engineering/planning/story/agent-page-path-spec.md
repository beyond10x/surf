---
format: aep.planning-md/3
id: story:agent-page-path-spec
kind: story
status: implemented
title: Specify the action log and the fields of the page line
summary: Action rows owned by a session give path and log their lines; the session shows the status, media type and failure reason of its page.
relations:
- decomposes: epic:agent-cli-target
- depends_on: story:agent-session-spec
scope:
- confidence: cited
  path: conformance/src/target.rs
- confidence: cited
  path: conformance/tests/planted_defect.rs
- confidence: cited
  path: conformance/tests/review2_planted_defects.rs
- confidence: cited
  path: conformance/tests/two_of_each_entity.rs
- confidence: cited
  path: crates/ports
- confidence: cited
  path: crates/surf/src/lib.rs
- confidence: cited
  path: crates/surf/tests/library.rs
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
revision: 9
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:10Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:10Z", actor: "human:timo", revision: 5}
- {from: "active", to: "implemented", at: "2026-11-21T02:59:46Z", actor: "human:timo", revision: 9, decided_on: {"recorded":{"test_result":1,"verification":1}}}
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

`Action` is not in the specification yet (`spec/`, format `ess/37`); this story introduces it,
and `agent-links-spec`, `agent-forms-spec` and `agent-history-spec` add their commands to its
enumeration.

The design field "outcome" is not stored, decided here (scope critic, round 1, finding 1): a
refused command records no action, so every stored action was accepted; what came of it (the
navigation committed or failed) is the state of the navigation, which the session already shows
on the `page` line, and a second copy would have a second writer.

Who records an action, decided here and measured on a draft of the model (below): the session
process, with a command of the `agent` component, `RecordAction`, after the agent's command was
accepted and before it answers. It holds one session and is the only writer of its log, so it
supplies the sequence number, from 1 (`agent-session-process` draws it). The action's identity
is minted by the component (`{generated: true}`): ESS gives an entity one identity field, the
sequence number is unique only within a session, and no command addresses an action after it is
recorded, so no caller needs to name one in advance. Rejected: recording through bindings from
the agent's own events (`SessionOpened`, `NavigationRequested`, `SessionClosed`), the session
counting its actions in a stored field and each event carrying the next number. Measured on the
same draft with one note command (item "page line" below): 85 scenarios and 33 refusals against
81 and 25 recorded directly, the 8 more being `ESS-SYNTH-010` for the three bindings (`flow`,
`delivery`, `on-failure`; `record-on-session-opened/binding/flow` alone was synthesized), and
the interpreter failed `record-on-session-opened/binding/flow` and `/mapping`; the sequence
would also be a value expression (`{expr: "subject.last_sequence + 1"}`) on `Go` and
`CloseSession`, which ESS documents as an obligation in generated Rust (not generated here, since
that draft kept the one note command). The design does not
rule either out (section 6 says what the log holds, not who writes it).

What `open` records, from design section 4 (`open [<url>]` is `OpenSession`, then `Go`) and
`docs/agent-answer.md` (its example's `path` holds `1 open` and `2 go
"http://example.org/login"`): `open` records an action `open` with no argument, and
`open <url>` is two actions, `open` and then `go` with the address. `OpenSession` gains no
address input.

What an action stores of what the agent gave: the value as the specification types it, never
text the session process rendered. In this story that is the address of `go`, as
`surf.url.Url`, so no binding and no command converts a type. Design section 8 says of
recordings: "A recording can hold what the agent typed, passwords included. Proposed: values of
password controls are written as a reference to a variable (`${PASSWORD_1}`), supplied at run
time." It does not say that an action never stores a value the agent typed. Decision D22 of
`docs/plan/agent-plan-revision-2.md` governs what the kept log holds: a field of bytes as
`<n> bytes`, a password as its length. No field of `Action` holds bytes or a password in this
story; the story that adds `fill` stores a password's length, not its value.

A draft of the model this story describes, to compare with and not to copy without running it:
`$HOME/.cache/scratch/b84c1d1b-86a1-4880-9b86-d11fa8d6a813/waves/a3/page-path-draft/` (`spec/`,
`Taskfile.yml`, `taskfiles/`, `cli/`; `base/` is the branch as measured, `alternatives/` holds
the rejected drafts: `record-through-bindings`, `record-through-bindings-no-guard`,
`one-note-command`, `no-session-guard`). Its numbers are the ones in Acceptance.

## Domain relations

- `surf.agent.Session → surf.agent.Action`: one to many (zero before the first action); the
  session owns its actions; an action never outlives its session. Not in the specification yet;
  stated by the operator-accepted design, section 6
  (`approval-record:agent-design-accepted-2026-11-21`). Written here as a `relations:` entry on
  `Session` (`actions`, `owns`, `many`, `via: session_id` on `Action`).
- `surf.navigation.Navigation → surf.net.Request`: one to one through one identity
  (`spec/domains/navigation.yaml`, `Navigation` identity `navigation_id: surf.net.RequestId`;
  binding `fail-on-fetch-failed`, `navigation_id: event.request_id`): the path a failure reason
  takes to the navigation.

## Change

- `spec/domains/agent.yaml`:
  - types `surf.agent.ActionId` (a newtype of `Uuid`) and `surf.agent.ActionCommand` (an enum of
    `open`, `go`, `close`, spelled as the `path` and `did` lines print them);
  - entity `Action`: identity `action_id: surf.agent.ActionId`; fields `session_id`,
    `sequence: Integer`, `command: surf.agent.ActionCommand`, `url: Optional<surf.url.Url>` (the
    address of `go`, absent for `open` and `close`); one state `Recorded`, terminal. A later
    command with another kind of argument adds its own optional field;
  - command `RecordAction` (input `session_id`, `sequence`, `command`, `url`; no actor is
    granted it, like `surf.net.DeliverResponse`): `no-such-session`
    (`when_related: {via: input.session_id, exists: false}`, `SessionNotFound`), then
    `recorded`, which creates the `Action` with `action_id: {generated: true}` and emits
    `ActionRecorded` (`action_id`, `session_id`, `sequence`, `command`, `url`);
  - view `SessionLog(session_id)`: `action_id`, `session_id`, `sequence`, `command`, `url`,
    filtered by the session, `order_by: [sequence asc]`; granted to `SessionHolder`;
  - the session gains optional `status: surf.net.Status`, `media_type: surf.net.MediaType` and
    `failure_reason: surf.net.FailureReason`. `NoteNavigationStarted` clears all three. Two new
    commands set them, each selected as `NoteNavigationEnded` is (`instances:` by
    `navigation_id`): `NoteNavigationCommitted(navigation_id, status, media_type)` sets
    `navigation_state: Committed`, `status`, `media_type`; `NoteNavigationFailed(navigation_id,
    reason)` sets `navigation_state: Failed` and `failure_reason`. Both emit
    `SessionNavigationChanged`. `NoteNavigationEnded` stays, for the cancelled end. One command
    with `Optional` inputs was measured and refused: it validates, synthesizes and passes the
    interpreter, and `ess generate synthesize --target rust` refuses the workspace ("binding
    `note-on-navigation-committed` emits a plain clone of `surf.net.Status` for `status` of type
    `Optional<surf.net.Status>`; this assignment needs an explicit target representation", and
    the same for `media_type` and the reason);
  - `SessionState` and `Sessions` publish the three new fields, and the command-line shape
    `SessionReadout` mirrors `SessionState` with them (`task cli` still passes; the client's drift
    test reads input shapes only).
- `spec/domains/navigation.yaml`: `FailNavigation` and `NavigationFailed` gain
  `reason: surf.net.FailureReason`.
- `spec/components.yaml`: `fail-on-fetch-failed` maps `reason: event.reason`;
  `note-on-navigation-committed` invokes `NoteNavigationCommitted` with `status` and
  `media_type`; `note-on-navigation-failed` invokes `NoteNavigationFailed` with
  `reason: event.reason`; `note-on-navigation-cancelled` is unchanged. The `agent` component
  accepts `NoteNavigationCommitted`, `NoteNavigationFailed` and `RecordAction` and publishes
  `ActionRecorded`. Still ten bindings.
- Authored scenarios (`spec/scenarios/`, listed in `spec/ess-inputs.yaml`):
  - new `agent-session-shows-failure-reason.yaml`: a navigation in a session's tab commits, the
    next fails with `Unreachable`; `SessionState` shows `navigation_state: Failed`,
    `failure_reason: Unreachable` and no `status` or `media_type`;
  - new `agent-session-log-in-sequence-order.yaml`: `RecordAction` with sequence 2 (`go`, an
    address), then 1 (`open`, none); `SessionLog` holds two rows, `ranked: true`, the first
    `open` with `url: null`, the last `go` with the address;
  - `agent-session-reads-committed-navigation.yaml` also asserts `status: 200`,
    `media_type: text/html`, `failure_reason: null`; `agent-session-reads-started-navigation.yaml`
    asserts the three absent;
  - `two-of-each-entity.yaml` also opens two sessions over the two tabs and records two actions
    in the first, and reads `Sessions` and `SessionLog` back with exactly two rows and both
    identities, and the second session's log with none.
  An action recorded for `go` and none for a refused `go` are the session process's to show
  (`agent-session-process`): the model records what it is sent and cannot see the command it
  follows.
- `task generate`. Measured on the draft: `151 capabilities: 143 generated, 4 obligation(s), 4
  refused` (before: 138, 132, 2, 4) and `30 artifact(s)`. `RecordAction` and `SessionLog` are
  generated; `NoteNavigationCommitted` and `NoteNavigationFailed` are obligations
  (`instances:`), as the other two note commands are. The generated `TryContext` gains
  `try_generate_surf_agent_action_id`.
- `crates/ports`: `AgentPorts` stores `Action` and answers `try_generate_surf_agent_action_id`
  with a version 4 identity, as the other ports answer theirs; every other `TryContext` of
  `ports.rs` (the macro at `:80`) answers it with `unmet_context`. The hand-written obligations
  (`ports.rs:337-377`): `NoteNavigationStarted` also clears `status`, `media_type` and
  `failure_reason`; the new `NoteNavigationCommitted` sets `navigation_state` `Committed`,
  `status` and `media_type`; the new `NoteNavigationFailed` sets `navigation_state` `Failed` and
  `failure_reason`; `NoteNavigationEnded` is unchanged. `tests/agent.rs` builds `SessionData`
  with the three new fields.
- `crates/surf/src/lib.rs`: `AgentBehaviors` (`:275-294`), which lists every agent behaviour and
  view trait, gains `NoteNavigationCommittedBehavior`, `NoteNavigationFailedBehavior`,
  `RecordActionBehavior` and `SessionLogQuery`. `crates/surf/tests/library.rs:233` builds
  `SessionStateView` with the three new fields.
- `conformance/src/target.rs`: `FailNavigation` takes `reason` (`:998-1009`), the
  `NavigationFailed` event carries it (`:562`), the imports (`:44-57`, `FailNavigation` at
  `:53`); `RecordAction`, `NoteNavigationCommitted`, `NoteNavigationFailed` as commands and as
  binding invocations; `ActionRecorded`; the view `SessionLog`; a `Session` setup with the three
  new fields (`:386-437`).
- `conformance/tests/two_of_each_entity.rs`: `Session` and `Action` join its entities. A view
  cut to its first row is planted for both; one identity for every instance for `Action`, whose
  identity the context mints; `Session`, whose identity the caller chooses, has no identity
  defect of its own, as `Request` has none. `conformance/tests/planted_defect.rs` (`:42`) and
  `conformance/tests/review2_planted_defects.rs` (`:141-172`) answer or forward
  `try_generate_surf_agent_action_id`.
- `REFUSALS_MAX` is not written here (decision D30): the unit runs its gate as
  `slot task check REFUSALS_MAX=27`, and the coordinator writes the number on the wave branch.

Surfaces written: `spec/domains/agent.yaml`, `spec/domains/navigation.yaml`,
`spec/components.yaml`, `spec/scenarios/`, `spec/ess-inputs.yaml`, `generated/rust/`,
`crates/ports/`, `crates/surf/src/lib.rs`, `crates/surf/tests/library.rs`,
`conformance/src/target.rs`, `conformance/tests/two_of_each_entity.rs`,
`conformance/tests/planted_defect.rs`, `conformance/tests/review2_planted_defects.rs`.

## For AGENTS.md (handed over in the unit report, decision D7)

- "Components", the `agent` row: holds `Session`, `Action`; accepts `OpenSession`, `Go`,
  `CloseSession`, `NoteNavigationStarted`, `NoteNavigationCommitted`, `NoteNavigationFailed`,
  `NoteNavigationEnded`, `RecordAction`. The paragraph on the agent's session:
  `NavigationCommitted` invokes `NoteNavigationCommitted` with its status and media type,
  `NavigationFailed` invokes `NoteNavigationFailed` with its reason, `NavigationCancelled`
  invokes `NoteNavigationEnded`; the session process records each accepted action with
  `RecordAction`; `SessionLog` lists a session's actions in sequence order. The page-load
  paragraph: `FetchFailed` invokes `FailNavigation` with its reason.
- "Layout": the `spec/domains/agent.yaml` line names `Action`, the new commands and `SessionLog`;
  the `spec/scenarios/` line names the two new files and says that `two-of-each-entity.yaml`
  covers sessions and actions; the `tests/two_of_each_entity.rs` line holds for `Session` and
  `Action` too.
- "The gate's numbers": the totals of Acceptance and the three new refusals with their causes,
  under the two `ESS-SYNTH-001` of the note commands.

## Acceptance

`slot task check REFUSALS_MAX=27` exits 0 and prints `surf v1 — 9 file(s), 7 scenario(s),
valid` and `81 scenario(s) (7 authored), 27 refusal(s)` (before: 5 scenario files, `77
scenario(s) (5 authored), 24 refusal(s)`); against the interpreter (`task conform`, `ess verify
conform run --path spec --suite .out/suite.json --target interpreted --report-format 2`) and
against the Rust system each `81 scenarios: 81 passed, 0 failed, 0 error, 0 unsupported`, the
two new authored files among the passed; `conform-totals` reports 81 for both;
`generated-check` reports `30 artifact(s)`; the two per-entity tests of
`conformance/tests/two_of_each_entity.rs` pass with `Session` and `Action` among their entities.
The draft measured 81, 7, 27 and 81 passed on the interpreter with ess 0.96.0 on 2026-11-21; a
different count is a finding to report, not a number to reach by adding scenarios. The unit
report lists `grep '^refused:' .out/synthesize.log` and each new refusal with its cause. The new
refusals, all three:

1. `ESS-SYNTH-014`, `surf.agent.RecordAction/outcome/recorded`: "`surf.agent.SessionLog`
   declares an order and this scenario can put 1 row(s) in it: comparing two rows needs two
   `surf.agent.Action`s". The `no-such-session` guard leaves synthesis one action per scenario;
   `agent-session-log-in-sequence-order.yaml` covers the order. (The suite still holds a
   scenario of that id, which passes; without the guard the draft gives 80 scenarios and 26
   refusals and no such refusal, at the cost of actions for no session.)
2. `ESS-SYNTH-001`, `surf.agent.NoteNavigationCommitted/outcome/noted`: "no three rows the filter
   selects that the arranging commands can produce", as for the other note commands;
   `agent-session-reads-committed-navigation.yaml` covers it.
3. `ESS-SYNTH-001`, `surf.agent.NoteNavigationFailed/outcome/noted`: the same cause;
   `agent-session-shows-failure-reason.yaml` covers it.

## Out of scope

Rendering the lines (`agent-answer-format`, `agent-session-process`); issuing `RecordAction`
and the `did` and `path` lines of an `open <url>` (`agent-session-process`); the title
(`agent-follow-links`); the `log` command (`agent-change-and-log`).

## Revised 2026-11-21, before wave 3

- Acceptance: absolute numbers measured on a draft of the model (81 scenarios, 7 authored, 27
  refusals; base 77, 5, 24) and the three new refusals by id, replacing "at least 9 above" and
  "at most 8 above" a base in `AGENTS.md`, because a floor carried from a draft was wrong in
  wave 2.
- `open` records `open` with no argument, and `open <url>` is two actions, because `OpenSession`
  has no address and design section 4 maps `open <url>` to `OpenSession` then `Go`.
- Recording by a command the session process issues, with the sequence number as its input and
  an identity the component mints, because recording through bindings measured 8 refusals more
  and two interpreter failures, and no agent event carries a number.
- The argument is `url: Optional<surf.url.Url>`, because `Optional<String>` would need a
  conversion a binding cannot make and `conversions:` is ruled out.
- Two note commands (`NoteNavigationCommitted`, `NoteNavigationFailed`) instead of setting the
  page fields through `NoteNavigationEnded`, because ess 0.96.0 refuses Rust for a binding that
  maps a required event field into an `Optional` input; Change names what `crates/ports` sets.
- Change names the hand-written places in `conformance/src/target.rs` that `reason` reaches.
- `crates/surf/src/lib.rs` is written (its `AgentBehaviors` lists every agent behaviour), not
  "only if forced"; `crates/surf/tests/library.rs:233` too, which no other wave 3 story writes.
- Quotations corrected: `docs/agent-answer.md` and `crates/answer` hold `1 open`, `2 go
  "http://example.org/login"` and `4 close` (`tests/golden/path.txt`) and `did go
  "http://example.org/"` (`did.txt`); "`3 close`" and "`did close`" are in neither.
- "In no `ess/1` document" now reads "not in the specification yet (format `ess/37`)".
- The `AGENTS.md` lines are phrased against its six components and existing `agent` row.
- `two-of-each-entity.yaml` and `conformance/tests/two_of_each_entity.rs` cover `Session` and
  `Action` here, because this story adds the second agent view.
- "An action never stores a value the agent typed (design section 8)" is removed: section 8 does
  not say it. The rule is stated from the design's own sentence and D22.
