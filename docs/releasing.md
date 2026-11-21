# Releasing surf

A release is a bare version tag (`0.1.0`), annotated and written by the organization bot, on a
`main` commit whose `task check` is green. The tag is the version of the package that ships the
`surf` binary, `surf-cli` (`crates/surf-cli/Cargo.toml`); the release workflow fails on a tag
that differs.

## What a release holds

[`.github/workflows/release.yml`](../.github/workflows/release.yml) builds the binary with
`--locked`, warnings denied, runner paths remapped and symbols stripped, runs the command-line
tests against it, and packs it with `README.md`, `LICENSE` and `NOTICE`.

| target | runner | archive |
|---|---|---|
| `x86_64-unknown-linux-musl` | `ubuntu-22.04` | `surf-<version>-x86_64-unknown-linux-musl.tar.gz` |
| `aarch64-unknown-linux-musl` | `ubuntu-22.04-arm` | `surf-<version>-aarch64-unknown-linux-musl.tar.gz` |
| `x86_64-apple-darwin` | `macos-15-intel` | `surf-<version>-x86_64-apple-darwin.tar.gz` |
| `aarch64-apple-darwin` | `macos-15` | `surf-<version>-aarch64-apple-darwin.tar.gz` |

Each archive holds one directory, `surf-<version>-<target>/`, with `surf`, `README.md`,
`LICENSE` and `NOTICE`.
`SHA256SUMS` lists the four archives. The Linux binaries are statically linked (musl): they run
on any distribution, and they load no name service module from the host. The macOS binaries are
neither signed nor notarised.

The workflow publishes nothing and writes nothing to the repository: on a tag and on a manual
run alike, the four archives and `SHA256SUMS` are the run's artifacts (`release-<target>` and
`release-checksums`). A tag run also fails unless the tag is annotated and equals the package
version. The release is created by the organization bot, `b10x-bot[bot]`, from the artifacts of
the tag's run, after they are verified and scanned. No step uses `github.token` or a personal
account to write a release.

## Cutting a release

The commands use the gates as the bot, from a checkout of this repository:

    gates="b10x-gates --policy ../gates-policy/policy.json --repository beyond10x/surf"

1. **Version.** Set `version` in `crates/surf-cli/Cargo.toml` to the new version, then
   `cargo generate-lockfile --offline`. Release `0.1.0` shipped the binary from the package
   `surf`; `surf-cli` starts at that version, so the next release is above it.
2. **Changelog.** In `CHANGELOG.md`, rename `## [Unreleased]` to `## [<version>] - <date>` and
   put an empty `## [Unreleased]` above it. Commit both on `main`; `slot task check` is green on
   that commit.
3. **Try it.** Run the workflow by hand on `main`, wait for it, and take steps 6 and 7 on its
   run:

       $gates gh -- workflow run release.yml --ref main
       $gates gh -- run list --workflow release.yml --limit 1
       $gates gh -- run watch <run-id> --exit-status

4. **Tag.** Write the tag message (the version's changelog section) to a file, then:

       $gates bot -- tag -a <version> -F <message-file> <commit>
       $gates bot -- push origin <version>

5. **Build.** The push starts the workflow on the tag. Wait for that run to be green:

       $gates gh -- run list --workflow release.yml --branch <version> --limit 1
       $gates gh -- run watch <run-id> --exit-status

6. **Download and verify.** Download the tag run's artifacts by run id into an empty directory,
   check that the four archives are there, and check them against `SHA256SUMS`:

       $gates gh -- run download <run-id> --dir <dir>
       mv <dir>/release-*/* <dir>/
       ls <dir>/surf-<version>-{x86_64,aarch64}-unknown-linux-musl.tar.gz \
          <dir>/surf-<version>-{x86_64,aarch64}-apple-darwin.tar.gz <dir>/SHA256SUMS
       (cd <dir> && sha256sum --check SHA256SUMS)

7. **Scan.** Unpack the two Linux archives and scan their executables:

       tar -C <dir> -xzf <dir>/surf-<version>-x86_64-unknown-linux-musl.tar.gz
       tar -C <dir> -xzf <dir>/surf-<version>-aarch64-unknown-linux-musl.tar.gz
       $gates scan-artifact <dir>/surf-<version>-x86_64-unknown-linux-musl/surf
       $gates scan-artifact <dir>/surf-<version>-aarch64-unknown-linux-musl/surf

   `scan-artifact` reads ELF only, so the macOS binaries are covered by the workflow's own
   home-path check and not by it; whether that is accepted for surf is open, the operator's to
   decide.
8. **Release.** Write the notes from the annotated tag to a file, scan it, then create the
   release as the bot with the five files, and view it:

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
