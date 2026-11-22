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
- depends_on: story:tls-release
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
revision: 5
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
for a top-level label it does not name. Read 2026-11-22 (version line `2026-11-21_04-33-45_UTC`,
16,763 lines): it writes internationalised suffixes in Unicode, such as `中国` (line 6514, whose
comment at line 6511 gives `xn--fiqs8s`) and `公司.cn` (line 820; the comment at line 10858 gives
`xn--55qx5d` for `公司`). surf's hosts are ASCII only: surf has no IDNA (crates/fetch/src/lib.rs:78).
The form of bundling follows `story:tls-anchors` (tls-anchors.md, "Change"): the file unchanged, a
`SOURCE`, a refresh task outside the gate, an age check before each release.

## Decisions taken

Taken by the coordinating agent on 2026-11-22, reversible, reported to the operator:

1. U4 option A, built here; the host-only rule of `story:agent-cookie-jar` stays until it lands.
2. Round 2 of the panel: a rule with a non-ASCII label is converted label by label to its `xn--`
   form by Punycode (RFC 3492), written in Rust in `crates/cookie`; no IDNA mapping (no case
   folding, no normalisation, no check of the label's characters), and the crate documentation
   says so. A host of surf is compared with the converted rules as it is.
3. A test hands a jar another list through a constructor; the default constructor reads the
   included file.
4. `psl-refresh` takes its destination directory as an argument.
5. The digest of the committed list is checked by a step of the gate with `sha256sum -c` (POSIX
   sh); no Rust hash.
6. Placement: after the merges of `story:agent-cookie-jar` and `story:tls-release` (edges); the
   later writer of `docs/releasing.md` and `NOTICE` (tls-release.md, "Writes").

## Change

- `crates/cookie/src/public_suffix/public_suffix_list.dat` (the file, unchanged), `SHA256SUMS`
  (one line in the form `sha256sum` writes and reads: `<64 hex>  public_suffix_list.dat`) and
  `SOURCE` (URL, the version line of the file, the date fetched, the number of rules, the
  licence).
- `crates/cookie/src/punycode.rs`: `pub fn encode(label: &str) -> String`, RFC 3492 section 6.3
  with the parameters of section 5 (base 36, tmin 1, tmax 26, skew 38, damp 700, initial bias 72,
  initial n 128), checked arithmetic, an overflow refused (section 6.4); `to_ascii_label` gives
  `xn--` and the encoding for a label with a character above U+007F and the label unchanged
  otherwise.
- `crates/cookie/src/public_suffix/mod.rs`: `SuffixList::parse(text: &str) -> SuffixList` (the
  rules, comments and empty lines skipped, each label of a rule through `to_ascii_label`, the
  `// ===BEGIN PRIVATE DOMAINS===` section included); `SuffixList::bundled()` (the included file,
  parsed once); `public_suffix(host) -> &str` and `registrable_domain(host) -> Option<&str>` by
  the list's algorithm (the longest matching rule, an exception rule winning, the implied `*`); an
  IP address host has neither.
- `Jar::new()` uses `SuffixList::bundled()`; `Jar::with_suffixes(list: SuffixList)` uses the list
  given; `Jar::set_suffixes(&self, list: SuffixList)` replaces it (for a test of §5.8.3 step 3).
- `Jar::receive` (R4 widened): a `Domain` attribute that the request host domain-matches and that
  is not a public suffix gives a domain cookie (host-only flag false), sent to every host that
  domain-matches it; one that is a public suffix gives a host-only cookie when it equals the host
  and is ignored otherwise (§5.7 steps 9 and 10). `cookie_field` leaves out a domain cookie whose
  domain is a public suffix by the jar's list (§5.8.3 step 3).
- `site(initiator, url)` (R9 widened): same-site when there is no initiator, or the scheme is the
  same and the registrable domains are the same; for hosts without one (IP addresses, a host that
  is itself a public suffix) when the hosts are the same.
- `taskfiles/cookie-psl.sh` (POSIX sh) and `taskfiles/cookie-psl.yml`:
  - `psl-refresh DIR=<dir>` (`sh taskfiles/cookie-psl.sh refresh <dir> [<file>]`): fetches the
    list with `curl` (or copies `<file>`), and writes `public_suffix_list.dat`, `SHA256SUMS` (by
    `sha256sum`) and `SOURCE` into `<dir>`; the committed copy is refreshed with
    `DIR=crates/cookie/src/public_suffix`. Never in `check`, `check-runner` or a workflow; started
    without the `slot` prefix.
  - `psl-check` (a step of `check`, no network): `cd crates/cookie/src/public_suffix && sha256sum
    -c SHA256SUMS`, which prints `public_suffix_list.dat: OK` or fails with
    `public_suffix_list.dat: FAILED` and exit 1.
  - `psl-age [DIR=<dir>]` (no network; `DIR` defaults to `crates/cookie/src/public_suffix`):
    prints `psl-age: <n> rules of <YYYY-MM-DD>, <d> days old at <YYYY-MM-DD>` and exits 0 when the
    list (the date of its `// VERSION:` line) is at most 90 days old at `DATE` (default today);
    otherwise prints `psl-age: the list of <YYYY-MM-DD> is <d> days old at
    <YYYY-MM-DD>, over 90` and exits 1.
  - The root `Taskfile.yml` include and the step `psl-check` in `check` are the coordinator's.
- `docs/releasing.md`, "Cutting a release": a step before the tag, `task psl-age` passes, else
  `task psl-refresh DIR=crates/cookie/src/public_suffix` and a commit on `main` first. `NOTICE`:
  the list and its licence.
- Shared files: `docs/releasing.md` is also written by `story:agent-tasks` and by
  `story:tls-release`, both before it (edges through `story:agent-cookie-jar` and to
  `story:tls-release`); `NOTICE` by `story:tls-release`, before it; `crates/cookie` and the module
  `cookies` of `crates/fetch/tests/main.rs` by `story:agent-cookie-jar`, before it (an edge).

## Acceptance

`slot task check` exits 0 and prints `public_suffix_list.dat: OK`; the folder's bounds hold (the
test step within 120 s, no test over 60 s, no model call and no network request in the test step,
no Python; the refresh task is outside the gate and its test uses a local file). Parsing the
bundled list in a test takes under 1 s (time in the report). Cases in `crates/cookie/tests/main.rs`,
modules `punycode` and `public_suffix`, and in the module `cookies` of `crates/fetch/tests/main.rs`
(the test `Network` resolving every name below to 127.0.0.1):

1. `SuffixList::bundled()` has the number of rules `SOURCE` names. `psl-check` on a copy of the
   directory under `CARGO_TARGET_TMPDIR` with one byte of the list changed (a test running the
   step's command there) prints `public_suffix_list.dat: FAILED` and exits 1.
2. `punycode`: each sample string (A) to (S) of RFC 3492 section 7.1, as code points, encodes to
   the string that section gives for it (copied into the test from the RFC's text); and, from the
   list itself, `to_ascii_label("中国")` is `xn--fiqs8s` and `to_ascii_label("公司")` is
   `xn--55qx5d` (the comments at lines 6511 and 10858); `SuffixList::bundled()` holds the rules
   `xn--fiqs8s` and `xn--55qx5d.cn`.
3. `public_suffix`: `a.example.co.uk` gives `co.uk`, `a.b.ck` gives `b.ck` (wildcard), `www.ck`
   gives `ck` (exception), `example.test` gives `test` (implied `*`), `a.xn--55qx5d.cn` gives
   `xn--55qx5d.cn`; `registrable_domain`: `example.co.uk`, `a.b.ck`, `www.ck`, `example.test`,
   `a.xn--55qx5d.cn`; `127.0.0.1` and `co.uk` have none.
4. `http://a.example.co.uk:<p>/set` answers `d=1; Domain=example.co.uk; Path=/`:
   `http://b.example.co.uk:<p>/echo` receives `Cookie: d=1`; `http://example.co.uk:<p>/echo`
   receives `Cookie: d=1`; `http://other.co.uk:<p>/echo` has no `Cookie`.
5. `http://a.example.co.uk:<p>/set` answers `ps=1; Domain=co.uk; Path=/`: the cookie is ignored;
   `http://other.co.uk:<p>/echo` and `http://a.example.co.uk:<p>/echo` have no `Cookie`.
6. `http://co.uk:<p>/set` answers `h=1; Domain=co.uk; Path=/`: `http://co.uk:<p>/echo` receives
   `Cookie: h=1`; `http://a.co.uk:<p>/echo` has no `Cookie`.
7. `site(http://a.example.test, http://b.example.test/)` is `Same`; `site(http://a.example.co.uk,
   http://b.other.co.uk/)` is `Cross`; `site(http://127.0.0.1, http://127.0.0.2/)` is `Cross`. On
   the wire: after `http://a.example.test:<p>/set` answered `strict=1; SameSite=Strict; Path=/;
   Domain=example.test`, a `GET http://b.example.test:<p>/echo` with initiator
   `http://a.example.test:<p>` receives `Cookie: strict=1`.
8. `Jar::with_suffixes(SuffixList::parse("test\n"))`, from `http://a.example.test/`: `d=1;
   Domain=example.test; Path=/` and `e=1; Path=/`; `cookie_field(http://a.example.test/)` is `d=1;
   e=1`; after `set_suffixes(SuffixList::parse("test\nexample.test\n"))` it is `e=1`.
9. `psl-refresh` with `<file>` `crates/cookie/tests/psl-sample.dat` (a list of three rules whose
   version line is `// VERSION: 2026-11-21_04-33-45_UTC`; a test running `sh
   taskfiles/cookie-psl.sh refresh <dir> <file>` with `<dir>` under `CARGO_TARGET_TMPDIR`) writes
   the three files into `<dir>`, and `sha256sum -c SHA256SUMS` there prints
   `public_suffix_list.dat: OK`; the committed directory is unchanged. `psl-age DIR=<dir>` with
   `DATE=2027-02-20` (91 days after 2026-11-21) prints exactly `psl-age: the list of 2026-11-21 is
   91 days old at 2027-02-20, over 90` and exits 1; with `DATE=2027-02-19` it prints `psl-age: 3
   rules of 2026-11-21, 90 days old at 2027-02-19` and exits 0.
10. The cases of `story:agent-cookie-jar` pass, two updated in this change: jar case U10, where
   `site(http://a.test, http://b.a.test/)` becomes `Same` (both have the registrable domain
   `a.test`); wire case W4, where `http://b.example.test:<p>/echo` and
   `http://example.test:<p>/echo` now receive `Cookie: p=1; q=1`.

## Out of scope

- IDNA mapping of hosts (UTS 46): surf takes ASCII hosts only; fetching the list at run time or
  from a host's system copy; a list per session.
- Partitioned cookies (CHIPS), first-party sets, cookie policies a person sets (§7.2).
- The anchor list and its refresh (`story:tls-anchors`, `story:tls-release`).
