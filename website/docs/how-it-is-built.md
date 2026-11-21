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
| `shell` | the windows and tabs the user sees and acts on |
| `navigator` | decides what each tab loads and tracks every load to its end |
| `net` | fetches resources; the only component that talks to the network |
| `document` | parses response bodies into documents |
| `renderer` | styles, lays out and paints a document into a frame (specified, not rendering yet) |

Each component keeps its own data and reads no other component's. Components are coupled only by
the bindings declared in the specification; a value another component needs travels in an event.
Today all five run in one process. One operating-system process per component comes later.

## The gate

One command checks everything:

```console
$ task check
surf v1 — 9 file(s), 3 scenario(s), valid
51 scenario(s) (3 authored), 10 refusal(s), written to .out/suite.json
  51 scenarios: 51 passed, 0 failed, 0 error, 0 unsupported
51 scenario(s) (3 authored), 10 refusal(s), written to .out/suite.json
  51 scenarios: 51 passed, 0 failed, 0 error, 0 unsupported
the interpreter and the Rust system ran 51 scenario(s) each, of suite sha256:8901028dc08460e1ca139c7fa8611bdfef400597e263fb0ac4f206599d5aaeb5
generated/rust matches spec/ (28 artifact(s) and .ess-output)
cli/surf-agent.yaml — valid CLI presentation binding
Cargo.lock: 13 package(s), none from outside this repository
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.00s
59 test suites: 201 passed, 0 failed, 0 ignored
```

It validates the specification, runs the 51 scenarios against the interpreter built into `ess`
and against the Rust code, checks that the generated code still matches the specification and
that the lock file holds no outside package, then builds and tests.

The scenario runner in
[`conformance/`](https://github.com/beyond10x/surf/tree/main/conformance) is the one place a
third-party crate is used; nothing that ships depends on it.
