# Changelog

What changed in each release of surf. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[Semantic Versioning](https://semver.org/spec/v2.0.0.html). The history starts at the first
release; what came before it is in the git log and in `docs/plan/`.

## [Unreleased]

### Added

- `surf-agent open [<url>]`, `go <url>`, `look` and `close` drive a browser session from the
  command line, one call at a time, for an AI agent or a script. `open` prints the session's
  identity and its token once; later calls name them in `SURF_SESSION` and `SURF_TOKEN`. Each
  session runs in a process of its own, `surf-session`, under the daemon `surfd`, which
  `surf-agent open` starts. Every call answers in the line format of
  [`docs/agent-answer.md`](docs/agent-answer.md). How to use it:
  [Drive surf from an agent](https://beyond10x.github.io/surf/docs/drive-from-an-agent).
- The Linux archives hold `surf`, `surf-agent`, `surfd` and `surf-session`. Install the four
  into one directory: `surf-agent` starts `surfd` from its own directory, and `surfd` starts
  `surf-session` from its own.
- The agent binaries are for Linux only: they read the caller's user id from `/proc/self` and
  need `XDG_RUNTIME_DIR`, and macOS has neither. The macOS archives hold `surf`, as before.

## [0.1.0] - 2026-11-21

The first release.

### Added

- `surf <url>` loads one `http:` address and prints one line per navigation, request, document
  and failed fetch, with exit status 0 when the navigation commits. Output format, exit statuses
  and limits: [`docs/surf-command.md`](docs/surf-command.md).
- Pre-built `surf` binaries for Linux (x86_64 and aarch64, statically linked) and macOS (x86_64
  and aarch64), each archive with `README.md`, `LICENSE` and `NOTICE`, and a `SHA256SUMS`
  file. How a release is cut:
  [`docs/releasing.md`](docs/releasing.md).
