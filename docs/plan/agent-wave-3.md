# Agent interface plan, wave 3 of 22: the daemon, the log, the page

One wave of the [wave plan](agent-waves.md). Four units, each in its own worktree, each
implemented by a sub-agent on Opus; the coordinating session wrote no code. Every commit is the
organization bot's.

## Units

| story | what landed | the unit's own gate | unit commit |
|---|---|---|---|
| `agent-surfd-supervisor` | `crates/surfd`: the daemon; `wire::caller_uid` and the shared test base in `crates/agent`; one step in `check.yml` | 26 cases, five runs in a row; 2 planted defects each turn one case red | `662e4e5` |
| `agent-page-path-spec` | `Action`, `RecordAction`, `SessionLog`, the navigation result on `Session`, in `spec/`, `generated/rust`, `crates/ports`, `crates/surf`, the conformance target | the whole gate, exit 0: 81 scenarios, 27 refusals | `00cb53d` |
| `view-page-prototype` | `crates/view/web`: the page; `assets.rs`; the example `prototype` on scripted sessions | 43 tests; one page load in a headless browser, 4 timeline segments after 2 s and 12 after 10 s | `150fa04`, `1011948` |
| `runner-surf-cli-package` | `crates/surf-cli`: the binary `surf` and its five tests; the release workflow's package | the whole gate, exit 0, the test count unchanged | `434b77f` |

Each unit's claim was checked by the coordinator before its commit: `surfd` by two more runs of
its suite and the client's (55 passed each) and a search for processes and directories left
behind, the page by its three suites and a search of the page files for markup-writing calls,
`surf-cli` by its suite and `cargo metadata`, the model by the gate on this branch. The gates'
text scan refused none of the files.

## What was different from wave 2

Three of the four stories were read against the tree by a scoper and revised before their unit
started, because wave 2 had shown a floor carried over from a draft (120 scenarios asked, 77
measured).

- **`agent-surfd-supervisor`** disagreed with the client of wave 2 in 14 places. Five would
  have made a case unable to fail or unable to pass: the loser of the start lock reported
  `ready` before the socket accepted; an option was to shorten a timer of a daemon that the
  client starts without arguments; and three cases passed through `surf-agent` without the
  daemon running or seeing the input. The revision states the socket names with their byte
  arithmetic, which tests start the built daemon themselves, and that no peer check is done.
- **`agent-page-path-spec`** counted "at least 9 above" and "at most 8 above" a base nobody had
  measured. The reviser drafted the model in a scratch copy first: 81 scenarios (7 authored),
  27 refusals, 81 of 81 on the interpreter. The unit then measured the same four numbers on
  both runners. The draft also settled what the story had left open: the session process
  issues `RecordAction` and supplies the sequence number (recording through bindings measured
  33 refusals and 3 interpreter failures), and two note commands instead of one.
- **`view-page-prototype`** was added in wave 2 so that the page can be looked at four waves
  before `surfd` serves real sessions. The operator had it open on the day it landed and asked
  for a better layout at narrow widths; the finding went back to the unit (`1011948`).

## What went wrong, and what was decided

- **Two tests of `surfd` took their expected value from the code under test.** The unit planted
  a defect (a connection limit of 65) and its case stayed green, because the test counted with
  `surfd::CONNECTION_LIMIT`. The test now uses the story's literal 64; the oversize answer of
  the stand-in uses the wire's constant, not the link's. A planted defect per bound is the
  check that found it.
- **`open <address>` is two actions.** The model records `1 open` and `2 go "<address>"`, as
  `docs/agent-answer.md` shows. `story:agent-session-process` and case 13 of
  `story:view-surfd-serves` said one action; both are revised before their waves.
- **Rate limits cut the implementors twice.** Each was resumed on the same model with its
  context.
- **A probe of one reviser ran `rustc +1.99.1`**, which made rustup install a toolchain named
  `1.99.1` beside the default stable of the same version. Nothing depends on it.

## The gate on the wave branch

`slot task check`, exit 0, on `wave/a3-surfd` after the four units, the regenerated lock files
and `REFUSALS_MAX: 27`:

```
surf v1 — 9 file(s), 7 scenario(s), valid
81 scenario(s) (7 authored), 27 refusal(s), written to .out/suite.json
  81 scenarios: 81 passed, 0 failed, 0 error, 0 unsupported
the interpreter and the Rust system ran 81 scenario(s) each, of suite sha256:52cfd52d0708126b94c22777a19f089867264b0f44b14813330565e8079ea9b7
generated/rust matches spec/ (30 artifact(s) and .ess-output)
cli/surf-agent.yaml — valid CLI presentation binding
Cargo.lock: 20 package(s), none from outside this repository
88 test suites: 324 passed, 0 failed, 0 ignored
```

Before the wave: 77 scenarios, 24 refusals, 18 packages, 81 suites, 280 passed.

## Findings, and where each goes

| finding | from | goes to |
|---|---|---|
| the number of open sessions has no limit: each is a process and a thread of the same user | `agent-surfd-supervisor` | backlog draft; a caller that can connect is the same user, who can start processes anyway |
| a client that connects after `surfd` decided on its idle exit and before it removed the socket gets end of file and exits 3 | `agent-surfd-supervisor` | backlog draft: the client retries once through a fresh start |
| `surfd` cannot close its standard output without unsafe code; it writes and flushes its one line | `agent-surfd-supervisor` | accepted: the client reads one line |
| a lock file swapped for a FIFO between the check and the open blocks the open | `agent-surfd-supervisor` | the wave 2 backlog draft for the token file covers both |
| `cargo test -p <package> --test <name>` does not build the package's examples | `agent-surfd-supervisor`, `view-page-prototype` | `AGENTS.md`, as a note; the page's test builds its example itself |
| what `surfd` gives a session process and expects from it (arguments, environment, the start line, the link, the end) | `agent-surfd-supervisor` | `story:agent-session-process`, revised from the unit's list before wave 4 |
| the commands the session process issues for `open`, `go` and `close`, as the model has them | `agent-page-path-spec` | the same story |
| ESS-SYNTH-014 reports that `RecordAction/outcome/recorded` has no scenario, and the suite holds a passing scenario of that id | `agent-page-path-spec` | the list of ESS findings to file, which waits for the operator's word |
| a binding that maps a required event field into an optional command input passes `validate`, synthesis and the interpreter, and Rust generation refuses it | the reviser, `agent-page-path-spec` | the same list |
| generated `surf-types` fails `cargo clippy -D warnings` (28 lints) | `agent-page-path-spec`, `runner-surf-cli-package` | already in the backlog (wave 1) |
| wave 2's record said a `Sessions` view cut to its first row would likely pass the suite; planted, it fails 15 scenarios | `agent-page-path-spec` | corrected here; the per-entity test now plants it |
| a `SessionLog` in the wrong order cannot be planted through storage: the generated view sorts | `agent-page-path-spec` | accepted; the authored scenario claims the order |
| `docs/view-stream.md:138` shows a string example with characters left raw that the rule above it escapes | `view-page-prototype` | backlog draft: correct the example; the code follows the rule |
| the example took 1.5 s to print its address under load; the story's case allows 5 s | `view-page-prototype` | noted; the margin is 3 times |
| the page was not exercised in a browser for a reconnect or a `reset` event: the example triggers neither | `view-page-prototype` | `story:view-first-page`, whose browser cases cover both |
| two comment lines moved by the package move are over 100 columns | `runner-surf-cli-package` | accepted: rewrapping would change files the story keeps byte for byte |

## After this wave

The operator decided on 2026-11-21 that surf reaches a checkpoint after wave 4 and that work
then moves to the script engine (its own repository). The checkpoint: `surf-agent open`, `go`,
`look` and `close` against a real `surfd` and a real session process, and a release `0.2.0`
that ships `surf`, `surf-agent`, `surfd` and `surf-session`. The waves after 4 stay planned in
[the wave plan](agent-waves.md) and are not scheduled.
