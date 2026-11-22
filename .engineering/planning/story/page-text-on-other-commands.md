---
format: aep.planning-md/3
id: story:page-text-on-other-commands
kind: story
status: draft
title: 'Deferred: --as on follow, back, forward, fill, pick and press'
summary: 'Deferred: the commands that load or change a page take --as tree|markdown and print the text block of the page they left current.'
tags:
- deferred
relations:
- informed_by: epic:agent-cli-target
revision: 1
---
## What

`--as tree|markdown [--from-line <n>]` on `follow`, `back`, `forward`, `fill`, `pick` and
`press`, each of which loads or changes a page: the `text` block of the page the call left
current, as `page-text-look` gives it on `open <url>`, `go <url>` and `look`
(`docs/design/page-as-text.md` section 2). Until then those commands refuse `--as` as usage and
an agent follows them with `look --as <form>`.

## Why deferred

The commands do not exist yet (`agent-links-spec`, `agent-forms-spec`, `agent-history-spec` and
their builds), and one `look --as` after each costs one call; the epic does not promise it.

## What would un-defer it

The six commands built, and a recorded agent session in which the extra `look` after a
`follow` or `press` is a measured share of the calls.

## Acceptance

`cargo test -p surf-session` passes with a case in `crates/session/tests/core.rs` per command:
`follow l1 --as markdown` on a page whose `l1` leads to the `Offers` page of design note section
1 prints `text markdown 3 of 3 lines from 1, 62 of 62 bytes` and its three body lines; each test
within 60 s, the test step within 120 s.
