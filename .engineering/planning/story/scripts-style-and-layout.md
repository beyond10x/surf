---
format: aep.planning-md/3
id: story:scripts-style-and-layout
kind: story
status: draft
title: Scripts and the agent read computed style and layout
summary: CSS parsing, the cascade and a layout good enough for getBoundingClientRect, elementFromPoint and visibility; inert and dialog; the agent's `?` for visibility goes.
relations:
- decomposes: epic:app-journeys-in-surf
- depends_on: story:scripts-document-port
revision: 1
---
## Context

"Anything that needs layout or style: `getComputedStyle`, `offsetWidth`, `style`. surf computes
no style (`spec/domains/render.yaml` marker)"
(`.engineering/planning/epic/scripts-in-a-page.md:144-145`). The agent's answer says so: "`?`
means visibility is not computed; only the `hidden` attribute is known" (`README.md:97`).
`epic:detailed-mode` names computed style in its title and waits for a script engine.
Screenshots are "wanted and not planned in detail yet" and take "CSS, layout, painting and text
rendering, all written here" (`README.md:551-553`).

## What it holds

- A CSS parser, the cascade and computed values; `element.style`, `getComputedStyle`,
  `classList` effects.
- Layout without painting: boxes with positions and sizes, so that `getBoundingClientRect`,
  `offsetWidth`, `elementFromPoint` and "is it visible" have answers that do not vary between
  runs (fixed fonts metrics, a fixed viewport).
- `inert`, `<dialog>` and the top layer, focus order.

## Waits for

`story:scripts-document-port`; decisions of `epic:detailed-mode`. Without it a journey can
assert text in the tree and not what a user would see.
