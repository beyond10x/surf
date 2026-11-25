---
format: aep.planning-md/3
id: story:scripts-fetch-from-script
kind: story
status: draft
title: 'A script fetches: fetch, XMLHttpRequest and EventSource'
summary: fetch with Headers, Request and Response, body streams, XMLHttpRequest and EventSource over surf's own network stack, under the session's cookie jar and a same-origin policy with CORS.
relations:
- decomposes: epic:app-journeys-in-surf
- depends_on: story:scripts-event-loop-and-timers
revision: 1
---
## Context

"The Web API surface beyond the DOM: `fetch`, `XMLHttpRequest`, storage, cookies, `location`,
`history`, `navigator`, `console`, `innerHTML` (fragment parsing from script). Each is a
capability a script gains and is decided on its own later"
(`.engineering/planning/epic/scripts-in-a-page.md:141-143`). surf fetches documents with
`surf-fetch` (http and https, redirects, POST, a cookie jar); no request is made for a script.

## What it holds

- The request kind "from script" in `surf.net`: origin, CORS, credentials mode, the cookie
  jar's rules, bounds on count, size and time per document, and its record in the session's
  log.
- `fetch`, `Headers`, `Request`, `Response`, `AbortController`; a body as text, JSON and a
  stream; a long-poll held open under the wall clock rule of the event loop.
- `XMLHttpRequest` and `EventSource` over the same request kind.
- A security review of a hostile page's requests (a private address, a port scan, a body
  without end).

## Waits for

`story:scripts-event-loop-and-timers`; swell's slice 13 for `ArrayBuffer` bodies.
