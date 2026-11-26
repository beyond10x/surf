---
format: aep.planning-md/3
id: story:release-linux-static-check
kind: story
status: draft
title: Check in the release workflow that each Linux archive holds static binaries and runs one in a container
summary: A script of its own, run by the workflow and by a workstation, fails a Linux archive whose binary has an INTERP header or a NEEDED entry and runs surf-agent help --for-agent in an Alpine container named by digest; a gate test with made ELF files and a stand-in docker gives both sides of it.
scope:
- confidence: cited
  path: .github/workflows/release.yml
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: Taskfile.yml
- confidence: cited
  path: docs/releasing.md
- confidence: cited
  path: taskfiles/release-static.sh
- confidence: cited
  path: taskfiles/release.yml
revision: 7
---
## Context

The owner's decision for every Rust tool of the organization: Linux releases are static musl
archives, and the release workflow checks, for each Linux archive, that the binaries are static
and that one of them runs in a plain Alpine container. The reason: a build host with an older
glibc cannot run binaries linked against a newer one.

What surf has at `53ab365`:

- The matrix of the job `package` builds `x86_64-unknown-linux-musl` and
  `aarch64-unknown-linux-musl` for Linux, and no gnu target (`.github/workflows/release.yml:95-99`;
  `git grep -c linux-gnu` of that file prints nothing). That half of the decision holds.
- One static check exists: in the step "Build, test and package", for each Linux binary,
  `readelf -lW "$bin" | grep INTERP` fails the job (`release.yml:187-194`). It reads the binary
  in `target/ship`, before it is packed; it looks for no `NEEDED` entry; it lives in the workflow
  only, so a workstation cannot run it on an archive. `docs/releasing.md:44-46` says so.
- The step "Smoke-test the unpacked archive" runs `sh taskfiles/release-smoke.sh` on the
  unpacked directory, on the runner itself (`release.yml:211-228`). The runner has glibc, so a
  binary that needs glibc runs there too. Nothing runs a binary in a container without glibc.
- The runners' tools: `release.yml:189` and `:195` call `readelf` and `strings` in this job on
  both Linux images (`ubuntu-22.04`, `ubuntu-22.04-arm`, `:98` and `:100`), and
  `docs/design/tls.md:179` records both jobs green, so binutils is on them. `file` is called
  nowhere in the repository, so the story does not use it.

No binary of the four prints a version under a flag. `surf` reads an address, `--as`,
`--ca-file`, `--anchors` (`crates/surf-cli/src/main.rs:50-120`); `surfd` reads `--idle-exit` and
`--print-view-url` (`crates/surfd/src/main.rs:72-95`); `surf-session` reads two timeouts, an
identity and a socket path (`crates/session/src/process.rs:74-104`). The version is the first
line of `surf-agent help --for-agent`, `surf-agent <version>: a definition for agents`
(`crates/agent/src/help.rs:78-84`). That command asks no session and reads no environment
(`crates/agent/src/main.rs:27-30`), so it runs in a bare container. `release-smoke.sh:261-271`
already reads that line on the runner.

The sentences that describe the archives: `README.md:38` and `website/docs/try-it.md:19` say
"statically linked"; `docs/releasing.md:44-46` says "statically linked (musl)" and names the
`INTERP` check; `CHANGELOG.md:520` says it of the first release.

## Decisions

1. What "static" means, and the commands. A Linux binary is static when `readelf -lW` lists no
   program header of type `INTERP` and `readelf -d` lists no `(NEEDED)` entry. `readelf -d` of a
   binary with no dynamic section prints that it has none and passes; a static-pie binary has a
   dynamic section without `NEEDED` and passes. No `ldd` (it can run the binary through the
   host's loader) and no `file` (not known to be on the runner images). Binutils is on the
   runner images (Context) and on a workstation that builds this repository.
2. The check is a script of its own, `taskfiles/release-static.sh <dir> [--image <ref>]`, with
   the task `release-static` (`DIR`, optional `IMAGE`) in `taskfiles/release.yml`, not lines of
   `release-smoke.sh`. The smoke script builds and runs a cargo test and needs `openssl`
   (`release-smoke.sh:236-238`), so no gate test can run it; the static check needs only
   `readelf`, so a gate test can run it on made files. The script takes the four names of a
   Linux archive (`surf`, `surf-agent`, `surfd`, `surf-session`) and fails when one is missing;
   it prints one line for each binary that is not static, and exits 1 after checking all four.
3. The workflow calls the script in a new step after "Smoke-test the unpacked archive", with
   `if: contains(matrix.target, '-linux-')`, on `"$RUNNER_TEMP/unpacked/$ARCHIVE-$VERSION-$TARGET"`
   (the directory that step unpacked, `release.yml:217-219`): the archive's own content is what
   is checked. The inline `INTERP` check at `release.yml:187-194` goes: the script's check
   contains it, and one rule has one place.
4. The container runs `surf-agent help --for-agent`: no flag is added. With `--image <ref>` the
   script runs `docker run --rm --network none --volume <dir>:/archive:ro <ref>
   /archive/surf-agent help --for-agent`, requires exit 0 and the first line
   `surf-agent <version>: a definition for agents`, with `<version>` read from the directory's
   name as `release-smoke.sh:257` reads it, and prints one line naming the image. Alpine has
   musl, so the container run alone does not prove a binary static: a binary linked against
   glibc fails there, a binary linked dynamically against musl does not. The `readelf` check
   catches that one; the two together are the check.
5. The image is named by digest. The script refuses a `<ref>` that does not end in
   `@sha256:` and 64 hex digits (exit 2, before it starts docker). The workflow holds it as one
   value, `STATIC_IMAGE: alpine@sha256:<digest>`, in the `env:` block beside `PACKAGE` and
   `ARCHIVE` (`release.yml:21-27`), with the tag it was read from in a comment, as the actions
   are pinned by commit with their tag in a comment (`release.yml:41`). The digest of a
   multi-arch index serves both runners. `docker` absent or failing makes the step fail: the
   step never skips the container. The variable `DOCKER` names the program (default `docker`)
   so the test can put a stand-in there.
6. The test is a table of cases in `release-static-test`, in the form of
   `taskfiles/planning-only.yml:6-73` (a `SCRIPT=` variable runs a copy with a planted defect;
   one line per case decided otherwise; one line with the count), and a line of `check`
   after `test-layout-test` (`Taskfile.yml:583`). The test builds nothing for musl and starts no
   container. It writes four small ELF files with `printf` under `.out/release-static-test/`:
   static (one `PT_LOAD`), dynamic (`PT_LOAD` and `PT_INTERP`), a dynamic section with a
   `NEEDED` entry and no `INTERP`, and a static-pie (a dynamic section without `NEEDED`). Before
   the cases, a guard step runs `readelf` on each file and fails the test when the file does not
   read as named, so a malformed file cannot pass a case for the wrong reason. A stand-in
   `docker` (a shell script that records its arguments and prints a line the case sets) stands
   for the container. Cost, by what it runs: 13 cases of at most four `readelf` calls and one
   `sh`, no cargo, no network; well under 1 s.
7. Words. `docs/releasing.md:44-46` names the new check in place of the `INTERP` sentence and
   gives the reason (a host with an older glibc than the build's cannot run a glibc binary);
   the step 8 of "Cutting a release" adds `sh taskfiles/release-static.sh <dir>/surf-<version>-
   x86_64-unknown-linux-musl` beside the smoke command (`:165-167`). `README.md:38` and
   `website/docs/try-it.md:19` keep "statically linked": the claim now has a check. No entry in
   `CHANGELOG.md`: it lists what a release's users can see (`CHANGELOG.md:1-6`), `AGENTS.md`
   has no rule that a workflow check gets one, and no binary changes.
8. The workflow change is proved once on the forge by a manual run on the story's branch
   (`workflow_dispatch`; it publishes nothing, `release.yml:4-5`), as
   `story:one-test-binary-per-crate` did for its workflow lines.

## Change

- `taskfiles/release-static.sh` (new): decisions 1, 2, 4 and 5.
- `taskfiles/release.yml`: the tasks `release-static` and `release-static-test` (decisions 2
  and 6).
- `Taskfile.yml`: `- task: release-static-test` after `test-layout-test`, in `check`
  (`:583`).
- `.github/workflows/release.yml`: the `STATIC_IMAGE` value, the new step, the inline check
  removed (decisions 3 and 5).
- `docs/releasing.md`: decision 7.
- `AGENTS.md`, four places: the list of task files (`:147-149`), the release paragraph
  (`:1483-1494`), the sentence that lists what `check` ends with (`:1614-1626`), and the
  commands (`:1859-1861`), each naming the script, the two tasks, and what they check.

## Acceptance

1. `slot task release-static-test` ends `release-static-test: 13 case(s), each decided as
   expected by taskfiles/release-static.sh`, exit 0. The cases: four static binaries (0); four
   static-pie binaries (0); `surf-session` with an `INTERP` header (1, the output names
   `surf-session`); `surfd` with a `NEEDED` entry and no `INTERP` (1, names `surfd`);
   `surf-agent` that is a text file (1); no `surf` in the directory (1); an image given as a tag
   only (2, the stand-in `docker` is not called); an image by digest and a stand-in that prints
   the directory's version (0, the stand-in's arguments hold `run --rm --network none` and the
   image); the same with another version (1); with a stand-in that exits 125 (1); with `DOCKER`
   naming no program (1); a dynamic binary and an image (1, the stand-in is not called); an
   unknown option (2). On the base the task does not exist and `task` says so; a case decided
   otherwise prints `release-static-test: <case>: exit <n>, expected <m>` and the task exits 1.
2. Three planted defects, each in a copy named by `SCRIPT=` (the repository's form,
   `planning-only.yml:18-19`), each exits 1 and names its cases: the `INTERP` test removed
   (the `INTERP` case and the dynamic-with-image case); the `NEEDED` test removed (the `NEEDED`
   case); the digest requirement removed (the tag-only case). A guard of item 1, so no case can
   pass for the wrong reason.
3. In `.github/workflows/release.yml`: `git grep -n 'release-static.sh'` prints one line, in a
   step whose `if:` is `contains(matrix.target, '-linux-')`; `git grep -n -E
   'STATIC_IMAGE: alpine@sha256:[0-9a-f]{64}$'` prints one line; `git grep -n INTERP -- .github`
   prints nothing (the base prints `release.yml:189`); `git grep -c 'linux-gnu'` of the file
   prints nothing, on the base too (a guard: no gnu archive is added).
4. On a Linux host whose default target links glibc dynamically, `slot cargo build --locked -p
   surf-cli -p surf-agent -p surfd -p surf-session --bins` and the four files of `target/debug`
   copied into one directory named `surf-0.0.0-x86_64-unknown-linux-gnu`: `sh
   taskfiles/release-static.sh <dir>` prints four lines, one per binary, each ending
   `INTERP header (dynamically linked)`, and exits 1. Run once by the implementor, not a step of
   the gate; the base has no script.
5. On the unpacked archive of the manual run of decision 8, on a Linux x86_64 host:
   `sh taskfiles/release-static.sh <dir>` prints `release-static: <dir> holds 4 static
   binaries: surf surf-agent surf-session surfd` and exits 0; with `--image
   alpine@sha256:<digest>` on a host that has docker it also prints `release-static:
   surf-agent help --for-agent ran in alpine@sha256:<digest>: surf-agent <version>` and exits 0.
6. The manual run of decision 8: its two Linux `package` jobs print both lines of item 5 in the
   new step, its two macOS jobs show that step as skipped, and the run ends `success`. The
   evidence holds the run's address and its conclusion. A runner image without docker fails the
   step: that is red, not a skip.
7. `git grep -n -F 'shows an `INTERP` header' -- docs/releasing.md` prints nothing (the base
   prints `:46`); `git grep -n 'release-static' -- docs/releasing.md` prints at least two lines,
   `git grep -c 'release-static' -- AGENTS.md` at least four.
8. Guards, true on the base and after: `git diff <base> -- taskfiles/release-smoke.sh
   CHANGELOG.md README.md website/docs/try-it.md` prints nothing.
9. The folder's bounds: the test step of `check` ends within 120 s and no test runs over 60 s;
   the new table adds no cargo call, no network request and no container to it; the
   implementor's report names the table's time; no Python in the script, the task or the test;
   every compiling or testing command goes through `slot`, and each pool job ends within 10
   minutes (item 4's build names its time with `slot --timeout` if the shim's default is too
   short).

## Out of scope

- An allocator crate for musl: `AGENTS.md:30-31` allows no third-party crate in what ships.
- A `--version` flag on any binary: the first line of `surf-agent help --for-agent` is the
  version, and no flag is needed to run a binary in the container.
- A gnu archive, macOS archives, and the other three binaries in the container: the order asks
  for one binary to run.
- A container run in `task check` or in any test.

## Placement

No edge. `story:agent-cookie-public-suffixes` (active) also writes `docs/releasing.md`, in a step
before the tag (`agent-cookie-public-suffixes.md:122-123`), and adds a step to `check` in
`Taskfile.yml` (`:121`); this story writes "What a release holds" and step 8, and one line of
`check`. The merge order is free; a conflict would be textual. No other open story writes
`.github/workflows/release.yml`, `taskfiles/release.yml` or `taskfiles/release-smoke.sh`. The
story needs the image's digest (decision 5) before it is implemented.
