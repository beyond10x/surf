---
format: aep.planning-md/3
id: story:session-form-leaves-https
kind: story
status: draft
title: 'A form''s values do not leave an https: page for an http: address without a refusal'
summary: 'press on an https: page with an http: action, and a 307 or 308 from an https: action to http:, send the form''s values in clear today; the rule and its refusal reason are to be decided'
relations:
- decomposes: epic:tls-https
- informed_by: story:session-follow-press-https
- depends_on: story:session-follow-press-https
revision: 1
---
## Why

A session that read a page over `https:` sends a form's values in clear text in two cases, and
neither gives the agent a word about it:

- the form's action is an `http:` address: `press` submits to it (only an `https:` action was
  ever refused, and `story:session-follow-press-https` lifts that);
- the action is `https:` and the server answers 307 or 308 with an `http:` address:
  `surf-fetch` follows a redirect from `https:` to `http:` and sends the method and the body
  again (`crates/fetch/src/lib.rs`, the redirect rules at the top of the file).

`epic:tls-https` decided that redirects are followed "`http:` to `https:` and `https:` to
`http:`" and says nothing of a form's values. The plan critics of
`story:session-follow-press-https` (2026-11-24) found that a rule for the first case alone does
not hold what it says, and that no existing refusal reason fits.

## Change

To be decided before this story is proposed; the design critic's recommendation:

- `press` refuses any submission from an `https:` document to an `http:` address, `GET` and
  `POST` alike, before any request, with a new reason `insecure-action`, status 1, declared in
  `RefusalReason` (`crates/answer/src/lib.rs`), the status maps of `crates/surfd/src/daemon.rs`
  and `crates/session/src/lib.rs`, and the table and grammar of `docs/agent-answer.md`.
- `surf-fetch` ends a fetch whose 307 or 308 would send a body again from `https:` to `http:`,
  without sending it, with a failure reason the decision names.

The second item changes a rule of `epic:tls-https` ("Decisions taken") and so needs the
epic's decision changed first.

## Acceptance

Draft, to be made exact with the decision:

1. On the built binaries against `surf-tls-test-server`: a form on an `https:` page with an
   `http://127.0.0.1:<q>/send` action, `method=post` and `method=get`: `press` answers `did
   refused insecure-action`, status 1, and a listener on port `<q>` accepts 0 connections.
2. A form whose `https:` action answers 307 to `http://127.0.0.1:<q>/send`: the listener on
   `<q>` accepts 0 connections, and the answer names the failure.
3. A link from an `https:` page to an `http:` address is followed as before.
4. `docs/agent-answer.md` holds a row for the new reason, and the grammar names it.

## Out of scope

HSTS, cookies marked `Secure`, a warning in place of a refusal.

## Writes

`crates/answer/src/lib.rs`, `crates/surfd/src/daemon.rs`, `crates/session/src/lib.rs`,
`crates/fetch/src/lib.rs`, their tests, `docs/agent-answer.md`, `docs/design/tls.md`.
