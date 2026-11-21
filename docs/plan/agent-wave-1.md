# Agent interface plan, wave 1 of 22: foundations

One wave of the [wave plan](agent-waves.md). Seven units, each in its own worktree, each
implemented by a sub-agent on Opus; the coordinating session wrote no code.

## Units

| story | what landed | the unit's own gate | unit commit |
|---|---|---|---|
| `agent-secret-primitives` | `crates/secret`: random source, comparison in constant time, token | 13 of 13 cases | `45c7a3a` |
| `agent-answer-format` | `crates/answer`, `docs/agent-answer.md`: renderer, `parse`, `unquote` | 38 tests; 50 of 50 forged values read back equal | `83f6625` |
| `agent-url-parser` | `crates/url`: WHATWG URL parser | `urltestdata: 904 cases, 872 run, 872 passed, 32 skipped` | `4341079` |
| `agent-cli-spec` | `spec/domains/agent.yaml`, `cli/surf-agent.yaml`, `taskfiles/cli.yml` | the whole gate, exit 0, on its second attempt | `42dbca8` |
| `runner-decisions` | `docs/design/scenario-runner.md` | 13 of 13 questions answered, two ESS refusals measured again | `23f2069` |
| `view-decisions` | `docs/design/live-view.md`, `docs/view-stream.md` | 10 items, 6 requests, 3 entry kinds, 3 stream events | `4a54d19` |
| `html-decisions` | `docs/design/html-parser.md` | 81 data files, 9,107 cases, tag lines sum to 9,107 | `d9cb16f` |

Each unit's claim was checked by the coordinator before its commit: the three crates by running
their tests again in the unit's tree, the three notes by reading them against their acceptance,
`agent-cli-spec` by the gate on this branch.

## What went wrong, and what was decided

- **`agent-cli-spec` came back red.** ess 0.96.0 generates for a component with no command and
  no view a port whose `behaviors` field nothing reads; the gate denies warnings. Decision D32
  (`agent-plan-revision-2.md`): the story lands the domain `surf.agent` held by `net`, and
  `story:agent-session-spec` adds the component with its commands. The first attempt is kept on
  the branch `park/agent-sixth-port` (`9e43dcf`). The unit ran `python3 --version` once in its
  first attempt, against the no-Python rule; nothing depends on it.
- **The first gate run on the wave branch failed**: four tests of `conform_totals` copy the
  root `Taskfile.yml` alone into a temporary directory, where the new include did not exist.
  The include is now `optional: true`; `check` still fails when the task is missing.
- **`runner-surf-cli-package` moves from wave 2 to wave 3**: after D32,
  `story:agent-session-spec` may write `crates/surf/Cargo.toml`, which that story edits.

## The gate on the wave branch

`slot task check`, exit 0, with all seven units merged:

```
surf v1 — 9 file(s), 3 scenario(s), valid
51 scenario(s) (3 authored), 10 refusal(s), written to .out/suite.json
  51 scenarios: 51 passed, 0 failed, 0 error, 0 unsupported      (interpreter)
  51 scenarios: 51 passed, 0 failed, 0 error, 0 unsupported      (Rust system)
generated/rust matches spec/ (28 artifact(s) and .ess-output)
cli/surf-agent.yaml — valid CLI presentation binding
Cargo.lock: 13 package(s), none from outside this repository
59 test suites: 201 passed, 0 failed, 0 ignored
```

Before the wave, on `main`: 43 test suites, 138 passed; 10 lock packages; 27 artifacts.

## Findings, and where each goes

For ESS (not filed; filing waits for the operator's word):

- `ess-synth/src/rust/port.rs:158`: the unread `behaviors` field (D32).
- `ess-cli/2`: no environment source for a secret; a `protected` source requires the `stdin`
  and `prompt` flags; constrained types are refused as arguments.
- `ess-http/1`: refused while components are in process (`ESS-HTTP-013`); with two components
  on the network, the authored scenarios are refused (`ESS-AUTHOR-040`) and the suite is not
  written.
- `document` domain: a per-element command cannot refuse an unknown or already-parsed document
  without a hand-written obligation.
- The generated `SessionToken` derives `Debug`, so a secret can reach debug output.

For later stories, routed before wave 2 starts:

- `agent-page-path-spec`, `agent-session-process`: `did` writes its argument quoted and
  optional, as `crates/answer` does.
- `agent-surfd-supervisor`: a session identity is written in lower case.
- `runner-formats`: `parse` takes whole answers; reading single lines of a recording needs an
  entry point per block.
- `html-conformance-harness`: a second data source, `data/wpt/`, with its licence; the `.dat`
  reader handles `#description`; the tree has a processing-instruction node.
- `html-tokenizer-markup`: the processing-instruction states; 52 tokenizer tests need them.
- `html-input-stream`, `html-tree-body`: seven encoding cases need a `meta` handled during tree
  construction, so `tag encoding n/n/n` cannot pass before `html-tree-body`.
- `html-tree-complete`: `tests13.dat` does not exist; the `not-run` tag cannot reach `n/n/n`.
- `html-conformance-gate`: two sections have no data file.
- `html-actionable-elements`: "hidden" follows the note's definition, not the attribute alone.
- `html-character-references`: the generated table carries the standard's licence notice.
- `view-http-server`: `Date` and `WWW-Authenticate` in its answers; `session` events are live
  only. `view-surfd-serves`: U+007F to U+009F are escaped too.
- `agent-session-events`: `NavigationCommitted.body` is a byte field as well.
- `runner-surf-cli-package`: the moved tests' comments name `README.md` for the exit statuses,
  which are now in `docs/surf-command.md`.

Pre-existing, for the backlog: five test files are not rustfmt-clean and clippy fails on the
generated `behaviour.rs`; the gate runs neither.

## Beside the wave

Asked for by the operator while it ran, and landed on this branch: a README for people, with
local badges and a logo; the `surf` command's reference moved to `docs/surf-command.md`; the
survey for making the repository public (`publishing.md`).
