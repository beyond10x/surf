---
format: aep.planning-md/3
id: story:runner-decisions
kind: story
status: implemented
title: Decide the scenario format, locator ties and unrecorded requests
summary: A design note answers the epic's three open questions and the decisions the later stories rest on.
relations:
- decomposes: epic:e2e-scenario-runner
scope:
- confidence: cited
  path: docs/design/scenario-runner.md
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:13Z", actor: "human:timo", revision: 5}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:13Z", actor: "human:timo", revision: 6}
- {from: "active", to: "implemented", at: "2026-11-21T01:25:58Z", actor: "human:timo", revision: 7, decided_on: {"recorded":{"test_result":1,"verification":1}}}
---
## Context

`epic:e2e-scenario-runner` leaves three questions to its first story: the scenario file format
and whether it can be `ess-scenario/6` run by the ESS runner (design
`docs/design/agent-interface.md`, section 10, question 2), how a locator is resolved when two
elements share kind and name, and what a replayed run does with a request the recording does
not hold. Every later story of the epic builds on the answers, so this story is a decision and
its outcome is a design note, written before any format or code.

What was read and measured for the format question (2026-11-21, ess 0.96.0; the ESS
documentation checkout at `$HOME/beyond10x/ess/website/docs` is at `0.92.0-5-g4ab2ea1449`):

- `ess-scenario/6` (`reference/formats.md`, `guides/verify/author-scenarios.md`) drives commands
  with literal inputs (`{$instance}`, `{$fixture}` references aside) and claims an act's
  `response:`, `events:`, `error:` and `capture:`. View claims (`contains`, `excludes`, `at`, ...)
  stand only in the scenario-level `assert:` after the whole timeline.
- Measured with `ess verify conform author --path spec --scenarios <file>` on copies of
  `spec/scenarios/page-load-commits-and-parses.yaml` under `.out/runner-probe/`: an `assert:` key
  on a timeline act is refused `ESS-AUTHOR-001` ("unknown field `assert`, expected one of `at`,
  `command`, `actor`, `caller`, `input`, `outcome`, `refused`, `response`, `error`, `events`,
  `no_events`, `capture`, `elapsed`, `mark`"). The binary accepts `ess-scenario/7`, which the
  older documentation does not list; its refusal of `ess-scenario/99` names `/7` as "`covers:`"
  and adds: "a document in a repository-defined trace format is not a scenario input".
- The two authored scenarios under `spec/scenarios/` already replay a network the ESS way: an act
  `surf.net.DeliverResponse` with the body as base64, the request identity captured from
  `NavigationStarted`.
- A recorded step "follow the Sign in link" would read, in `ess-scenario/6` (command names from
  the draft on branch `spec/agent-session`, which has no `Follow` yet):

  ```yaml
  - at: 2026-01-06T09:00:02Z
    command: surf.agent.Follow
    actor: surf.agent.SessionHolder
    caller: {session_id: {$instance: session}}
    input: {session_id: 3f1d5b7e-0000-4000-8000-000000000001, handle: l4}
    outcome: followed
    capture: {instance: login, event: surf.navigation.NavigationStarted, field: navigation_id}
  - at: 2026-01-06T09:00:03Z
    command: surf.net.DeliverResponse
    input: {request_id: {$instance: login}, status: 200, media_type: text/html, body: PGh0bWw+...}
    outcome: responded
  ```

  What it cannot express: an expectation after each step (view claims only at the end); the
  `changed` lines, which the session computes outside the model (design section 6: ESS has no
  text rendering of a view row); a locator resolved against the current document at run time
  (an input is a literal fixed when the suite is written, so the step names a handle, which the
  design rejects in section 7); a secret supplied when the scenario runs; and a live run, because
  the scenario states every response. Not confirmed: whether `capture:` can read a
  `RequestStarted` published by a command a binding invoked (the authored scenarios rely on a
  navigation and its request sharing one identity instead).
- `surf run` ships. The ESS runner is `ess verify conform run` or the `ess-conformance` crate,
  which brings third-party crates and is allowed only under `conformance/` (`AGENTS.md`,
  "Goals"). A shipped `surf run` cannot embed it.

## Change

A new design note `docs/design/scenario-runner.md`, status "proposed". This story writes no
other file; a pointer from `docs/design/agent-interface.md` section 10 is the coordinator's to
add. One section per decision, each with the answer, the alternative rejected and why. Two of
them (D12, D13) are the coordinator's, taken in `docs/plan/agent-plan-revision-1.md` (D2, D3);
the note restates them so a reader of the runner finds them here. The answers to take:

- **Q1, the format.** surf's own line-oriented text formats, `surf-recording 1` and
  `surf-scenario 1`, printable ASCII, every page- or server-derived value quoted and escaped as
  the answer format does. Not `ess-scenario/6`, for the reasons measured above. An export of a
  recording to an `ess-scenario` document is left for later and not promised.
- **Q2, the locator.** A step names kind, accessible name and tree path. Candidates are the
  elements of the current document with exactly that kind and name. One candidate: it. Several:
  the one whose tree path equals the recorded one; none equal: the step fails as ambiguous and
  prints every candidate. No candidate: the step fails and prints the elements of that kind. The
  path breaks ties only; it never selects an element whose name differs, and position is never
  used.
- **Q3, an unrecorded request.** The replay transport answers it as `Unreachable`, so the
  request ends and nothing hangs; the step during which it was asked fails with the line
  `unrecorded <method> "<address>"`. A replayed run never opens a connection. A recorded
  exchange left unused is not a failure; the run prints a count of them.
- **D4, matching.** A request is matched by its key: method, address, and request body. The n-th
  request with a key gets the n-th recorded exchange with that key, and is answered with the
  recorded status, media type, body and the address the response was served from, which differs
  from the request address after a redirect (revision D14). Each exchange records the sequence
  number of the action during which it happened, for messages only.
- **D5, scenario and recording.** A scenario names its recording by a path relative to the
  scenario file; responses are not copied into the scenario. Changing a recorded response means
  editing the recording.
- **D6, secrets.** A value typed into a `password` control is written as `${PASSWORD_<n>}`,
  numbered by first appearance, wherever the recording would hold it: the action, request bodies
  (as typed and form-encoded) and response bodies. At run time `${NAME}` is read from the
  environment variable `SURF_VAR_<NAME>`, never from an argument word (design section 3, the
  token); a missing one refuses the run before its first step. The replay transport compares
  bodies with references in place.
- **D7, files.** A recording and a scenario are created with mode 0600 and never overwrite an
  existing file. Neither holds the session token; the session identity appears once in the
  recording header and is never compared; no wall-clock time is written.
- **D8, running.** Steps run in order; the first failing step stops the run and the rest are
  printed as skipped. Exit 0 when every step passed, 1 when a step failed, 2 for usage, an
  unreadable or malformed file or a missing variable, 3 when the run stopped otherwise (the
  statuses `crates/surf` uses). No duration or timing is printed, so two runs print the same
  bytes.
- **D9, expectations.** In a scenario every handle in a line is written as its locator. Lines
  marked `expect` are asserted and must equal the actual block; a person demotes a line to `seen`
  to keep it without asserting it. `surf scenario from` marks the page's status and address and
  the `changed` lines `expect`. A recording mirrors the action log one to one, `open` and `close`
  included; `surf scenario from` makes no step of them, because `surf run` opens a fresh session
  before the first step and closes it after the last.
- **D10, the model.** No recording, scenario or run becomes an ESS entity, command input or event.
  The formats are specified as documents with golden files, as the answer format is (design
  section 6). The specification gains one field, `record`, on the type the `open` callable of the
  command-line document takes: a command-line shape in `surf.agent`, not an input of
  `OpenSession` and not a field of `Session`.
- **D11, the binary.** `surf run` and `surf scenario from` are subcommands of `surf`. Because the
  runner drives the session core (`crates/session`, story `agent-session-process`), which depends
  on the `surf` library, the `surf` binary moves into a package of its own, first.
- **D12, no tree change before scripts** (revision D2). Parsing builds a document's tree and
  nothing changes it. What an agent changes is control state, held in the specification's
  control rows, so a same-document `changed` block holds control-state lines only.
- **D13, no detailed lines in this epic** (revision D3). The detailed mode (class, attribute and
  style changes) waits for scripts and belongs to `epic:detailed-mode`, with `--detail`. The
  recording format reserves the block kind `detail` for its lines, which a version 1 reader keeps
  as opaque quoted lines and writes back unchanged, so the format version need not change when
  they arrive. No story of this epic writes one.

Surfaces: `docs/design/scenario-runner.md` (new). Nothing else.

## Acceptance

`docs/design/scenario-runner.md` answers Q1 to Q3 and D4 to D13, each with its answer and the
rejected alternative, quotes the measured `ESS-AUTHOR-001` refusal and the `ess-scenario/7`
finding, and the operator's acceptance of it is recorded as an `approval-record` that
`decides` this story, which `aep plan artifact show story:runner-decisions` lists.

## Out of scope

Any code, format grammar or golden file (stories `runner-formats` and later). An `ess-scenario`
export. The detailed mode and `--detail`, which `epic:detailed-mode` holds.

## Domain relations

None rests on an `ess/1` document: no entity this note talks about (recording, scenario, step,
exchange, variable) is declared, and D10 keeps them out of the model. The relations the note
fixes, each answered above and none inferred: session to recording, one to zero or one, the file
outlives the session (Q1, D7); recording to action, one to one with the action log (D9); scenario
to recording, many to one by path (D5); recording to exchange, one to many, each exchange tied to
one action by sequence number (D4); variable to password value, one to one per distinct value
(D6).

## Scope

Derived 2026-11-21 by `story-scoper`.

- **Primary surface:** `docs/design/scenario-runner.md` — cited. The story says "Surfaces: `docs/design/scenario-runner.md` (new). Nothing else." (`.engineering/planning/story/runner-decisions.md:134`). It is a document-only story with no code and no crate.
- **Files written:** `docs/design/scenario-runner.md`, created, status "proposed" — cited (`runner-decisions.md:69`, `:134`). The path does not exist on the branch (`ls docs/design` shows only `agent-interface.md`).
- **Files read, not written:** `docs/design/agent-interface.md`, sections 3, 6, 7 and 10 (question 2 is the open format question), cited by the story (`runner-decisions.md:16`, `:70`) — cited.
- **Files read, not written:** `docs/plan/agent-plan-revision-1.md`, D2, D3 and D14. The story restates D2 and D3 as its D12 and D13 and uses D14 in its D4 (`runner-decisions.md:63-66`, `:96-97`) — cited.
- **Files read, not written:** `spec/scenarios/page-load-commits-and-parses.yaml`, one of the two authored scenarios the story measured against (`runner-decisions.md:28-29`) — cited.
- **Files read, not written:** `AGENTS.md` "Goals" (the third-party-crate rule behind the "no ESS runner in a shipped `surf run`" argument) — cited (`runner-decisions.md:58-60`).
- **Files read, not written:** `crates/surf`, the exit statuses it uses (`runner-decisions.md:118-119`), and the ESS documentation checkout `$HOME/beyond10x/ess/website/docs` (`reference/formats.md`, `guides/verify/author-scenarios.md`) — cited.
- **Files read, not written:** the draft branch `spec/agent-session`, for the `surf.agent` command names — cited (`runner-decisions.md:35`). This is not on the `plan/agent-epics` branch.
- **Handed to the coordinator:** a pointer from `docs/design/agent-interface.md` section 10, question 2 (line 338), to `docs/design/scenario-runner.md` — cited ("a pointer from `docs/design/agent-interface.md` section 10 is the coordinator's to add", `runner-decisions.md:70`). This file is not on the list in D7, so the story's own wording decides it.
- **Handed to the coordinator:** the `approval-record` that `decides` this story. It is a store write, and the acceptance needs it before the story counts as done (`runner-decisions.md:138-141`) — cited.
- **Handed to the coordinator:** `AGENTS.md`, `README.md`, the root `Taskfile.yml` and `Cargo.lock` — none needed. The story asks for no section or line in any of them — cited (the story says "Nothing else").
- **Symbols:** none. No code, grammar or golden file (`runner-decisions.md:131`, "Out of scope"). D10's new `record` field on the `open` callable of the command-line document is only decided in the note, and a later story specifies it — cited.
- **Would collide with:** no story that writes `docs/design/scenario-runner.md` — inferred. I searched `.engineering/planning` for the path. Only this story's body names it (plus one critic review that says where the decisions go). The other runner stories (`runner-formats`, `runner-record-option`, `runner-record-session`, `runner-replay-transport`, `runner-run`, `runner-scenario-from`, `runner-surf-cli-package`, `runner-e2e`, `runner-detail-option`, `runner-detailed-changes`) were not read in full.
- **Would collide with:** the pointer into `docs/design/agent-interface.md` is the coordinator's, so there is no unit collision on it. `html-decisions` writes a different file, `docs/design/html-parser.md` (`html-decisions.md:137`) — cited.
- **Not yet existing when it starts:** nothing. The story has only a `decomposes` edge to `epic:e2e-scenario-runner` and no `depends_on`. The `surf.agent.Follow` command, which the story's example uses, is not on this branch. The story marks it as a draft ("which has no `Follow` yet", `runner-decisions.md:35`). It is only quoted as an example.
- **Confidence:** high. The story names its one file and says nothing else is written. The acceptance is entirely about a document, so it cannot collide with a code unit.

