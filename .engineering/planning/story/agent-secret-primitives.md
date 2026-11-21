---
format: aep.planning-md/3
id: story:agent-secret-primitives
kind: story
status: implemented
title: 'surf-secret: the random source, the comparison in constant time and the session token, in one crate'
summary: A standard-library crate with the read of the operating system random source, a byte comparison that reads every byte, and the 32-byte token with its 64-digit form, used by surfd, surf-session and surf-view.
relations:
- decomposes: epic:agent-cli-target
scope:
- confidence: cited
  path: crates/secret
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T00:46:10Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-21T00:46:10Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-11-21T01:25:57Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"test_result":1,"verification":1}}}
---
## Context

Decision D25 of `docs/plan/agent-plan-revision-2.md`: one home for secrets. Three crates need the
operating system random source and a comparison in constant time: `surfd` (session identities,
`story:agent-surfd-supervisor`), `surf-session` (the session token,
`story:agent-session-process`) and `surf-view` (the view token, `story:view-http-server`).
`surf-session` depends on `surfd`, so `surfd` cannot call into it; written three times, the
copies would drift. So the three primitives live in one crate with no dependency, at depth 1,
and the three stories depend on it and write none of them themselves.

`crates/ports/src/random.rs` (`OsRandom`) reads `/dev/urandom` through the standard library;
this crate reads it the same way. `OsRandom` stays as it is in this plan;
`story:random-source-failure-cause` (backlog) notes that it moves onto `surf-secret`.

## Change

- `crates/secret` (package `surf-secret`; a library; standard library only;
  `#![forbid(unsafe_code)]`):
  - `fill_random(buffer)`: fills the buffer from `/dev/urandom`, or returns the error of opening or
    reading it; never a value of its own. `fill_random_from(path, buffer)` is the same over a
    path, so a test can name a device that does not exist.
  - `equal(a, b)`: true when the two byte slices are equal; slices of different length are
    unequal; for slices of one length it reads every byte, whatever the input, and decides once
    at the end (an OR over the XOR of each pair). A count of the bytes read, compiled in tests
    only, makes that observable.
  - `Token`: 32 bytes. `Token::draw()` from `fill_random`; `Token::to_hex()` writes 64 lower-case
    hexadecimal digits; `Token::from_hex(text)` accepts exactly 64 lower-case hexadecimal digits
    and nothing else; `Token::matches(&other)` compares through `equal`. Its `Debug` writes
    `Token(..)` and no digit; it has no `Display`.
- `crates/secret/tests/`, the counted cases:
  1. two draws of 32 bytes differ;
  2. `fill_random_from` a path that does not exist: an error, and the buffer is not reported as
     filled;
  3. `to_hex` of a token is 64 characters, each in `0-9a-f`;
  4. `from_hex(to_hex(t))` gives a token that matches `t`;
  5. 63 digits refused; 6. 65 digits refused; 7. upper-case digits refused; 8. a `g` refused;
  9. `equal` on two equal 32-byte slices: true;
  10. slices differing only in the first byte: false, and 32 bytes read;
  11. slices differing only in the last byte: false, and 32 bytes read;
  12. slices of 31 and 32 bytes: false;
  13. `format!("{:?}", token)` holds none of the 64 digits.

Surfaces written: `crates/secret/`.

## For AGENTS.md (handed over in the unit report, decision D7)

- "Layout": "`crates/secret` (`surf-secret`): the operating system random source, the comparison
  in constant time and the 32-byte token. Every secret and every random identity of the agent
  and the view comes from here. No dependency."

## Acceptance

`cargo test -p surf-secret` passes with the 13 numbered cases, each a test named after its case,
cases 10 and 11 reporting 32 bytes read, and `cargo tree -p surf-secret` prints the crate alone.

## Out of scope

Moving `surf-ports` `OsRandom` onto this crate (`story:random-source-failure-cause`); a hash of
the token (`story:agent-token-hash`); any cryptographic primitive beyond these three.

## Scope

Derived 2026-11-21 by `story-scoper`.

- **Primary surface:** `crates/secret` — cited, the story's "Surfaces written: `crates/secret/`" (`.engineering/planning/story/agent-secret-primitives.md:54`); the directory does not exist on the branch (`ls crates` shows `fetch`, `ports`, `surf`)
- **Files written:** `crates/secret/Cargo.toml` — cited, package `surf-secret`, standard library only (story line 28); the manifest layout is inferred from `crates/ports/Cargo.toml` (edition 2024, `publish = false`)
- **Files written:** `crates/secret/src/lib.rs` — inferred, a library crate whose three primitives and `#![forbid(unsafe_code)]` the story names (lines 28-40) but whose file split it does not give. It may split into modules.
- **Files written:** `crates/secret/tests/` — cited, "the counted cases" 1-13 (story line 41). The file names inside are inferred, with one test per case named after the case (acceptance, line 64).
- **Files read, not written:** `crates/ports/src/random.rs` (`OsRandom`, `RandomSource`, `DEVICE = "/dev/urandom"`, lines 28-45) — cited, the story says this crate reads the device "the same way" and leaves `OsRandom` untouched (story lines 21-24)
- **Files read, not written:** `docs/plan/agent-plan-revision-2.md` D25 (lines 70-76) — cited
- **Handed to the coordinator:** `AGENTS.md`, section "Layout": the line "`crates/secret` (`surf-secret`): the operating system random source, the comparison in constant time and the 32-byte token. Every secret and every random identity of the agent and the view comes from here. No dependency." — cited, story lines 56-59 and D7
- **Handed to the coordinator:** root `Cargo.lock`, one new `[[package]]` entry `surf-secret` with no dependencies — inferred. The root `Cargo.toml` member glob `crates/*` picks the crate up with no edit (`Cargo.toml`), and `AGENTS.md` says `--locked` fails until `cargo generate-lockfile --offline` has run. The story does not mention the lock.
- **Handed to the coordinator:** root `Taskfile.yml`, `README.md` — none. `task build` and `task test` run over the whole workspace (`Taskfile.yml:157-170`), and the story adds no task. — inferred
- **Symbols:** crate `surf-secret`: `fill_random(buffer)`, `fill_random_from(path, buffer)`, `equal(a, b)` — cited (story lines 29-35)
- **Symbols:** crate `surf-secret`: `Token` with `draw`, `to_hex`, `from_hex`, `matches`, and a `Debug` that writes `Token(..)` and no `Display` — cited (story lines 36-39)
- **Symbols:** crate `surf-secret`: a read count compiled in tests only (`#[cfg(test)]`), used by cases 10 and 11 — cited as a requirement (story lines 33-35); its name, and whether an integration test under `tests/` can see it, are inferred
- **Would collide with:** nothing that writes `crates/secret/`. The three consumers (`story:agent-surfd-supervisor`, `story:agent-session-process`, `story:view-http-server`) only add `surf-secret` to their own manifests and write none of the primitives. — cited (story lines 18-20 and 24; `agent-surfd-supervisor.md:65`, `agent-session-process.md:44`, `view-http-server.md:27,41`)
- **Would collide with:** `story:random-source-failure-cause` (backlog) later moves `OsRandom` in `crates/ports` onto this crate, so it would write `crates/secret/src` if it adds an error cause there — cited that it depends on this crate (`random-source-failure-cause.md:47-51`), inferred that it edits this crate
- **Would collide with:** `story:agent-token-hash` (deferred) changes how the token is compared and kept, so it may touch `Token` — inferred from its summary
- **Would collide with:** the root `Cargo.lock`, `AGENTS.md` "Layout" and the root `Cargo.toml` member glob, which any story adding a crate touches — cited (D7; `AGENTS.md` "Layout"). All are the coordinator's.
- **Not yet existing when it starts:** nothing. The story has only a `decomposes` edge to `epic:agent-cli-target` and depends on no other story; it is at depth 1 (story line 20). Everything it needs, the standard library and `/dev/urandom`, exists. — cited
- **Confidence:** high — the story names the one directory, the package name and every symbol, and the tree confirms that nothing exists there and that the glob needs no manifest edit. The only guesses are the internal file split and the `Cargo.lock` hand-off.
