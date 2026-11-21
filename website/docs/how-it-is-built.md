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
surf v1 — 9 file(s), 12 scenario(s), valid
93 scenario(s) (12 authored), 28 refusal(s), written to .out/suite.json
  93 scenarios: 93 passed, 0 failed, 0 error, 0 unsupported
93 scenario(s) (12 authored), 28 refusal(s), written to .out/suite.json
  93 scenarios: 93 passed, 0 failed, 0 error, 0 unsupported
the interpreter and the Rust system ran 93 scenario(s) each, of suite sha256:4cd1257a06a838a1bb2779b0309c347b1b6262703ff814da340988c32ecb7a3a
generated/rust matches spec/ (30 artifact(s) and .ess-output)
cli/surf-agent.yaml — valid CLI presentation binding
Cargo.lock: 22 package(s), none from outside this repository
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.00s
118 test suites: 486 passed, 0 failed, 0 ignored
view-stream-check: 19 event name(s), each a message of the AsyncAPI projection
  24 passed (14.6s)
html-conformance: 9107 cases, 9095 run, 7121 passed
```

It validates the specification, runs the 93 scenarios against the interpreter built into `ess`
and against the Rust code, checks that the generated code still matches the specification and
that the lock file holds no outside package, then builds and tests, checks that every event
name of the live view's stream is an event of the specification, runs the live view's page
tests in a browser (Playwright and Chromium's headless shell), and last holds the HTML parser's
test counts where they are.

The scenario runner in
[`conformance/`](https://github.com/beyond10x/surf/tree/main/conformance) is the one place a
third-party crate is used; nothing that ships depends on it.

## Build from source

Building is for working on surf; to use it, [install the binary](try-it.md).

You need a Rust toolchain with `cargo` (surf is built with 1.99.1), [`task`](https://taskfile.dev)
and [`ess`](https://github.com/beyond10x/ess) 0.96.0.

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
