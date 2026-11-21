# Design: the scenario runner's decisions

Status: **proposed**. Written for `story:runner-decisions` on 2026-11-21. It answers the three
questions `epic:e2e-scenario-runner` leaves to its first story (design
`docs/design/agent-interface.md`, section 10, question 2, and the epic's "Open" list) and fixes
the decisions the later runner stories build on: `runner-formats`, `runner-record-option`,
`runner-replay-transport`, `runner-record-session`, `runner-scenario-from`, `runner-run`,
`runner-e2e`, and `runner-surf-cli-package` in `epic:agent-cli-target`.

Every section has the answer, the alternative rejected and what the answer rests on. D12 and D13
are the coordinator's decisions D2 and D3 of `docs/plan/agent-plan-revision-1.md`, restated here
so that a reader of the runner finds them in one place. The note agrees with revision 1 (D2, D3,
D14) and with revision 2 (`docs/plan/agent-plan-revision-2.md`): `surfd` owns the sessions and
talks to one session process per session; an answer is read back with `surf_answer::parse`, and
no runner story writes a parser of the answer grammar (D20); `--record` travels as an absolute
path and its refusal is the session process's answer to `open` (D28).

## The parts, as the runner sees them

```
recording a session                          running a scenario
-------------------                          ------------------
surf-agent open --record <file> <url>        surf run <scenario> [--live]
   │ absolute path, quoted (D28)                │ one process: no surfd, no socket,
   ▼                                            │ no session process
surfd ── passes the request on unchanged        ▼
   ▼                                         session core (crates/session) as a library
session process ── creates <file>, 0600         │ over a Transport
   │ writes each action and exchange            ▼
   ▼                                         ReplayTransport (default) or surf-fetch (--live)
<file>: surf-recording 1
   │
   ▼
surf scenario from <file>  ──>  <name>.scenario: surf-scenario 1, names <file> by relative path
```

The recording file is written by the session process itself. It does not travel over the link to
`surfd`, and `surfd` never reads it: what `surfd` keeps is the log of revision 2 (D19, D22), with
bodies as their length. The recording holds bodies in full, because a replay answers from them.

## Measured for Q1

On 2026-11-21 with `ess 0.96.0` on `PATH`, on a copy of `spec/` under `.out/runner-probe/` (git
ignores `.out/`). The probes are copies of `spec/scenarios/page-load-commits-and-parses.yaml`:

- `base.yaml`: the copy, unchanged (`type: ess-scenario/6`).
- `v7.yaml`: `type: ess-scenario/7`, nothing else changed.
- `act-assert.yaml`: `base.yaml` with a view claim on the first timeline act, after its
  `outcome: started` (lines 21 to 23):

  ```yaml
      assert:
        - view: surf.navigation.Navigations
          contains: {navigation_id: {$instance: load}, state: Started}
  ```

- `v7-act-assert.yaml`: `act-assert.yaml` with `type: ess-scenario/7`.
- `v99.yaml`: `type: ess-scenario/99`, nothing else changed.

Run from `.out/runner-probe/`:

```
$ ess verify conform author --path spec --scenarios base.yaml
1 authored scenario(s) from 1 file(s), 0 refusal(s), suite ess-conformance/34, nothing written
exit 0

$ ess verify conform author --path spec --scenarios v7.yaml
1 authored scenario(s) from 1 file(s), 0 refusal(s), suite ess-conformance/34, nothing written
exit 0

$ ess verify conform author --path spec --scenarios act-assert.yaml
refusal[ESS-AUTHOR-001]: act-assert.yaml
  unreadable: timeline[0]: unknown field `assert`, expected one of `at`, `command`, `actor`, `caller`, `input`, `outcome`, `refused`, `response`, `error`, `events`, `no_events`, `capture`, `elapsed`, `mark` at line 21 column 5
  help: the document is YAML with the keys `type`, `domain`, `scenario` and `summary`; a key it does not know is refused rather than ignored
0 authored scenario(s) from 1 file(s), 1 refusal(s), suite ess-conformance/34, nothing written
exit 1

$ ess verify conform author --path spec --scenarios v7-act-assert.yaml
refusal[ESS-AUTHOR-001]: v7-act-assert.yaml
  unreadable: timeline[0]: unknown field `assert`, expected one of `at`, `command`, `actor`, `caller`, `input`, `outcome`, `refused`, `response`, `error`, `events`, `no_events`, `capture`, `elapsed`, `mark` at line 21 column 5
  help: the document is YAML with the keys `type`, `domain`, `scenario` and `summary`; a key it does not know is refused rather than ignored
0 authored scenario(s) from 1 file(s), 1 refusal(s), suite ess-conformance/34, nothing written
exit 1

$ ess verify conform author --path spec --scenarios v99.yaml
refusal[ESS-AUTHOR-002]: v99.yaml
  `ess-scenario/99` is not `ess-scenario/1`
  help: a scenario's `type:` is `ess-scenario/1`, `ess-scenario/2` for entity setup, `ess-scenario/3` for fixture values, `ess-scenario/4` for direct responses, `ess-scenario/5` for caller values, `ess-scenario/6` for member paths in claims, or `ess-scenario/7` for `covers:`; a document in a repository-defined trace format is not a scenario input, so leave it out of `--scenarios` and `ess-inputs.yaml`
0 authored scenario(s) from 1 file(s), 1 refusal(s), suite ess-conformance/34, nothing written
exit 1
```

What this shows:

- An act of a timeline claims only what the key list of `ESS-AUTHOR-001` names: its `response`,
  `error`, `events` (or `no_events`) and a `capture`. A view claim (`contains`, `excludes`,
  `at`) stands only in the scenario's own `assert:`, read after the whole timeline.
- `ess-scenario/7` is accepted by the binary, and adds `covers:` and nothing for per-act claims:
  under `/7` the act-level `assert` is refused the same way. The ESS documentation checkout
  (`$HOME/beyond10x/ess`, now at `0.96.0-0-g9e91e8b575`) says the same in
  `website/docs/reference/formats.md`, the rows for `ess-scenario/6` ("member paths in claims")
  and `ess-scenario/7` ("adds `covers:`", marked "Not yet in a release").
- ESS itself names the case of a recording: "a document in a repository-defined trace format is
  not a scenario input, so leave it out of `--scenarios` and `ess-inputs.yaml`".

## Q1. The format

**Answer.** Recordings and scenarios are surf's own line-oriented text formats,
`surf-recording 1` and `surf-scenario 1`: printable ASCII, every value derived from a page, a
server or an address written inside double quotes and escaped with `surf-answer`'s quoting
function, as the answer format does. Their answer lines are the answer's own lines, read with
`surf_answer::parse` and quoted values with `surf_answer::unquote` (revision 2, D20). Neither file
is listed in `spec/ess-inputs.yaml` or passed to `--scenarios`. An export of a recording to an
`ess-scenario` document is left for later and not promised.

**Rejected: `ess-scenario/6` (or `/7`), run by the ESS runner.** Measured above and read in the
ESS documentation; what a recorded step needs and the format cannot hold:

- an expectation after each step: view claims stand only in the scenario-level `assert:`, after
  the whole timeline (`ESS-AUTHOR-001` above, `/6` and `/7` alike);
- the `changed` lines: the session computes them outside the model, and ESS has no text
  rendering of a view row (design, section 6);
- a locator resolved against the current document while the step runs: an act's `input:` is a
  literal fixed when the suite is written (`{$instance}` and `{$fixture}` references aside), so
  the step would name a handle, which the design rejects (section 7);
- a secret supplied when the scenario runs (D6);
- a live run: the scenario itself states every response, as the two authored scenarios under
  `spec/scenarios/` do with `surf.net.DeliverResponse` and a base64 body.

And it cannot ship: `surf run` is part of the `surf` binary, and the ESS runner is
`ess verify conform run` or the `ess-conformance` crate, which brings third-party crates and is
allowed only under `conformance/` (`AGENTS.md`, "Goals").

**Rejected: `ess-scenario/6` as the file format, run by surf's own runner.** It would keep the
YAML and still lack every item of the list above; surf would need a YAML reader with no
third-party crate, and the file would claim a format whose meaning ESS defines differently.

**Rejected: JSON or another general data format.** A reader without a third-party crate, and the
answer lines would be strings inside strings. Line text with the answer's quoting is already the
format the session writes and `surf-answer` reads.

**Rests on:** the measurement above; design sections 5 to 7; `AGENTS.md`, "Goals"; revision 2,
D20 and D26.

## Q2. A locator, and two elements with the same kind and name

**Answer.** A step names its element by a locator: kind, accessible name and tree path
(`button "Sign in" at /html/body/form[1]/button[1]`; the notation is `runner-formats`'). The
candidates are the elements of the current document's `now` with exactly that kind and that
accessible name.

- One candidate: it.
- Several: the one whose tree path equals the recorded one. None equal: the step fails as
  ambiguous and prints every candidate with its tree path.
- None: the step fails and prints the elements of that kind.

The tree path breaks ties only: it never selects an element whose name differs. Position among
the candidates (the n-th in document order) is never used. The sibling indices inside a tree path
are part of the path, compared whole, not a rank among candidates.

**Rejected: the tree path as the selector.** A wrapper added around a form changes every path
below it; a locator exists so that a scenario survives such an edit (design, section 7).

**Rejected: the first candidate, or the n-th in document order.** When a row is added above, the
step acts on another element and passes, and a test that passes on the wrong element is worse
than one that fails.

**Rejected: a near match on the name.** The epic is accepted on a relabelled button failing its
step (`runner-e2e`); a near match would hide exactly that change.

**Rests on:** design sections 5.5 and 7; the epic's Acceptance; `runner-run` cases 4 and 5.

## Q3. A request the recording does not hold

**Answer.** In a replayed run the replay transport answers it as
`Answer::Failure(FailureReason::Unreachable)`, so the request ends through the page load's normal
failure path (`FailRequest`, then `FailNavigation`) and nothing hangs. The step during which it
was asked fails with the line `unrecorded <method> "<address>"`. A replayed run never opens a
connection: the replay transport holds no socket and no address resolver. A recorded exchange
left unused is not a failure; the last line of the run prints how many there were.

**Rejected: falling through to the network.** The run would depend on the site and the network,
and the default for a test in a gate is replayed (design, section 7).

**Rejected: leaving the request unanswered.** `Browser::run` asks the transport once per started
request; a request with no answer stays `Pending` and its navigation never ends.

**Rejected: a new failure reason `Unrecorded`.** `surf.net.FailureReason` is in the
specification; a reason for the runner would put the runner into the model (D10). The step's own
failure line carries what is specific.

**Rejected: an unused exchange as a failure.** A person who deletes a step from a scenario leaves
its exchanges unused, and the run's expectations already decide whether the page still does what
it should.

**Rests on:** `crates/surf/src/lib.rs` (`Transport`, `Answer`, `FailureReason::Unreachable`,
asked once per `Pending` request); `spec/domains/net.yaml`, `surf.net.FailureReason`;
`runner-replay-transport` cases 5 and 6; `runner-run` case 6.

## D4. Matching a request to an exchange

**Answer.** A request is matched by its key: method, address and request body. The n-th request
with a key gets the n-th recorded exchange with that key. It is answered with the recorded
status, media type, body and the address the response was served from, which differs from the
request address after a redirect followed inside one request (revision 1, D14). Each exchange
also records the sequence number of the action during which it happened (`during <seq>`); that
number is used for messages only, never for matching.

**Rejected: the address alone.** Two `POST`s to one address, a wrong password and then the right
one, would be answered in each other's place.

**Rejected: the order of all requests alone.** One request added or dropped by a changed page
shifts every later answer onto the wrong request.

**Rejected: matching on the action's sequence number.** A request would then belong to a step,
and a person who deletes or reorders a step would break the replay of every step after it.

**Rejected: answering at the request address.** After a `POST` answered with 303, the navigation
would commit at the form's address instead of the landing page.

**Rests on:** revision 1, D14; `runner-replay-transport` cases 2 and 4.

## D5. Scenario and recording

**Answer.** A scenario names its recording by a path relative to the scenario file
(`recording "<relative path>"`). Responses are not copied into the scenario. Changing a recorded
response means editing the recording.

**Rejected: copying the exchanges into the scenario.** The scenario is the file a person reads and
edits; every body in it would bury the steps, and one response would live in two files.

**Rejected: an absolute path, or one relative to the working directory.** The pair could not be
moved or committed together, and the run's result would depend on where it is started.

**Rests on:** design, section 7 (a scenario is edited, a recording is the record);
`runner-scenario-from`, `runner-run`.

## D6. Secrets

**Answer.** A value typed into a `password` control is written as `${PASSWORD_<n>}`, numbered by
first appearance, wherever the recording would hold it: in the action, and in request and response
bodies, both as typed and form-encoded (`application/x-www-form-urlencoded`). At run time
`${NAME}` is read from the environment variable `SURF_VAR_<NAME>`, never from an argument word
(design, section 3, the token). A missing one refuses the run before its first step. The replay
transport compares request bodies with the references in place: the body the run sends gets the
same replacement the recording transport applied, and the results are compared.

**Rejected: the value in clear.** A recording is meant to be kept and committed beside its
scenario.

**Rejected: leaving the value out.** The replay could not match the `POST` body and the run could
not type the value.

**Rejected: `--var NAME=value` on the command line.** Other local users read a process's
arguments.

**Rejected: reading `${NAME}` from the environment variable `NAME`.** A scenario, which an agent
may write, could then name any variable of the person running it, `SURF_TOKEN` included. The
prefix limits it to variables set for the purpose.

**Rejected: failing at the step that uses a missing variable.** In a live run the steps before it
would already have acted on the site.

**Rests on:** design, sections 3 and 8; epic Outcome "Secrets"; `runner-replay-transport` case 7,
`runner-record-session` case 1, `runner-run` case 7.

## D7. Files

**Answer.** A recording and a scenario are created with mode 0600 and never overwrite an existing
file (`create_new`). Neither holds the session token. The session identity appears once, in the
recording's header, and is never compared. No wall-clock time is written. Under `surfd`, the
client sends the `--record` path made absolute against its own working directory, `surfd` passes
it on unread, and the session process creates the file before it issues any command; a path that
cannot be recorded to is the session process's refusal of `open`, exit 2, and `surfd` lists no
session for it (revision 2, D28).

**Rejected: the default mode (0644 under the usual umask).** A recording holds page content and
what the agent typed; the epic's constraint asks for a mode only the user can read.

**Rejected: overwriting.** It destroys a recording or an edited scenario a person kept.

**Rejected: writing the token, the time, or comparing the identity.** The token would let a reader
of the file drive the live session; a time or an identity makes two recordings of the same
actions differ and no run reproducible, because every run draws a new session.

**Rejected: a path relative to `surfd`'s or the session process's working directory.** Neither is
the directory the person ran `surf-agent` in (`runner-record-option` case 2).

**Rests on:** epic Constraints; design, sections 3 and 7; revision 2, D28.

## D8. Running

**Answer.** Steps run in order. The first failing step stops the run; every later step is printed
as skipped. Exit 0 when every step passed, 1 when a step failed, 2 for usage, an unreadable or
malformed file or a missing variable, 3 when the run stopped otherwise. These are the statuses
`crates/surf` uses (`EXIT_COMMITTED` 0, `EXIT_NOT_COMMITTED` 1, `EXIT_USAGE` 2, `EXIT_STOPPED` 3)
and the design gives `surf-agent` (section 4). No duration or timing is printed, so two runs print
the same bytes.

**Rejected: running every step after a failure.** The later steps act on a page the scenario did
not expect: their failures are noise, and in a live run they may submit a form on the wrong page.

**Rejected: printing durations.** Two runs would differ, and the epic's acceptance compares two
runs' output byte for byte.

**Rejected: an exit status of its own per failure kind.** One meaning per number across `surf`
and `surf-agent`; the output line says which kind.

**Rests on:** `crates/surf/src/lib.rs`, the four constants; design, section 4; epic Acceptance;
`runner-run` cases 2, 3 and 7.

## D9. Expectations, and what a step is

**Answer.** In a scenario every handle in a line is written as its locator. Lines marked `expect`
are asserted and must equal the actual block; a person demotes a line to `seen` to keep it without
asserting it. `surf scenario from` marks the page's status and address (the `page` line) and the
`changed` lines `expect`. A recording mirrors the action log one to one, `open` and `close`
included; `surf scenario from` makes no step of them, because `surf run` opens a fresh session
before the first step and closes it after the last.

**Rejected: handles in expected lines.** A handle is a name inside one session and shifts when an
element is added above (design, section 5.5).

**Rejected: asserting the whole `now` block by default.** Every change of page content fails the
scenario; the design asks for structural changes by default (section 7).

**Rejected: deleting unasserted lines.** A person keeps the context of a step when reading or
editing it; `seen` keeps it and says it is not checked.

**Rejected: steps for `open` and `close`.** `surf run` has no session to open by name; a fresh
session per run is what keeps runs independent.

**Rests on:** design, section 7; `runner-scenario-from` case 1; `runner-run`.

## D10. The model

**Answer.** No recording, scenario or run becomes an ESS entity, command input or event. The
formats are specified as documents with golden files, as the answer format is (design, section 6).
The specification gains one field, `record`, on the type the `open` callable of the command-line
document takes: a command-line shape in `surf.agent`, not an input of `surf.agent.OpenSession`
and not a field of `surf.agent.Session`.

**Rejected: recordings, steps and exchanges as entities.** They are files outside any
component's rows, and running a scenario is not a behaviour of the browser; modelling them would
add a component that synthesis tests and the browser does not need.

**Rejected: `record` on `OpenSession` or on `Session`.** A user's file path would become a
command input and a row field, carried in events into `surfd`'s kept log and the live view.

**Rests on:** design, section 6; `AGENTS.md`, "Rules" (specs first; a component holds its own
rows); the `ESS-AUTHOR-002` text above (a repository-defined trace format is not a scenario
input); `runner-record-option`, "Where the field lives".

## D11. The binary

**Answer.** `surf run` and `surf scenario from` are subcommands of `surf`. The runner drives the
session core (`crates/session`, story `agent-session-process`), which depends on the `surf`
library, so the `surf` binary first moves into a package of its own, `crates/surf-cli`
(revision 1, D6; `story:runner-surf-cli-package`).

**Rejected: the binary staying in `crates/surf`.** A binary of that package cannot depend on a
crate that depends on the package's own library; cargo refuses the cycle.

**Rejected: `surf-agent run`.** `surf-agent` is a thin client of `surfd` and holds no page
(design, section 3); `surf run` runs with no `surfd` and no socket (`runner-run` case 1).

**Rejected: a third binary, `surf-run`.** One more binary to build, ship and find, for what the
design already names `surf run`.

**Rests on:** design, sections 3 and 7; revision 1, D6.

## D12. No tree change before scripts (revision 1, D2)

**Answer.** Parsing builds a document's tree and nothing changes it. What an agent changes is
control state (value, checkedness, selectedness), held in the specification's control rows, not
in the tree. So a same-document `changed` block holds control-state lines only.

**Rejected: a node identity stable across changes within one document.** There is nothing for it
to be stable across before a script engine exists.

**Rests on:** revision 1, D2, which reads the HTML Standard as keeping control state apart from
the attributes (not checked against the standard's text here).

## D13. No detailed lines in this epic (revision 1, D3)

**Answer.** The detailed mode (class, attribute and style changes) waits for scripts and belongs
to `epic:detailed-mode`, with `--detail`. The recording format reserves the block kind `detail`
for its lines; a version 1 reader keeps them as opaque quoted lines and writes them back
unchanged, so the format version need not change when they arrive. No story of this epic writes
one, and `surf scenario from` does not carry one into a scenario.

**Rejected: specifying detailed lines now.** Nothing produces them before scripts; their tests
could only assert states built by hand (revision 1, D3).

**Rejected: no reserved kind.** The first detailed line would then need `surf-recording 2` and a
reader for both versions.

**Rests on:** revision 1, D3; `runner-formats`; `runner-scenario-from` case 6.

## Relations the note fixes

No `ess/1` document holds them; D10 keeps these things out of the model.

- Session to recording: one to zero or one; the file outlives the session (Q1, D7).
- Recording to action: one to one with the action log, `open` and `close` included (D9).
- Scenario to recording: many to one, by relative path (D5).
- Recording to exchange: one to many; each exchange tied to one action by sequence number, for
  messages only (D4).
- Variable to password value: one to one per distinct value (D6).
