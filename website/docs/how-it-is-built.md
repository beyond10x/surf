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
surf v1 — 9 file(s), 44 scenario(s), valid
203 scenario(s) (44 authored), 56 refusal(s), written to .out/suite.json
  203 scenarios: 203 passed, 0 failed, 0 error, 0 unsupported
203 scenario(s) (44 authored), 56 refusal(s), written to .out/suite.json
conform-noticed: conformance/tests/noticed.txt matches its sources
conform-rust: the narrowed suites of 29 planted defects in conformance/target/noticed/3c078a8015b3e1cb30f295ad7e4cee71733675f170dc35bf2bc3d30aef7d592b are current
Summary [   1.685s] 39 tests run: 39 passed, 0 skipped
  203 scenarios: 203 passed, 0 failed, 0 error, 0 unsupported
conformance step: 3.3 s of its budget of 120 s
the interpreter and the Rust system ran 203 scenario(s) each, of suite sha256:27d50358e22d9b7b06eb74922b781b6ee0956c4e4d2ec3ed7504ae41285bbb86
generated/rust matches spec/ (30 artifact(s) and .ess-output)
cli/surf-agent.yaml — valid CLI presentation binding
10 CLI artifact(s), generated in .out/cli
Cargo.lock: 51 package(s), 19 from outside this repository, each on the allow-list
test-layout: 25 test directories, 193 test files, one test binary each
tls-certs: 33 cases, certs/ matches its sources
tls-flights: 6 recordings match SOURCE, the recorder matches DIGEST
tls-flights: self-test: a changed copy of aes128-x25519-ecdsa.bin fails naming it
doc-tests-check: no doc test outside crates/ports
    Finished `dev` profile [unoptimized] target(s) in 1.31s
1504 tests: 1504 passed, 0 failed, 0 ignored (cargo-nextest 1502, doc tests 2)
test step: 11.1 s of its budget of 120 s
552 tests: 552 passed, 0 failed, 0 ignored (cargo-nextest 552)
tls test step: 3.0 s of its budget of 120 s
agent-tasks: 6 tasks, 6 pass, 0 fail, 0 skipped, lane gate, binaries target/debug
sh-echo: Summary [  10.326s] 51 tests run: 51 passed, 115 skipped
view-stream-check: 23 event name(s), each a message of the AsyncAPI projection
view-tests: node_modules installed from package-lock.json
  71 passed (18.3s)
page step: 18.7 s of its budget of 120 s
html-conformance: 9107 cases, 9095 run, 9095 passed
x509-limbo: 5143 cases, 5089 run, 5060 passed
tls-x509-conformance: 1 s
forge-watch-test: 6 case(s), each decided as expected by taskfiles/forge-watch.sh
planning-only-test: 9 case(s), each decided as expected by taskfiles/planning-only.sh
test-layout-test: 39 case(s), each decided as expected by taskfiles/test-layout.sh
sh-echo-test: 2 case(s), each decided as expected by taskfiles/sh-echo.sh
doc-tests-check-test: 14 case(s), each decided as expected by taskfiles/doc-tests-check.sh
four steps: 36.1 s (test step 11.1 s, tls test step 3.0 s, conformance step 3.3 s, page step 18.7 s)
```

It validates the specification, runs the 203 scenarios against the interpreter built into `ess`
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
own lines and the rows of the two conformance counts. It is the run of 2026-11-24 on `91f7ff1`,
the Web IDL reader's branch, that `task check-runner` ended in 67 s, at a load of 34.40 at its
start and 52.88 at its end; its test line counts the reader's tests as they stand after it
(80, where that run had 72). The page tests' line, `71 passed (18.3s)`, has its slowest test
(`an entry over 1 MiB`) at 2.350 s; the run of 2026-11-24 on `8fab187` had `71 passed
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
own, and `surf-tls`, whose `ring` and the packages it needs are the 19 outside packages of the
root lock, each on the allow-list `taskfiles/lock-allow.txt`; the lock check fails on one that
is not. Nothing that ships depends on the scenario runner. Every binary built from `main` is
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
