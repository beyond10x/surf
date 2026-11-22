---
format: aep.planning-md/3
id: story:tls-release
kind: story
status: draft
title: 'Release builds carry ring on the four targets, and the smoke test checks https: and the anchors'
summary: The compiler step of the design note in release.yml, the smoke test checks surf https://127.0.0.1:1/ and surf --anchors, the anchor age step in releasing.md, NOTICE, changelog and website.
relations:
- decomposes: epic:tls-https
- depends_on: story:tls-decisions
- depends_on: story:tls-session-anchors
scope:
- confidence: cited
  path: .github/workflows/release.yml
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: NOTICE
- confidence: cited
  path: docs/releasing.md
- confidence: cited
  path: taskfiles/release-smoke.sh
- confidence: cited
  path: website/docs
revision: 3
---
## Context

`.github/workflows/release.yml`, job `package`: four targets on the runners of the forge, no C
compiler step, `readelf -lW` refuses an `INTERP` header on Linux, the smoke test
`taskfiles/release-smoke.sh` on the unpacked archive. Its step 2 today: `surf` without an
argument exits 2, `surf http://127.0.0.1:1/` exits 1 with `fetch-failed <id> Unreachable
http://127.0.0.1:1/`. `docs/releasing.md`: what a release holds and the steps to cut one.

The epic, "Changes outside": "`taskfiles/release-smoke.sh` checks that `surf
https://127.0.0.1:1/` ends `Unreachable` (the scheme is accepted) and that the binary finds its
anchors"; "the release workflow gains what the chosen source needs to build on the four
runners; `docs/releasing.md` gains the anchor list age check"; "README principle 4 and
`AGENTS.md` Goals ...; What works, What surf is not and the website follow". U1: ring needs a C
compiler on every build; `docs/design/tls.md` item 3 gives the step per runner, measured by the
probe of `story:tls-decisions`; item 11 the licences of ring and of the list. `task
anchors-age` and `anchors-refresh`: `story:tls-anchors`.

## Change

- `.github/workflows/release.yml`: the compiler step of item 3 for each target that needs it, in
  the job `package` before the build, with a comment saying why.
- `taskfiles/release-smoke.sh` step 2: also `surf https://127.0.0.1:1/` exits 1 and prints
  `fetch-failed <id> Unreachable https://127.0.0.1:1/`; `surf --anchors` exits 0 and prints
  `anchors <n> bundled <date> sha256:<hex>` with the count and date of
  `crates/tls/src/anchors/SOURCE` of the checkout. The header comment of the script says so.
- `docs/releasing.md`: "What a release holds" names ring and the compiler step; "Cutting a
  release" gains a step before the tag: `task anchors-age` passes, else `task anchors-refresh`
  and a commit on `main` first; the smoke test paragraph names the two new checks.
- `NOTICE`: ring 0.17.14 (Apache-2.0 AND ISC, its copyright lines) and the root list (its
  licence, item 11).
- `CHANGELOG.md`, `## [Unreleased]`: `https:` addresses over TLS 1.3, the five reasons,
  `surf --ca-file` and `--anchors`, `surf-agent open --ca-file`, TLS 1.2 servers end
  `TlsVersionUnsupported`.
- The website: `website/docs/what-works.md`, `status.md`, `roadmap.md`, `principles.md`,
  `how-it-is-built.md`, `try-it.md`, `drive-from-an-agent.md`, and
  `website/docs/reference/surf-command.md` (the hand copy of `docs/surf-command.md`).
- For the coordinator: `README.md` ("What works", "What surf is not", the roadmap, principle 4 if
  not done), `AGENTS.md` ("Publishing", "Commands": only `http:` loads goes), the badges.

## Acceptance

A manual run of the release workflow on the story branch, started and watched by the coordinator
(all forge work is the coordinator), is green on the four targets, and each smoke test prints the
`https:` line and the anchors line.

1. That run: the four `package` jobs and `checksums` conclude `success`; each smoke log holds
   the line for `https://127.0.0.1:1/` and `anchors <n> bundled ...` with the `SOURCE` count;
   the Linux binaries pass the `INTERP` check.
2. Here, on the x86_64 musl archive of that run: `sh taskfiles/release-smoke.sh <dir>` ends
   `release-smoke: ok, surf, surf-agent, surfd and surf-session on Linux`.
3. Hostile: the same script on the unpacked x86_64 musl archive of release 0.3.0, whose `surf`
   answers `UnsupportedScheme` for `https:`: exit 1 naming that line.
4. `slot task anchors-age` exits 0 on the story commit.
5. `grep -c ring NOTICE` prints at least 1.

Budget: no test of the root workspace is added; the smoke test runs in the release workflow and
by hand, never in `task test`; no model call.

## Out of scope

The tag and the release (the coordinator, on the word of `host-operator`, as
`docs/releasing.md` says); Windows; signing and notarisation; the model lane of
`story:agent-tasks`.

## Writes

`.github/workflows/release.yml`, `taskfiles/release-smoke.sh`, `docs/releasing.md`, `NOTICE`,
`CHANGELOG.md`, the website pages above. `docs/releasing.md` is also written by
`story:agent-tasks` (the model lane line of the release notes): never in the same wave. The
probe workflow of `story:tls-decisions` is on a branch that is not merged. Coordinator:
`README.md`, `AGENTS.md`, the badges.
