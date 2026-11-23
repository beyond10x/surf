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
surf v1 — 9 file(s), 37 scenario(s), valid
196 scenario(s) (37 authored), 56 refusal(s), written to .out/suite.json
  196 scenarios: 196 passed, 0 failed, 0 error, 0 unsupported
196 scenario(s) (37 authored), 56 refusal(s), written to .out/suite.json
conform-noticed: conformance/tests/noticed.txt matches its sources
conform-rust: the narrowed suites of 29 planted defects in conformance/target/noticed/6d99491696333b9a70c2b009b42634243f65bbe23190765d981e5e6ab8c09ee1 are current
Summary [   8.819s] 39 tests run: 39 passed, 0 skipped
  196 scenarios: 196 passed, 0 failed, 0 error, 0 unsupported
conformance step: 20.8 s of its budget of 120 s
the interpreter and the Rust system ran 196 scenario(s) each, of suite sha256:dcc7e7c2c534eb6f98ef03cbd07da620efd16fbe16e8d448de697919b0a56bf3
generated/rust matches spec/ (30 artifact(s) and .ess-output)
cli/surf-agent.yaml — valid CLI presentation binding
10 CLI artifact(s), generated in .out/cli
Cargo.lock: 49 package(s), 19 from outside this repository, each on the allow-list
test-layout: 23 test directories, 150 test files, one test binary each
tls-certs: 33 cases, certs/ matches its sources
tls-flights: 6 recordings match SOURCE, the recorder matches DIGEST
tls-flights: self-test: a changed copy of aes128-x25519-ecdsa.bin fails naming it
doc-tests-check: no doc test outside crates/ports
    Finished `dev` profile [unoptimized] target(s) in 9.46s
1233 tests: 1233 passed, 0 failed, 0 ignored (cargo-nextest 1231, doc tests 2)
test step: 38.1 s of its budget of 120 s
545 tests: 545 passed, 0 failed, 0 ignored (cargo-nextest 545)
tls test step: 18.0 s of its budget of 120 s
agent-tasks: 4 tasks, 4 pass, 0 fail, 0 skipped, lane gate, binaries target/debug
sh-echo: Summary [   4.018s] 41 tests run: 41 passed, 112 skipped
view-stream-check: 23 event name(s), each a message of the AsyncAPI projection
view-tests: node_modules installed from package-lock.json
  60 passed (58.5s)
page step: 60.6 s of its budget of 120 s
html-conformance: 9107 cases, 9095 run, 9095 passed
x509-limbo: 5143 cases, 5089 run, 5060 passed
tls-x509-conformance: 15 s
forge-watch-test: 6 case(s), each decided as expected by taskfiles/forge-watch.sh
planning-only-test: 9 case(s), each decided as expected by taskfiles/planning-only.sh
test-layout-test: 39 case(s), each decided as expected by taskfiles/test-layout.sh
sh-echo-test: 2 case(s), each decided as expected by taskfiles/sh-echo.sh
doc-tests-check-test: 14 case(s), each decided as expected by taskfiles/doc-tests-check.sh
four steps: 137.5 s (test step 38.1 s, tls test step 18.0 s, conformance step 20.8 s, page step 60.6 s)
```

It validates the specification, runs the 196 scenarios against the interpreter built into `ess`
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
own lines and the rows of the two conformance counts. It is the run of 2026-11-23 that
`task check-runner` ended in 283 s. The page tests' line, `60 passed (58.5s)`, has its slowest
test (`an entry over 1 MiB`) at 9.7 s; `59 passed (1.9m)` had it at 27.3 s at a load average of
96 to 132; `59 passed (1.6m)` at 16.0 s at a load average of 111 to 117; `59 passed (3.0m)`
took the task 220 s at a load average of 126 to 127, `53 passed` 1.1m at a load average of 55
to 65, `47 passed` 2.5m at a load average of 110 to 140, and the 38 of before took 15.8 s.

Third-party crates are used in two places: the scenario runner in
[`conformance/`](https://github.com/beyond10x/surf/tree/main/conformance), with a lock of its
own, and `surf-tls`, whose `ring` and the packages it needs are the 19 outside packages of the
root lock, each on the allow-list `taskfiles/lock-allow.txt`; the lock check fails on one that
is not. Nothing that ships depends on either.

## Build from source

Building is for working on surf; to use it, [install the binary](try-it.md).

You need a Rust toolchain with `cargo` (surf is built with 1.99.1), [`task`](https://taskfile.dev)
and [`ess`](https://github.com/beyond10x/ess) 0.97.4. `task check` also needs
[`cargo-nextest`](https://nexte.st) 0.9.143 or later.

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
