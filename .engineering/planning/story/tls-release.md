---
format: aep.planning-md/3
id: story:tls-release
kind: story
status: active
title: 'Release builds carry ring on the four targets, and the smoke test checks https: and the anchors'
summary: The compiler step of the design note in release.yml, the smoke test checks surf https://127.0.0.1:1/ and surf --anchors, the anchor age step in releasing.md, NOTICE, changelog and website.
relations:
- decomposes: epic:tls-https
- depends_on: story:tls-decisions
- depends_on: story:tls-session-anchors
- depends_on: story:tls-tests-as-tables
- depends_on: story:session-follow-press-https
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
revision: 12
transitions:
- {from: "draft", to: "proposed", at: "2026-11-22T06:24:31Z", actor: "human:operator", revision: 9, decided_on: {"recorded":{"verification":1}}}
- {from: "proposed", to: "active", at: "2026-11-22T06:24:31Z", actor: "human:operator", revision: 10, decided_on: {"recorded":{"verification":1}}}
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

Placement: the last of the four stories built as one unit (`tls-spec`, `tls-fetch-https`,
`tls-session-anchors`, `tls-release`), at a boundary between two rounds of surf, under the path
rule of `story:tls-decisions` ("Order of the stories of the epic"). It follows
`story:tls-fetch-https`, whose dev-dependencies of `crates/surf-cli` on the test server and the
test certificates turn on `test-tooling` in every `cargo test` build of `surf-cli`.

## Change

- `.github/workflows/release.yml`, job `package`, step "Build, test and package" (lines
  129-176): the compiler step of item 3 for each target that needs it, before the step, with a
  comment saying why; and the order of the step changed so that `test-tooling` never reaches a
  shipped binary. Today the build comes first (line 140 on Linux, line 148 on macOS), then
  `cargo test ... --test main cli:: -- --skip review1_cli::` (line 152), which with the dev-dependencies of `story:tls-fetch-https`
  rebuilds `target/$TARGET/release/surf` with `test-tooling`, then the copy from that path
  (lines 155-169). After this story, with the test line kept as it is at the base of the unit
  (its `--test` argument is the one `story:one-test-binary-per-crate` left: `--test main cli::`
  with `-- --skip review1_cli::`); this story changes only the order of build and test and
  the target directory of the build:
  1. the test line, first, unchanged;
  2. then the builds of lines 140 and 148, each with `--target-dir target/ship`: a build of
     `--bins` only turns on no dev-dependency feature, and no file of the test build lies in
     that directory;
  3. the copy loop reads `target/ship/$TARGET/release/$binary` (line 156); before each `cp`
     (line 169) `grep -q -F "surf-tls test-tooling, never in a release" "$bin"` fails the job
     naming the binary when it finds the marker of item 6; and once, as a check of the search
     itself, the same search must find the marker in `target/$TARGET/release/surf`, the binary
     the test built.
  The comment of the step says why. The cost (ring and the crates built twice per target) is
  in the report.
- `taskfiles/release-smoke.sh` step 2: also `surf https://127.0.0.1:1/` exits 1 and prints
  `fetch-failed <id> Unreachable https://127.0.0.1:1/`; `surf --anchors` exits 0 and prints
  `anchors <n> bundled <date> sha256:<hex>` with the count and date of
  `crates/tls/src/anchors/SOURCE` of the checkout. The header comment of the script says so.
  Its step 3 (lines 96-98 today) runs the end-to-end case with `--test main e2e::`, as
  `story:one-test-binary-per-crate` left it (the case reads
  `SURF_BIN_DIR`; the filter keeps the other tests of `surf-session` out of the smoke run).
  This story does not change those lines. It tests the unpacked binaries
  (`SURF_BIN_DIR`), so its own build may carry `test-tooling`.
- `docs/releasing.md`: "What a release holds" names ring and the compiler step; "Cutting a
  release" gains a step before the tag: `task anchors-age` passes, else `task anchors-refresh`
  and a commit on `main` first; the smoke test paragraph names the two new checks.
- `NOTICE`: ring 0.17.14 (Apache-2.0 AND ISC, its copyright lines) and the root list (its
  licence, item 11).
- `CHANGELOG.md`, `## [Unreleased]`: `https:` addresses over TLS 1.3, the five reasons,
  `surf --ca-file` and `--anchors`, `surf-agent open --ca-file`, a server limited to TLS 1.2
  ends `TlsVersionUnsupported`, or `TlsHandshakeFailed` with a detail saying so when it ignores
  `supported_versions`.
- The website: `website/docs/what-works.md`, `status.md`, `roadmap.md`, `principles.md`,
  `how-it-is-built.md`, `try-it.md`, `drive-from-an-agent.md`, and
  `website/docs/reference/surf-command.md` (the hand copy of `docs/surf-command.md`); where
  `what-works.md` and `try-it.md` describe `https:`, the sentence "surf does not check
  revocation: a revoked certificate that has not expired loads."
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
5. `grep -c ring NOTICE` prints at least 1; `grep -c "does not check revocation"` prints at least
   1 in `website/docs/what-works.md` and in `website/docs/reference/surf-command.md`.
6. In that run, each `package` log shows the steps in the order above: the test first, then the
   build into `target/ship`; the marker search finds `surf-tls test-tooling, never in a release`
   in `target/<t>/release/surf` (the binary the test built) and in none of the copied
   binaries.
7. Hostile, here on the workstation, the same two builds by hand through `slot`: after the
   `cargo test` line of `release.yml` as this story leaves it (without `--target`), `grep -c
   -F "surf-tls test-tooling, never in a release" target/release/surf` prints at least 1 (a
   release that copied the test build would be refused); after `cargo build --release -p
   surf-cli --bins --target-dir target/ship`, the same search in `target/ship/release/surf`
   prints 0. Times in the report.
8. `git diff <unit base> -- .github/workflows/release.yml taskfiles/release-smoke.sh` shows no
   change to a `--test` argument or a test filter: only the order of build and test, the
   `--target-dir`, the copy path, the marker search, the compiler step and step 2 of the smoke
   test.

Budget: no test of the root workspace is added; the smoke test runs in the release workflow and
by hand, never in `task test`; no model call.

## Out of scope

The tag and the release (the coordinator, on the word of `host-operator`, as
`docs/releasing.md` says); Windows; signing and notarisation; the model lane of
`story:agent-tasks`.

## Writes

`.github/workflows/release.yml`, `taskfiles/release-smoke.sh`, `docs/releasing.md`, `NOTICE`,
`CHANGELOG.md`, the website pages above. `docs/releasing.md` is also written by
`story:agent-tasks` (the agent task steps and lines of the release notes), before this story,
and by `story:agent-cookie-public-suffixes` (the suffix list age step) after it; `NOTICE` by
`story:agent-cookie-public-suffixes` after it; `taskfiles/release-smoke.sh` by
`story:agent-skill-definition` (the step `help --for-agent`) after it: never in the same wave. The
probe workflow of `story:tls-decisions` is on a branch that is not merged. Coordinator:
`README.md`, `AGENTS.md`, the badges.
