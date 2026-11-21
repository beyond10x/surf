# Agent command line: first draft of the session domain (2026-11-20)

For `epic:agent-cli-target` (draft). Slice 1 only: session, token, open, go, read state, close.
The draft is uncommitted in the managed worktree
`~/.local/state/worktree/trees/b10x/surf/surf-agent-spec` (id `surf-agent-spec`, branch
`spec/agent-session`, base `3aaa3aa`): `spec/domains/agent.yaml`, `cli/surf-agent.yaml`, two
authored scenarios, and additions to `spec/components.yaml`, `spec/system.yaml` and
`spec/ess-inputs.yaml`. Nothing of it is on `main`, and nothing is generated from it.

## State

`ess specify validate` accepts the draft (`surf v1 — 9 file(s), 4 scenario(s), valid`). The gate
would not pass with it as drafted, for three measured reasons:

- `ess generate synthesize --target rust` refuses the whole model because the token is marked
  `one_time_response`.
- 12 of 148 synthesized scenarios fail against the interpreter, all for one cause: the suite
  fixes the caller's `session_id` when it is written, and the session's identity is minted when
  it runs.
- 33 refusal lines against `REFUSALS_MAX: 10`.

## Decisions the coordinating agent took on the draft's open markers

Each is reversible and each is the operator's to overrule. Numbers are the author's marker
numbers in the report below.

- 7, session identity: chosen by the caller, drawn by `open`, with a refusal for an identity
  that exists. This is how `surf.net.Fetch` takes its `request_id`, and it removes the cause of
  the 12 failing scenarios (5 remained on the author's scratch copy; those are to be read next).
- 4, the token: not marked `one_time_response` in the specification for now. The session
  process issues and checks it, and the specification says so in an `UNMAPPED:` marker. With the
  marker on, no Rust code can be generated for any component.
- 1, token form: 32 bytes from the operating system's random source, written as 64 lower-case
  hexadecimal digits. Hexadecimal needs no encoder beyond what `crates/ports` already has.
- 6, where the window and tab come from: in slice 1 the session process runs `OpenWindow`,
  `OpenTab` and `OpenSession` itself. A caller-chosen `tab_id` on `OpenTab` is a story of its
  own.
- 8, who reads a session's state: one process per session, answering only for its own session
  (the epic's decision), since a view filter cannot read the caller.
- 11 and measurements A and B, the command line itself: `cli/surf-agent.yaml` (`ess-cli/2`)
  stays the specification of the command line, checked by `ess`. The parser and the text
  representation are written here, with the standard library, from that document. Both ESS
  generators write code that needs third-party crates (`clap`, `clap_complete`, `serde`,
  `serde_json`, `rpassword`, `rustix`), which the no-third-party-crate rule keeps out of
  anything that ships, and `ess-cli/2` writes JSON only where the operator asked for dense text.
- 2, 5, 9, 10: the author's answers stand (one tab per session, no check beyond the user-only
  session directory, later slices, a `CloseWindow` story).
- 3 and 12 wait for slice 2 and for ESS.

## Not done, and whose move it is

- The nine ESS limits in part 5 of the report are not filed as issues. Filing is an act toward
  another repository; it waits for the operator's word.
- The draft is not revised to the decisions above. That is the first step of the epic's slice 1
  when the epic is taken up, after wave 5.
- The ESS checkout at `~/beyond10x/ess` was at `0.92.0-5-g4ab2ea1449` when the author read its
  documentation; the `ess` binary is 0.96.0. The author confirmed each claim against the binary.

## The author's report, as returned

### 1. Verdict

The draft validates: `surf v1 — 9 file(s), 4 scenario(s), valid` (exit 0, no warning, no `ESS-ENTITY-019` or `ESS-COMMAND-019`). The gate would still fail in three places:
- **Generation:** `task generate` cannot run. The Rust target refuses the whole model because of the one-time token.
- **Conformance:** 12 scenarios fail against the interpreter.
- **Refusals:** `task suite` would need `REFUSALS_MAX: 33`.

Completeness counts: `{"unscenarioed":65,"outside":0,"unanswered":2}`.

### 2. The model

- **Component** `agent` owns `surf.agent`, `reached_by: in_process`.
- **Entity** `Session`, identity `SessionId` (a Uuid). Lifecycle Open → Closed (`close`).
  - Stored fields: `window_id`, `tab_id`, `navigation_id?`, `url?`, `navigation_state?`.
  - Relations, all `references`, cardinality one: `window` → `surf.shell.Window`, `tab` → `surf.shell.Tab`, `navigation` → `surf.navigation.Navigation`.
- **Actors**
  - `Opener`: may `OpenSession`, read `Sessions`.
  - `SessionHolder`: credential attribute `session_id`; may `Go`, `CloseSession`, read `SessionState`.
- **Commands**
  - `OpenSession(window_id, tab_id)` → `opened`. Answers `{session_id, token}`, with `token` marked `one_time_response`; emits `SessionOpened`.
  - `Go(session_id, url)` → `not-holder` (`when: session_id != caller.session_id`), `no-such-session`, `closed` (state Closed), or `requested` (emits `NavigationRequested`).
  - `CloseSession(session_id)` → `not-holder`, `no-such-session`, `closed` (emits `SessionClosed`), or `wrong-state`.
  - `NoteNavigationStarted(tab_id, navigation_id, url)` and `NoteNavigationEnded(navigation_id, state)`: change the session whose `tab_id` / `navigation_id` matches (`instances:`). Both emit `SessionNavigationChanged{navigation_id, state, sessions}`. No actor is granted them; bindings invoke them.
- **Errors:** `SessionNotFound`, `NotSessionHolder`, `SessionStateConflict`.
- **Views**
  - `Sessions`: every stored field, no filter.
  - `SessionState(session_id)`: `session_id, state, url?, navigation_state?`. Given `naming.code: SessionStateView` because the Rust target names it and `Session.State` both `SessionState`.
- **New bindings** (6, all `at_most_once` + `drop`; the existing 4 are untouched, no line removed):
  - `NavigationRequested` → `surf.navigation.Navigate`
  - `SessionClosed` → `surf.shell.CloseTab`
  - `NavigationStarted` → `NoteNavigationStarted`
  - `NavigationCommitted`, `NavigationFailed`, `NavigationCancelled` → `NoteNavigationEnded`, with the literal `Committed`, `Failed` or `Cancelled`.
- **Opening a session:** no binding can learn the window and tab it opened, because `OpenWindow`/`OpenTab` mint their identities and their events name no session. So the session process runs `OpenWindow`, then `OpenTab`, then `OpenSession` itself (marker 6).
- **The command line** (binary `surf-agent`, `ess-cli/2`, one global flag `--session-dir`, JSON output):

| Command | What it runs |
|---|---|
| `open` | `OpenWindow` + `OpenTab` + `OpenSession`; prints `{session_id, token}` once |
| `go <session> <url>` | `Go`; answers the `SessionState` row |
| `state <session>` | reads `SessionState` |
| `close <session>` | `CloseSession`; answers the `SessionState` row |

  - `go`, `state` and `close` read the token only from `--token-file`, `--token-stdin` or `--token-prompt`, never from an argument word, which other local users can read in the process table.
  - All callables are `local`. A `service_forward` input must equal the command input exactly, and the token is not an input of any command.
  - Four structs in `surf.agent` exist only for the binding: `SessionCall`, `GoCall`, `OpenedSession`, `SessionReadout`.
- **Authored scenarios:** 2, because synthesis refuses `NoteNavigation*/noted`. Both pass.

### 3. UNMAPPED markers

Each with what would settle it, then the answer I would take.

1. **Token alphabet and length** (`SessionToken`). Settled by: the owner choosing them. Answer: 32 bytes from the OS random source, base64url.
2. **One tab per session** (the `tab` relation). Settled by: a later slice deciding whether the session follows popups. Answer: `one` until then.
3. **The session's document.** `DocumentParsed` names no navigation or tab, so no session can claim a parsed document. Settled by: that event carrying `navigation_id`. Answer: that change, in slice 2.
4. **That the credential is the token `OpenSession` issued.** ESS has nothing tying a caller attribute to a one-time response value. Settled by: an ESS construct, or the owner accepting that the session process checks the token. Answer: the process checks it, plus an ESS issue.
5. **Who may open a session.** Settled by: the owner. Answer: whoever can reach the user-only session directory, with no further check.
6. **Where the window and tab come from.** Settled by: `OpenTab` taking a caller-chosen `tab_id`, as `Fetch` takes `request_id` (a change to an existing command, so its own story). Answer: that change; a binding then opens the tab.
7. **Who mints `session_id`.** If the target mints it, synthesized holder scenarios fail (10 here). A caller-chosen id plus an `existing_instance:` refusal leaves 5 failures on a scratch copy. Settled by: the owner, or ESS. Answer: caller-chosen, drawn by `open`.
8. **Only the holder reads `SessionState`.** A view filter cannot read the caller. Settled by: ESS supporting that, or one process per session answering only its own session. Answer: the latter.
9. **Links, controls, handles, visibility.** The document domain has nothing to list yet. Settled by: slices 2 to 5.
10. **A closed session leaves its window open** (`components.yaml`). Nothing closes a window. Settled by: a `CloseWindow` story.
11. **The line-oriented text representation** (`cli/surf-agent.yaml`). `ess-cli/2` writes JSON only, and ESS has no text rendering of a view row. Settled by: the owner. Answer: JSON for slice 1; decide the grammar with slice 3.
12. **CLI refusals** (`cli/surf-agent.yaml`). A callable's `errors:` cannot name a model error, so `not-holder`, `no-such-session` and `closed` can only surface as an internal failure. Settled by: ESS, or mirror structs. Answer: an ESS issue first.

### 4. Measurements

**A. `ess generate cli`**: runs. Output: `10 CLI artifact(s), generated in …/agent-spec/cli`. Files: `Cargo.toml`, `binding.json`, `manifest.json`, `help.txt`, `README.md`, `completions/surf-agent.bash`, `src/{lib,main,runtime,wire}.rs`, `.ess-output/state.json`. Dependency lines:
```
clap = { version = "4", features = ["string"] }
clap_complete = "4"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
rpassword = "7"
rustix = { version = "1", features = ["fs", "process"] }
```
This generated parser cannot ship under the no-third-party-crate rule.

**B. `ess generate synthesize --target clap`**: exit 1, nothing written:
```
clap target cannot emit this workspace
commands.surf.agent.OpenSession.outcomes.opened.one_time_response: one_time_response requires implementation-owned atomic durable consumption and fresh issuance; this code target cannot implement that policy
```
On a scratch copy without that marker it runs: `136 capabilities: 129 generated, 3 obligation(s), 4 refused`, `8 artifact(s)`, crate `crates/surf-cli`, dependencies `clap = "4"`, `clap_complete = "4"`. The command tree is empty (`Command::new("unnamed")`): `TARGET.md` has 17 target refusals, "no component declaring `reached_by: command_line` accepts this command". It cannot ship either.

**Also measured, because it decides `task generate`:** `--target rust` on the draft fails with the same refusal. Without the marker it runs: 136 capabilities (129 generated, 3 obligations, 4 refused), no third-party crate. The 3 obligations are `NoteNavigationStarted`, `NoteNavigationEnded` (`instances:`) and `OpenSession` (typed `response:`).

**C. Gate numbers**

| | main | draft |
|---|---|---|
| scenarios | 50 (2 authored) | 148 (4 authored) |
| refusals | 10 | 65 (33 distinct causes; 33 `^refused:` lines) |
| passed / failed | 50 / 0 | 136 / 12 |

Verbatim: `148 scenario(s) (4 authored), 65 refusal(s), 33 distinct cause(s)` and `148 scenarios: 136 passed, 12 failed, 0 error, 0 unsupported`.

Refusals by kind (the Taskfile's 33 lines):
- `ESS-SYNTH-010`: 10 `binding/delivery` + 10 `binding/on-failure` (6 of each are the new bindings, same reasons as main's) + 2 `binding/flow` (main's two).
- `ESS-SYNTH-001`, 4 outcomes:
  - `Go` and `CloseSession` `no-such-session`: "has too few values inside the guards the input meets to name an identity no other scenario sends".
  - `NoteNavigationStarted` and `NoteNavigationEnded` `noted`: "has no three rows the filter selects that the arranging commands can produce".
- One-time token cells: `ESS-SYNTH-001` ×5 lines (37 cells), `ESS-SYNTH-002` ×1, `ESS-SYNTH-008` ×1.

Notes printed: "declares actors and serves no component … no `<command>/grant/denied` scenario is owed" and "names a view in a grant and serves no component … no `<view>/grant/read/…` scenario is owed".

`REFUSALS_MAX` would need to be **33**, because `task suite` counts `^refused:` lines and folds the 32 "and N more" cells. Without the one-time marker it would be 26 (scratch copy). It should not be raised before markers 4 and 7 are decided.

All 12 failures are the caller-versus-minted-identity problem (marker 7). The target answers `not-holder`; the interpreter showed `expected error = SessionStateConflict, observed error = NotSessionHolder` and `the command published no events` once the one-time marker, which masks diagnostics as "protected observation did not satisfy the declared rule", was off. The failing scenarios:
- `close-tab-on-session-closed/binding/{flow,mapping}`
- `navigate-on-navigation-requested/binding/{flow,mapping}`
- `CloseSession/outcome/closed`, `Go/outcome/{closed,requested}`
- `Session/state/Closed/refuses/{CloseSession,Go}`, `Session/transition/close/…/closed`
- two `OpenSession/disclosure/…/as/actor/SessionHolder` cells

### 5. ESS limits hit

**Worth an ESS issue:**
1. **`one_time_response` blocks every code target, whole workspace** (text in B). Docs: guides/verify/one-time-responses.md, "Implementation synthesis refuses a marked model when its target cannot implement durable consumption and fresh issuance". It could stay an obligation, as the typed response already does.
2. **Synthesized scenarios send a caller value fixed when the suite is written** against an identity minted at run time. Their own interpreter fails them, with no refusal. Docs: guides/verify/author-scenarios.md, "a caller's values are fixed when the suite is written".
3. **`ESS-SYNTH-008` is an internal-drift message**: "its strategy is `default_branch` and it declares no guard / help: `TestStrategy` and `OutcomeCondition` have drifted apart in `ess-domain`".
4. **`ESS-SYNTH-002`: disclosure cells cannot decide a caller guard**, while the ordinary scenarios can: "`session_id != caller.session_id` cannot be decided against an input of `surf.agent.CloseSession` … `caller.session_id` reads `caller`, which the input's types do not declare".
5. **Set-effect arrangement refused although the creator writes `tab_id` from its input**, which is what the help text asks for. The refusal survived removing the actors, guards, response and relation on scratch copies. Docs: guides/specify/selection-effects.md.
6. **ess-cli refuses Uuid**: `error: unsupported CLI primitive \`Uuid\``. Every surf identity is a Uuid; worked around with `String` in the binding structs. Docs: design/cli-presentation-binding.md, "Other primitives … are explicit refusals".
7. **ess-cli cannot name a model error**: `error: unresolved CLI type \`surf.agent.SessionStateConflict\``. It also cannot name a command input, response or view row, which is why the binding needs the four structs. And `service_forward` has no channel for a credential.
8. **No row-level read grant**: `ESS-VIEW-003` "`caller.session_id` reads `caller`, which is not a declared observable root" (scratch probe).
9. **Both CLI generators need third-party crates** (A and B). Docs: guides/synthesize.md target table; reference/cli.md, "standalone Rust/Clap package". Worth an issue only if the owner wants the generated parser to ship rather than writing one here from the same binding document.

**Not worth an issue** (by design, or a modelling matter):
- `ESS-COMMAND-003` `unobservable_fact`: `when_subject` cannot read the identity; worked around with `when:`.
- `ESS-COMMAND-004`: "a preserving outcome declares no assignment, event or error"; `Go` uses `updates:` with no `sets:`.
- `ESS-COMMAND-007` `empty_change`: hence `SessionNavigationChanged`.
- No return path from a binding to its cause (marker 6).
- ESS admits a cross-component one-hop `related` view read without any warning (scratch probe). Only `AGENTS.md`'s rule keeps it out.

**Also note:** the documentation checkout is at `0.92.0-5-g4ab2ea1449`, older than the 0.96.0 binary used here. Everything above was confirmed against the binary.

### 6. Files

`git status --short`:
```
 M spec/components.yaml
 M spec/ess-inputs.yaml
 M spec/system.yaml
?? cli/
?? spec/domains/agent.yaml
?? spec/scenarios/agent-session-reads-committed-navigation.yaml
?? spec/scenarios/agent-session-reads-started-navigation.yaml
```
`git --no-pager diff --stat`:
```
 spec/components.yaml | 112 +++++++++++++++++++++++++++++++++++++++++++++++++++
 spec/ess-inputs.yaml |   3 ++
 spec/system.yaml     |   1 +
 3 files changed, 116 insertions(+)
```
New files (untracked, so not in the stat):
- `$HOME/.local/state/worktree/trees/b10x/surf/surf-agent-spec/spec/domains/agent.yaml`
- `$HOME/.local/state/worktree/trees/b10x/surf/surf-agent-spec/cli/surf-agent.yaml`
- the two scenarios under `spec/scenarios/`

Written outside the worktree, all under `$HOME/.cache/scratch/b84c1d1b-86a1-4880-9b86-d11fa8d6a813/agent-spec/`: `measure-c/`, `cli/`, `cli.log`, `clap.log`, `clap-no-otr/`, `clap-no-otr.log`, `rust.log`, `rust-no-otr/`, `rust-no-otr.log`, `probe/` (spec copies `p1`–`p11` with their suites, logs and run reports). Nothing under `generated/`. The lease is released.

### Review step

Before anything is generated from this draft, all 12 markers go to the owner. Markers 4 and 7 come first: they decide whether the gate can go green at all. With them go these contradictions:
- The epic's "they get a token, an ID" against synthesis needing a caller-chosen identity (marker 7).
- The epic's text representation against ess-cli's JSON-only output (marker 11).
- The goal of a specified command line against both generators needing third-party crates (A and B).
- The one-time token against `task generate`: with the token marked, the Rust workspace cannot be regenerated.
