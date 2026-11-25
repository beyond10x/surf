---
format: aep.planning-md/3
id: story:scripts-external-and-deferred
kind: story
status: draft
title: A page's scripts loaded by src, and defer, async and module scripts, run
summary: A script element with src is fetched as a subresource and run; defer and async scripts run at the end of parsing; module scripts wait for swell's modules.
relations:
- decomposes: epic:app-journeys-in-surf
- depends_on: story:scripts-document-port
revision: 1
---
## Context

`epic:scripts-in-a-page` leaves out external scripts ("a fetch per script is a new request kind
for `surf.net` (subresource, not navigation), its own story and its own security review;
parser-blocking fetches also need the parser pause",
`.engineering/planning/epic/scripts-in-a-page.md:117-119`) and `async`, `defer` and module
scripts (`:113-116`). A bundled application is one or more scripts with `src`, most of them
`defer` or `type=module`. The consumer's study did not name this gap; it is the first one a
bundle meets.

## What it holds

- A subresource request for a classic script: same-origin and cross-origin rules, the response's
  type check, a size bound (the note's 512 KiB per source is for inline text), a time bound, and
  what is recorded with the document when the fetch fails.
- The parser pause for a parser-blocking script, and the end of parsing with its deferred
  scripts (`docs/design/html-parser.md` item 9).
- Module scripts once swell delivers slice 11 (`README.md:11-12 @ 0.9.0`).

## Waits for

`story:scripts-document-port`; a security review of the new request kind; swell's slice 11 for
modules.
