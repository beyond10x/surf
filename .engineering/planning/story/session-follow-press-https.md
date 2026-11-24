---
format: aep.planning-md/3
id: story:session-follow-press-https
kind: story
status: active
title: 'follow and press load https: targets with the session''s anchors'
summary: 'a session that reached an https: page takes its links and sends its forms: follow and press load https: targets through the session''s trust anchors; every other scheme stays refused'
relations:
- decomposes: epic:tls-https
- depends_on: story:tls-session-anchors
- informed_by: story:tls-session-anchors
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-24T03:45:42Z", actor: "human:operator", revision: 5, decided_on: {"recorded":{"verification":1}}}
- {from: "proposed", to: "active", at: "2026-11-24T03:45:42Z", actor: "human:operator", revision: 6, decided_on: {"recorded":{"verification":1}}}
---
## Why

Since `story:tls-session-anchors` a session loads an `https:` page with `go`. `follow` and
`press` still refuse every `https:` target with `did refused unsupported-scheme`
(`crates/session/src/lib.rs`, "Following a link": "`follow` and `press` refuse `https:`"; the
two returns of `RefusalReason::UnsupportedScheme`, for a link's target and for a form's
action). A page served over `https:` links to `https:` addresses, so an agent that reached such
a page can read it and can take no link and send no form on it.

`epic:tls-https` promises it: "`surf::fetch_http` and the session's transport stop refusing
`https:`", and `surf-agent open|go https://…` "show it as an `http:` page is shown".
`story:tls-session-anchors` names only `go` in its Change and Acceptance, so this part was
claimed by no story.

`crates/session/tests/follow_refusals.rs`
(`an_https_target_is_refused_unsupported_scheme_as_the_documents_say`) holds the sentence
"`follow` and `press` refuse an `https:` target with `did refused unsupported-scheme`" in
`README.md`, `docs/surf-command.md` and `docs/agent-answer.md`; its comment says: "When a
session follows `https:`, this fails, and the sentence leaves the documents with it."

## Placement

After `story:tls-session-anchors` is on `main` and before `story:tls-release`, which depends on
this story: `tls-release` writes the website pages that repeat the refusal
(`website/docs/drive-from-an-agent.md`, lines "`javascript:`, `mailto:` or `https:`
(`unsupported-scheme`)" and "Only `http:` addresses load", and
`website/docs/reference/surf-command.md`). Never beside a unit that writes `crates/session`,
`crates/tls-test-server` or `docs/agent-answer.md` (the path rule of `story:tls-decisions`).

## Change

- `crates/session/src/lib.rs`: the two returns of `RefusalReason::UnsupportedScheme` let an
  `https:` target through, for a link and for a form's action, and the module text of
  "Following a link" and of the mapping of `press` says so. Nothing else changes: a redirect
  after a `follow` already uses the session's anchors, `host_is_local` reads the host alone,
  `back` and `forward` load again with `GET` under the same rule, and the submission step of
  `crates/surf` already takes an `https:` action. Every other scheme stays
  `unsupported-scheme`.
- `crates/tls-test-server/src/conn.rs`: the server keeps the body of a request beside its
  request line (bounded by the request bound it has), so that a test reads what a form sent.
- `docs/agent-answer.md` and `docs/surf-command.md`: the sentence above leaves; the row of
  `unsupported-scheme` names the schemes that remain.
- Tests: module `https` of the test binary of `crates/session`; the cases of `follow_refusals`
  and `adversary_press` that hold the refusal of `https:` today turn into cases that hold the
  load or the remaining refusals.

## Acceptance

Statements 1 to 6 run on the built `surf-agent`, `surfd` and `surf-session` (or those of
`SURF_BIN_DIR`) in a private `XDG_RUNTIME_DIR`, against `surf-tls-test-server` on 127.0.0.1
port 0 with the case `ok-ecdsa-p256` of `crates/tls-test-certs/certs/`, in a session opened with
`--ca-file <ok-ecdsa-p256 root.pem>` unless the statement says otherwise. The server's pages are
fixed before it binds, so a link or an action to the same server is relative.

1. The page `https://localhost:<p>/` holds `<a href="/next">`: `follow <handle>` answers `page
   200 "text/html" "https://localhost:<p>/next"` (today: `did refused unsupported-scheme`).
2. The page holds a form with `method=post`, the action `/send` and a text field `q`: after
   `fill <handle> "blue"`, `press <handle>` answers `did press "<handle>"` and `page 200
   "text/html" "https://localhost:<p>/send"`; the server's record holds one request with the
   line `POST /send HTTP/1.1` and the body `q=blue`.
3. A second server with the case `untrusted-root` runs on port `<p2>` and is started first; the
   page of the first holds `<a href="https://localhost:<p2>/">`: `follow <handle>` answers `page
   failed CertificateNotTrusted "https://localhost:<p2>/"`, and a `look` after it answers the
   same page line (the page line is that of the last navigation, as after a failed `go`).
4. A session opened without `--ca-file` on an `http:` page (the plain page server of the
   session tests) with a link to `https://localhost:<p>/`: `follow <handle>` answers `page
   failed CertificateNotTrusted "https://localhost:<p>/"` (today: `did refused
   unsupported-scheme`).
5. The page of the TLS server holds `<a href="http://127.0.0.1:<q>/plain">` to the plain page
   server, started first: `follow <handle>` answers `page 200 "text/html"
   "http://127.0.0.1:<q>/plain"`.
6. On the page of the TLS server, links to `javascript:void(0)`, `mailto:a@example.test`,
   `data:text/plain,x`, `file:///etc/hostname` and `ftp://127.0.0.1/x`: `follow` answers `did
   refused unsupported-scheme`, status 1, for each; a form with the action `ftp://127.0.0.1/x`:
   `press` answers the same.
7. On the in-process session of `crates/session/tests/follow_refusals.rs` (its recording
   transport): from the document `http://shop.example/start.html`, a link to
   `https://127.0.0.1/x` answers `did refused forbidden-destination` (today:
   `unsupported-scheme`), and the transport records no second address.
8. `git grep -F 'refuse an `https:` target' -- README.md docs/surf-command.md
   docs/agent-answer.md` prints nothing (the `README.md` sentence is the coordinating role's
   write, in the same merge).
9. The test `an_https_target_is_refused_unsupported_scheme_as_the_documents_say` and the case
   `adversary_press_an_https_action_is_refused_before_a_submission_starts` no longer exist
   under those names; each has a successor that statement 1, 2 or 6 names in its test name.

Budget: each test under 10 s and all new tests under 20 s together (times in the report); no
test over 60 s, the test step within 120 s; 127.0.0.1 only, no network, no model call.

## Out of scope

- A form on an `https:` page whose action is `http:`, and a 307 or 308 that sends a form's body
  again from `https:` to `http:`: both are sent today (`epic:tls-https`: redirects are followed
  "`http:` to `https:` and `https:` to `http:`"), before and after this story.
  `story:session-form-leaves-https` holds the question and the rule.
- Cookies marked `Secure`, HSTS, mixed content inside a page (a session loads no
  sub-resources), client certificates.

## Writes

`crates/session/src/lib.rs`; the modules `https`, `follow_refusals` and `adversary_press` of the
test binary of `crates/session`; `crates/tls-test-server/src/conn.rs`, `src/lib.rs` (the record
`Seen`) and the server's own
tests; `docs/agent-answer.md`, `docs/surf-command.md`. Coordinating role: `README.md`,
`AGENTS.md`. `story:tls-release`, after this story: the two website pages named under
"Placement".
