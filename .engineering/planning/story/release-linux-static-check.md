---
format: aep.planning-md/3
id: story:release-linux-static-check
kind: story
status: implemented
title: Check in the release workflow that each Linux archive holds static binaries and runs one in a container
summary: A script of its own, run by the workflow before the upload and by a workstation, fails a Linux archive whose binary has an INTERP header, a NEEDED entry or no ELF form, and runs surf-agent help --for-agent in an Alpine container named by digest; a gate test with made ELF files, a stand-in docker and twelve planted defects gives both sides of it.
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
  path: taskfiles/release-static-test.sh
- confidence: cited
  path: taskfiles/release-static.sh
- confidence: cited
  path: taskfiles/release.yml
revision: 14
transitions:
- {from: "draft", to: "proposed", at: "2026-11-26T07:31:43Z", actor: "human:operator", revision: 11}
- {from: "proposed", to: "active", at: "2026-11-26T07:31:43Z", actor: "human:operator", revision: 12, executor: "agent:implementor"}
- {from: "active", to: "implemented", at: "2026-11-26T10:01:51Z", actor: "human:operator", revision: 14, decided_on: {"recorded":{"test_result":3,"verification":1}}, executor: "agent:coordinator"}
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
  unpacked directory, on the runner itself (`release.yml:211-228`); the step after it uploads
  the archive (`release.yml:229`). The runner has glibc, so a binary that needs glibc runs there
  too. Nothing runs a binary in a container without glibc.
- The runners' tools: `release.yml:189` and `:195` call `readelf` and `strings` in this job, which
  runs on both Linux images (`ubuntu-22.04`, `ubuntu-22.04-arm`, `:98` and `:100`), so binutils
  is on them. `file` is called nowhere in the repository, so the story does not use it.

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

1. What "static" means. A Linux binary is static when `readelf -lW` lists no program header row
   of type `INTERP` (the row starts with the type, `^ *INTERP`) and `readelf -d` lists no
   `(NEEDED)` entry. `readelf -d` of a binary with no dynamic section prints that it has none
   and passes; a static-pie binary has a dynamic section without `NEEDED` and passes. A
   non-zero status of `readelf` on a binary fails that binary: a file that is no ELF file prints
   no `INTERP` row, and a check on the text alone would pass it. A binary that has both an
   `INTERP` row and a `NEEDED` entry gets the `INTERP` line only. No `ldd` (it can run the
   binary through the host's loader) and no `file` (not known to be on the runner images).
2. The check is a script of its own, `taskfiles/release-static.sh <dir> [--image <ref>]`, with
   the task `release-static` (`DIR`, optional `IMAGE`) in `taskfiles/release.yml`, not lines of
   `release-smoke.sh`. The smoke script builds and runs a cargo test (`release-smoke.sh:236-238`)
   and needs `openssl` (`:150-158`), so no gate test can run it; the static check needs only
   `readelf`, so a gate test can run it on made files. Like `release-smoke.sh:49` it resolves
   `<dir>` to an absolute path before it uses it. It takes the four names of a Linux archive in
   this order, `surf surf-agent surfd surf-session` (the order of `release.yml:163`), checks all
   four before it exits, and writes every value with `printf`, never `echo`
   (`taskfiles/sh-echo.sh:2-3`: dash's `echo` expands backslashes). Its lines, `<dir>` being the
   resolved path and `<name>` one of the four:
   - `release-static: <dir>/<name>: INTERP header (dynamically linked)`;
   - `release-static: <dir>/<name>: NEEDED entry (dynamically linked)`;
   - `release-static: <dir>/<name>: not an ELF file readelf can read` (also for a missing file);
   - at the end, `release-static: <dir> holds 4 static binaries: surf surf-agent surfd
     surf-session` and exit 0, or exit 1 after the lines above; exit 2 for a usage error.
3. The workflow calls the script in one new step, between the step that smoke-tests the
   unpacked archive and the `upload-artifact` step of the job `package` (`release.yml:229`),
   with `if: contains(matrix.target, '-linux-')`, on
   `"$RUNNER_TEMP/unpacked/$ARCHIVE-$VERSION-$TARGET"` (the directory that smoke step unpacked,
   `release.yml:217-219`). Why before the upload: a step after it would upload a dynamic archive
   before it fails; a failing step before it skips the upload, and the job `checksums` needs
   `package` (`release.yml:237`). The step comes after the smoke step by choice: one place, the
   archive's own content, checked once. The inline `INTERP` check at `release.yml:187-194` goes:
   the script's check contains it, and one rule has one place. No comment of the workflow names
   the script, so one line of the file names it.
4. The container runs `surf-agent help --for-agent`: no flag is added. With `--image <ref>` the
   script runs `docker run --rm --network none --volume <dir>:/archive:ro <ref>
   /archive/surf-agent help --for-agent`, `<dir>` being the resolved absolute path (a relative
   path is a named volume to docker). It requires exit 0 and the first line
   `surf-agent <version>: a definition for agents`, with `<version>` read from the directory's
   name as `release-smoke.sh:257` reads it, and prints `release-static: surf-agent help
   --for-agent ran in <ref>: surf-agent <version>`. It runs the container only after all four
   binaries are static. Alpine has musl, so the container run alone does not prove a binary
   static: a binary linked against glibc fails there, a binary linked dynamically against musl
   does not. The `readelf` check catches that one; the two together are the check.
5. The image is named by digest. The script refuses a `<ref>` that does not end in `@sha256:`
   and 64 hex digits (exit 2, before it starts docker). The workflow holds it as one value,
   `STATIC_IMAGE: alpine@sha256:<digest>`, in the `env:` block beside `PACKAGE` and `ARCHIVE`
   (`release.yml:21-27`), with the tag it was read from in a comment on the line above the
   value (a trailing comment, as `release.yml:41` has, would break the `$` of item 3). The
   digest of a multi-arch index serves both runners. `docker` absent or failing makes the step
   fail: the step never skips the container. The variable `DOCKER` names the program (default
   `docker`) so the test can put a stand-in there. Whether `ubuntu-22.04-arm` has docker is not
   verified; decision 8 says what follows.
6. The test is `taskfiles/release-static-test.sh [<script>]`, run by the task
   `release-static-test` (`SCRIPT` variable as `planning-only.yml:18-19`) and listed in `check`
   after `test-layout-test` (`Taskfile.yml:583`). The test builds nothing for musl and starts
   no container. It is a POSIX sh script that writes five small ELF64 files with `printf`
   (octal escapes) under `.out/release-static-test/surf-0.0.0-x86_64-unknown-linux-musl/`, a
   name the version read accepts. Each file has the 64-byte header (little-endian, no section
   headers) and 56-byte program headers; none was run through `readelf` by the author of this
   story, so the layout below is not verified and the guard step decides:
   - static: `ET_EXEC`, one `PT_LOAD`;
   - interp: `PT_LOAD` and `PT_INTERP`, whose offset names the string `/lib/ld-musl-x86_64.so.1`
     in the file;
   - needed: `ET_DYN`, a `PT_LOAD` that covers the file with `p_vaddr` equal to `p_offset`, and
     a `PT_DYNAMIC` over `DT_NEEDED`, `DT_STRTAB`, `DT_STRSZ`, `DT_NULL` and a string table;
   - static-pie: `ET_DYN`, a `PT_LOAD`, and a `PT_DYNAMIC` that holds `DT_NULL` only;
   - both: `PT_INTERP` and the dynamic part of needed.
   A guard step runs `readelf` on each file first and stops the test when a file does not read
   as named (the `INTERP` row only in interp and both; the `(NEEDED)` entry only in needed and
   both), so a malformed file cannot pass a case for the wrong reason. A stand-in `docker` (a
   shell script that records its arguments and prints the line and exit status the case sets)
   stands for the container. The table asserts, for each case, the exit status, a text the
   output holds (and one it does not hold where the case says so) and, for the container
   cases, the recorded arguments: `run`, `--rm`, `--network none`, `--volume` with the
   absolute directory and `:ro`, and the image. That differs from `planning-only.yml:45-52`,
   which compares the exit status only. Each rule of the script stands on a line of its own,
   and the test makes one copy of the script for each planted defect of item 2 with `sed`
   inside the task, failing when an edit finds no line, as `sh-echo.yml:61-64` does. Cost, by
   what it runs: 16 cases on the script and on 12 planted copies, 208 case runs of at most
   eight `readelf` calls and one `sh` each; no cargo, no network, no container; the
   implementor times the task and names a time over 5 s.
7. Words. `docs/releasing.md:44-46` names the new check in place of the `INTERP` sentence and
   gives the reason (a host with an older glibc than the build's cannot run a glibc binary);
   step 8 of "Cutting a release" adds `sh taskfiles/release-static.sh <dir>/surf-<version>-
   x86_64-unknown-linux-musl` beside the smoke command (`:165-167`). `README.md:38` and
   `website/docs/try-it.md:19` keep "statically linked": the claim now has a check. No entry in
   `CHANGELOG.md`: it lists what a release's users can see (`CHANGELOG.md:1-6`), `AGENTS.md`
   has no rule that a workflow check gets one, and no binary changes.
8. The workflow change is proved by one manual run on the story's branch (`workflow_dispatch`;
   it publishes nothing, `release.yml:4-5`), as `story:one-test-binary-per-crate` did for its
   workflow lines. If that run shows the step failing on `ubuntu-22.04-arm` because the image
   has no docker, one change follows and no other: the workflow passes `--image` only when
   `matrix.target` is `x86_64-unknown-linux-musl`, the aarch64 archive is checked by `readelf`
   alone, decision 5 and `docs/releasing.md` say so, and the run is made again once.

## Change

- `taskfiles/release-static.sh` (new): decisions 1, 2, 4 and 5.
- `taskfiles/release-static-test.sh` (new): decision 6.
- `taskfiles/release.yml`: the tasks `release-static` and `release-static-test` (decisions 2
  and 6).
- `Taskfile.yml`: `- task: release-static-test` after `test-layout-test`, in `check`
  (`:583`).
- `.github/workflows/release.yml`: the `STATIC_IMAGE` value, the new step, the inline check
  removed (decisions 3 and 5).
- `docs/releasing.md`: decision 7.
- `AGENTS.md`, four places, each worded as item 7 greps for it: the list of task files
  (`:147-149`), the release paragraph (`:1483-1494`), the sentence that lists what `check`
  ends with (`:1614-1626`), and the commands (`:1859-1861`).

The script and the workflow lines hold neither `overflow-checks` nor `rustflags` with an `@`:
`taskfiles/lock-check.sh:38-43` fails a file of `taskfiles/` or `.github/workflows/` that does
(`:127`), and the two new scripts are such files.

## Acceptance

1. `slot task release-static-test` ends `release-static-test: 16 case(s), each decided as
   expected by taskfiles/release-static.sh`, exit 0; the gate runs the table this way. Each
   case holds an exit status, a text and, for the container cases, the stand-in's recorded
   arguments (decision 6). The cases, with the fixtures named in decision 6:
   1. four static files (0);
   2. four static-pie files (0);
   3. `surf-session` is interp (1, the output holds the `INTERP` line naming `surf-session`);
   4. `surfd` is needed (1, the `NEEDED` line naming `surfd`);
   5. `surf-agent` is a text file (1, the "not an ELF file" line);
   6. no `surf` in the directory (1, the same line naming `surf`);
   7. `surf` and `surfd` are both dynamic (1, the output names both);
   8. an image given as a tag only (2, the stand-in is not called);
   9. an image by digest, the stand-in prints the directory's version (0, the arguments hold
      `run`, `--rm`, `--network none`, `--volume <absolute dir>:/archive:ro` and the image);
   10. the same with another version (1);
   11. the same with a stand-in that prints the right line and exits 125 (1);
   12. `DOCKER` names no program (1);
   13. `surf-session` is interp and an image is given (1, the stand-in is not called);
   14. an unknown option (2);
   15. a relative directory and an image by digest (0, the recorded `--volume` holds the
       absolute path);
   16. `surf` is both (1, the output holds the `INTERP` line and no `NEEDED` line).
   On the base the task does not exist and `task` says so; a case decided otherwise prints
   `release-static-test: <case>: exit <n>, expected <m>` and the task exits 1.
2. Twelve planted defects, each a `sed` copy made inside the task, each run through the table,
   each turning the named cases red (the task fails when a copy stays green or an edit finds
   no line): `INTERP` test removed (3, 13, 16); `NEEDED` test removed (4); status of `readelf`
   not read (5, 6); the loop stops at the first bad binary (7); digest form not required (8);
   container run before the static check (13); status of docker not read (11); version not
   compared (10); an absent docker skips the container (12); an unknown option accepted (14);
   `--network none` left out (9); `<dir>` not resolved before the mount (15). Case 13's and
   case 11's stand-ins print the right first line, so only the rule under test can turn them
   red.
3. In `.github/workflows/release.yml`, the five commands below. On the base the first prints
   nothing, the second prints `release.yml:189`, the third prints nothing (a guard: it holds
   after the change too), and the two awk programs exit 1. Each awk program is one command,
   its lines broken at the ends of its rules to keep 100 columns.
   - `git grep -n -E '^  STATIC_IMAGE: alpine@sha256:[0-9a-f]{64}$' \
     -- .github/workflows/release.yml` prints one line;
   - `git grep -n INTERP -- .github` prints nothing;
   - `git grep -c linux-gnu -- .github/workflows/release.yml` prints nothing;
   - the order: exit 0 and prints `<smoke> <call> <upload> 1` with smoke < call < upload (the
     base prints `211  229 `, no call):

         awk '/^      - name: Smoke-test the unpacked archive$/ { s = NR }
         /release-static\.sh/ { c = NR; n++ }
         /uses: actions\/upload-artifact@/ && !u { u = NR }
         END { print s, c, u, n; exit !(n == 1 && s < c && c < u) }' \
           .github/workflows/release.yml

   - the step that calls the script has the Linux `if:` and passes `--image "$STATIC_IMAGE"`
     (exit 0, prints nothing; the base has no such step):

         awk 'function f() { if (k) { z = 1; ok = (w && m) } }
         /^      - / { f(); k = w = m = 0 }
         /^        if: contains\(matrix\.target, .-linux-.\)$/ { w = 1 }
         /release-static\.sh/ { k = 1 }
         /--image "\$STATIC_IMAGE"/ { m = 1 }
         END { f(); exit !(z && ok) }' .github/workflows/release.yml

   Both programs were run on `53ab365` (the results above) and on edited copies of that file
   (the step inserted after the smoke step: both exit 0; inserted after the upload: the first
   exits 1; without the `if:` or without `--image`: the second exits 1; the script named in
   a second line: the first exits 1).
4. On a Linux host whose default target links glibc dynamically, `slot cargo build --locked -p
   surf-cli -p surf-agent -p surfd -p surf-session --bins` and the four files of `target/debug`
   copied into one directory named `surf-0.0.0-x86_64-unknown-linux-gnu`: `sh
   taskfiles/release-static.sh <dir>` prints four lines, in the order `surf`, `surf-agent`,
   `surfd`, `surf-session`, each ending `INTERP header (dynamically linked)`, and exits 1. Run
   once by the implementor, not a step of the gate; the base has no script.
5. On the unpacked archive of the manual run of decision 8, on a Linux x86_64 host:
   `sh taskfiles/release-static.sh <dir>` prints `release-static: <dir> holds 4 static
   binaries: surf surf-agent surfd surf-session` and exits 0; with `--image
   alpine@sha256:<digest>` on a host that has docker it also prints `release-static:
   surf-agent help --for-agent ran in alpine@sha256:<digest>: surf-agent <version>` and exits 0.
6. The manual run of decision 8: its two Linux `package` jobs print both lines of item 5 in the
   new step, its two macOS jobs show that step as skipped, and the run ends `success`. The
   evidence holds the run's address and its conclusion. A runner image without docker fails the
   step: that is red, not a skip, and decision 8 says the one change that follows.
7. The words, each place checked on its own (on the base the first prints `:46`, the rest
   print nothing):
   - `git grep -n -F 'shows an `INTERP` header' -- docs/releasing.md` prints nothing;
     `git grep -c release-static -- docs/releasing.md` prints `docs/releasing.md:` and 2 or more;
   - `git grep -c -F 'the tasks `release-static` and `release-static-test`' -- AGENTS.md` prints
     `AGENTS.md:1` (the list of task files);
   - `git grep -c -F 'release-static.sh` (task `release-static`' -- AGENTS.md` prints
     `AGENTS.md:1` (the release paragraph);
   - `git grep -c -F 'Since `story:release-linux-static-check`' -- AGENTS.md` prints
     `AGENTS.md:1` (the sentence on `check`);
   - `git grep -c -F -e '- `task release-static DIR=' -- AGENTS.md` prints `AGENTS.md:1` (the
     commands).
8. Guards, true on the base and after: `git diff 53ab365 -- taskfiles/release-smoke.sh
   CHANGELOG.md README.md website/docs/try-it.md` prints nothing, where `53ab365` is the commit
   the story's branch started from if `main` has moved.
9. The folder's bounds: the test step of `check` ends within 120 s and no test runs over 60 s;
   a test over 5 s is named with its time (the table's time, item 1); no test makes a network
   request or starts a container, no cargo call is added to the test step, and no Python is
   written; every compiling or testing command goes through `slot`, a script that measures or
   waits (the manual run's watching, a timing) starts without it, and a pool job ends within 10
   minutes: item 4's build names its time with `slot --timeout` if the shim's default is too
   short, and a job the daemon ended is not started again unchanged.

## Out of scope

- An allocator crate for musl: `AGENTS.md:30-31` allows no third-party crate in what ships.
- A `--version` flag on any binary: the first line of `surf-agent help --for-agent` is the
  version, and no flag is needed to run a binary in the container.
- A gnu archive, macOS archives, and the other three binaries in the container: the order asks
  for one binary to run.
- A container run in `task check` or in any test.

## Placement

No edge. Three active stories write files this story writes, and each overlap is textual:
`story:agent-cookie-public-suffixes` writes `docs/releasing.md` in a step before the tag
(`agent-cookie-public-suffixes.md:122-123`) and adds a step to `check` in `Taskfile.yml`
(`:121`); `story:tests-without-wall-time-bounds` adds `test-clocks-check-test` to `check` after
`doc-tests-check-test`, two lines below this story's insert after `test-layout-test`, and edits
the same two regions of `AGENTS.md` (its Placement, `tests-without-wall-time-bounds.md:785-789`);
`story:test-step-holds-with-tls` writes `Taskfile.yml` and `AGENTS.md` too. No edge: the story
that merges second keeps both sides of the lists in its merge of `main`. In all, 13 other open
stories name `AGENTS.md` in their scope lists (7 active, 3 draft, 3 proposed, counted at the
plan store's commit `78d57a6`), so a textual merge there is to be expected. No other open story
writes `.github/workflows/release.yml`, `taskfiles/release.yml` or `taskfiles/release-smoke.sh`.
The story needs the image's digest (decision 5) before it is implemented.

## As built

The unit is on `main` at `77957d0` (the merge of `impl/release-linux-static-check` at
`c1fc89a`). It changes 7 files, 813 lines added and 14 removed: two new scripts under
`taskfiles/`, the two tasks, the workflow, `docs/releasing.md` and `AGENTS.md`. No Rust file,
`README.md`, `CHANGELOG.md`, `website/` or `taskfiles/release-smoke.sh` changes.

- What was built.
  - `taskfiles/release-static.sh <dir> [--image <ref>]` (116 lines). For `surf`, `surf-agent`,
    `surfd`, `surf-session`, in that order and all four before it exits, it runs `readelf -lW`
    and `readelf -d` and fails a binary that has an `INTERP` row, a `(NEEDED)` entry, or a
    `readelf` status other than 0 (a text file, a missing file); a binary with both gets the
    `INTERP` line only. A static-pie binary and a binary with no dynamic section pass. Lines:
    `<dir>/<name>: INTERP header (dynamically linked)`, `... NEEDED entry (dynamically
    linked)`, `... not an ELF file readelf can read`, each after `release-static: `; at the end
    `release-static: <dir> holds 4 static binaries: surf surf-agent surfd surf-session`.
    Exit 0, 1, or 2 for a usage error. `<dir>` is resolved to an absolute path first; every
    value is written with `printf`.
  - The image. With `--image` and only when all four are static, the script runs
    `$DOCKER run --rm --network none --volume <dir>:/archive:ro <ref> /archive/surf-agent help
    --for-agent` (`DOCKER` defaults to `docker`). It requires exit 0 and the first line
    `surf-agent <version>: a definition for agents`, the version read from the directory's name,
    and prints `release-static: surf-agent help --for-agent ran in <ref>: surf-agent <version>`.
    A `<ref>` that does not end in `@sha256:` and 64 hex digits, an empty value included, is a
    usage error (exit 2) before docker starts; `--image` given never skips the container, and
    docker absent or failing fails the check. In the workflow the image is one value in `env:`,
    beside `PACKAGE` and `ARCHIVE`, with its tag in a comment on the line above:

        STATIC_IMAGE: alpine@sha256:f59abe656b692d901e184be3445e084786602108599d2a51d229c0f5a48c71bf

    The tag is `alpine:3.24.4`; the digest is that of its multi-arch index (amd64 and arm64).
  - `taskfiles/release-static-test.sh [<script>]` (589 lines). It writes five ELF64 files with
    `printf` octal escapes under `.out/release-static-test/` (`static` 120 bytes, `interp`
    201, `needed` 249, `static-pie` 200, `both` 337) and a text file. A guard runs `readelf`
    on each first and stops the test when a file does not read as named (the `INTERP` row only
    in `interp` and `both`; the `(NEEDED)` entry only in `needed` and `both`; a dynamic section
    only in `needed`, `static-pie` and `both`; no warning or error). All five read as named on
    the first run; no layout change was needed. A stand-in `docker` (a shell script) records its
    arguments, prints the first line and the status a case sets, and can write a line on stderr
    first. The 18 cases are functions; each holds an exit status, a text the output holds (one
    it does not hold where the case says so) and, for the container cases, whether the stand-in
    was called and its arguments (`run`, `--rm`, `--network none`, `--volume <absolute
    dir>:/archive:ro`, the image and the command). Cases 1 to 16 are the story's. Case 17: an
    empty `--image` value, exit 2, stand-in not called, no "static binaries" line. Case 18: the
    stand-in writes `Unable to find image ... locally` on stderr before the definition, exit 0
    and the "ran in" line.
  - 14 planted defects, each a `sed` copy of the script made inside the test. Each copy runs the
    cases its defect names, which must be decided otherwise, and case 1 (four static files, no
    image), which must be decided as expected, so that a copy that does not run at all fails;
    an edit that finds no line (a copy equal to the script) fails the test. The defects, with
    the cases they name: `INTERP` test removed (3, 13, 16); `NEEDED` test removed (4); status of
    `readelf` not read (5, 6); the loop stops at the first bad binary (7); digest form not
    required (8); container run before the static check (13); status of docker not read (11);
    version not compared (10); an absent docker skips the container (12); an unknown option
    accepted (14); `--network none` left out (9); `<dir>` not resolved before the mount (15);
    an empty image value taken as no image (17); docker's stderr read with its stdout (18).
    The stand-ins of cases 11 and 13 print the right first line, so only the rule under test
    turns them red.
  - The workflow, `.github/workflows/release.yml`: the `STATIC_IMAGE` value; one new step "Check
    that the Linux binaries are static and run in a plain container" between the smoke step and
    the `upload-artifact` step of the job `package`, with `if: contains(matrix.target,
    '-linux-')`, calling `sh taskfiles/release-static.sh
    "$RUNNER_TEMP/unpacked/$ARCHIVE-$VERSION-$TARGET" --image "$STATIC_IMAGE"`; the inline
    `INTERP` check of the step "Build, test and package" is removed.
  - The task lines. `taskfiles/release.yml`: `release-static` (`DIR`, optional `IMAGE`) and
    `release-static-test` (optional `SCRIPT`). `Taskfile.yml`: `- task: release-static-test` in
    `check`, after `test-layout-test` and before `sh-echo-test`.
  - The sentences. `docs/releasing.md`: the `INTERP` sentence is replaced by the check and the
    reason (a host with an older glibc than the build's cannot run a binary linked against
    glibc), and step 8 of "Cutting a release" gains `sh taskfiles/release-static.sh <dir>/surf-
    <version>-x86_64-unknown-linux-musl` beside the smoke command. `AGENTS.md`, four places:
    the list of task files, the release paragraph, the sentence on what `check` runs ("Since
    `story:release-linux-static-check` it runs `release-static-test` after `test-layout-test`,
    before `sh-echo-test`") and the commands. `README.md` and `website/docs/try-it.md` keep
    "statically linked"; no `CHANGELOG.md` entry.
- Acceptance, at `c1fc89a` unless a line says otherwise.
  1. Shown. `slot task release-static-test` ends `release-static-test: 18 case(s), each decided
     as expected by taskfiles/release-static.sh`, exit 0 (the story's text says 16, deviation
     1). Red before: the 16 cases against an empty script, 16 of 16 red; the base has no such
     task (`git show 53ab365:taskfiles/release.yml | grep -c release-static` prints 0). A case
     decided otherwise prints `release-static-test: <n>. <name>: exit <n>, expected <m>`.
  2. Shown, with 14 defects in place of 12 (deviation 1): the same run prints `14 planted
     defect(s), each caught by the cases it names; 49 script runs in all`. An empty script,
     a copy that does not run and an edit that finds no line were each shown to fail the test.
  3. Shown at `c1fc89a` by the five commands: the `STATIC_IMAGE` grep prints one line (`:30`),
     `git grep -n INTERP -- .github` and `git grep -c linux-gnu` print nothing, the order
     program prints `206 237 238 1` and exits 0 (base: `211  229 `), the step program exits 0.
  4. Shown, once, at the implementation commit: `slot --timeout 30m cargo build --locked -p
     surf-cli -p surf-agent -p surfd -p surf-session --bins` (exit 0), the four files of
     `target/debug` copied to a directory named `surf-0.0.0-x86_64-unknown-linux-gnu`;
     `sh taskfiles/release-static.sh <dir>` printed four `INTERP header (dynamically linked)`
     lines in the order `surf`, `surf-agent`, `surfd`, `surf-session` and exited 1.
  5. Shown by the unpacked archives of the manual run (item 6): the two Linux archives were
     downloaded, `sha256sum -c SHA256SUMS` printed `OK` for all four archives, and `sh
     taskfiles/release-static.sh <unpacked directory>` printed `holds 4 static binaries: surf
     surf-agent surfd surf-session` and exited 0 for the `x86_64` and for the `aarch64`
     archive. The half with `--image` on a machine with docker was not run there (no container
     program); the two runner jobs of item 6 print that line.
  6. Shown by the manual run of the workflow, run 45780781969 on `c1fc89a`
     (`https://github.com/beyond10x/surf/actions/runs/45780781969`): conclusion `success`,
     every job `success`. In the two Linux `package` jobs the new step is `success` and prints
     `release-static: <runner temp>/unpacked/surf-0.9.0-<target> holds 4 static binaries: surf
     surf-agent surfd surf-session`, then the pull of the image by its digest and
     `release-static: surf-agent help --for-agent ran in alpine@sha256:f59abe65...c71bf:
     surf-agent 0.9.0`, for `x86_64-unknown-linux-musl` and `aarch64-unknown-linux-musl`. The
     two jobs pulled different layers (`881fc932939e`, `b90627b8f429`) of the one digest. In
     the two macOS jobs the step is `skipped`. The `ubuntu-22.04-arm` image has docker, so the
     one change of decision 8 did not follow.
  7. Shown at `c1fc89a`: `git grep -n -F 'shows an `INTERP` header' -- docs/releasing.md`
     prints nothing; `git grep -c release-static -- docs/releasing.md` prints 3; the four
     `AGENTS.md` greps print `AGENTS.md:1` each.
  8. Shown: `git diff e4604fb c1fc89a` and `git diff 53ab365 cf4be06` of
     `taskfiles/release-smoke.sh`, `CHANGELOG.md`, `README.md` and `website/docs/try-it.md`
     print nothing. (`main` moved: `53ab365` to `c1fc89a` differs in `CHANGELOG.md` and
     `README.md` by other units.)
  9. Shown in part: no test over 60 s (the test runs 2.4 to 20.5 s, see the numbers), no
     network request, no container, no cargo call in the test step, no Python; the one build
     named its time with `slot --timeout 30m` and ran once; the time of the task is named
     below. Not shown: the 120 s of the whole test step of `check`, which no report of the unit
     measures.
- What differs from the story. Every decision, Change line and acceptance item not named here
  was built as written.
  1. 18 cases and 14 planted defects, not 16 and 12 (acceptance 1 and 2, decision 6, the
     story's summary). The review wrote cases 17 and 18 and the fix round the two defects that
     hold them; the texts that count (`AGENTS.md`, `taskfiles/release.yml`, the test's header
     and its output line) say 18 and 14.
  2. A planted copy runs the cases its defect names and case 1, not the whole table (decision
     6: 208 case runs). The test makes 49 script runs. Three defects showed other cases red
     as well on the whole table (defect 1: 7; defect 2: 7; defect 11: 10, 11, 15); with
     the named cases only, those are no longer run.
  3. A planted copy must also keep case 1 green (the story asks that the named cases turn red
     and that a copy that equals the script fails). A copy that does not run now fails.
  4. Where the story names no status or stream: a missing `<dir>` argument, `--image` without
     a value and a `<dir>` that is no directory exit 2; `readelf` not on `PATH`, and with
     `--image` a directory whose name gives no version, exit 1. The failure lines go to stderr
     and the two success lines to stdout; the test reads both.
- The review (one pass on `cf4be06`; verdict NEEDS-CHANGE, all findings introduced by the
  unit; no must):
  - 9 findings: 4 shoulds and 5 notes.
  - The 4 shoulds: `--image ''` was taken as no image, so the check ended green without the
    container (the story: "the step never skips the container"); a planted defect whose copy
    did not run at all counted as caught, because every case turned red, the named ones too;
    the 16 cases stayed green on a copy that reads docker's stderr with its stdout, which fails
    every release on a runner that pulls the image first; each planted defect ran the whole
    table, 43 of the 208 script runs giving the same proof.
  - The 5 notes: an ELF without program headers, a file without the execute bit and a
    symlink to a static file pass (the smoke step runs the binaries first and the job packs
    regular files with `cp`; nothing found reaches it); with `eu-readelf` as `readelf` a
    `NEEDED`-only file passes; a second `--image` replaces the first, and a `:` in the
    directory path is split by docker's `--volume` parser; the removed inline check printed
    `::error::` annotations and the script prints plain lines; with two more cases the texts that
    say sixteen cases need the new count.
  - Cases written (one commit, `b4da138`, 35 lines added and 2 removed in the test file only):
    case 17, red on the script (`exit 0, expected 2`), and case 18, green on the script and red
    on the copy with `2>&1`; and the rule that a copy keeps case 1 green. Run on the review's
    commit the suite was red on case 17 alone.
  - Fix round 1 (the merge `5f7c1e8` of the two cases, then `b348459`): the script keeps that
    `--image` was given, so an empty value is a usage error and case 17 is green; the two
    planted defects for cases 17 and 18; each copy runs its named cases and case 1; the cases
    became functions; the counts in the texts say 18 and 14. Red run after the merge: case 17
    only (`1 of 18 case(s) decided otherwise`). At `b348459`: `slot task release-static-test`
    exit 0, `slot task lock-check` exit 0, `sh -n` on both scripts exit 0.
  - The merge of `main` (`c1fc89a`, three other units): one conflict, in `AGENTS.md`, in the
    sentence on what `check` runs; both sides' "Since `story:...`" sentences are kept. `check`
    lists 30 steps, each once. Run again on the merge: `slot task release-static-test`,
    `lock-check`, `test-clocks-check` and `test-clocks-check-test` exit 0; `sh
    taskfiles/test-layout.sh .` exit 0 (26 test directories, 246 test files, one binary each).
    The unit's own change is the same 7 files.
- Numbers:
  - Cases: 0 before (the base has no task), 16 after the first commit, 18 at the tip. Planted
    defects: 12, then 14.
  - Script runs of the test: 208 at `cf4be06` (13 tables of 16 cases), 49 at `b348459` and at
    the tip (18 + 31: 17 named cases and case 1 in each of 14 copies). The fix round's commit
    body says "in place of 234" (18 cases x 13 tables, had the merged table been green); the
    measured figure of the first test is 208.
  - Time of `sh taskfiles/release-static-test.sh` started directly, with the 1-minute load on 20
    CPUs where a report gives it: first commit, 12.7 s (no load given); the review's three
    runs of that commit, 20,528 ms at load 50.49, 12,899 ms at 52.08, 16,166 ms at 47.91; the
    fix round's three runs, 2,434 ms at 23.62, 2,486 ms at 23.33, 2,545 ms at 23.33.
  - Time of `slot task release-static-test`: first commit 27.6 s (sync included); fix round
    16.4 s (sync and queue included, load 17.62); merge 10.1 s. One script run: 97 ms; the first
    test was over 5 s and its time was named; it is under 60 s. The direct time at `b348459` is
    2.4 to 2.5 s.
  - The new step in the manual run, from its first to its last log line: 08:26:25.50 to
    08:26:26.77 UTC (`x86_64`) and 08:18:28.16 to 08:18:29.34 UTC (`aarch64`).
  - Lines over 100 columns: 9 in `release-static.sh` (each rule on one line, decision 6), 4 in
    the test.
  - `Cargo.lock: 58 package(s), 25 from outside this repository, each on the allow-list`
    (`lock-check` at the fix round and at the merge).
- Left as it is (the review's notes that nobody changed):
  - An ELF with no program headers, a regular file without the execute bit and a symlink to a
    static file outside the archive pass the check; the packing step makes none of them.
  - With `eu-readelf` as `readelf` a `NEEDED`-only file passes (it prints `NEEDED` without
    parentheses); the runners use GNU binutils, and the test's guard stops under `eu-readelf`.
  - `--image` twice: the last wins, so a tag followed by a digest passes. A `:` in `<dir>` is
    split by docker's `--volume` parser; the stand-in cannot show docker's answer, and
    `RUNNER_TEMP` has no `:`.
  - The step prints plain lines on stderr; the removed inline check printed `::error::`
    annotations.
- A later story takes up: none; no report names one.
