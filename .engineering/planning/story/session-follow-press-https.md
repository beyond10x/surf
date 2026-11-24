---
format: aep.planning-md/3
id: story:session-follow-press-https
kind: story
status: draft
title: 'follow and press load https: targets with the session''s anchors'
summary: 'a session that reached an https: page takes its links and sends its forms: follow and press load https: targets through the session''s trust anchors; a form on an https: page is not sent to an http: action'
relations:
- decomposes: epic:tls-https
- depends_on: story:tls-session-anchors
- informed_by: story:tls-session-anchors
revision: 1
---
## Why

Since `story:tls-session-anchors` a session loads an `https:` page with `go`. `follow` and
`press` still refuse every `https:` target with `did refused unsupported-scheme`
(`crates/session/src/lib.rs`, "Following a link": "`follow` and `press` refuse `https:`"; the
two returns of `RefusalReason::UnsupportedScheme` for a link's target and for a form's action).
A page served over `https:` links to `https:` addresses, so an agent that reached such a page
can read it and can take no link and send no form on it. `story:tls-session-anchors` did not
lift the refusal: its text does not ask for it.

`crates/session/tests/follow_refusals.rs`
(`an_https_target_is_refused_unsupported_scheme_as_the_documents_say`) holds the sentence
"`follow` and `press` refuse an `https:` target with `did refused unsupported-scheme`" in
`README.md`, `docs/surf-command.md` and `docs/agent-answer.md`; its comment says: "When a
session follows `https:`, this fails, and the sentence leaves the documents with it."

## Change

- `crates/session/src/lib.rs`: `follow` and `press` take an `https:` target as they take an
  `http:` one: the same bounds (`address-too-long`), the same rule for a local destination from
  a page that is not local (`forbidden-destination`), the load through the session's anchors
  (bundled plus the file of `open --ca-file`). Every other scheme stays `unsupported-scheme`.
- A form on an `https:` page whose action is `http:`: refused before any request with a state
  of its own or an existing one, named in the story's first acceptance statement when the
  design is decided; the filled values of a page read over TLS do not leave in clear text
  without a word of the agent. A link from `https:` to `http:` is followed.
- `docs/agent-answer.md`, `README.md`, `docs/surf-command.md`: the sentence above leaves; the
  row of `unsupported-scheme` names the schemes that remain.
- Tests: module `https` of `crates/session/tests/main.rs`, and the two cases that hold the
  refusal today turn into cases that hold the load.

## Acceptance

Each on the built `surf-agent`, `surfd` and `surf-session` in a private `XDG_RUNTIME_DIR`,
against `surf-tls-test-server` on 127.0.0.1 port 0 with the certificates of
`crates/tls-test-certs/certs/`, a session opened with `--ca-file <root.pem>`:

1. A page at `https://localhost:<port>/` with a link to `https://localhost:<port>/next`:
   `follow <handle>` answers `page 200 "text/html" "https://localhost:<port>/next"`.
2. The same page with a form whose action is `https://localhost:<port>/send`: `fill`, then
   `press <handle>` sends one `POST /send` over TLS with the filled value, and the answer holds
   the page the server answered with.
3. A link on that page to a second server whose certificate the session's anchors do not
   cover: `follow` answers `page failed CertificateNotTrusted "<address>"`, and `back` returns
   to the first page.
4. A session opened without `--ca-file` that reached an `http:` page with a link to
   `https://localhost:<port>/`: `follow` answers `page failed CertificateNotTrusted ...` (the
   target is tried with the session's own anchors, never refused as a scheme).
5. A form on an `https:` page with the action `http://127.0.0.1:<port>/send`: `press` is
   refused with status 1, the server at that port receives no request, and
   `docs/agent-answer.md` names the state.
6. A link on an `https:` page to `http://127.0.0.1:<port>/plain`: `follow` loads it.
7. `javascript:`, `mailto:`, `data:`, `file:` and `ftp:` targets are refused
   `unsupported-scheme` as before, for `follow` and for `press`.
8. The rule for a local destination holds for `https:` as for `http:`: from a document whose
   address is not local by its text, an `https:` target whose host is local is refused
   `forbidden-destination` (the case of `follow_refusals.rs`, with the scheme changed).
9. No one of the three documents holds the sentence "`follow` and `press` refuse an `https:`
   target"; the test that held it is gone with it.

Budget: each test under 10 s and all new tests under 20 s together; no test over 60 s, the test
step within 120 s; 127.0.0.1 only, no network, no model call.

## Out of scope

Cookies marked `Secure`, HSTS, mixed content inside a page (a session loads no sub-resources),
client certificates.

## Writes

`crates/session/src/lib.rs`, the modules `https`, `follow_refusals` and `adversary_press` of the
test binary of `crates/session`, `docs/agent-answer.md`, `README.md`, `docs/surf-command.md`.
