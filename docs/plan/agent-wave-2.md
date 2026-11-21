# Agent interface plan, wave 2 of 22: sessions

One wave of the [wave plan](agent-waves.md). Four units, each in its own worktree, each
implemented by a sub-agent on Opus; the coordinating session wrote no code. Every commit is the
organization bot's.

## Units

| story | what landed | the unit's own gate | unit commit |
|---|---|---|---|
| `agent-session-spec` | the `agent` component in `spec/`, `generated/rust/crates/agent`, `AgentPorts`, six components in `crates/surf`, the conformance target | the whole gate, exit 0: 77 scenarios, 24 refusals | `679f99e` |
| `agent-cli-client` | `crates/agent`: the `surf-agent` command line, its token and its wire | 29 tests in 5 suites | `4d3545f`, `123fb74` |
| `view-http-server` | `crates/view`: the live view's HTTP server | 33 of 33 cases; 4 planted defects each turn one case red | `f127a44` |
| `html-conformance-harness` | `crates/html` (the parser's interface), `crates/html-conformance` (vendored data, the harness) | 8 tests; `html-conformance: 9107 cases, 9095 run, 5 passed` | `91354b9` |

Each unit's claim was checked by the coordinator before its commit: the three new crates by
running their tests and `cargo tree` again in the unit's tree, the harness also by its total
line and by checksums of the vendored data as committed, `agent-session-spec` by the gate on
this branch. The gates' text scan refused none of the 178 files the four units wrote.

## What went wrong, and what was decided

- **`agent-session-spec` came back red on one number.** The story asked for at least 120
  scenarios; the model gives 77, and the gate is green. The floor was measured on the draft at
  `b48bc87`, whose `OpenSession` response carried the one-time token marker: 148 scenarios, most
  of them the token cells that marker creates. The story's own Change removes the marker
  (decision D21). The unit measured the draft without it at 73 and added no scenario to reach a
  number. The story's acceptance now states 77 scenarios and 24 refusals, with a note that says
  why. A floor in a later story is a measurement to make on the model as it is then, not a
  number carried over from a draft; the wave 3 scopers are asked for every such number.
- **The holder check reads a stored field.** Written against the input
  (`when: session_id != caller.session_id`), ess 0.96.0 synthesis arranged three scenarios with
  the caller of another scenario, and its own interpreter failed them. The model stores `holder`
  on the session and guards with `when_subject: holder != caller.session_id`. Cost: a caller
  that is not the holder learns whether a session exists; the `Sessions` view shows that to
  every opener already.
- **The client's tests depended on the length of the checkout path.** They bound sockets under
  `target/tmp`, 104 of the 107 bytes a socket path may have. The finding went back to the unit:
  the directories are now under `$XDG_RUNTIME_DIR`, the longest path 55 bytes (`123fb74`).
- **Rate limits cut all four implementors once.** Each was resumed on the same model with its
  context; no work was repeated.

## The gate on the wave branch

`slot task check`, exit 0, on `wave/a2-sessions` after the four units, the merge of `main`
(website, release workflow, licence), the regenerated lock files and `REFUSALS_MAX: 24`:

```
surf v1 — 9 file(s), 5 scenario(s), valid
77 scenario(s) (5 authored), 24 refusal(s), written to .out/suite.json
  77 scenarios: 77 passed, 0 failed, 0 error, 0 unsupported
the interpreter and the Rust system ran 77 scenario(s) each, of suite sha256:60bd619e38e974b0040ce0b1213d8318ee53e0aa7c647339132e559b3f5038af
generated/rust matches spec/ (30 artifact(s) and .ess-output)
cli/surf-agent.yaml — valid CLI presentation binding
Cargo.lock: 18 package(s), none from outside this repository
81 test suites: 280 passed, 0 failed, 0 ignored
```

Before the wave: 51 scenarios, 10 refusals, 28 artifacts, 13 packages, 59 suites, 201 passed.

## Findings, and where each goes

| finding | from | goes to |
|---|---|---|
| ess 0.96.0 synthesis sends the arranging act of a scenario with a caller guard as the caller of another scenario | `agent-session-spec` | the list of ESS findings to file, which waits for the operator's word |
| ess 0.96.0: a caller with no `session_id` makes the generated holder check answer an unmet obligation, not `not-holder` | `agent-session-spec` | the same list; a test asserts today's answer |
| `two-of-each-entity.yaml` covers no `Session`: a `Sessions` view cut to its first row would likely pass the suite | `agent-session-spec` | `story:agent-page-path-spec`, which extends that view; the scoper is told |
| `crates/surf/Cargo.toml` is written by `agent-session-spec` (one dependency line) and by `runner-surf-cli-package` | `agent-session-spec` | no collision left: the first is merged before wave 3 starts |
| `UnixStream::peer_cred` is unstable in rustc 1.99.1: `surfd` cannot read the peer's user id with the standard library | `agent-cli-client` | `story:agent-surfd-supervisor`: the directory mode and the token carry that check; the scoper is told |
| the caller's user id comes from `/proc/self`, Linux only: on macOS the token-file owner check refuses | `agent-cli-client` | the story that first ships `surf-agent` in a release; `story:release-binaries` ships `surf` only |
| a token file swapped for a FIFO between the check and the open blocks the open: the standard library has no `O_NONBLOCK` | `agent-cli-client` | backlog draft, minor |
| a local client can hold all 16 connections for 5 s each and get 503 answered to everyone else | `view-http-server` | accepted: the limits of the story's item 8 imply it, and the view listens on loopback |
| the source's contract is stated only in code: `read_from(newest())` never answers `TooOld`, an item holds no CR or LF, `Reader::next(wait)` returns within `wait` | `view-http-server`, the view decomposer | `story:view-page-prototype` states them in its Context; `story:view-surfd-serves` reads them there |
| `AtomicUsize::fetch_update` is deprecated in rustc 1.99.1 and fails under `-D warnings` | `view-http-server` | `AGENTS.md`, as a rule |
| the web-platform-tests data has 1,973 cases, not the 1,870 `docs/design/html-parser.md` gave; the total 9,107 was right | `html-conformance-harness` | corrected in the note in this wave |
| `every_case_has_exactly_one_tag` can only check names and sums: the tag type holds one value | `html-conformance-harness` | `story:html-conformance-gate`, which owns the tag lines |
| the vendored html5lib-tests `LICENSE` holds person names in its copyright line | `html-conformance-harness` | stays: a third party's licence text is copied unchanged; the gates' scan did not refuse it |
| four test files of `crates/surf` are not rustfmt-clean, so `cargo fmt -p surf` rewrites files a unit does not own | `agent-session-spec` | already in the backlog (wave 1); `AGENTS.md` gains the rule to restore them |

## Beside the wave

- **The first release.** `0.1.0` was tagged on `c008179` and released by the organization bot on
  2026-11-21: four archives (Linux x86_64 and aarch64, statically linked; macOS x86_64 and
  aarch64) and `SHA256SUMS`. The install commands of the README were run as written against the
  release, and the downloaded binary loaded a local page with exit status 0. `scan-artifact`
  passed both Linux binaries; it reads ELF only, so the macOS binaries are covered by the
  workflow's own path check.
- **Licence.** Apache-2.0 with a `NOTICE` that asks users to name beyond10x; the `NOTICE` lists
  the vendored test data of `crates/url` and `crates/html-conformance`.
- **A prototype of the view's page comes early.** `story:view-page-prototype` (wave 3) puts the
  first version of the page and an example program with scripted sessions on the HTTP server
  of this wave, so that the page can be opened in a browser four waves before `surfd` serves
  real sessions. `story:view-first-page` now completes those files instead of creating them, and
  `story:view-surfd-serves` is ordered after the prototype because both write
  `crates/view/src/lib.rs`.
- **The script engine** is its own repository, `swell`, local and not yet published; its first
  design document is being written.
