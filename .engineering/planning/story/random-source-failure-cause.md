---
format: aep.planning-md/3
id: story:random-source-failure-cause
kind: story
status: draft
title: Carry the cause when the random source fails
summary: A failing /dev/urandom ends the run with exit 3 and an unmet obligation; the operating system's error is dropped at crates/ports/src/random.rs:71.
tags:
- from-finding
relations:
- informed_by: story:memory-ports
revision: 2
---
## Context

Found in review 1 by the security reviewer (`review-result:review-1-security`, finding S6, a
note, origin `introduced` by `story:memory-ports`).

`crates/ports/src/random.rs:71` maps the random source's `io::Error` to an unmet obligation with
`.map_err(|_| unmet_context(kind))`. The failure reaches the caller: the binary prints the unmet
obligation and exits 3. The cause (for example `ENOENT` for a missing `/dev/urandom`, or `EMFILE`)
does not, because the generated `UnmetObligation` (`generated/rust`, written by `ess`) holds two
`&'static str` fields and nothing else.

The coordinator did not send this to the correction round of review 1. Reason: the binary cannot
reach the source either. The generated port keeps the ports value private
(`generated/rust/crates/shell/src/lib.rs:34`), so the cause can only travel through state shared
between the ports of different components, or through a change in what ESS generates. Both are
design choices and neither is a correction of a line.

## What is open

How the cause of an unmet obligation reaches the person who ran `surf`:

- a diagnostic channel handed to every ports type when the binary builds them (state shared
  between components, for diagnostics only), or
- a request to ESS for an `UnmetObligation` that can carry a cause, or
- leaving it as it is and saying so in `README.md`.

## Acceptance

When `/dev/urandom` cannot be opened or read, `surf <url>` exits 3 and standard error names the
operating system's error, shown by a test that points the source at a path that does not exist.

## Note, 2026-11-21 (decision D25 of `docs/plan/agent-plan-revision-2.md`)

`story:agent-secret-primitives` adds the crate `crates/secret` (`surf-secret`) with a read of the
operating system's random source of its own. `surf-ports` `OsRandom` is not moved onto it in
that plan. Whoever takes this story moves `OsRandom` onto `surf-secret`, so that the repository
has one reader of the random source, and gives the failure its cause there.
