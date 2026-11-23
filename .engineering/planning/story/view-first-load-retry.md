---
format: aep.planning-md/3
id: story:view-first-load-retry
kind: story
status: draft
title: A failed first load of the live view's page, and the text for a 403 that is not about the token
summary: A failed first load stays offline with no retry; the refused token's text is also shown for a refused Host. Found in wave 7, decided first.
relations:
- decomposes: epic:live-view
- informed_by: story:view-live-means-stream-open
revision: 1
---
## Context

Found by the adversary's pass over `story:view-first-page` in wave 7
(`docs/plan/agent-wave-7.md`), by reading, and by the unit that fixed that pass's findings.

1. When the first request of the page fails (the view is not reachable at load, or answers a
   status that is no refusal), the page reads `offline` and asks nothing again
   (`crates/view/web/view.js`). After the page has loaded once, a lost connection is retried
   every 5 s. A person who opens the address a moment before `surfd` listens must reload by
   hand.
2. Since wave 7 a 401 or 403 after the page loaded stops the page, with the text for a
   refused token. The view answers 403 also for a `Host` it does not accept
   (`crates/view/src/http.rs`), so that text is then not what happened. No case covers a 403
   that is not about the token.

## Change

Decided in this story before code changes:

- whether a failed first load is retried as a lost connection is (every 5 s, with the same
  marker), and which failures are not retried (a refusal);
- the text shown for a 403: one text for both refusals that is true for each, or a text of
  its own for a refused `Host`, if the page can tell the two apart from the answer.

## Acceptance

- A page case loads the page while the view does not answer, then lets it answer, and reads
  the session list without a reload, or reads the text that says to reload, as decided.
- A page case answers 403 to a request after the load and reads the decided text; the page
  asks nothing again.
- The page asks no faster than every 5 s in any of these cases.
