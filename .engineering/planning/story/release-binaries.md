---
format: aep.planning-md/3
id: story:release-binaries
kind: story
status: implemented
title: Pre-built binaries for Linux and macOS from a release workflow
summary: A release workflow that builds surf for Linux and macOS on a version tag and attaches the archives and checksums to the release; a manual run publishes nothing.
scope:
- confidence: cited
  path: .github/workflows/release.yml
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: docs/releasing.md
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-21T01:32:42Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-21T01:32:42Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-11-21T02:10:31Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1,"verification":1}}}
---
## Context

The operator, 2026-11-21: "Make sure surf comes basically from the start with pre-built
binaries for Linux and Mac, maybe, and Windows, I don't know." The sibling repositories
`harness` and `metaharness` release through a workflow on a bot-made tag and the gates'
`scan-artifact`.

Windows is not a target yet, and that is a fact about the code, not a preference: `surf-secret`
and `surf-ports` read the random source from `/dev/urandom`, and the agent command line
(waves 2 to 4) talks over a Unix socket; the Rust standard library has neither on Windows.

## Change

- `.github/workflows/release.yml`: on a version tag, build the shipped binaries (today `surf`)
  for Linux x86_64 and aarch64 and macOS aarch64 and x86_64, with `--locked` and warnings
  denied, pack each with the licence notice once a licence exists, write `SHA256SUMS`, and
  attach them to the GitHub Release of that tag. A manual run builds everything and publishes
  nothing, so the workflow can be tried before the first tag. Actions pinned to commits,
  permissions declared.
- `CHANGELOG.md`, started at the first release.
- `docs/releasing.md`: how a release is cut (version, changelog, the bot's annotated tag, the
  gates' artifact scan), after the siblings' procedure.
- `README.md` and the website gain an "Install" section when the first release exists; the
  coordinator writes it.

## Acceptance

A manual run of the release workflow on `main` succeeds and its four build jobs each upload an
archive holding an executable `surf`; the Linux x86_64 binary from that run, downloaded and run
here as `surf http://127.0.0.1:<port>/` against a local server, prints the three rows of a
committed page load and exits 0. The first tagged release is the operator's word, not part of
this story.

## Out of scope

Windows. Package managers. Signing and notarisation of the macOS binaries. A release of
`surf-agent` and `surfd`, which do not exist yet; they join the workflow when they do.
