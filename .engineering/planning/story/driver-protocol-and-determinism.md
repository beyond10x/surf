---
format: aep.planning-md/3
id: story:driver-protocol-and-determinism
kind: story
status: draft
title: 'A test harness drives surf: a protocol, a virtual clock, seeded randomness, a frame per step'
summary: A decision between a subset of the Chrome DevTools Protocol and surf's own API over a pipe; a clock the driver advances; a seed for Math.random; a text frame (or an image) per step.
relations:
- decomposes: epic:app-journeys-in-surf
- depends_on: story:scripts-event-loop-and-timers
revision: 1
---
## Context

The Chrome DevTools Protocol is "wanted and not planned in detail yet" (`README.md:557`). swell
takes clock, randomness and job scheduling from the host (`README.md:88-90 @ 0.9.0`), so a run
can repeat; surf has no clock a driver can advance, and `Math.random` does not exist at swell
`0.9.0` (HOST-26 `pending slice 4`, `conformance/host/requirements.txt:26 @ 0.9.0`). surf's
evidence per step is text (`look --as tree|markdown`); it makes no image (`README.md:553`).

## What it holds

- The decision: a named subset of the Chrome DevTools Protocol (which domains and methods an
  existing driver calls: init scripts, `Runtime.evaluate`, input, page lifecycle) or surf's own
  commands over a pipe, with what each costs a consumer that has a driver today.
- Init scripts (a script run before the page's first) and `evaluate` (a script run in the page
  by the driver, its result as JSON), both under the document's limits.
- A virtual clock: the driver sets and advances the time timers and `Date` read.
- A seed for `Math.random` and `crypto.getRandomValues` per session.
- A frame per step: the page as text at that instant; an image waits for painting.

## Waits for

`story:scripts-event-loop-and-timers`; swell's slice 4 for `Math.random`; an architecture
decision of the owner on the protocol.
