---
format: aep.planning-md/3
id: story:net-fetch-failure-reasons
kind: story
status: implemented
title: Specify the three ways a fetch ends without a response
summary: FailureReason gains UnsupportedScheme and MalformedResponse; http is the only scheme fetched in this slice.
relations:
- decomposes: epic:headless-rust-target
scope:
- confidence: cited
  path: AGENTS.md
- confidence: inferred
  path: README.md
- confidence: cited
  path: Taskfile.yml
- confidence: cited
  path: spec/domains/net.yaml
revision: 10
transitions:
- {from: "draft", to: "proposed", at: "2026-11-20T21:27:44Z", actor: "human:timo", revision: 7}
- {from: "proposed", to: "active", at: "2026-11-20T21:27:44Z", actor: "human:timo", revision: 8}
- {from: "active", to: "implemented", at: "2026-11-20T21:31:13Z", actor: "human:timo", revision: 10, decided_on: {"recorded":{"test_result":1,"verification":1}}}
---
## Context

`spec/domains/net.yaml` declares `surf.net.FailureReason` with one variant, `Unreachable`, and an
`UNMAPPED:` marker that leaves open which schemes are fetched. The first target fetches over
plain HTTP only (epic `headless-rust-target`, Out of scope: HTTPS). The transport that will issue
`surf.net.FailRequest` can end without a response in three ways it can tell apart, and the
specification names one of them.

This is a specification change and comes before any code that depends on it (`AGENTS.md`,
"Rules": specs first).

## Change

In `spec/domains/net.yaml`:

- `surf.net.FailureReason` declares three variants, each with a one-line summary:
  `Unreachable` (no connection could be made, or it ended before a response was complete),
  `UnsupportedScheme` (the address has a scheme this slice does not fetch) and
  `MalformedResponse` (bytes arrived that are not an HTTP response).
- The `UNMAPPED:` marker above `entities:` no longer lists schemes as open. A comment says that
  `http` is the only scheme fetched and that every other scheme ends the request with
  `UnsupportedScheme`. Redirects, methods, headers, cookies, caching and TLS stay in the marker.

In `AGENTS.md`, "The gate's numbers": the scenario and refusal counts `task check` prints after
the change, and `REFUSALS_MAX` in `Taskfile.yml` if the refusal count moved.

No authored scenario changes outcome. `spec/scenarios/page-load-fails-without-response.yaml`
keeps `reason: Unreachable`.

## Acceptance

`slot task check` exits 0, and `ess specify inspect` (or the compiled IR) shows
`surf.net.FailureReason` with exactly the variants `Unreachable`, `UnsupportedScheme` and
`MalformedResponse`, each carrying a summary.

## Out of scope

Any Rust code. Any other domain file.

## Scope

Derived 2026-11-20 by `story-scoper`. Every line is **cited** (read from the story or the tree) or
**inferred** (a reading that could be wrong).

- **Primary surface:** `spec/domains/net.yaml` — cited
- **Files:** `spec/domains/net.yaml:29-33` (the `FailureReason` enum) and `spec/domains/net.yaml:35-37` (the `UNMAPPED:` marker above `entities:`) — cited
- **Symbols:** `surf.net.FailureReason`, variants `Unreachable`, `UnsupportedScheme`, `MalformedResponse` — cited
- **Documents:** `AGENTS.md:135-140`, section "The gate's numbers" — cited
- **Also likely:** `Taskfile.yml:6` (`REFUSALS_MAX: 10`) — cited, the story names it but only "if the refusal count moved"
- **Also likely:** `README.md:29-31` — inferred, the sample `task check` output repeats the counts `50 scenario(s) (2 authored), 10 refusal(s)` and goes stale if they move; the story does not name it
- **Confidence:** high — the story names both files, the section and the symbol, and each was found at the cited lines
- **Would collide with:** any unit that edits `spec/domains/net.yaml`, `AGENTS.md` (above all "The gate's numbers") or the `vars:` block of `Taskfile.yml` — cited
- **Safety fact:** nothing in `spec/` branches on a variant of `surf.net.FailureReason`: it is an input at `spec/domains/net.yaml:165-166`, copied to the event at `:184` and `:222-223`, the one binding on `surf.net.FetchFailed` maps only `request_id` (`spec/components.yaml:123-133`), and the only authored use is `reason: Unreachable` (`spec/scenarios/page-load-fails-without-response.yaml:30`), a variant that stays — step 2, unproven — inferred

Confirmed by the implementor, wave 1, merge `ee4c775` (2026-11-20):

- `spec/domains/net.yaml` was the only file changed (14 lines).
- `AGENTS.md`, `Taskfile.yml` and `README.md` were not edited: the counts stayed at 50 scenarios and 10 refusals, so the lines naming them are still correct. The three "Also likely" and "Documents" lines above did not apply.
- Inferred line `README.md:29-31`: confirmed as the place of the counts; no edit needed.
- Inferred safety fact (nothing in `spec/` branches on a variant): confirmed by `grep` and by the unchanged suite.
