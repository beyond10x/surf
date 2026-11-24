---
format: aep.planning-md/3
id: story:scripts-spec
kind: story
status: active
title: Specify the record of a script's end and the scripting option of surf-agent open
summary: surf.document records each script's end with its document and surf.agent.OpenCall gains the scripting option, in the shape the note gives; scenarios for every outcome; the binding declares the option and the client refuses it until the document port.
relations:
- decomposes: epic:scripts-in-a-page
- depends_on: story:scripts-decisions
scope:
- confidence: cited
  path: cli/surf-agent.yaml
- confidence: cited
  path: conformance/src/target.rs
- confidence: cited
  path: conformance/tests/noticed.txt
- confidence: inferred
  path: crates/agent/tests/drift.rs
- confidence: cited
  path: crates/ports/src
- confidence: inferred
  path: crates/ports/tests
- confidence: cited
  path: crates/session/src/entries.rs
- confidence: cited
  path: crates/session/src/lib.rs
- confidence: inferred
  path: crates/session/tests
- confidence: cited
  path: crates/surf/src/lib.rs
- confidence: inferred
  path: crates/surf/tests
- confidence: inferred
  path: crates/view/examples/prototype.rs
- confidence: inferred
  path: crates/view/tests/json.rs
- confidence: inferred
  path: docs/view-stream.md
- confidence: cited
  path: generated/rust
- confidence: inferred
  path: spec/components.yaml
- confidence: cited
  path: spec/domains/agent.yaml
- confidence: cited
  path: spec/domains/document.yaml
- confidence: inferred
  path: spec/ess-inputs.yaml
- confidence: cited
  path: spec/scenarios
- confidence: cited
  path: website/docs/principles.md
- confidence: cited
  path: website/docs/status.md
revision: 33
transitions:
- {from: "draft", to: "proposed", at: "2026-11-24T10:46:12Z", actor: "human:operator", revision: 26}
- {from: "proposed", to: "active", at: "2026-11-24T12:08:07Z", actor: "human:operator", revision: 33, executor: "agent:coordinator"}
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
- `cli/surf-agent.yaml:100-111`: the `open` command and its arguments (`url`, `as`, `from-line`,
  `ca-file`). `ess-cli/2` maps each input field to exactly one argument, and a Boolean option
  takes an explicit `true` or `false` (ESS `docs/design/cli-presentation-binding.md:175-178`).
  The client's parser is held to the binding by `crates/agent/tests/drift.rs`; what the binding
  declares and the parser refuses is listed in `BINDING_ONLY` (line 11) beside an `UNMAPPED:`
  marker of the binding (`--token-stdin`, `--token-prompt`, `--session-dir`).
- What a change of the model forces outside `spec/`, read from the last change of the model,
  `ee86329` ("Specify the initiator of a navigation and the cookie rules"): besides `spec/` and
  `generated/rust/` it touched `conformance/src/target.rs`, `conformance/tests/noticed.txt` (its
  digest covers the suite, `conformance/src`, `conformance/tests`, `conformance/Cargo.toml` and
  `conformance/Cargo.lock`, `taskfiles/conform-noticed.sh:10-13`; a stale list fails the fast
  check of `task conform-rust`, `Taskfile.yml:118-123`), `crates/ports/src/ports.rs`,
  `crates/ports/tests`, `crates/session/src/entries.rs` ("every field the spec declares"),
  `crates/session/src/lib.rs`, `crates/session/tests`, `crates/surf/src/lib.rs`,
  `crates/surf/tests`, `crates/view/examples/prototype.rs`, `crates/view/tests/json.rs` and
  `docs/view-stream.md`. A new command of `document` adds a generated `...Behavior` trait that
  `DocumentBehaviors` in `crates/surf/src/lib.rs:1362-1381` (and its blanket impl from line 1383)
  lists and the document port implements.


## Domain relations

- `surf.document.Document` and the record of a script's end: requires-stakeholder-input; filed as
  `decision-blocker:script-end-record`, which blocks this story. `docs/design/scripts.md`, Q7,
  answers it with its five lines (`cardinality:`, `owner:`, `written:`, `outlives its document:`,
  `without scripts:`): many records per document, one per script element the parser reached
  (`not run: eof` included) and one per element a script inserted (`not run: inserted`), never two
  for one element, each with its position from 1; owned by the document through a relation
  `scripts`, `owns`, many, via `document_id`, as `forms` is; written while the document is
  `Parsing`, by `surf.document.AddScriptEnd` from the parser step, before `FinishParsing`, in the
  order the ends happened; never outliving its document; none for a document parsed without
  scripts. The operator clears the blocker.
- `surf.document.Element` to `surf.document.Document`, many to one: inferable,
  `spec/domains/document.yaml`, entity `surf.document.Element`, relation `document` (line 297). An
  element a script made is recorded as a parsed one is; nothing in this relation changes.
- `surf.agent.Session` and its scripting option, one per session, set by `open`, ending with the
  session: inferable (inferred from `surf.agent.OpenCall.ca_file`,
  `spec/domains/agent.yaml:102-119`, an `ess/1` field and its comment, no `relations:` entry; S3
  and S7 of the epic's "Decisions taken", lines 594 and 596); Q7 records it on `surf.agent.Session`
  as `scripts: Boolean`.

## Decisions taken

1. The `refs:` of each new command and outcome cite the HTML Standard's sections the note names
   (Q1, Q2); a deliberate deviation says so in a comment beside it (`AGENTS.md`, "Rules").
2. The option reaches the session the way `ca_file` does: in the `open` request, passed by `surfd`
   unchanged; the comment beside the field says so.
3. The binding declares the option in `cli/surf-agent.yaml` now, because each field of
   `OpenCall` needs its one argument; until `story:scripts-document-port` gives it behaviour, the
   client refuses it: an `UNMAPPED:` marker beside the argument says "the client refuses the
   option until `story:scripts-document-port`; what settles it: that story", and
   `BINDING_ONLY` in `crates/agent/tests/drift.rs` lists it. No session runs a script after this
   story.
4. One scenario per new outcome under `spec/scenarios/`, so both conformance reports hold a
   passed scenario for each (`task conform`, `task conform-rust`). `conformance/src/target.rs`
   maps the new command and field, and `conformance/tests/noticed.txt` gets its new digest and
   lines from `task conform-slow WRITE=1` (`Taskfile.yml:215`).
5. The hand-written code the generated change forces follows `ee86329`: the document port
   implements the new behaviour as the specification says (it records the end it is given; no code
   issues the command until `story:scripts-parse-run` and `story:scripts-document-port`),
   `DocumentBehaviors` lists it, the session's entries write every field the specification
   declares, and a hand-written value of a new field is the option's off value until the code that
   sets it comes with its test. The event `ScriptEndAdded` carries no page text (Q7); the view
   stream changes only where the generated change of the event or of `surf.agent.Session.scripts`
   reaches `crates/view`, as `ee86329` did.
6. The record of a script's end is the YAML `docs/design/scripts.md`, Q7, proposes for
   `spec/domains/document.yaml`, taken as it stands: the types `surf.document.ScriptId` (a newtype
   of `Uuid`, minted when the record is written), `surf.document.ScriptEnd` (`Ran`, `Threw`,
   `Refused`, `Limit`, `Faulted`, `NotRun`), `surf.document.ScriptReason` (23 variants, reached
   through `Limit` or `NotRun` only: `Empty`, `Src`, `Type`, `Module`, `Importmap`,
   `Speculationrules`, `Disconnected`, `Nomodule`, `Event`, `Svg`, `Inserted`, `Eof`,
   `StepsPerScript`, `Memory`, `SourceBytes`, `WallClock`, `Nodes`, `Depth`, `Attributes`,
   `StoredBytes`, `ScriptsPerDocument`, `StepsPerDocument`, `Faulted`, each summary its word) and
   `surf.document.ScriptErrorKind` (`Error`, `EvalError`, `RangeError`, `ReferenceError`,
   `SyntaxError`, `TypeError`, `UriError` with the summary `URIError`, and `Value` for a thrown
   value that is no error object); the entity `surf.document.Script` (identity `script_id`; fields
   `document_id`, `position`, `end`, `reason`, `error_kind`, `message`, `line`, `column`,
   `dynamic`, all but the first three optional; lifecycle `Added`, terminal) and the relation
   `scripts` of `surf.document.Document`; the command `surf.document.AddScriptEnd` (wire
   `add-script-end`,
   `refs: ["whatwg-html:prepare-the-script-element", "whatwg-html:execute-the-script-element"]`)
   with the outcome `added`, which creates the record and emits `surf.document.ScriptEndAdded`
   (`script_id`, `document_id`, `position`, `end`), and the outcomes `no-such-document` and
   `already-parsed` as `AddForm` has them.
7. A recorded error holds what Q16 reads: the kind and own `message` from `Agent::error_summary`,
   the line and column (from 1, the column in UTF-16 code units) and `dynamic` from
   `Agent::last_entry_error_site`; the message decoded from UTF-16 with unpaired surrogates
   replaced and cut to its first 1,024 characters, the comment beside the field says so and that
   the text is the page's, untrusted; nothing else of the error.
8. The option is Q7's: `surf.agent.OpenCall` gains `scripts: Optional<Boolean>` with the comment Q7
   gives (absent is false; only `open` takes it), `surf.agent.OpenSession`'s input and
   `surf.agent.Session`'s fields gain `scripts: Boolean`, set by `opened`; `cli/surf-agent.yaml`,
   path `[open]`, gains the argument `field: scripts`, `source: {kind: option, long: scripts}`. The
   spelling is `surf-agent open --scripts true <url>`; `--scripts false` and no option are the
   default, and a bare `--scripts` is not a form `ess-cli/2` takes (Q7).
9. The marker at `spec/components.yaml:429-430` loses "script execution"; its new text is Q7's:
   "UNMAPPED: no component for storage (cookies, cache, history) or input events. None is declared
   until its first command is. Script execution is part of the document component's parser step
   (`docs/design/scripts.md`, Q7)."
10. What an answer prints of the record (Q7's `scripts` line and the lines after it) is not this
    story's: `story:scripts-document-port` writes it with the code that prints it.

## Change

- `spec/domains/document.yaml`: decisions 6 and 7 (the types, the record, the relation `scripts` of
  `Document`, the command, event and outcomes of Q7).
- `spec/domains/agent.yaml`: decision 8 (`OpenCall`, `OpenSession` and `Session`), with Q7's
  comment.
- `spec/components.yaml`: the marker of lines 429-430, decision 9.
- `spec/ess-inputs.yaml`: only if `task generate` asks for it, as `ee86329` changed it.
- `spec/scenarios/`: decision 4.
- `generated/rust/`: `task generate`.
- `cli/surf-agent.yaml`, `crates/agent/tests/drift.rs`: decisions 3 and 8.
- `conformance/src/target.rs`, `conformance/tests/noticed.txt`: decision 4.
- `crates/ports/src`, `crates/surf/src/lib.rs`, `crates/session/src/entries.rs`,
  `crates/session/src/lib.rs`, and the constructors of changed types in `crates/ports/tests`,
  `crates/session/tests` and `crates/surf/tests`: decision 5.
- `crates/view/examples/prototype.rs`, `crates/view/tests/json.rs`, `docs/view-stream.md`: only as
  decision 5 says.
- In the last commit, the public surfaces of acceptance 8.

## Acceptance

1. `slot task validate` and `slot task generated-check` pass; `generated/rust/` changed only by
   `task generate`.
2. The conformance reports of `slot task conform` and `slot task conform-rust` each hold at least
   one passed scenario for every outcome of `surf.document.AddScriptEnd` (`added`,
   `no-such-document`, `already-parsed`) and of `OpenSession` with `scripts` true and with it
   false; the scenarios of `added` together record each of the six variants of `ScriptEnd`, one of
   them `NotRun` with the reason `Eof` and one `Threw` with an error kind, a message, a line and a
   column; the report lists the scenario names per outcome.
3. `sh taskfiles/conform-noticed.sh check` passes, and `slot --timeout 30m task conform-slow`
   prints `.out/noticed.txt equals conformance/tests/noticed.txt`.
4. `slot task cli` passes with the new argument of `open`;
   `git grep -c 'UNMAPPED:' cli/surf-agent.yaml` prints one more than on the story's base (6 on
   `main` of 2026-11-24); `drift` passes with the option in `BINDING_ONLY`.
5. `surf-agent open --scripts true <url>` exits 2 with the usage on standard error and starts no
   session (a test in `crates/agent/tests/`, module `drift` or the client's refusal module).
6. Each new command and outcome of `spec/domains/document.yaml` and `spec/domains/agent.yaml` has a
   `refs:` line or a comment beside it that says `deliberate deviation` (as
   `spec/domains/document.yaml:82` and `:163` do); the report lists each with the line number of
   its `refs:` or comment, read with `git grep -n` on the story's tip.
7. `slot task check` exits 0 (`validate`, `conform`, `conform-rust`, `generated-check` and `cli`
   among its steps); `git diff --stat` names only the files of the Change.
8. Public surfaces: the story changes, in its last commit and after merging `main`, every public
   surface its change makes false, each number taken from the `slot task check` log of its tip.
   After that commit, `git grep -n -F -e '<old>' -- README.md website/docs docs/badges` prints
   nothing for each old string below, as the story's base holds it (the strings and lines given are
   those of `main` at `29945f5`, 2026-11-24); each new number equals the one in that log; and the
   gate block, `README.md:584-625` and `website/docs/how-it-is-built.md:40-81`, holds that log's
   lines. The old strings of this story: the scenario count, `203 of 203` (`README.md:10`,
   `docs/badges/make.sh:37`, `docs/badges/conformance.svg`, `website/docs/status.md:22`),
   `203 scenario` (`README.md:262`, `:627`, the gate blocks, `website/docs/how-it-is-built.md:83`,
   `website/docs/principles.md:33`), `203 specified` (`README.md:283`,
   `website/docs/what-works.md:15`) and `(44 authored)` (the gate blocks); the test count
   `1976 passed` (`README.md:11`, `docs/badges/make.sh:38`, `docs/badges/tests.svg`), `1,976` and
   `1,424, and 552` (`README.md:285`, `website/docs/status.md:23`,
   `website/docs/what-works.md:17`).
9. The folder's bounds hold: the test step ends within 120 s and no test runs over 60 s; no model
   call and no network request in a test; the synthesized specification is made once per step run,
   not per test; no Python; the ESS output `generated/rust/` keeps its own check,
   `generated-check`, and no other generator with a committed output is added; `task conform-slow`
   runs in the lane `slow` because the sources changed; a build or test job in the pool ends within
   10 minutes or names its time with `slot --timeout`, at most 30 minutes; a test over 5 s is named
   with its time.

## Out of scope

Any code that runs a script or issues the new command; the client's acceptance of the option and
the session's use of it (`story:scripts-document-port`); the `scripts` line of an answer (Q7),
which `story:scripts-document-port` writes with the code that prints it.

## Placement

After `story:scripts-decisions` and the clearing of `decision-blocker:script-end-record`; beside
`story:webidl-reader`, `story:scripts-engine-dependency`, `story:session-process-hardening` and
`story:html-script-hook`. `spec/domains/agent.yaml` and `cli/surf-agent.yaml` are also written by
the active `story:runner-record-option` (`open --record`), `story:agent-open-tab-caller-id` and
`story:agent-skill-definition` (which reads the binding when it is built);
`spec/domains/agent.yaml`, `conformance/src/target.rs`, `conformance/tests/noticed.txt`,
`crates/session` and `generated/rust` by `story:agent-cookie-jar` until its merge: never in the
same wave. `story:scripts-document-port` writes `crates/surf/src/lib.rs`, `crates/session` and
`conformance/` after this story, ordered by its edge.
`website/docs/principles.md` is also written by `story:scripts-engine-dependency`, which may run
beside this story: this story changes the scenario count of line 33 (at `29945f5`), that story
principle 4 at lines 18-25; neither changes the other's lines, and the second to merge keeps the
first's.

## Why `epic:scripts-in-a-page`

The specification of what the epic's outcome records and of the option that turns it on, which
the epic's "Specs first" puts before its code.
