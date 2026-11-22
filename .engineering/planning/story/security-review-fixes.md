---
format: aep.planning-md/3
id: story:security-review-fixes
kind: story
status: active
title: Four findings of the security review are fixed before release 0.3.0
summary: A refused call no longer carries the session's address, a redirect from a public address into a loopback or private network is refused at the address dialed, surf-agent checks the runtime directory and the socket before it writes the token, and surfd prints the live view's address with its token only when asked.
relations:
- decomposes: epic:agent-cli-target
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: crates/agent/src/client.rs
- confidence: cited
  path: crates/agent/src/wire.rs
- confidence: cited
  path: crates/fetch/src/lib.rs
- confidence: cited
  path: crates/session/src/lib.rs
- confidence: cited
  path: crates/session/src/process.rs
- confidence: cited
  path: crates/surfd/src/daemon.rs
- confidence: cited
  path: crates/surfd/src/directory.rs
- confidence: cited
  path: crates/surfd/src/main.rs
- confidence: cited
  path: spec/domains/agent.yaml
- confidence: cited
  path: spec/domains/net.yaml
revision: 14
transitions:
- {from: "draft", to: "proposed", at: "2026-11-22T04:05:32Z", actor: "human:operator", revision: 13}
- {from: "proposed", to: "active", at: "2026-11-22T04:05:32Z", actor: "human:operator", revision: 14}
---
## Context

A security review of surf at `ad8d0e5` and at the tag `0.2.0`, made on 2026-11-22 by reading the
source, found four things in the agent binaries and named three more boundaries. The operator,
2026-11-22: "handle". This story holds the four that are fixed before release `0.3.0`; the
others are named under "Out of scope" with where they go.

1. A refused call still carries the address. The session process refuses a request whose token
   is wrong, and its reply record still holds the session's current address
   (`crates/session/src/lib.rs`, the refusal path and the reply's metadata). `surfd` drops the
   field before it answers a client, so a reader needs the session's own socket: another process
   of the same user. An address can hold a secret in its query or fragment. Both `0.2.0` and
   `main`.
2. A redirect can lead into a private network. Since wave 7 a `go` follows redirects. A public
   site the agent visits chooses the next address, and surf resolves and connects to it with no
   rule on the address it dials (`crates/fetch/src/lib.rs`, the connect and the redirect loop;
   `crates/session/src/lib.rs`, where the session fetches). A page on the public web can so send
   a `GET` to a loopback or private service the machine reaches. `main` only; `0.2.0` returns the
   redirect to the caller. Going to a local address on purpose stays a use of surf.
3. The client can hand the token to a listener it did not check. `surf-agent` encodes the token,
   checks the length and form of the socket path, connects and writes
   (`crates/agent/src/client.rs`, `crates/agent/src/wire.rs`). It does not check the owner and
   mode of the runtime directory or of surf's directory in it. `surfd` checks its directory when
   it starts (`crates/surfd/src/directory.rs`), which is not run when the socket exists. With a
   runtime directory another user can write, the token goes to that user's listener. Both
   versions.
4. `surfd` prints the live view's address with its token on standard error whenever it is
   started by hand (`crates/surfd/src/main.rs`). Whoever reads that log can read every session
   while the daemon lives. `surf-agent` starts `surfd` with standard error closed, and the file
   `view-url` has mode 0600. `main` only.

## Change

- The session process answers a request it refuses for its token, and one it cannot parse, with
  the refusal alone: no address, no title, no state of the session beyond what the refusal needs.
  `surfd` keeps what it knew of the session before the refused call.
- The network component has a destination rule, applied to the address it dials, for the first
  request and for every redirect: a request that follows a redirect is refused when its
  destination is a loopback, private, link-local, unique-local, shared (100.64.0.0/10) or
  unspecified address and the first request of the chain went to none of these. The name is
  resolved once per request and the checked address is the one connected to. A chain that
  starts at such an address may stay among them (a local fixture redirects to itself). The
  refusal is a failure of the request with a reason of its own in the specification, shown in
  the answer as `page failed <reason>`.
- `surf-agent` checks, before it connects with a token, that the runtime directory and surf's
  directory in it are directories (no symbolic link), owned by the caller, with no access for
  group and others, and that the socket is owned by the caller; a failed check exits with the
  code of a start failure and names the path and what is wrong, and no byte is written. The
  peer's credentials are checked too if the standard library offers that without unsafe code;
  if it does not, the story says so.
- `surfd` started by hand prints where the address of the live view is kept (the path of
  `view-url`), without the token. An option prints the address with its token for a person who
  asks for it.
- The specification, the scenarios, `docs/` and the changelog follow; the changelog names the
  four under a heading for security.

## Acceptance

- A request with a wrong token on the session's own socket gets a reply in which the session's
  address does not occur, in both forms of a refusal (wrong token, unreadable request); a call
  with the right token after it gets the page as before. A test on the reply's bytes.
- With a resolver and a transport of the test: a chain that starts at a public address and is
  redirected to 127.0.0.1, 10.0.0.1, 169.254.169.254, `[::1]` and `[fc00::1]` fails with the new
  reason and connects to none of them; the same with a name that resolves to a public address
  for the first request and to a private one for the redirect; a chain from 127.0.0.1 to
  127.0.0.1 is followed; a first request to 127.0.0.1 is made.
- `surf-agent look` with a runtime directory of mode 0755, with surf's directory owned
  correctly but reached through a symbolic link, and with a socket path that is no socket each
  exits non-zero, names the path, and a listener of the test behind that path receives no byte.
- `surfd` started by hand writes no line that holds the view's token; with the option it does.
- The gate (`task check-runner`) passes; the test step stays within its budget.

## Out of scope

- A limit on live sessions and on a session's kept history: `story:agent-surfd-session-limit`,
  accepted, with the review's lines on the kept rows of a session added to it.
- A rule the operator of an agent configures (allow-lists of destinations): a later story.
- The environment and the privileges of the session process before it runs scripts:
  `epic:scripts-in-a-page`.
- The limits of the HTML parser: `story:html-parser-limits`, wave 10.
- A release `0.2.1`: the fixes ship in `0.3.0`.
