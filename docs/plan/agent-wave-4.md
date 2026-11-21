# Agent interface plan, wave 4 of 22: a session on real pages

One wave of the [wave plan](agent-waves.md), and the release story after it. Four units, each in
its own worktree, each implemented by a sub-agent on Opus; the coordinating session wrote no
code. Every commit is the organization bot's.

After this wave `surf-agent open`, `go`, `look` and `close` run against a real `surfd` and a
real session process that loads real pages over `http:`. It is the checkpoint the operator set
on 2026-11-21: release `0.2.0` follows, and work then moves to the script engine.

## Units

| story | what landed | the unit's own gate | unit commit |
|---|---|---|---|
| `agent-session-process` | `crates/session`: the session core and the binary `surf-session`; `Browser::forget_delivered` and `held` in `crates/surf`; two additions to `docs/agent-answer.md` | 20 tests, five runs in a row; 20 planted defects, 19 red, the last held by another case | `508ae21`, `e045b43` |
| `agent-redirect-spec` | the response's address and `TooManyRedirects` in `spec/`, `generated/rust`, the ports, the assembly, the conformance target | the whole gate, exit 0: 83 scenarios, 27 refusals; 3 planted defects each caught | `c50ff20` |
| `html-conformance-gate` | `crates/html-conformance/expected`, `--check`, `taskfiles/html-conformance.yml`, the base in `check.yml` | 25 tests; 16 defects planted in the check and 4 on the real data, each failing | `92b115a` |
| `release-agent-binaries` (after the wave, alone) | four binaries in the release workflow, `taskfiles/release-smoke.sh`, versions `0.2.0`, the install and agent pages | the workflow's package steps run locally for one target; 6 defects planted in the smoke script, each failing | `ec9a1c1` |

Each unit's claim was checked by the coordinator before its commit. The session process: its
suite and the daemon's and the client's twice (74 passed each), then the three built binaries
by hand against a local server: `open`, `look`, `go`, a `look` with a wrong token (the head
line and `did refused token`, exit 1), a `go` to a `file:` address (`page failed
UnsupportedScheme`), `close`, and a `look` after it (`did refused closed`); no token was found
on disk afterwards. The redirect model by the gate on this branch. The conformance gate by its
suites and its task. The release unit by its smoke script on the archive it packed and the
artifact scan of the four binaries.

## The stories were measured before the wave

All three wave stories were read against the tree and revised first, as in wave 3. For
`agent-redirect-spec` the reviser drafted the model in a scratch copy: 83 scenarios (9
authored), 27 refusals, 83 of 83 on the interpreter; the unit measured the same numbers on both
runners. For `agent-session-process` the revision took what `surfd` gives a session process and
what the model asks it to issue from the reports of the two wave 3 units, and corrected five
places where a case could not run or could not fail.

Decided before the wave:

- **The `did` line names the command the agent issued; the path lists the actions it became.**
  `open <url>` prints `did open "<url>"` and a path of `1 open`, `2 go "<url>"`.
- **A request with a wrong token gets the head line and the refusal only**: no page, no path,
  no note. So does a second `open` and a request that does not decode.
- **After a redirect the page's address is the address of the final response.** The request
  keeps the address that was asked for, and the path keeps it as the `go` argument.

## What went wrong, and what was decided

- **The session kept every response body for its life.** The browser core kept each published
  event and recorded invocation, up to four copies of a body of up to 16 MiB per `go`. The
  unit reported it as a finding with a patch for a file that was another unit's in the wave;
  it went back to the unit after the merge. The session now forgets what was delivered after
  each answer, and a test holds 0 events, 0 invocations and 0 body bytes over `open` and 20
  `go` commands of a 256 KiB page (with the call removed: 1,048,576 bytes after the first).
- **`docs/agent-answer.md` promised more than the binaries print.** The page line "with its
  title when it has one" and the `note parser:` line belong to the HTML parser, which parses
  nothing yet; the note section named two identifiers that do not exist. The release unit found
  it by comparing the document with the built binaries. The document gained the section "What
  an answer holds today"; the format itself is unchanged.
- **Synthesis only ever sends the first variant of an enum.** Before this wave, mishandling
  `TooManyRedirects` in the conformance target would have passed every scenario. An authored
  scenario now sends it, and the planted defect is caught by it.
- **The disk ran low.** `/` fell to 41 GB free and the build pool starts no build below 50 GB.
  The sessions on the machine were asked to clear their own build output; 61 GB were free
  afterwards. The units of this wave were told that a queued build is not a hung one.
- **The agent that updated the documents ran `python3 --version` once**, against the no-Python
  rule; it reported it itself. Nothing was written and nothing depends on it.
- **Rate limits cut the implementors three times.** Each was resumed on the same model with
  its context.

## The gate on the wave branch

`slot task check`, exit 0, on `wave/a4-session` after the four units, the regenerated lock
files and the two new includes of the root `Taskfile.yml`:

```
surf v1 — 9 file(s), 9 scenario(s), valid
83 scenario(s) (9 authored), 27 refusal(s), written to .out/suite.json
  83 scenarios: 83 passed, 0 failed, 0 error, 0 unsupported
the interpreter and the Rust system ran 83 scenario(s) each, of suite sha256:4d9ad0c31479f50662514ef890cfa8bf01b2eaee94669f5df38269e13feda726
generated/rust matches spec/ (30 artifact(s) and .ess-output)
cli/surf-agent.yaml — valid CLI presentation binding
Cargo.lock: 21 package(s), none from outside this repository
97 test suites: 363 passed, 0 failed, 0 ignored
html-conformance: 9107 cases, 9095 run, 5 passed
```

Before the wave: 81 scenarios, 27 refusals, 20 packages, 88 suites, 324 passed, and no HTML
conformance line.

## Findings, and where each goes

| finding | from | goes to |
|---|---|---|
| a session's rows grow with the number of commands (an action, a navigation, a request and a document per `go`), with no bound | `agent-session-process` | backlog: the model does not say how many navigations a session keeps; `story:agent-history-spec` settles what history holds |
| `forget_delivered` drops the events once the answer is made; the view's event log must read them before that call, or the method must hand back what it takes | `agent-session-process` | `story:agent-session-events` |
| `look` with a wrong token on a closed session answers `did refused closed`: `surfd` answers for a session whose process is gone and keeps no token | `release-agent-binaries` | `story:agent-token-hash` (accepted 2026-11-21), which gives `surfd` something to compare with |
| `surf-agent` alone does not say where it looked for `surfd`; without `surf-session`, `surfd` stays up for its idle time after the failed open | `release-agent-binaries` | backlog, with `story:agent-surfd-idle-exit-race` |
| case 1 of the session cannot see a `close` that leaves the process running, because `surfd` closes its standard input anyway | `agent-session-process` | accepted: case 14 holds that line |
| the nested build of the end-to-end case ignores `--target`; the release path sets `SURF_BIN_DIR` | `agent-session-process` | accepted, noted in the test |
| the conformance crate's tests need `.out/suite.json` in a fresh tree | `agent-session-process` | `AGENTS.md`, as a note |
| synthesis adds no scenario for a new input or a new enum variant | the reviser, `agent-redirect-spec` | recorded here; a new variant gets an authored scenario in the story that adds it |
| a `Location` that does not parse is open in the model (`UNMAPPED:` in `net.yaml`) | `agent-redirect-spec` | `story:agent-fetch-redirects`, with what the transport must report for each case |
| the base comparison of the conformance gate does not cover a section going from complete back to incomplete | `html-conformance-gate` | `story:html-tree-complete`, the first story that marks a section complete |
| the gate names failing cases by count, not by name: 9,000 names are not usable | `html-conformance-gate` | accepted; `--list-failing <tag>` prints them |
| the workflow on GitHub, the aarch64 target and the macOS branch of the smoke script have not run anywhere | `release-agent-binaries` | the manual run of the release workflow on `main`, before the tag |

## After this wave

Release `0.2.0` is cut from `main` by `docs/releasing.md`. Then surf pauses: the waves from 5 on
stay planned in [the wave plan](agent-waves.md) and are not scheduled, and the stories accepted
on 2026-11-21 are listed there behind them. What a later session needs to resume is in
`AGENTS.md`, "Planning and waves".
