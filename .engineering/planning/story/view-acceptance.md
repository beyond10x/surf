---
format: aep.planning-md/3
id: story:view-acceptance
kind: story
status: active
title: The live view's acceptance case, as named tests in the gate
summary: Seven tests named after the epic's acceptance sentences, three over HTTP and four driving the page, run by slot task check against a surfd the fixture starts.
relations:
- decomposes: epic:live-view
- depends_on: story:view-now-panel
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:14Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:14Z", actor: "human:timo", revision: 4}
---
## Context

`epic:live-view`, Acceptance: "`slot task check` exits 0 on `main`, and in that run named tests
show: with two sessions open, `GET /api/sessions` with the view token lists both and without it
answers 401; a request with another `Host` is refused; `GET /events` delivers, in order, the events
of a `go` issued while the stream is open; the page, driven by the test tooling against a `surfd`
the test starts, lists both sessions, shows the selected session's page line and its links after
a `go` to a served page with three links, moves its "latest" marker when a second `go` happens, and
shows a page title containing `<script>` as text."

This story owns that case: one set of tests named after its sentences, in the gate. The earlier
stories test their own parts; this one runs the epic's sentence as one scenario against the
fixture of `story:view-first-page`, with the panels of `story:view-now-panel`. It needs links
and titles, so `story:agent-follow-links` (through `story:view-now-panel`).

The page tests run in `task view-test` of `story:view-first-page`, with the tooling item 4 of
`docs/design/live-view.md` decides: `@playwright/test` 1.65.0 with Chromium's headless shell,
`npm ci --prefer-offline --no-audit --no-fund`, and no `slot` inside the task (a `slot` inside a
task takes a second job in the pool). The page opens the stream from `newest` of `/api/sessions`
and loads `/api/sessions` again after each reconnect (`docs/view-stream.md`, "The stream"); the
answers the HTTP tests read are the ones that document gives.

## Change

- `crates/view-fixture/pages/acceptance/`: a page with three links and a title holding
  `<script>alert(1)</script>`, and the pages they lead to.
- `crates/view-fixture/tests/acceptance.rs` (the HTTP part, over `std::net::TcpStream`, against
  the fixture started by the test; it uses the binaries `task build` leaves in the target
  directory, which `check` builds before `test`):
  1. `api_sessions_lists_both_with_token_and_401_without`: with the token, `sessions` holds both;
     without it, 401 with `WWW-Authenticate: Bearer` and `{"error":"unauthorized"}`;
  2. `request_with_another_host_is_refused`: 403 with `{"error":"host"}`;
  3. `events_deliver_a_go_in_order`: the stream opened without `Last-Event-ID` (live after the
     newest number), a `go` issued: the stream delivers, in
     the order of decision D19 of `docs/plan/agent-plan-revision-2.md`, the `go`'s action entry,
     its events in the order `/api/sessions/<id>` lists them afterwards between that action entry
     and its answer entry, and the answer entry; each `id:` larger than the one before (D23).
- `view-tests/tests/acceptance.spec.mjs` (the page part, one fixture with two sessions):
  4. `page lists both sessions`;
  5. `page line and three links after go`;
  6. `latest marker moves on second go`;
  7. `script in title is shown as text`: the title's exact text, with no `\x` escape of the link left in it (D26), is on the page line and in the list,
     the number of `script` elements is unchanged, no dialog opens.

Surfaces written: `crates/view-fixture/tests/acceptance.rs`, `crates/view-fixture/pages/acceptance/`,
`view-tests/tests/acceptance.spec.mjs`.

## For AGENTS.md (decision D7)

- "The gate's numbers": the count of page tests `task view-test` runs and that the epic's seven
  acceptance tests are among them and in `task test`.

## Acceptance

`slot task check` exits 0, and in its output `crates/view-fixture/tests/acceptance.rs` reports its
3 tests passed by the names above and `task view-test` reports the 4 tests of
`view-tests/tests/acceptance.spec.mjs` passed by the names above.

## Out of scope

Controls and changes (`story:view-controls-and-changes`, not in the epic's acceptance); anything
the earlier stories already count.
