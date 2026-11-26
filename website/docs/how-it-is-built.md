---
title: How it is built
description: The specification comes first, the components are generated from it, and one gate checks every step.
---

# How it is built

## Specification first

The design lives in [`spec/`](https://github.com/beyond10x/surf/tree/main/spec) as an
[ESS](https://github.com/beyond10x/ess) specification. A behaviour gets its entity, command,
event or binding in the specification, and a green check, before any code for it is written.

`ess` generates the components from the specification into
[`generated/rust/`](https://github.com/beyond10x/surf/tree/main/generated/rust), and generates the
scenarios that hold the code to it. The generated code is never edited by hand. Hand-written
crates under [`crates/`](https://github.com/beyond10x/surf/tree/main/crates) supply what the
generated code leaves open: talking to the network, keeping each component's data, the
command lines and the session process.

## Components

| component | what it does |
|---|---|
| `agent` | the sessions an AI agent opens and drives from the command line |
| `shell` | the windows and tabs the user sees and acts on |
| `navigator` | decides what each tab loads and tracks every load to its end |
| `net` | fetches resources; the only component that talks to the network |
| `document` | parses response bodies into documents |
| `renderer` | styles, lays out and paints a document into a frame (specified, not rendering yet) |

Each component keeps its own data and reads no other component's. Components are coupled only by
the bindings declared in the specification; a value another component needs travels in an event.
Today all six run in one process. One operating-system process per component comes later.

## The gate

One command checks everything:

```console
$ task check
surf v1 — 9 file(s), 57 scenario(s), valid
221 scenario(s) (57 authored), 57 refusal(s), written to .out/suite.json
  221 scenarios: 221 passed, 0 failed, 0 error, 0 unsupported
221 scenario(s) (57 authored), 57 refusal(s), written to .out/suite.json
conform-noticed: conformance/tests/noticed.txt matches its sources
conform-rust: the narrowed suites of 31 planted defects in conformance/target/noticed/bf8e07a7fadd2c930f14f7df5e3dd43e29214ae1869916263ee911b671eee82d are current
Summary [   4.295s] 63 tests run: 63 passed, 0 skipped
  221 scenarios: 221 passed, 0 failed, 0 error, 0 unsupported
conformance step: 7.0 s of its budget of 120 s
the interpreter and the Rust system ran 221 scenario(s) each, of suite sha256:03a9baffc55cf9dba071245f7718a60181adf1641388478333f7084f80fa90f7
generated/rust matches spec/ (30 artifact(s) and .ess-output)
cli/surf-agent.yaml — valid CLI presentation binding
10 CLI artifact(s), generated in .out/cli
Cargo.lock: 58 package(s), 25 from outside this repository, each on the allow-list
test-layout: 26 test directories, 244 test files, one test binary each
tls-certs: 33 cases, certs/ matches its sources
tls-flights: 6 recordings match SOURCE, the recorder matches DIGEST
tls-flights: self-test: a changed copy of aes128-x25519-ecdsa.bin fails naming it
webidl-check: the 18 sources of crates/script/src/bindings.rs match its header
webidl-check: Cargo.lock holds surf-html without dependencies and surf-webidl with surf-html alone, both of the workspace, so it decides no byte of crates/script/src/bindings.rs
webidl-check: self-test: the locks of the nine forms of a dependency outside the workspace fail, each naming its copy
webidl-check: self-test: one changed byte of selection.txt fails naming it
webidl-check: every byte of crates/script/src/bindings.rs matches the sha256 in its header
webidl-check: self-test: a copy of bindings.rs with one byte after the header changed fails naming it
webidl-check: self-test: a copy of bindings.rs with a //! line of the header rewritten fails naming it
webidl-check: self-test: a copy of bindings.rs with an inner attribute inserted before the digest line fails naming it
doc-tests-check: no doc test outside crates/ports
    Finished `dev` profile [unoptimized] target(s) in 0.04s
2003 tests: 2000 passed, 0 failed, 3 ignored (cargo-nextest 2001, doc tests 2)
test step: 17.0 s of its budget of 120 s
552 tests: 552 passed, 0 failed, 0 ignored (cargo-nextest 552)
tls test step: 3.3 s of its budget of 120 s
agent-tasks: 6 tasks, 6 pass, 0 fail, 0 skipped, lane gate, binaries target/debug
sh-echo: Summary [  10.434s] 51 tests run: 51 passed, 155 skipped
view-stream-check: 23 event name(s), each a message of the AsyncAPI projection
view-tests: node_modules installed from package-lock.json
  72 passed (27.6s)
page step: 28.4 s of its budget of 120 s
html-conformance: 9107 cases, 9095 run, 9095 passed
x509-limbo: 5143 cases, 5089 run, 5060 passed
tls-x509-conformance: 2 s
forge-watch-test: 6 case(s), each decided as expected by taskfiles/forge-watch.sh
planning-only-test: 9 case(s), each decided as expected by taskfiles/planning-only.sh
test-layout-test: 39 case(s), each decided as expected by taskfiles/test-layout.sh
sh-echo-test: 2 case(s), each decided as expected by taskfiles/sh-echo.sh
doc-tests-check-test: 14 case(s), each decided as expected by taskfiles/doc-tests-check.sh
four steps: 55.7 s (test step 17.0 s, tls test step 3.3 s, conformance step 7.0 s, page step 28.4 s)
```

It validates the specification, runs the 221 scenarios against the interpreter built into `ess`
and against the Rust code, checks that the generated code still matches the specification and
that every package in the lock file from outside the repository is on its allow-list, then
builds and tests, the TLS crates in a step of their own, checks that every event name of the
live view's stream is an event of the specification, runs the live view's page tests in a
browser (Playwright and Chromium's headless shell), holds the HTML parser's and the X.509
checker's test counts where they are, and last tests the gate's own scripts. The tests run under
cargo-nextest; a test still running after 60 seconds is ended and fails, each of the four timed
steps (the conformance step, the test step, the TLS test step and the page step) fails when it
takes over 120 seconds, and the last line adds the four up. The block leaves out the ten slowest
tests each step also prints, the agent tasks' rows after their counts, npm's and Playwright's
own lines and the rows of the two conformance counts. It is the gate run of 2026-11-26 on
`5664184`, the tip of `story:scripts-dom-reading`, that `task check-runner` ended in 111 s,
export included, at a load of 48.07 at its start and 59.05 at its end; every line and every time
of the block is that run's (2003 tests, 3 of them ignored, and 244 test files, where
`story:html-script-hook`'s tip `58d720b` had 1845 and 230; its conformance step found the
narrowed suites current, where the run on `58d720b` wrote them anew). Before it the block was
the gate run of 2026-11-25 on `58d720b`, which ended in 231 s at a load of 33.20 at its start
and 57.83 at its end, and before that the run of 2026-11-24 on `91f7ff1`, the Web IDL reader's
branch, with the lines of the stories merged after it counted on their tips. The page tests'
line of the run on `5664184`, `72 passed (27.6s)`, has its slowest test (`adversary: an action
streamed while the detail loads after a reset is not lost`) at 3.299 s and `an entry over 1
MiB` at 1.289 s;
the run on `58d720b` had `72 passed (32.7s)`, its slowest test (`reset after a long absence`)
at 3.034 s and `an entry over 1 MiB` at 2.992 s; the block's line before that, `72 passed
(18.3s)`, had its slowest test (`an entry over 1 MiB`) at 2.350 s; the run of 2026-11-24 on
`8fab187` had `71 passed
(12.7s)`, its slowest test (`adversary: an action streamed while the detail loads after a
reset is not lost`) at 1.214 s and `an entry over 1 MiB` at 0.682 s; the run of
2026-11-24 on `eb97549`, the commit of release `0.8.0`, had `71 passed (13.0s)`, the same
slowest test at 1.237 s and `an entry over 1 MiB` at 0.682 s; the run of
2026-11-24 on `b39b77b` had `71 passed (17.3s)`, its slowest test (`page line and three links
after go`) at 1.703 s and `an entry over 1 MiB` at 779 ms; the run of 2026-11-23 at a load
average of 87 to 90 on 20 CPUs had `71 passed (1.4m)`, its slowest test (`reset after a long
absence`) at 5.6 s and `an entry over 1 MiB` at 4.7 s; `60 passed (58.5s)` had `an entry over 1 MiB` as its slowest at
9.7 s; `59 passed (1.9m)` had it at 27.3 s at a load average of 96 to 132; `59 passed (1.6m)`
at 16.0 s at a load average of 111 to 117; `59 passed (3.0m)` took the task 220 s at a load
average of 126 to 127, `53 passed` 1.1m at a load average of 55 to 65, `47 passed` 2.5m at a
load average of 110 to 140, and the 38 of before took 15.8 s.

Third-party crates are used in two places: the scenario runner in
[`conformance/`](https://github.com/beyond10x/surf/tree/main/conformance), with a lock of its
own, and `surf-tls`, whose `ring` and the packages it needs are 19 of the 25 outside packages
of the root lock; the other six are the packages of swell, the organization's script engine,
which `surf-script` alone takes by a git dependency on a release tag. Each is on the
allow-list `taskfiles/lock-allow.txt`; the lock check fails on one that is not. Nothing that ships depends on the scenario runner. Every binary built from `main` is
built with `surf-tls` and so with ring, and so is every binary of every release since `0.7.0`.

## Build from source

Building is for working on surf; to use it, [install the binary](try-it.md).

You need a Rust toolchain with `cargo` (surf is built with 1.99.1), a C compiler (ring compiles
its C and assembly with it: `cc` on Linux, the Xcode command line tools on macOS),
[`task`](https://taskfile.dev) and [`ess`](https://github.com/beyond10x/ess) 0.97.4. `task check`
also needs [`cargo-nextest`](https://nexte.st) 0.9.143 or later.

```console
$ git clone https://github.com/beyond10x/surf
$ cd surf
$ task build
$ task check
```

`task build` builds the four binaries into `target/debug/`, and `task check` runs everything
described above. The page tests in `task check` also need Node and Chromium's headless shell;
with Node there, install the shell once with `npm --prefix view-tests ci`, then
`npx --prefix view-tests playwright install --only-shell chromium`.
