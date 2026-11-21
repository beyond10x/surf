---
format: aep.planning-md/3
id: story:view-page-memory
kind: story
status: draft
title: The live view's page bounds what it keeps, and adds an entry without building the timeline again
summary: The page keeps every entry of every session it loaded and rebuilds the whole timeline on each entry; found by the adversary of wave 7, not measured.
relations:
- decomposes: epic:live-view
revision: 2
---
## Context

Found by the adversary's pass over `story:view-first-page` in wave 7
(`docs/plan/agent-wave-7.md`), by reading; no test shows it, and the story said nothing about
it.

The page (`crates/view/web/state.js`) keeps every entry of every session whose detail it has
loaded, for as long as the page is open. `surfd` bounds what it keeps per session; the page
has no bound. On each entry of the selected session the page builds the whole timeline again
(`crates/view/web/timeline.js`), so a session with 5,000 actions costs a rebuild of 5,000
segments for each new entry.

Added in wave 8 (`docs/plan/agent-wave-8.md`), from the adversary's pass over
`story:view-detail-panel` and the unit that fixed its findings: each stream entry also builds
every row of the requests list again (`crates/view/web/detail.js`), and a segment with 300
marks makes the timeline about 7,000 px tall, since a segment's marks have no bound of their
own. Both belong to what this story measures and bounds.

## Change

Measured first, decided in this story before code changes:

- how long one entry takes to show with 1,000 and with 5,000 actions held, and how much the
  page holds after a stream of that length, in the headless shell the page tests use;
- a bound on what the page keeps per session and over all sessions (for example the bound
  `surfd` has, with older entries shown as the gap the page already draws for dropped
  entries), and what the playhead does when its action leaves the bound (the rule of wave 7:
  it moves once to the oldest action held);
- whether a new entry adds one segment in place of building the timeline again.

## Acceptance

- A page case streams more entries than the bound to one session and reads that the page
  holds no more than the bound, that the gap counts the rest, and that the playhead follows
  the rule above.
- A page case measures the time from an entry on the stream to its segment with the bound
  full, against a number written in the case.
- `docs/view-stream.md` or the page's own comment states the bound.
