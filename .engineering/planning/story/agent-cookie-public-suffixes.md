---
format: aep.planning-md/3
id: story:agent-cookie-public-suffixes
kind: story
status: draft
title: 'Cookies across subdomains: the public suffix list bundled as data, domain cookies and same site by registrable domain'
summary: Bundles the Public Suffix List with its date and digest, refreshed before each release; Domain cookies reach sibling hosts unless the domain is a public suffix; same site compares registrable domains.
relations:
- decomposes: epic:agent-cli-target
- depends_on: story:agent-cookie-jar
scope:
- confidence: cited
  path: NOTICE
- confidence: cited
  path: Taskfile.yml
- confidence: cited
  path: crates/cookie
- confidence: cited
  path: crates/fetch/tests
- confidence: cited
  path: docs/releasing.md
- confidence: cited
  path: taskfiles/cookie-psl.sh
- confidence: cited
  path: taskfiles/cookie-psl.yml
revision: 4
---
## Context

Decision U4 of the agent plan, taken by the coordinating agent on 2026-11-22: option A, "the
public suffix list is bundled as data, with its date and digest, refreshed before each release".
`story:agent-cookie-jar` builds the host-only rule first (its R4: every accepted `Domain`
attribute gives a host-only cookie, so a login on `login.example.org` that sets a cookie for
`example.org` is not signed in on `app.example.org`) and defines "same site" as same scheme and
same host (its R9). This story bundles the list and widens both.

Draft 22 of the cookie revision (draft-ietf-httpbis-rfc6265bis-22, read 2026-11-22):

- §5.7 step 9: "If the user agent is configured to reject 'public suffixes' and the
  domain-attribute is a public suffix": when the attribute is identical to the canonical request
  host the cookie becomes host-only, otherwise it is ignored. Step 10 then makes a cookie whose
  attribute the host domain-matches a domain cookie (host-only flag false).
- §5.8.3 step 3: a domain cookie is sent where the host domain-matches its domain and, for a user
  agent that rejects public suffixes, "the cookie's domain is not a public suffix" (the list may
  have changed since the cookie was stored).
- §5.2: two origins are same-site by the "same site" criteria of HTML, which compare the scheme
  and the registrable domain (the public suffix plus one label), and the host itself where there
  is none (an IP address).

The list: the Public Suffix List of Mozilla (https://publicsuffix.org/list/public_suffix_list.dat,
MPL-2.0), with normal, wildcard (`*.ck`) and exception (`!www.ck`) rules and the implied rule `*`
for a top-level label it does not name. The form of bundling is the one `story:tls-anchors` uses
for the anchors (tls-anchors.md, "Change"): the file unchanged, a `SOURCE` with its URL, date,
digest and licence, a refresh task outside the gate, an age check before each release.

## Change

- `crates/cookie/src/public_suffix/public_suffix_list.dat` (the file, unchanged) and `SOURCE`
  (URL, the date fetched, the sha256 of the file, the number of rules, the licence).
- `crates/cookie/src/public_suffix/mod.rs`: the rules read once from the included file;
  `public_suffix(host) -> &str` and `registrable_domain(host) -> Option<&str>` by the list's
  algorithm (the longest matching rule, an exception rule winning, the implied `*`); an IP address
  host has neither.
- `Jar::receive` (`story:agent-cookie-jar`, R4 widened): a `Domain` attribute that the request
  host domain-matches and that is not a public suffix gives a domain cookie (host-only flag
  false), sent to every host that domain-matches it; one that is a public suffix gives a
  host-only cookie when it equals the host and is ignored otherwise (§5.7 steps 9 and 10).
  `cookie_field` leaves out a domain cookie whose domain is a public suffix by the bundled list
  (§5.8.3 step 3).
- `site(initiator, url)` (R9 widened): same-site when there is no initiator, or the scheme is the
  same and the registrable domains are the same; for hosts without one (IP addresses, a host that
  is itself a public suffix) when the hosts are the same.
- `taskfiles/cookie-psl.sh` (POSIX sh) and `taskfiles/cookie-psl.yml`: `psl-refresh` fetches the
  file with `curl`, writes it and `SOURCE`; `FILE=<path>` takes a local file instead; it never
  runs in `check`, `check-runner` or a workflow, and runs without the `slot` prefix. `psl-age`
  (no network) prints `public suffixes: <n> rules of <date>, <d> days old` and fails when the
  list is older than 90 days at `DATE` (default today). The root `Taskfile.yml` include is the
  coordinator's.
- `docs/releasing.md`, "Cutting a release": a step before the tag, `task psl-age` passes, else
  `task psl-refresh` and a commit on `main` first. `NOTICE`: the list and its licence.
- Shared files: `docs/releasing.md` is also written by `story:agent-tasks` (before it, through
  `story:agent-cookie-jar`) and by `story:tls-release` (tls-release.md:145-147; never in the same
  wave); `NOTICE` by `story:tls-release` (same rule); `crates/cookie` and the module `cookies` of
  `crates/fetch/tests/main.rs` by `story:agent-cookie-jar`, before it (an edge). It starts after
  the merge of `story:agent-cookie-jar`.

## Acceptance

`slot task check` exits 0; the folder's bounds hold (the test step within 120 s, no test over 60
s, no model call and no network request in the test step, no Python; the refresh task is outside
the gate and its test uses a local file). Reading the bundled list in a test takes under 1 s (time
in the report). Cases in `crates/cookie/tests/main.rs`, module `public_suffix`, and in the module
`cookies` of `crates/fetch/tests/main.rs` (the test `Network` resolving every name below to
127.0.0.1):

1. The number of rules read equals `SOURCE`, and the sha256 of the included bytes, computed in
   the test as `story:tls-anchors` case 1 computes its own, equals `SOURCE`.
2. `public_suffix`: `a.example.co.uk` gives `co.uk`, `a.b.ck` gives `b.ck` (wildcard), `www.ck`
   gives `ck` (exception), `example.test` gives `test` (implied `*`); `registrable_domain`:
   `example.co.uk`, `a.b.ck`, `www.ck`, `example.test`; `127.0.0.1` and `co.uk` have none.
3. `http://a.example.co.uk:<p>/set` answers `d=1; Domain=example.co.uk; Path=/`:
   `http://b.example.co.uk:<p>/echo` receives `Cookie: d=1`; `http://example.co.uk:<p>/echo`
   receives `Cookie: d=1`; `http://other.co.uk:<p>/echo` has no `Cookie`.
4. `http://a.example.co.uk:<p>/set` answers `ps=1; Domain=co.uk; Path=/`: the cookie is ignored;
   `http://other.co.uk:<p>/echo` and `http://a.example.co.uk:<p>/echo` have no `Cookie`.
5. `http://co.uk:<p>/set` answers `h=1; Domain=co.uk; Path=/`: `http://co.uk:<p>/echo` receives
   `Cookie: h=1`; `http://a.co.uk:<p>/echo` has no `Cookie`.
6. `site(http://a.example.test, http://b.example.test/)` is `Same`; `site(http://a.example.co.uk,
   http://b.other.co.uk/)` is `Cross`; `site(http://127.0.0.1, http://127.0.0.2/)` is `Cross`. On
   the wire: after `http://a.example.test:<p>/set` answered `strict=1; SameSite=Strict; Path=/;
   Domain=example.test`, a `GET http://b.example.test:<p>/echo` with initiator
   `http://a.example.test:<p>` receives `Cookie: strict=1`.
7. A jar holding a domain cookie for `example.test` stored before the list was swapped in the test
   for one that names `example.test` as a suffix: `cookie_field(http://a.example.test/)` leaves it
   out.
8. `psl-refresh` with `FILE` (a test running the script under `CARGO_TARGET_TMPDIR`) writes the
   file and a `SOURCE` naming its sha256; `psl-age` with `DATE` 91 days after the list's date exits
   1 naming the age, with 90 days exits 0.
9. The cases of `story:agent-cookie-jar` that do not involve a `Domain` attribute pass unchanged;
   its W4 now gives `Cookie: p=1; q=1` on `http://b.example.test:<p>/echo` and on
   `http://example.test:<p>/echo` as well, and that case is updated in this story's change.

## Out of scope

- Fetching the list at run time or from a host's system copy; a list per session.
- Partitioned cookies (CHIPS), first-party sets, cookie policies a person sets (§7.2).
- The anchor list and its refresh (`story:tls-anchors`, `story:tls-release`).
