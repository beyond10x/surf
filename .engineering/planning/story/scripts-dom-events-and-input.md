---
format: aep.planning-md/3
id: story:scripts-dom-events-and-input
kind: story
status: draft
title: Scripts receive DOM events, and the agent's actions fire them
summary: EventTarget's three members work; Event, focus, keyboard and pointer events are dispatched; fill, pick and press fire the events a browser fires, and press acts on a button only a script handles.
relations:
- decomposes: epic:app-journeys-in-surf
- depends_on: story:scripts-event-loop-and-timers
revision: 1
---
## Context

"Events: `EventTarget`'s three members are generated and throw `NotSupportedError`; event
handlers, `Event` and dispatch need the event loop, and an agent's action firing script is a new
path in `surf.agent`; after this epic"
(`.engineering/planning/epic/scripts-in-a-page.md:123-125`). Today "A button that only a script
would act on is refused" (`README.md:109`).

## What it holds

- `addEventListener`, `removeEventListener`, `dispatchEvent`, `Event` and its subclasses a
  framework reads (`MouseEvent`, `KeyboardEvent`, `InputEvent`, `FocusEvent`), capture and
  bubble, default actions, event handler attributes.
- Focus and blur, `document.activeElement`.
- The agent's `fill`, `pick`, `press` and `follow` fire the event sequence of the HTML and UI
  Events standards before their default action; `press` on a button with a listener runs it.
- The specification's `surf.agent` commands gain what a script's handler can do to an action
  (cancel it, change the tree under it).

## Waits for

`story:scripts-event-loop-and-timers`.
