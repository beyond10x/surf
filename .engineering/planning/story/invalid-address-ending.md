---
format: aep.planning-md/3
id: story:invalid-address-ending
kind: story
status: active
title: Specify how a fetch of an address that is not an address ends
summary: An http address that cannot be split into host, port and target ends as Unreachable today; the specification has no reason for it.
tags:
- from-finding
relations:
- informed_by: story:http-fetch-library
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T03:02:27Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-21T03:02:27Z", actor: "human:timo", revision: 3}
---
## Context

Found in wave 3 by the implementor of `story:http-fetch-library` and reported as a decision it
took, not as a defect of that story.

`surf.net.FailureReason` has three variants: `Unreachable`, `UnsupportedScheme` and
`MalformedResponse` (`spec/domains/net.yaml`). None fits an address that has the `http:` scheme
and still cannot be fetched because it is not an address: no `//`, an empty host, a port that is
not a number, user information, a byte outside printable ASCII. `surf.url.Url` is a `String`
newtype that nothing checks (`spec/domains/url.yaml`, the `UNMAPPED:` marker on it).

`crates/fetch` answers such an address with `Failure::Unreachable` and makes no connection
(commit `2652165`, crate documentation and the tests on address rules). `Unreachable` is
specified as "No connection could be made, or it ended before a response was complete", which is
true of an address nobody can connect to and says nothing about why.

## What is open

Whether the specification gains a fourth reason for an address that is not one, or whether
`surf.url.Url` refuses such a value before a navigation is ever started, which is the `UNMAPPED:`
question of the URL domain (a parser of its own, following the WHATWG URL Standard).

## Acceptance

The specification states how a navigation to an `http:` address that cannot be parsed ends, a
scenario shows it, and `crates/fetch` and the transport report that ending instead of
`Unreachable`.
