---
format: aep.planning-md/3
id: story:runner-replay-transport
kind: story
status: active
title: Record and replay the network through the surf transport
summary: A recording transport writes each exchange; a replay transport answers from them by key, never from the network.
relations:
- decomposes: epic:e2e-scenario-runner
- depends_on: story:runner-formats
- depends_on: story:agent-fetch-post
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:13Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:13Z", actor: "human:timo", revision: 4}
---
## Context

The epic records each HTTP request and response of a session and, by default, runs a scenario
against them instead of the network, through the transport the `surf` library takes as a
parameter (`crates/surf/src/lib.rs`, `Transport`, `Answer`; `Browser::run` asks the transport
once per started request that is still `Pending`). Recording and replay are both a `Transport`:
one wraps the live transport and appends what it answered, the other answers from a recording.
`story:runner-decisions` fixes the matching and the response address (D4), the unrecorded request
(Q3) and secrets in bodies (D6); `story:runner-formats` gives the exchange block and its reader
and writer.

Stands on `epic:agent-cli-target` (edges of this story or of `runner-formats`):
`agent-request-body-spec` gives the transport request its method and body, the redirect story of
revision D4 follows redirects inside one request and puts the response address in
`Answer::Response`, and `agent-fetch-post` makes `surf-fetch` send `POST` and turn it into `GET`
on a 303. A redirect is therefore one transport call and one exchange.

## Change

In `crates/scenario` (package `surf-scenario`), which gains a path dependency on `surf`:

- `RecordingTransport<T: Transport>`: passes each request to `T`, returns its answer unchanged,
  and appends an exchange (key, the sequence number of the current action, the answer: response
  with its status, media type, body and response address, failure reason, or unanswered) to a
  recording writer. Before writing, every occurrence of a registered secret in the request and
  response bodies, as typed and `application/x-www-form-urlencoded`-encoded (WHATWG URL Standard,
  from memory, to be confirmed by this story), is replaced by its reference (D6).
- `ReplayTransport`: built from the exchanges of a parsed recording and the supplied variable
  values. A request's key is its method, address and request body (the note, D4). The n-th
  request with a key gets the n-th recorded exchange with that key; before comparing, the body the
  run sends gets the same replacement of secrets by references that the recording transport
  applied, and the results are compared (D6). It is answered with the recorded response including its
  response address. A request with no exchange left is answered
  `Answer::Failure(FailureReason::Unreachable)`, so the request ends through the page load's
  normal failure path (`FailRequest`, then `FailNavigation`) and nothing hangs, and it is reported
  to its owner as unrecorded (method, address), so the run can fail the step (Q3). It holds no socket and no address resolver; the
  unused exchanges are counted.
- `crates/scenario/tests/replay.rs`, each case against a `surf::MemoryBrowser` and a server the
  test starts on `127.0.0.1` with `std::net::TcpListener` for the recording. For every replay the
  test keeps that listener bound and non-blocking and, after the replay, asserts that `accept()`
  answers `WouldBlock`, so a transport that tried the network and fell back to the recording
  fails. The cases:
  1. `replayed_load_matches_the_recorded_one`: a load recorded and replayed gives the same
     `Report::lines()`;
  2. `same_address_answered_in_recorded_order`: two `GET`s of one address answered by the first
     and the second recorded exchange in that order, and two `POST`s to one address with
     different bodies (a wrong password, then the right one) each answered by the exchange of its
     own body, whichever order the replay sends them in;
  3. `recorded_failure_replays_as_the_same_failure`;
  4. `redirected_post_commits_at_the_response_address`: a `POST` the server answers with 303 to
     a page served by `GET`, recorded as one exchange, replayed so that the navigation commits at
     the response address and not at the request address;
  5. `unrecorded_request_answers_unreachable_and_is_reported`: the request ends `Failed` with
     reason `Unreachable`, its navigation ends `Failed`, and the report names its method and
     address;
  6. `unused_exchange_is_counted`;
  7. `secret_in_a_body_is_written_as_its_reference`: the password `s3cret &pass=1`, whose
     form-encoded spelling `s3cret+%26pass%3D1` differs from the typed one; neither spelling is in
     the recording's bytes, `${PASSWORD_1}` is, and the replay matches the request body.

Surfaces: `crates/scenario/src/` (new modules for the two transports, and `lib.rs`),
`crates/scenario/Cargo.toml`, `crates/scenario/tests/replay.rs`. Follows `runner-formats` on
`crates/scenario`; `runner-record-session` follows this story there.

## Acceptance

`cargo test -p surf-scenario` passes with the seven named replay cases, in each of which the
recording server's listener, kept bound through the replay, answers `accept()` with
`WouldBlock` afterwards.

## Out of scope

Wiring either transport into the session process (`runner-record-session`) or into `surf run`
(`runner-run`). Live mode, which is the plain `surf-fetch` transport.

## Domain relations

- An exchange and a `surf.net.Request`, one to one: inferable (inferred from
  `crates/surf/src/lib.rs:473-481`, which asks the transport once per `RequestStarted` whose
  request is still `Pending`; no `ess/1` document declares a transport call).
- An exchange and the action during which it happened, many to one: `story:runner-decisions`,
  D4; no `ess/1` document.
