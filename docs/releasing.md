# Releasing surf

A release is a bare version tag (`0.2.0`), annotated and written by the organization bot, on a
`main` commit whose `task check-runner` is green. The tag is the version of the package that
ships the `surf` binary, `surf-cli` (`crates/surf-cli/Cargo.toml`); the release workflow fails on
a tag that differs. The three packages of the agent binaries, `surf-agent` (`crates/agent`), `surfd`
(`crates/surfd`) and `surf-session` (`crates/session`), carry the same version.

## What a release holds

[`.github/workflows/release.yml`](../.github/workflows/release.yml) runs the command-line
tests of `surf` first, then builds the binaries with `--locked`, warnings denied, runner paths
remapped and symbols stripped, into a directory of their own (`target/ship`), packs them with
`README.md`, `LICENSE` and `NOTICE`, and smoke-tests the unpacked archive with
[`taskfiles/release-smoke.sh`](../taskfiles/release-smoke.sh). The test's build turns on the
feature `test-tooling` of `surf-tls`, whose marker `surf-tls test-tooling, never in a release`
the job finds in the `surf` the test built and refuses in every binary it packs
([`docs/design/tls.md`](design/tls.md), item 6).

Every binary is built with outside code: `ring` 0.17.14, the cryptographic primitives of
`surf-tls`, and the four crates it needs (`cfg-if`, `getrandom`, `libc`, `untrusted`); `NOTICE`
names them and the bundled trust anchors. ring compiles C and assembly with a C compiler for the
target. The Linux runners have none for musl, so the job installs `musl-tools` and builds ring
with `musl-gcc` (`CC_<target>`); the macOS runners build it with Apple clang and need no step
([`docs/design/tls.md`](design/tls.md), item 3).

| target | runner | archive |
|---|---|---|
| `x86_64-unknown-linux-musl` | `ubuntu-22.04` | `surf-<version>-x86_64-unknown-linux-musl.tar.gz` |
| `aarch64-unknown-linux-musl` | `ubuntu-22.04-arm` | `surf-<version>-aarch64-unknown-linux-musl.tar.gz` |
| `x86_64-apple-darwin` | `macos-15-intel` | `surf-<version>-x86_64-apple-darwin.tar.gz` |
| `aarch64-apple-darwin` | `macos-15` | `surf-<version>-aarch64-apple-darwin.tar.gz` |

Each archive holds one directory, `surf-<version>-<target>/`:

- a Linux archive: `surf`, `surf-agent`, `surfd`, `surf-session`, `README.md`, `LICENSE` and
  `NOTICE`;
- a macOS archive: `surf`, `README.md`, `LICENSE` and `NOTICE`. The agent binaries run on Linux
  only: they read the caller's user id from `/proc/self` and need `XDG_RUNTIME_DIR`, and macOS
  has neither.

The four binaries of a Linux archive stay in one directory, also when installed: `surf-agent`
starts `surfd` from the directory it runs from, and `surfd` starts `surf-session` from its own.
`SHA256SUMS` lists the four archives. The Linux binaries are statically linked (musl): they run
on any distribution, and they load no name service module from the host. The macOS binaries are
neither signed nor notarised.

The smoke test runs on the unpacked directory of every archive: it checks the list of files
above, that `surf` exits 2 without an argument and 1 on `http://127.0.0.1:1/`, that `surf
https://127.0.0.1:1/` exits 1 and ends `Unreachable` (the scheme is accepted; a binary without
`https:` ends `UnsupportedScheme`), that `surf --anchors` exits 0 and prints `anchors <n>
bundled <date> sha256:<hex>` with the date and sha256 of `crates/tls/src/anchors/SOURCE` of the
checkout and `<n>` at most its count (the binary finds its anchors), and on Linux, in a
private `XDG_RUNTIME_DIR`, runs the end-to-end case of the session process
(`crates/session/tests/e2e.rs`) on the directory's own `surf-agent`, `surfd` and
`surf-session`: `open`, `go`, `look` and `close` against a page server inside the test, each
answer compared with its golden text. It then fails if a process from the directory still runs.
It runs unchanged on a Linux workstation, from a checkout, on an unpacked archive:

    sh taskfiles/release-smoke.sh <dir>/surf-<version>-x86_64-unknown-linux-musl

The workflow publishes nothing and writes nothing to the repository: on a tag and on a manual
run alike, the four archives and `SHA256SUMS` are the run's artifacts (`release-<target>` and
`release-checksums`). A tag run also fails unless the tag is annotated and equals the package
version. The release is created by the organization bot, `b10x-bot[bot]`, from the artifacts of
the tag's run, after they are verified and scanned. No step uses `github.token` or a personal
account to write a release.

## Cutting a release

The commands use the gates as the bot, from a checkout of this repository:

    gates="b10x-gates --policy ../gates-policy/policy.json --repository beyond10x/surf"

1. **Trust anchors.** The bundled list of `surf-tls` is at most 90 days old on the day of the
   tag:

       slot task anchors-age

   It prints `anchors: <n> certificates of <date>, <d> days old` and passes, or fails with
   `anchors: <d> days is older than 90 days; run task anchors-refresh before a release`. When it
   fails, run `task anchors-refresh`, read the titles it prints as added and removed (below),
   run the tests of `surf-tls`, and commit the new `cacert.pem` and `SOURCE` on `main` before
   any other step; `DATE=<yyyy-mm-dd>` checks the age at the day the tag is planned for.
2. **Version.** Set `version` to the new version in the four manifests of the shipped
   binaries, `crates/surf-cli/Cargo.toml`, `crates/agent/Cargo.toml`,
   `crates/surfd/Cargo.toml` and `crates/session/Cargo.toml`, then
   `cargo generate-lockfile --offline`. `git diff Cargo.lock` then shows the version lines of
   these four packages and nothing else: no third-party package moves in a release commit. The
   command takes the newest version the local registry cache holds, so it can move one; pin it
   back with `cargo update --offline -p <name> --precise <version of the lock before>`, and the
   lock check passes again (on 2026-11-23, for `0.5.0` and again for `0.6.0`, it moved `cc`
   from 1.11.1 to 1.11.3, which `taskfiles/lock-allow.txt` does not list). Release `0.1.0`
   shipped the binary from the package `surf`; `surf-cli` starts at that version, so the next
   release is above it.
3. **Changelog.** In `CHANGELOG.md`, rename `## [Unreleased]` to `## [<version>] - <date>` and
   put an empty `## [Unreleased]` above it. Commit both on `main`; `slot task check-runner` is
   green on that commit, and the gate's output holds no line that starts with `raise:` (a task
   of `crates/agent-tasks/status.txt` listed `pending` that passes; give it its `pass` row
   first).
4. **Try it.** Run the workflow by hand on `main`, wait for it, and take steps 7 and 8 on its
   run:

       $gates gh -- workflow run release.yml --ref main
       $gates gh -- run list --workflow release.yml --limit 1
       $gates gh -- run watch <run-id> --exit-status

5. **Tag.** Write the tag message (the version's changelog section) to a file, then:

       $gates bot -- tag -a <version> -F <message-file> <commit>
       $gates bot -- push origin <version>

6. **Build.** The push starts the workflow on the tag. Wait for that run to be green:

       $gates gh -- run list --workflow release.yml --branch <version> --limit 1
       $gates gh -- run watch <run-id> --exit-status

7. **Download and verify.** Download the tag run's artifacts by run id into an empty directory,
   check that the four archives are there, check them against `SHA256SUMS`, and list them: each
   Linux archive lists `surf`, `surf-agent`, `surfd`, `surf-session` and the three files, each
   macOS archive `surf` and the three files:

       $gates gh -- run download <run-id> --dir <dir>
       mv <dir>/release-*/* <dir>/
       ls <dir>/surf-<version>-{x86_64,aarch64}-unknown-linux-musl.tar.gz \
          <dir>/surf-<version>-{x86_64,aarch64}-apple-darwin.tar.gz <dir>/SHA256SUMS
       (cd <dir> && sha256sum --check SHA256SUMS)
       for archive in <dir>/surf-<version>-*.tar.gz; do tar -tzf "$archive"; done

8. **Scan.** Unpack the two Linux archives and scan their eight executables, four per archive:

       tar -C <dir> -xzf <dir>/surf-<version>-x86_64-unknown-linux-musl.tar.gz
       tar -C <dir> -xzf <dir>/surf-<version>-aarch64-unknown-linux-musl.tar.gz
       $gates scan-artifact <dir>/surf-<version>-x86_64-unknown-linux-musl/surf
       $gates scan-artifact <dir>/surf-<version>-x86_64-unknown-linux-musl/surf-agent
       $gates scan-artifact <dir>/surf-<version>-x86_64-unknown-linux-musl/surfd
       $gates scan-artifact <dir>/surf-<version>-x86_64-unknown-linux-musl/surf-session
       $gates scan-artifact <dir>/surf-<version>-aarch64-unknown-linux-musl/surf
       $gates scan-artifact <dir>/surf-<version>-aarch64-unknown-linux-musl/surf-agent
       $gates scan-artifact <dir>/surf-<version>-aarch64-unknown-linux-musl/surfd
       $gates scan-artifact <dir>/surf-<version>-aarch64-unknown-linux-musl/surf-session

   Then run the smoke test on the downloaded x86_64 archive on a Linux x86_64 machine, from a
   checkout of the commit the run built (see "What a release holds"). It ends with
   `release-smoke: ok, surf, surf-agent, surfd and surf-session on Linux`:

       sh taskfiles/release-smoke.sh <dir>/surf-<version>-x86_64-unknown-linux-musl

   Then measure the agent tasks with the same binaries (the listed states of
   `crates/agent-tasks/status.txt` play no part):

       slot task agent-tasks-archive DIR=<dir>/surf-<version>-x86_64-unknown-linux-musl

   Its first line, `agent-tasks: <n> tasks, <p> pass, <f> fail, <s> skipped, lane archive,
   binaries <dir>`, goes into the notes (step 9) as `agent tasks (scripted): <n> tasks, <p>
   pass, <f> fail, <s> skipped`. `<dir>` is `DIR` as typed; an absolute `DIR` is named by its
   last component (`surf-<version>-x86_64-unknown-linux-musl`), the home directory itself and
   `/` by `-`, so the line holds no path of this machine and no user's name. The model lane's
   line names its binaries the same way (`target/debug`).

   `scan-artifact` reads ELF only, so the macOS binaries are covered by the workflow's own
   home-path check and not by it; whether that is accepted for surf is open, the operator's to
   decide.
9. **Release.** Write the notes from the annotated tag to a file, add the line of the agent
   tasks of step 8 to it, scan it, then create the release as the bot with the five files, and
   view it. When the model lane (`task agent-tasks-model MODEL=<model>`) was run on the release
   commit, the notes also carry `agent tasks: <n> tasks, <p> pass, <f> fail, <s> skipped, model
   <model>, <YYYY-MM-DD>`, the numbers, model and date copied from the first line of
   `.out/agent-tasks-model.txt`:

       git tag -l --format='%(contents)' <version> > <notes-file>
       $gates scan-text <notes-file>
       $gates gh -- release create <version> --verify-tag --title "surf <version>" \
         --notes-file <notes-file> \
         <dir>/surf-<version>-x86_64-unknown-linux-musl.tar.gz \
         <dir>/surf-<version>-aarch64-unknown-linux-musl.tar.gz \
         <dir>/surf-<version>-x86_64-apple-darwin.tar.gz \
         <dir>/surf-<version>-aarch64-apple-darwin.tar.gz \
         <dir>/SHA256SUMS
       $gates gh -- release view <version> --json author,tagName,isDraft,assets

   The release is done when it is owned by `b10x-bot[bot]`, names the tag `<version>`, is not a
   draft, and holds the four archives and `SHA256SUMS`. A green tag run with no release is
   queued, not released. When an upload is interrupted, attach what is missing with
   `$gates gh -- release upload <version> <file>...`; it does not touch the notes.

A refused scan creates no release: fix the cause on `main` and release the next patch version.

The trust anchors of `surf-tls` are refreshed with `task anchors-refresh`, whose `.sha256` comes
from the same server as the list and proves an uncut download only, so the person who refreshes
reads the titles it prints as added and removed before committing the new list.
