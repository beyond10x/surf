---
format: aep.planning-md/3
id: story:view-tests-need-a-font
kind: story
status: draft
title: The page step fails at once on a host without fonts
summary: on a build host without fonts 13 page tests ran into their 60 s timeout and the step took 439.3 s; the setup of the page tests checks for a font before any browser starts and fails with one line
relations:
- decomposes: epic:live-view
- informed_by: story:view-live-marker-under-load
revision: 1
---
## Why

A gate run of the page step on a build host without fonts (75c888c, 2026-11-24) ended with 58
of 71 tests passed and 13 tests at "Test timeout of 60000ms exceeded", 439.3 s for a step with
a budget of 120 s. Inside the job `fc-list | wc -l` printed 0. Each of the 13 tests clicks a
button whose size comes only from its text (`.segment .select` and `.mark-select` in
`crates/view/web/view.css`); Playwright reported "element is not visible" and retried the
click until the test's 60 s ended. With a fontconfig file that named two fonts, the same 13
tests passed in 3.6 s. Inferred, not shown: with no font the text gets no box.

The host was repaired. The run still cost 13 × 60 s and its report named a timeout, not a font.
`view-tests/global-setup.mjs` already fails a run with one line when it is not in a network
namespace of its own.

## Change

To be made exact by the scoper and the critics; the direction:

- `view-tests/global-setup.mjs` checks, before any browser starts, that fontconfig finds at
  least one font, and fails the run with one line that names the check when it finds none.
- `AGENTS.md`: the page step needs fonts on the host, next to what it says about the
  namespace.

## Acceptance

Draft:

1. A page step started with a fontconfig file that names an empty directory ends within 10 s
   with one line that says no font was found, and starts no browser.
2. A page step on a host with fonts prints its result line as before; the check adds under
   1 s (both times in the report).
3. The check starts no process per test.

## Out of scope

Which fonts a host has; making the page's layout the same on every host.

## Writes

`view-tests/global-setup.mjs`, `AGENTS.md` (coordinating role).
