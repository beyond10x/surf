---
format: aep.planning-md/3
id: story:agent-go-address-parsed
kind: story
status: draft
title: The address of a go and an open is parsed as a URL before it is fetched
summary: 'A go to an address with a space ends Unreachable: the fetch refuses the byte and nothing passes the address through the URL parser first. Found in wave 8, read in the source.'
relations:
- decomposes: epic:agent-cli-target
revision: 1
---
## Context

Found in wave 8 by the unit of `story:view-detail-panel` (`docs/plan/agent-wave-8.md`): a
page case that went to an address with a space in its query got `FetchFailed` with
`Unreachable` and no `ResponseReceived`. The unit did not trace it. The coordinator read the
source and did not run it:

- `crates/fetch/src/lib.rs`, `Target::parse`, refuses an address that holds a byte outside
  0x21 to 0x7e ("the address holds the byte 0x.., which is not printable ASCII") and ends the
  fetch `Unreachable`;
- `Url::parse` of `crates/url` is called in `crates/fetch` and in `crates/url` only, so the
  address of a `go` or an `open` reaches the fetch as the caller typed it. The URL parser
  would percent-encode the space, as a browser does with an address typed by a person or
  found in a link.

The address of a redirect's `Location` is resolved through the URL parser since wave 7, so
the same address is accepted from a server and refused from the agent.

## Change

Measured first: a `go` to `http://<host>/?a b`, to an address with a tab or a line break
inside, to one with a non-ASCII letter in the path and in the host, each against the page
server of the tests; what the session's log says for each.

Decided in this story before code changes:

- where the address of a `go` and an `open` is parsed (the agent's command, the session, the
  `Navigate` behaviour or the transport), and what the events carry: the address as typed or
  the parsed one;
- what an address the URL parser refuses ends as (`Unreachable` as now, or a failure of its
  own), and whether `spec/` states it;
- whether `Target::parse` keeps its check for callers that hand it bytes.

## Acceptance

- A case sends a `go` to an address with a space in its query and reads the request line the
  server got (`%20`) and a `ResponseReceived` in the session's log.
- A case for an address the URL parser refuses reads the failure the story decided.
- The cases of `crates/fetch` and `crates/session` pass without a change to their
  assertions, or each changed assertion is named with its reason.
