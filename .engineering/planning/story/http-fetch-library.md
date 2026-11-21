---
format: aep.planning-md/3
id: story:http-fetch-library
kind: story
status: implemented
title: Fetch an http address with the standard library only
summary: surf-fetch returns status, media type and body, or one of three failures, with bounded time and size.
relations:
- decomposes: epic:headless-rust-target
- depends_on: story:generated-rust-workspace
scope:
- confidence: cited
  path: crates/fetch
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-11-20T21:27:44Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"review_outcome":3}}}
- {from: "proposed", to: "active", at: "2026-11-20T21:27:44Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"review_outcome":3}}}
- {from: "active", to: "implemented", at: "2026-11-20T21:58:07Z", actor: "human:timo", revision: 8, decided_on: {"recorded":{"test_result":1,"review_outcome":3,"verification":1}}}
---
## Context

The `net` component's transport has to perform the fetch that `surf.net.Fetch` starts and report
how it ended. Nothing in the generated code talks to the network: that is work outside the model.
The first target fetches `http:` addresses only, with GET, and follows no redirect (epic
`headless-rust-target`, Out of scope).

The standard for the wire format is HTTP/1.1 (RFC 9112, message syntax; RFC 9110, semantics;
both named from memory, the implementor confirms the sections it relies on). surf follows it for
what this crate does and claims no section complete.

The crate joins the root Cargo workspace that story `generated-rust-workspace` creates, which is
why it depends on that story. The root `Cargo.lock` gains an entry for it; in a wave that file is
the coordinator's and this unit's change to it is not merged.

## Change

A library crate `crates/fetch` (package `surf-fetch`), standard library only, no dependency on
any generated crate, with one entry point that takes an address as text and a set of limits, and
returns either a response (status code, media type, body bytes) or one of three failures:

- unsupported scheme: the address does not start with the `http:` scheme;
- unreachable: the host does not resolve, the connection is refused or times out, the server
  stops sending for longer than the read timeout, or the connection ends before the response is
  complete;
- malformed response: bytes arrived that are not an HTTP/1.1 response, or a limit on header or
  body size was exceeded.

What it does:

- splits an `http:` address into host, port (default 80) and request target, and sends
  `GET <target> HTTP/1.1` with `Host` and `Connection: close`;
- reads the status line and headers, then the body by `Content-Length`, by
  `Transfer-Encoding: chunked`, or until the connection closes;
- reports the media type as the type and subtype of `Content-Type` in lower case without
  parameters, which is what `surf.net.MediaType` says a media type is; a response with no
  `Content-Type` has the media type `application/octet-stream`;
- bounds what a server can make it do. The limits are a value the caller passes (connect
  timeout, read timeout, maximum header bytes, maximum body bytes) with documented defaults, so a
  test can set a short timeout. Each exceeded limit is a failure and never a hang or an unbounded
  allocation.

The cases, each against a server the test starts itself on `127.0.0.1` with
`std::net::TcpListener`:

1. a response with `Content-Length`;
2. a chunked response;
3. a response ended by close;
4. a `Content-Type` with parameters and mixed case;
5. no `Content-Type`;
6. a 404 (a response, not a failure);
7. a refused connection (unreachable);
8. a connection closed mid-body (unreachable);
9. a server that accepts and sends nothing, with a read timeout of under one second
   (unreachable, returned in under five seconds);
10. a server that sends the headers and then stalls mid-body, same timeout (unreachable);
11. a non-HTTP reply (malformed response);
12. a header block over the limit (malformed response);
13. a body over the limit, once by `Content-Length` and once by close (malformed response);
14. an address with another scheme (unsupported scheme), with no connection attempted.

The connect timeout is applied with the standard library's connect-with-timeout call and has no
test: no address on the local machine stalls a connect reliably, and a test may not reach
outside the machine. The crate documentation says so.

The crate's documentation lists what it does not do (HTTPS, redirects, other methods, request
bodies, persistent connections, content codings, trailers) so nobody reads it as a complete
HTTP client.

## Acceptance

`cargo test -p surf-fetch` passes with at least one test for each of the 14 numbered cases, each
test named after its case, and `cargo tree -p surf-fetch` shows no dependency.

## Out of scope

Mapping the result onto `surf.net.DeliverResponse` and `surf.net.FailRequest`; that is story
`headless-cli`. URL parsing beyond what an `http:` fetch needs.

## Scope

Derived 2026-11-20 by `story-scoper`. Every line is **cited** (read from the story or the tree) or
**inferred** (a reading that could be wrong).

- **Primary surface:** `crates/fetch` (package `surf-fetch`), a new directory that does not exist yet — cited, story body "Change"
- **Files:** `Cargo.lock` at the repository root gains one entry for `surf-fetch`; the body says it is the coordinator's file in a wave and this unit's change to it is not merged — cited, story body "Context"
- **Files:** `crates/fetch/Cargo.toml`, `crates/fetch/src/lib.rs` — inferred, the standard layout of a library crate; the body names only the directory and the package
- **Also likely:** `crates/fetch/tests/` for the 14 server-backed cases — inferred, they may equally be a test module inside `src/lib.rs`
- **Symbols:** `surf.net.MediaType` at `spec/domains/net.yaml:21`, read as the definition of a media type and not changed — cited
- **Symbols:** `std::net::TcpListener` in the tests — cited, story body
- **Symbols:** `std::net::TcpStream::connect_timeout` for the connect timeout — inferred from "the standard library's connect-with-timeout call"
- **Documents:** none outside the crate; the limits, the untested connect timeout and the not-done list go in the crate's own documentation, and the body names no `AGENTS.md`, `README.md` or `Taskfile.yml` change — cited, whole body read
- **Confidence:** high — the body names the crate path, the package and the one shared file; only the file layout inside the crate is inferred
- **Would collide with:** any unit touching `crates/fetch`; any unit that adds, removes or renames a crate in the root workspace, through the root `Cargo.lock` only; any unit that creates or restructures the root `Cargo.toml`, which must land first — cited, story body and `story:generated-rust-workspace` body
- **Safety fact:** the root workspace's members are the glob `crates/*`, so creating `crates/fetch` edits no existing file except `Cargo.lock`, and `surf-fetch` depends on no generated crate, so a regeneration of `generated/rust` cannot break it — step 1 (stated in `story:generated-rust-workspace`, "Change"; the root `Cargo.toml` does not exist yet), unproven

Coordinator's note, 2026-11-20: `Cargo.lock` is not written as a typed scope entry. The bodies
of this story and of `story:memory-ports` make it the coordinator's file, regenerated on the
integration branch, so recording it would have the wave computation keep apart two units that
edit no common file.

Confirmed by the implementor, wave 3, unit `2652165`, gated at `aa52c47` (2026-11-20):

- Landed on `crates/fetch/Cargo.toml`, `crates/fetch/src/lib.rs` (1,025 lines) and `crates/fetch/tests/fetch.rs` (522 lines). Nothing else.
- Inferred file layout: confirmed. The 14 cases are in `tests/fetch.rs`; parser edge cases are a unit-test module in `src/lib.rs`.
- Inferred `TcpStream::connect_timeout`: confirmed, once per resolved address.
- Safety fact (glob `crates/*`, no generated dependency): confirmed; `cargo tree -p surf-fetch` prints the crate alone.
- `Cargo.lock` and `crates/.gitkeep` were written by the coordinator in `aa52c47`, not by the unit.
