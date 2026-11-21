---
format: aep.planning-md/3
id: story:view-surfd-start-cases
kind: story
status: draft
title: Tests for a refused start of the view server, the mode of view-url under another umask, and an unreadable entry
summary: Three things story:view-surfd-serves decided and no test holds; tests in crates/surfd/tests, no change of behaviour unless one shows a defect.
relations:
- decomposes: epic:live-view
revision: 1
---
## Context

From the findings of wave 6 (`docs/plan/agent-wave-6.md`), sent on by the reader of
`story:view-first-page` before wave 7: these belong to `surfd`'s tests and not to the page.

`story:view-surfd-serves` left without a test:

- a start of `surfd` where listening on 127.0.0.1 fails, and one where writing
  `$XDG_RUNTIME_DIR/surf/view-url` fails: the story decided that either refuses the start and
  that no message names the token;
- the mode of `view-url` under a umask other than the test's own (the file is created with
  mode 0600 whatever the umask; setting a umask in a test needs unsafe code or a child
  process);
- an unreadable entry: `Action.sequence`, `Action.command` and `Event.name` are written `null`
  for an entry `surf_answer` refuses, the entry is kept and the detail counts it in
  `unreadable`; no case reaches that branch.

## Change

Tests in `crates/surfd/tests/` and no change of behaviour unless a test shows a defect:

- a start with the view's port taken or the runtime directory not writable is refused, the
  refusal names the address or the path and not the token, `surfd.sock` and the lock are
  released, and the next start serves;
- `view-url` has mode 0600 when `surfd` is started as a child process under umask 0000 and
  under umask 0077;
- a stand-in session answers with an entry `surf_answer` refuses: the stream and the detail
  hold the entry with `null` members and `unreadable` counts it.

## Acceptance

- The cases are in `crates/surfd/tests/`, named by what they hold, and each fails for a
  planted defect the unit reports.
- `slot task check` exits 0.
