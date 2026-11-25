---
format: aep.planning-md/3
id: story:agent-selectors-and-keys
kind: story
status: draft
title: The agent names an element by a selector and presses a key
summary: surf-agent gains a locator by CSS selector beside handles, and a key command (Escape, Enter, Tab, arrows) that fires the keyboard events.
relations:
- decomposes: epic:app-journeys-in-surf
- depends_on: story:scripts-dom-events-and-input
revision: 1
---
## Context

`surf-agent` names elements by handles the last answer listed (`l3`, `f1`, `b1`,
`README.md:95-97`); it has no selector and no key command (`README.md:91-110`). A test journey
written against a browser driver names elements by selector and presses Escape.

## What it holds

- A locator `--by css:<selector>` (the subset of `story:scripts-dom-selectors`) for `follow`,
  `fill`, `pick`, `press`, with the refusal when it matches none or more than one.
- `key <name>` on the focused element or a handle; the list of key names.
- The specification's `surf.agent` commands and scenarios for both.

## Waits for

`story:scripts-dom-selectors` for the selector subset; `story:scripts-dom-events-and-input` for
the events of a key.
