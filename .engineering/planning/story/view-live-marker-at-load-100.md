---
format: aep.planning-md/3
id: story:view-live-marker-at-load-100
kind: story
status: draft
title: No wait for live fails in ten runs of the page step at a load of 100 or more
summary: 'the claim that story:view-live-marker-under-load could not show: ten runs of the page step that start at a 1-minute load of 100 or more, at the load the host has, each listed, none with a failed wait for live'
relations:
- decomposes: epic:live-view
- informed_by: story:view-live-marker-under-load
revision: 1
---
## Why

`story:view-live-marker-under-load` started from two failed runs of the page step at a start
load of 107 and 121 and asked for 10 runs in a row at a load of 100 or more without a failed
wait for `live`. It found a cause that does not depend on the load (the browser cancels requests
when an address of the host changes) and fixed it: the page tests run in a network namespace of
their own. While it was open, no run of the page step started at a load of 100 or more, and no
load is made for a test. So the claim at that load is not shown. This story holds it.

The two first failures printed no diagnostic. Whether they had the same cause is not known.

## Change

None planned. The story collects runs. If a run fails on a wait for `live`, its diagnostic
(`expectLive` in `view-tests/tests/fixture.mjs`) names what the page did, and the fix is a story
of its own.

## Acceptance

A counted run is a run of `task view-test`, or of a gate that holds it, on `main` at or after the
merge `4386cad`, with a 1-minute load of 100 or more at its start. The load is the one the host
has; none is made.

1. Ten counted runs are listed in this story, each with its commit, its loads at the start and
   the end, its step line and its count of passed and failed tests. None is left out.
2. No listed run has a failed wait for `live`. A run that fails on one is listed with the whole
   text the wait printed, and this story then names the story that takes the cause.
3. A run over the step's 120 s is listed with its seconds; the step's time under load is
   `story:page-step-holds-under-load`'s and fails nothing here.

## Out of scope

Making load. The step's seconds.
