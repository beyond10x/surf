---
format: aep.planning-md/3
id: story:scripts-spec
kind: story
status: draft
title: Specify the record of a script's end and the scripting option of surf-agent open
summary: surf.document records each script's end with its document and surf.agent.OpenCall gains the scripting option, in the shape the note gives; scenarios for every outcome; the binding declares the option and the client refuses it until the document port.
relations:
- decomposes: epic:scripts-in-a-page
- depends_on: story:scripts-decisions
scope:
- confidence: cited
  path: cli/surf-agent.yaml
- confidence: inferred
  path: conformance/src/target.rs
- confidence: inferred
  path: crates/agent/tests/drift.rs
- confidence: cited
  path: generated/rust
- confidence: inferred
  path: spec/components.yaml
- confidence: cited
  path: spec/domains/agent.yaml
- confidence: cited
  path: spec/domains/document.yaml
- confidence: cited
  path: spec/scenarios
revision: 3
---
## Context

The epic's fourth candidate (lines 429-436) and its rule "Specs first" (line 544): the
specification records each script's end with its document and gives the session the scripting
option of S3 on `surf-agent open` only (S7), green in the gate before any code fills it. The
epic's acceptance 8 (lines 587-588): both conformance reports hold at least one passed scenario
for every outcome of the commands that record a script's end and of the scripting option.

The tree on `main` of 2026-11-24:

- `spec/domains/document.yaml`: `surf.document.Document` (line 136) with `stopped_by:
  Optional<ParseLimit>` and the lifecycle `Parsing` to `Parsed`; the node tree is not modelled
  (lines 130-133); `ParseLimit` (lines 83-99) names the parser limits; `Element` (line 239)
  references its document (relation `document`, line 297).
- `spec/domains/agent.yaml`: `surf.agent.OpenCall` (lines 107-119) has `url`, `form`,
  `from_line` and `ca_file`; the comment at lines 102-106 says how the anchors of `ca_file` reach
  the session (the bytes in the `open` request, passed by `surfd` over the session socket) and
  that no entity holds them. `surf.agent.Session` is at line 271.
- `spec/components.yaml:429-430`: "UNMAPPED: no component for script execution, storage
  (cookies, cache, history) or input events. None is declared until its first command is."
- `cli/surf-agent.yaml:97-108`: the `open` command and its arguments (`url`, `as`, `from-line`,
  `ca-file`). `ess-cli/2` maps each input field to exactly one argument, and a Boolean option
  takes an explicit `true` or `false` (ESS `docs/design/cli-presentation-binding.md:175-178`).
  The client's parser is held to the binding by `crates/agent/tests/drift.rs`; what the binding
  declares and the parser refuses is listed in `BINDING_ONLY` (line 11) beside an `UNMAPPED:`
  marker of the binding (`--token-stdin`, `--token-prompt`, `--session-dir`).

## Domain relations

- `surf.document.Document` and the record of a script's end: requires-stakeholder-input; filed as
  `decision-blocker:script-end-record`, which blocks this story. The note's Q7 proposes the answer
  and the operator clears the blocker.
- `surf.document.Element` to `surf.document.Document`, many to one: inferable,
  `spec/domains/document.yaml`, entity `surf.document.Element`, relation `document` (line 297).
  An element a script made is recorded as a parsed one is; nothing in this relation changes.
- `surf.agent.Session` and its scripting option, one per session, set by `open`, ending with the
  session: inferable (inferred from `surf.agent.OpenCall.ca_file`, `spec/domains/agent.yaml:
  102-119`, an `ess/1` field and its comment, no `relations:` entry; S3 and S7 of the epic's
  "Decisions taken", lines 594 and 596).

## Waits for the note

- waits for the note: Q7, the YAML of the record of a script's end (entity or field, the closed
  list of ends, command, event, outcomes, which process issues it), and the answer of
  `decision-blocker:script-end-record`;
- waits for the note: Q7, the option's field, type and spelling under `ess-cli/2` (a bare
  `--scripts` is not a form the binding takes), whether the session's row records it, and what an
  answer prints;
- waits for the note: Q1 and Q16, the reasons a script is not run and the fields of a recorded
  error with their bounds;
- waits for the note: Q7, what becomes of the marker at `spec/components.yaml:429-430`.

## Decisions taken

1. The `refs:` of each new command and outcome cite the HTML Standard's sections the note names
   (Q1, Q2); a deliberate deviation says so beside it (`AGENTS.md`, "Rules").
2. The option reaches the session the way `ca_file` does: in the `open` request, passed by `surfd`
   unchanged; the comment beside the field says so.
3. The binding declares the option in `cli/surf-agent.yaml` now, because each field of
   `OpenCall` needs its one argument; until `story:scripts-document-port` gives it behaviour, the
   client refuses it: an `UNMAPPED:` marker beside the argument says "the client refuses the
   option until `story:scripts-document-port`; what settles it: that story", and
   `BINDING_ONLY` in `crates/agent/tests/drift.rs` lists it. No session runs a script after this
   story.
4. One scenario per new outcome under `spec/scenarios/`, so both conformance reports hold a
   passed scenario for each (`task conform`, `task conform-rust`). Where `conform-rust` needs the
   Rust target to issue the new command, `conformance/src/target.rs` gains it here.

## Change

- `spec/domains/document.yaml`: the types, the record and the command, event and outcomes of Q7.
- `spec/domains/agent.yaml`: the field of `OpenCall` (and of `Session`, if Q7 says so), its
  comment.
- `spec/components.yaml`: the marker of lines 429-430 as Q7 says.
- `spec/scenarios/`: decision 4.
- `generated/rust/`: `task generate`.
- `cli/surf-agent.yaml`, `crates/agent/tests/drift.rs`: decision 3.
- `conformance/src/target.rs`: only as decision 4 says.

## Acceptance

`slot task check` exits 0 (`validate`, `conform`, `conform-rust`, `generated-check` and `cli`
among its steps). The folder's bounds hold: the test step ends within 120 s and no test runs over
60 s; no model call and no network request in a test; the synthesized specification is made once
per step run, not per test; no Python; the ESS output `generated/rust/` keeps its own check, `generated-check`, and no other generator with a committed output is added; a build or test job in the pool ends within 10 minutes or names its time with `slot --timeout`, at most 30 minutes; a test over 5 s is named with its time.

1. `slot task validate` and `slot task generated-check` pass; `generated/rust/` changed only by
   `task generate`.
2. The conformance reports of `slot task conform` and `slot task conform-rust` each hold at least
   one passed scenario for every outcome of the new command and of `OpenSession` with and without
   the option; the report lists the scenario names per outcome.
3. `slot task cli` passes with the new argument of `open`; `task open` lists the new `UNMAPPED:`
   marker; `drift` passes with the option in `BINDING_ONLY`.
4. `surf-agent open` with the option exits 2 with the usage on standard error and starts no
   session (a test in `crates/agent/tests/`, module `drift` or the client's refusal module).
5. Every new command and outcome that follows the standard has `refs:`; `git grep -n
   'UNMAPPED:' spec cli` shows each open question of the change at its place.
6. `git diff --stat` names only the files of the Change.

## Out of scope

Any code that runs a script; the client's acceptance of the option and the session's use of it
(`story:scripts-document-port`); the answer's grammar if Q7 adds a line (written with the code
that prints it, in `story:scripts-document-port`).

## Placement

After `story:scripts-decisions` and the clearing of `decision-blocker:script-end-record`; beside
`story:webidl-reader`, `story:scripts-engine-dependency` and `story:session-process-hardening`.
`spec/domains/agent.yaml` and `cli/surf-agent.yaml` are also written by the active
`story:runner-record-option` (`open --record`), `story:agent-open-tab-caller-id` and
`story:agent-skill-definition` (which reads the binding when it is built); `spec/domains/agent.yaml`,
`conformance/src/target.rs` and `generated/rust` by `story:agent-cookie-jar` until its merge:
never in the same wave.

## Why `epic:scripts-in-a-page`

The specification of what the epic's outcome records and of the option that turns it on, which
the epic's "Specs first" puts before its code.
