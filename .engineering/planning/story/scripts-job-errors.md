---
format: aep.planning-md/3
id: story:scripts-job-errors
kind: story
status: draft
title: Record a job's throw and an unhandled rejection, as HTML reports them
summary: A promise job that throws and a promise that stays rejected after a script's checkpoint reach the record of the script or the specification's word for it, instead of leaving the script's own end as it was.
relations:
- decomposes: epic:scripts-in-a-page
- depends_on: story:swell-0-9-0
revision: 1
---
## Context

`story:swell-0-9-0` (decision 7) runs the microtask checkpoint after each script and records
neither a job's throw nor an unhandled rejection. `host_clean_up_after_job` and
`host_promise_rejection_tracker` keep swell's defaults, so a script whose job threw, or whose
promise stays rejected, ends as the script itself ended. The design note
(`docs/design/scripts.md`, Q2) names the deviation from HTML's "report an exception"
(`#hostenqueuepromisejob`, step 2.4) and "notify about rejected promises"
(`#the-hostpromiserejectiontracker-implementation`). A script element may spend two entries'
steps, its own and its checkpoint's (HOST-193).

The record of a script is the specification's (`spec/domains/document.yaml`, synthesized
under a bound); a new end word or a count per script changes the specification.

swell `0.10.0` offers reads that may serve, listed as candidates and not as a design:
`PromiseState`, `Agent::promise_state`, `promise_is_handled` and `mark_promise_as_handled`
(`CHANGELOG.md:204-209 @ 0.10.0`); the engine calls `host_promise_rejection_tracker`
(`CHANGELOG.md:130 @ 0.10.0`). A job's throw reaches `host_clean_up_after_job` as `Err(value)`
(HOST-82).

## Change

- The decision between the two forms of decision 7: a job's throw as the script's end, or a
  count per script in the record (a question to swell where the engine does not answer).
- The change of the specification that holds the form, with its scenarios, and the runner's
  writer of the record.
- The note's Q2 states HTML's behaviour and how surf meets it; the deviation sentence goes.

## Acceptance

1. A script `Promise.resolve().then(() => { throw new Error("j"); });` leaves a record that
   names the throw (the form decided), read back through the specification's view.
2. A script that rejects a promise and handles it in the same checkpoint leaves a record with
   no rejection.
3. A script that leaves a promise rejected and unhandled after its checkpoint leaves a record
   that names it; `mark_promise_as_handled` by a later script is not read as a change of it.
4. `git grep -n -F 'host_clean_up_after_job' -- crates/script/src/runner.rs` prints the new
   implementation, no longer swell's default.
5. The scenarios of the specification pass under the synthesis bound; `slot cargo nextest run
   -p surf-script` ends within 120 s, each test within 60 s.

## Out of scope

Generic, timeout and cleanup jobs beyond timers; modules; `new_promise_capability`; timers and
events (`story:scripts-event-loop-and-timers`).
