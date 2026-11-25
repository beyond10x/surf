---
format: aep.planning-md/3
id: story:scripts-seeds-and-time-zone
kind: story
status: draft
title: Run the ignored seeds and time zone cases when a swell tag holds slice 4
summary: The two tests `seeds_fixed` (HOST-26) and `time_zone` (HOST-127), ignored until swell delivers `Math.random` and the time zone, run and pass on the first swell tag whose requirements list is no longer `pending slice 4` for them.
relations:
- decomposes: epic:scripts-in-a-page
- depends_on: story:swell-0-9-0
revision: 1
---
## Context

At swell `0.10.0` HOST-26 and HOST-127 are `pending slice 4` (`conformance/host/requirements.txt`),
and `Math.random` and the named time zone are not delivered. `crates/script/tests/engine.rs` keeps
`seeds_fixed` (`#[ignore = "HOST-26"]`) and `time_zone` (`#[ignore = "HOST-127"]`) ignored with
their ids; `story:swell-0-9-0` un-ignores `clock_advances` (HOST-125) only, because swell `0.10.0`
holds the clock.

This story waits for a swell release: no tag at or before `0.10.0` holds slice 4, and surf
pins tags, never a branch. When a tag holds it, the story is ready; until then it stays a
draft and nothing in surf changes.

## Change

- The pin moves to the first tag whose `requirements.txt` has no `pending slice 4` line for
  HOST-26 and HOST-127, with the lock, the allow-list, the test constants and the tag sentences
  of the pin story's list.
- The two `#[ignore]` lines go; the tests run as written, or are corrected to what the tag
  delivers, each correction named in the report.
- The design note's rows for `Math.random` and the time zone (`docs/design/scripts.md`) are
  rewritten as held, cited at that tag.

## Acceptance

1. `git grep -n -F '#[ignore' -- crates/script/tests/engine.rs` prints nothing.
2. `slot cargo nextest run -p surf-script -E 'test(seeds_fixed) | test(time_zone)'` runs both
   tests and both pass, each under 60 s.
3. `git grep -n -F 'pending slice 4' -- docs/design/scripts.md` prints nothing.
4. The tag in `crates/script/Cargo.toml` is the tag named in the first item of Change, and
   `Cargo.lock`
   still lists 25 external packages or the report names each added package.
5. `slot cargo nextest run -p surf-script` and `-p surf` end within 120 s, each test within 60 s.

## Out of scope

Any change in swell; a page that sets a time zone; the document's clock (HOST-125, taken by
`story:swell-0-9-0`); timers.
