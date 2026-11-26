---
format: aep.planning-md/3
id: story:scripts-seeds-and-time-zone
kind: story
status: draft
title: Run the ignored seeds case when a swell tag after 0.12.0 holds Math.random
summary: The test `seeds_fixed` (HOST-26), ignored until swell delivers `Math.random`, runs and passes on the first swell tag after `0.12.0` whose requirements list no longer has HOST-26 pending; `time_zone` (HOST-127) is taken by the pin story for 0.12.0.
relations:
- decomposes: epic:scripts-in-a-page
- depends_on: story:swell-0-9-0
revision: 3
---
## Context

At swell `0.12.0`, the tag that `story:swell-0-12-0` pins, `HOST-127` has a test and
`engine::time_zone` runs, but `HOST-26` is still `pending slice 4`
(`conformance/host/requirements.txt:26 @ 0.12.0`) and `Math.random` is not delivered
(`crates/swell/src/builtins/math.rs:24 @ 0.12.0`: "`random` is a later story of slice 4").
`crates/script/tests/engine.rs` keeps `seeds_fixed` (`#[ignore = "HOST-26"]`) ignored with its
id. `story:swell-0-9-0` un-ignores `clock_advances` (`HOST-125`) and `story:swell-0-12-0`
un-ignores `time_zone` (`HOST-127`), so `seeds_fixed` is the one ignored test of `surf-script`
that is left.

This story waits for a swell release: no tag at or before `0.12.0` holds the seeds, and surf
pins tags, never a branch. When a tag after `0.12.0` has a requirements list without
`pending slice 4` for `HOST-26`, the story is ready; until then it stays a draft and nothing in
surf changes.

## Change

- The pin moves to the first tag after `0.12.0` whose `requirements.txt` has no
  `pending slice 4` line for `HOST-26`, with the lock, the allow-list, the test constants and
  the tag sentences of the pin story's list.
- The one `#[ignore]` line goes; `seeds_fixed` runs as written, or is corrected to what the tag
  delivers, the correction named in the report.
- The design note's row for `Math.random` and the seeds (`docs/design/scripts.md`) is rewritten
  as held, cited at that tag.

## Acceptance

1. `git grep -n -F '#[ignore' -- crates/script/tests/engine.rs` prints nothing.
2. `slot cargo nextest run -p surf-script --test main -E 'test(=engine::seeds_fixed)'` runs one
   test and it passes within 60 s.
3. `git grep -n -F 'pending slice 4' -- docs/design/scripts.md | /usr/bin/grep -E
   'HOST-26([^0-9]|$)'` prints nothing.
4. The tag in `crates/script/Cargo.toml` is the tag named in the first item of Change, and
   `Cargo.lock` still lists the `<m>` external packages that `slot task lock-check` printed at
   this story's base, or the report names each added package.
5. `slot cargo nextest run -p surf-script --test main` and `slot cargo nextest run -p surf`
   end within 120 s, each test within 60 s.

## Out of scope

Any change in swell; a page that sets a time zone; the document's clock (`HOST-125`, taken by
`story:swell-0-9-0`); the time zone case `time_zone` (`HOST-127`, taken by
`story:swell-0-12-0`); timers.
