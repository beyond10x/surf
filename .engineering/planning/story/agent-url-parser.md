---
format: aep.planning-md/3
id: story:agent-url-parser
kind: story
status: implemented
title: 'surf-url: the WHATWG URL parser surf needs to resolve addresses'
summary: A hand-written URL parser with relative resolution, origin and the form-urlencoded serializer, held to the web-platform-tests URL data.
relations:
- decomposes: epic:agent-cli-target
scope:
- confidence: cited
  path: crates/url
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:11Z", actor: "human:timo", revision: 5}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:11Z", actor: "human:timo", revision: 6}
- {from: "active", to: "implemented", at: "2026-11-21T01:25:57Z", actor: "human:timo", revision: 7, decided_on: {"recorded":{"test_result":1,"verification":1}}}
---
## Context

Three stories compute an address from another: `follow` resolves a link target against the base
URL of the page (`agent-follow-links`), a redirect resolves `Location` (`agent-fetch-redirects`),
and a form submission resolves its action and serializes its entries (`agent-form-port`).
Nothing in surf does that today: `surf.url.Url` is a `String` newtype nothing checks
(`spec/domains/url.yaml`, its `UNMAPPED:` marker), and `crates/fetch` splits an `http:` address
by its own rules. `epic:html-parser` keeps a link target as its `href` value as written
(`html-decisions`, item 6).

Decided here: the URL parser belongs in this epic, because step 3 is the first place surf
resolves an address and no story of another epic needs it first. It is a library like
`surf-fetch`, a port-side computation whose specification is the WHATWG URL Standard and its test
data, as the HTTP client follows RFC 9110; nothing in `spec/` changes, because no command of the
model parses a URL. Whether `surf.url.Url` refuses a string that is not a URL stays the question
of backlog `story:invalid-address-ending`.

Sections, named from memory and to be confirmed: URL Standard "URL parsing" (the basic URL parser
with a base), "URL serializing", "Hosts (domains and IP addresses)", "Percent-encoded bytes",
"Origin", "application/x-www-form-urlencoded" (the serializer). Test data: web-platform-tests
`url/resources/urltestdata.json` (from memory), test tooling under its own licence (3-clause BSD,
from memory, to be confirmed).

## Change

- `crates/url` (package `surf-url`, no dependency, `#![forbid(unsafe_code)]`): parse an input
  against an optional base into a URL record or a failure; serialize; origin and same-origin
  comparison; the form-urlencoded serializer over name and value pairs, UTF-8. Domain to ASCII
  (IDNA, UTS #46) is not written: a host with a character outside ASCII is a failure with a reason
  of its own. The input length is limited and documented; no input panics.
- `crates/url/tests/data/`: the file at a recorded web-platform-tests commit, its licence, and a
  `SOURCE` note with the address, the commit, the date and the number of test cases the file
  holds (its objects with an `input`, counted when it is vendored). A reader written in the test.
- A test that runs every case and prints `urltestdata: <total> cases, <run> run, <passed> passed,
  <skipped> skipped`, asserts that `<total>` equals the number in `SOURCE`, that every case that
  runs passes, and that a case is skipped only when its input or base holds a host with a character
  outside ASCII; the skipped cases are listed by input in `crates/url/tests/data/SKIPPED`.
- Unit tests for the serializer: space, `&`, `=`, `+`, `*`, and bytes outside ASCII.
- Crate documentation: what it does not do, and the sections it claims; a section is complete only
  when every case of it passes (`AGENTS.md`, "Goals").

Surfaces written: `crates/url/`.

## For AGENTS.md (handed over in the unit report, decision D7)

- "Layout": "`crates/url` (`surf-url`): the WHATWG URL parser surf resolves addresses with, held to
  the web-platform-tests URL data; no IDNA. No dependency."

## Acceptance

`cargo test -p surf-url` passes and prints `urltestdata:` with `<total>` equal to the case count in
`SOURCE`, `<passed>` equal to `<run>`, and `<skipped>` at most 15 percent of `<total>`, each skipped
case listed in `SKIPPED`, and `cargo tree -p surf-url` prints the crate alone.

## Out of scope

IDNA; the script interfaces `URL` and `URLSearchParams`; the form-urlencoded parser; checking
`surf.url.Url` in the specification (`story:invalid-address-ending`).

## Scope

Derived 2026-11-21 by `story-scoper`.

- **Primary surface:** `crates/url/` — cited, the story's "Surfaces written: `crates/url/`" (story:agent-url-parser, Change section)
- **Files written:** `crates/url/Cargo.toml` (package `surf-url`, no dependency) — cited (Change, first bullet); the file name is inferred from the layout of `crates/fetch/Cargo.toml`
- **Files written:** `crates/url/src/**` (`lib.rs` plus modules for parser, host, percent-encoding, origin and the form-urlencoded serializer; `#![forbid(unsafe_code)]`; the crate documentation) — cited for the content (Change bullets 1 and 5), inferred for the file names
- **Files written:** `crates/url/tests/data/urltestdata.json`, `crates/url/tests/data/LICENSE`, `crates/url/tests/data/SOURCE`, `crates/url/tests/data/SKIPPED` — cited for `SOURCE` and `SKIPPED` and for "the file ... its licence" (Change bullet 2); the data and licence file names are inferred
- **Files written:** `crates/url/tests/*.rs` (the web-platform-tests runner with its reader, and the serializer unit tests) — cited that these tests exist (Change bullets 2 to 4), inferred for the file names (`crates/fetch/tests/*.rs` is the pattern)
- **Files read, not written:** none in the tree. `crates/url` has no dependency (cited: "no dependency"; Acceptance "`cargo tree -p surf-url` prints the crate alone"). Documents read: the WHATWG URL Standard and web-platform-tests `url/resources/urltestdata.json`, fetched once to vendor — cited (Context), sections "to be confirmed"
- **Handed to the coordinator:** `AGENTS.md` "Layout": the line "`crates/url` (`surf-url`): the WHATWG URL parser surf resolves addresses with, held to the web-platform-tests URL data; no IDNA. No dependency." — cited (story "For AGENTS.md", decision D7, `docs/plan/agent-plan-revision-1.md:67`)
- **Handed to the coordinator:** root `Cargo.lock`, which gains the package `surf-url` (`cargo generate-lockfile --offline`) — cited by `AGENTS.md` "Layout" (`Cargo.toml` and `Cargo.lock`: a new crate fails `--locked` until the lockfile is regenerated) and by D7
- **Handed to the coordinator:** root `Taskfile.yml`: nothing. The story adds no task, and `task test` already covers a new member of `crates/*` — inferred. The root `Cargo.toml` needs no change, because the member glob `crates/*` picks the directory up (cited, `Cargo.toml:3`)
- **Handed to the coordinator:** `README.md`: nothing — inferred, the story names no README section
- **Symbols:** crate `surf-url` adds public items, none named by the story. It names these capabilities: parse an input against an optional base into a URL record or a failure; serialize; origin and same-origin comparison; the form-urlencoded serializer over name and value pairs; a failure type with a reason of its own for a non-ASCII host — cited for the capabilities (Change, first bullet), inferred for every identifier. No existing crate's symbols change
- **Would collide with:** `crates/url/` is written by no other story I could find. `git grep` for `crates/url` and `surf-url` outside this story's body and the review results finds only `agent-fetch-redirects`, `agent-follow-links`, `agent-form-port`, `agent-links-spec` and `html-decisions`, and each of those uses `surf-url` as a dependency. The first three list `depends_on: story:agent-url-parser`. The two `Cargo.toml` files of those consumers are their own, not this story's — cited. Any unit that writes the root `Cargo.lock`, `AGENTS.md` "Layout" or `Cargo.toml` would meet the coordinator's writes, not the unit's — inferred
- **Not yet existing when it starts:** nothing. The story has no `depends_on` edge; its only relation is `decomposes epic:agent-cli-target`. The consumers (`agent-follow-links`, `agent-fetch-redirects`, `agent-form-port`) need `surf-url`, not the reverse
- **Confidence:** high — the story names its one directory and its test-data files, and no other unit's body writes there. The file names inside the directory are inferred and do not change the claim of disjointness

