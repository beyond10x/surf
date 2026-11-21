---
format: aep.planning-md/3
id: epic:live-view
kind: epic
status: active
title: 'The live view: every session of surfd, semantically, in a browser'
summary: surfd serves a read-only page on 127.0.0.1 that shows each session's page, possible actions, changes and events live, from a specified event stream.
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:08Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:08Z", actor: "human:timo", revision: 4}
---
## Context

The operator, 2026-11-21: "What if there is a surfd which is the surf daemon which is what owns or
maintains the active browser sessions and [the] CLI [...] would just be some thin client over
it and there would also be some web UI showing all the active sessions on that daemon and we're
using real-time events [of] ESS to basically push to a nice UI what's currently going on
semantically, not like on a HTML tag basis. UI wise you could just orient yourself with the SDC
project and the live view of the" (the sentence ends there; read as the live view of the SDC
run inspector, `sdc/ui` in the organization's folder).

The coordinating agent amended the accepted design for it on the same day
(`docs/design/agent-interface.md`, section 3 for `surfd`, section 12 for the live view) and
reported the amendment to the operator as a decision he can overrule. `surfd` itself and the
events the sessions publish to it are stories of `epic:agent-cli-target`
(`story:agent-surfd-supervisor`, `story:agent-session-events`). This epic is the view.

What exists: nothing of it. The reference for the layout is the SDC run inspector: a list on
the left, the subject in the centre, a timeline with a playhead at the bottom, a detail panel on
the right, server-sent events, plain JavaScript without a framework or a build step.

## Outcome

A person opens `http://127.0.0.1:<port>/` with the view token `surfd` printed and sees, live:

- every session `surfd` knows: state, current address and title, time of its last action;
- for the selected session: the page (status, media type, address, title, or the failure and
  its reason), what can be done on it now (the actionable elements of the agent's `now` block,
  grouped by kind, each with handle, label, state or target and visibility), and what the last
  action changed;
- a timeline of the session's actions with the components' events under them, and a playhead:
  the centre shows the session as it was after the selected action; "Latest" follows live;
- a detail panel for the selected action or event, and the session's requests.

It shows what happens in the specification's terms (sessions, navigations, requests, documents,
actions, elements), never a tree of tags.

How it gets there:

- The stream's shape is specified: the specification's events, projected by ESS as an AsyncAPI
  document where ESS can, and the read routes as an `ess-http/1` binding where ESS accepts one;
  what ESS cannot give is specified in `docs/` and the first story says which is which from a
  measurement.
- `surfd` serves `GET /api/sessions`, `GET /api/sessions/<id>`, `GET /events` (server-sent
  events) and the page's files, with a server written here: standard library only, HTTP/1.1,
  `GET` only, a fixed set of routes.
- The page is plain HTML, CSS and JavaScript in the repository, with no framework, no build
  step and nothing fetched from another origin.

## Constraints

- Read-only: no route changes anything, and the view cannot act on a session.
- 127.0.0.1 only; a request whose `Host` is not the address and port `surfd` printed is
  refused; every route that answers with data requires the view token (the page's own files,
  which hold no data, are served without it: the token arrives in the address's fragment, which
  a browser sends to no server); a session's token never reaches the view or the
  stream.
- Page text is untrusted: written as text and never as markup, under a content security policy
  that allows no inline script and no outside origin.
- No third-party crate in `surfd`. The page ships with the binary, so it holds no third-party
  code either. Tests that drive the page may use a browser automation tool as test tooling
  (the SDC inspector uses Playwright); what they use is decided by the first story and nothing
  that ships depends on it.
- No Python anywhere. Specs first.

## Out of scope

- Acting on a session from the view.
- Showing a page's rendering: surf has no layout or painting.
- The detailed mode's lines (`epic:detailed-mode`).
- A view over more than one machine or user.

## Acceptance

`slot task check` exits 0 on `main`, and in that run named tests show: with two sessions open,
`GET /api/sessions` with the view token lists both and without it answers 401; a request with
another `Host` is refused; `GET /events` delivers, in order, the events of a `go` issued while
the stream is open; the page, driven by the test tooling against a `surfd` the test starts,
lists both sessions, shows the selected session's page line and its links after a `go` to a
served page with three links, moves its "latest" marker when a second `go` happens, and shows a
page title containing `<script>` as text.
