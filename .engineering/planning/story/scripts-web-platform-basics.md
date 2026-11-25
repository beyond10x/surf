---
format: aep.planning-md/3
id: story:scripts-web-platform-basics
kind: story
status: draft
title: The small Web APIs and the HTML element interfaces a framework reads
summary: sessionStorage, the History API and location, URL and URLSearchParams, TextEncoder and TextDecoder, crypto.getRandomValues, requestAnimationFrame, console, navigator, innerHTML; and HTMLElement with the element interfaces of form controls.
relations:
- decomposes: epic:app-journeys-in-surf
- depends_on: story:scripts-document-port
revision: 1
---
## Context

The same exclusion as for `fetch` (`.engineering/planning/epic/scripts-in-a-page.md:141-143`),
and: "`document.write` and the rest of HTML's `Document`, `Window` beyond `document`, `window`
and `self`, and the element interfaces (`HTMLElement`, `HTMLAnchorElement`, ...): an element's
wrapper is an `Element`, a named deviation" (`:134-135`). A framework sets `input.value`,
`checked`, `selected`, `style` and `className` as properties of element interfaces.

## What it holds

- Per interface a decision of what a script gains: `sessionStorage` (per session and origin, a
  size bound), `history.pushState`/`replaceState`/`popstate` and `location` against the
  session's history (`story:agent-history-spec`), `URL`, `URLSearchParams`, `TextEncoder`,
  `TextDecoder`, `crypto.getRandomValues` over `surf-secret`'s source, `performance.now` and
  `requestAnimationFrame` over the session's clock, `console` into the session's log,
  `navigator`'s fixed values.
- The HTML element interfaces from the vendored HTML IDL blocks
  (`crates/webidl/data/whatwg/html-idl.html`, `story:webidl-bindings`): `HTMLElement` and the
  form controls first; `innerHTML` with the fragment parser.

## Waits for

`story:scripts-document-port`; `story:scripts-event-loop-and-timers` for `popstate` and
`requestAnimationFrame`.
