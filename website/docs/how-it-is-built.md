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
surf v1 — 9 file(s), 24 scenario(s), valid
169 scenario(s) (24 authored), 48 refusal(s), written to .out/suite.json
  169 scenarios: 169 passed, 0 failed, 0 error, 0 unsupported
169 scenario(s) (24 authored), 48 refusal(s), written to .out/suite.json
  169 scenarios: 169 passed, 0 failed, 0 error, 0 unsupported
the interpreter and the Rust system ran 169 scenario(s) each, of suite sha256:81fd1b7b25c7e95459d58100a7a1b7fb30551921658349aba516bdf4b150459a
generated/rust matches spec/ (30 artifact(s) and .ess-output)
cli/surf-agent.yaml — valid CLI presentation binding
Cargo.lock: 26 package(s), none from outside this repository
    Finished `dev` profile [unoptimized] target(s) in 0.00s
877 tests: 877 passed, 0 failed, 0 ignored (cargo-nextest 875, doc tests 2)
test step: 62.3 s of its budget of 120 s
agent-tasks: 4 tasks, 2 pass, 2 fail, 0 skipped, lane gate, binaries target/debug
sh-echo: Summary [   2.623s] 40 tests run: 40 passed, 54 skipped
view-stream-check: 19 event name(s), each a message of the AsyncAPI projection
  38 passed (15.8s)
html-conformance: 9107 cases, 9095 run, 9095 passed
```

It validates the specification, runs the 169 scenarios against the interpreter built into `ess`
and against the Rust code, checks that the generated code still matches the specification and
that the lock file holds no outside package, then builds and tests, checks that every event
name of the live view's stream is an event of the specification, runs the live view's page
tests in a browser (Playwright and Chromium's headless shell), and last holds the HTML parser's
test counts where they are. The tests run under cargo-nextest; a test still running after 60
seconds is ended and fails, the test step fails when it takes over 120 seconds, and the block
leaves out the ten slowest tests the step also prints and the agent tasks' rows after their
counts.

The scenario runner in
[`conformance/`](https://github.com/beyond10x/surf/tree/main/conformance) is the one place a
third-party crate is used; nothing that ships depends on it.

## Build from source

Building is for working on surf; to use it, [install the binary](try-it.md).

You need a Rust toolchain with `cargo` (surf is built with 1.99.1), [`task`](https://taskfile.dev)
and [`ess`](https://github.com/beyond10x/ess) 0.97.3. `task check` also needs
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
