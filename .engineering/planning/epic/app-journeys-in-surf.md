---
format: aep.planning-md/3
id: epic:app-journeys-in-surf
kind: epic
status: draft
title: The test journeys of a scripted single-page application run in surf
summary: 'What surf lacks before a test harness can open a bundled single-page application, click, type, press a key and read visible text without a second browser: ten gaps, each a draft story, found by a consumer''s study of 2026-11-25.'
relations:
- depends_on: epic:scripts-in-a-page
revision: 1
---
## Context

A consumer runs the end-to-end journeys of a single-page application (a bundled React 19
application: `fetch` and long-poll against an HTTP backend, `sessionStorage`, the History API)
in a headless Chrome over the Chrome DevTools Protocol: open a route, click a node, type into
inputs, pick options, press Escape, assert visible text, one image per step. Its study of
2026-11-25 asked what surf would need to take that place, and listed eleven gaps. The
coordinating agent read each against `main` at `9934c89` and swell at `0.9.0`.

What surf is today: no shipped binary runs page JavaScript ("no binary uses `surf-script` yet",
`README.md:254`). `epic:scripts-in-a-page` brings inline classic scripts that run while the page
parses, over a generated DOM, and names what it leaves out
(`.engineering/planning/epic/scripts-in-a-page.md:113-145`): `async`, `defer` and module scripts,
external scripts, timers and the event loop, events, the Web API surface beyond the DOM, and
anything that needs layout or style. The agent's surface is `surf-agent` with handles (`go`,
`look`, `follow`, `fill`, `pick`, `press`, `back`, `forward`, `log`, `close`,
`README.md:91-110`); "A button that only a script would act on is refused" (`README.md:109`);
screenshots and the Chrome DevTools Protocol are "wanted and not planned in detail yet"
(`README.md:551-557`).

What the engine is at swell `0.9.0`: seven of nineteen delivery slices (`README.md:22-33 @
0.9.0`). Not delivered and needed by a bundled React 19: `classes` (slice 8),
`collections-proxy-reflect` (slice 9), `modules` (slice 11), `binary-data` (slice 13, typed
arrays), the rest of `numbers-and-dates` (slice 4: `Math` but `max`, `parseInt`, the `Date`
field getters; `CHANGELOG.md:224-231 @ 0.9.0`; the slice table is
`docs/design/engine.md:3011-3029 @ 0.9.0`). Those are swell's slices, in swell's repository;
this epic holds no story for them and waits for them.

## The gaps and their stories

The study's eleven gaps, each confirmed against the lines above, and one the study did not
name (the first):

| gap | story |
|---|---|
| a script loaded by `src`, `defer`, `async`, `type=module` (a bundle is one) | `story:scripts-external-and-deferred` |
| 1. scripts after load: a task queue, timers | `story:scripts-event-loop-and-timers` |
| 2. the language a bundle uses | swell's slices 4, 8, 9, 11, 13; no story here |
| 3. DOM events, focus, keyboard and pointer events, and the agent's actions firing them | `story:scripts-dom-events-and-input` |
| 4. `fetch`, `Headers`/`Request`/`Response`, streams, `EventSource`, `XMLHttpRequest` | `story:scripts-fetch-from-script` |
| 5. `sessionStorage`, History, `location`, `URL`, `TextEncoder`, `crypto.getRandomValues`, `requestAnimationFrame`; and the HTML element interfaces | `story:scripts-web-platform-basics` |
| 6. computed style and layout | `story:scripts-style-and-layout` |
| 7. selector locators and a key command for the agent | `story:agent-selectors-and-keys` |
| 8. a driver protocol, a virtual clock, seeded randomness, a frame per step | `story:driver-protocol-and-determinism` |
| 9. recorded numbers of a page with scripts | `story:scripted-page-numbers` |
| 10. and 11. a Service Worker host, WebAssembly, IndexedDB | `story:worker-wasm-indexeddb` |

Each story is a draft of the size of an epic and is decomposed before it is proposed. None is
ordered to be built.

## Out of scope

swell's language slices; pixels as a product of their own (`README.md:553`); a decision that
surf takes the place of a second browser for any consumer.
