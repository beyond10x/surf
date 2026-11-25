---
format: aep.planning-md/3
id: story:scripted-page-numbers
kind: story
status: draft
title: The numbers of a page with scripts are measured and recorded
summary: Cold start, time to a loaded page, memory of a session whose page runs scripts, binary size and process count, each with its command, in the repository.
relations:
- decomposes: epic:app-journeys-in-surf
- depends_on: story:scripts-document-port
revision: 1
---
## Context

Measured today, without scripts: a session reads `VmHWM` 5,608 kB after `open` and 6,260 kB
after six pages, 3 threads (`docs/design/scripts.md:848-852`). Not measured: anything with a
script running, because no binary runs one (`README.md:254`).

## What it holds

- A fixture page with scripts of a stated size, and a measuring task outside the gate (it
  starts without `slot`): cold start of `surfd` and a session, time from `open` to the answer,
  `VmHWM` of the session, the size of the release archive, the processes and threads of a run.
- The numbers in `docs/design/scripts.md` with their commands, and a rule for when they are
  taken again.

## Waits for

`story:scripts-document-port`.
