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
generated code leaves open: talking to the network, keeping each component's data, and the
command line.

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
surf v1 — 9 file(s), 5 scenario(s), valid
77 scenario(s) (5 authored), 24 refusal(s), written to .out/suite.json
  77 scenarios: 77 passed, 0 failed, 0 error, 0 unsupported
77 scenario(s) (5 authored), 24 refusal(s), written to .out/suite.json
  77 scenarios: 77 passed, 0 failed, 0 error, 0 unsupported
the interpreter and the Rust system ran 77 scenario(s) each, of suite sha256:60bd619e38e974b0040ce0b1213d8318ee53e0aa7c647339132e559b3f5038af
generated/rust matches spec/ (30 artifact(s) and .ess-output)
cli/surf-agent.yaml — valid CLI presentation binding
Cargo.lock: 18 package(s), none from outside this repository
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.00s
81 test suites: 280 passed, 0 failed, 0 ignored
```

It validates the specification, runs the 77 scenarios against the interpreter built into `ess`
and against the Rust code, checks that the generated code still matches the specification and
that the lock file holds no outside package, then builds and tests.

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

`task build` builds the binary into `target/debug/surf`, and `task check` runs everything
described above.
