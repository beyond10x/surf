# Design: TLS 1.3 for `https:`, its primitives and its tests

Decision note of `story:tls-decisions` (`epic:tls-https`), written on 2026-11-22. It records
the decisions U1 to U3 and their reasons, the packages ring adds, the toolchain each runner
needs, the cost of the primitives in the test profile, the crate layout, the failure reasons,
the extra anchor file, the bounds and the test design. Every later story of the epic cites its
items by number; no other story writes this file. A later story that finds an item wrong says
so in its report, and the coordinating agent revises the note.

What was read, all on 2026-11-22:

| text | version read |
|---|---|
| RFC 8446 (TLS 1.3) | `https://www.rfc-editor.org/rfc/rfc8446.txt`, August 2018, 337,736 bytes, sha256 `47871bc8820a2c3b6ea89f061055577058862cf543686b82d10131239702b3bd` |
| RFC 8448 (example handshake traces) | `https://www.rfc-editor.org/rfc/rfc8448.txt`, January 2019, 159,343 bytes, sha256 `6564d1376d1ec744fc7a9993da15ebc1b9be361908b166091f47ef605c537fba` |
| ring | 0.17.14, the crates.io package (checksum in item 2) |
| cc | 1.11.1, the crates.io package (checksum in item 2) |
| rustls, rustls-webpki | 0.23.46 and 0.103.17, the crates.io packages, for the bounds of item 9 only |
| OpenSSL | 3.6.5 of 29 September 2026, the `openssl` command of the workstation |

Line numbers below are those of these versions. Every `rfc8446.txt:` and `rfc8448.txt:` line
cited was checked against the copies fetched from the RFC Editor on 2026-11-22 with the bytes and
sha256 above (the section 9.3 sentence quoted in items 1 and 12, "A client sending a ClientHello
MUST support all parameters advertised in it", stands on lines 5834-5835; its bullet runs on to
5836). The workstation is the shared build machine the
coordinating agent works on (x86_64 Linux, GCC 16.2.1, musl 1.2.6); its builds and tests run
through the machine's build pool (`slot`), which queues a job until a slot is free; the times
below are wall times of the running job, not of the queue.

## 1. The decisions U1, U2 and U3, and the TLS 1.2 suites

1. **U1, the source of the primitives: `ring` 0.17.14.** The operator, 2026-11-22 05:47 CET,
   relayed by the session `host-operator`: "A: ring 0.17.14". Of the three options of the epic
   that take an outside crate it adds the fewest packages (item 2; option D, primitives written
   here, adds none and was refused for the timing risk decision C names), covers every primitive TLS 1.3 needs, and is the source
   rustls calls easier to build. Its cost is a C compiler that also builds assembly on every
   build (item 3), and principle 4's word "audited" means what exists: ring was reviewed as a
   dependency in Cure53's audit of rustls of May to June 2020 (work package WP2), with 4 minor
   findings and no vulnerability. It is the one exception principle 4 of the README plans; the
   lock check allows its packages by name, version and checksum and only for the crate
   `surf-tls` (`story:tls-lock-allow`).
2. **U2, the trust anchors: option B**, the root list of Mozilla as curl publishes it, compiled
   into `surf-tls` and refreshed before each release, plus one extra anchor file per session
   (item 8). Taken by the coordinating agent on 2026-11-22 and reported to the operator, who can
   overrule it. Reason: the same list on Linux and macOS, and reading the macOS keychain needs
   foreign calls, unsafe code that principle 5 forbids in a hand-written crate. Details in
   item 11.
3. **U3, TLS 1.2: option C**, out of this epic and drafted as the epic after it. Taken by the
   coordinating agent on 2026-11-22 and reported to the operator, who can overrule it. Reason:
   this epic stays the size of one protocol version, and about one trusted site in four offers
   TLS 1.2 as its best protocol (Qualys SSL Pulse of 2025-06-01: 33,183 of 134,380 sites,
   24.7 %), so the gap is closed next and not left open. Until then a server limited to TLS 1.2
   ends with one of the three kinds of item 7.
4. **The TLS 1.2 suites in the ClientHello: taken in and reversed on the same day.** The
   coordinating agent took six TLS 1.2 ECDHE AEAD suites into the ClientHello beside the three
   TLS 1.3 suites on 2026-11-22, and reversed it on 2026-11-22 after the fifth critic round,
   because RFC 8446 section 9.3 says "A client sending a ClientHello MUST support all parameters
   advertised in it" (`rfc8446.txt:5834-5835`), and surf does not support TLS 1.2. The
   ClientHello offers the three TLS 1.3 suites and nothing else (item 12).

## 2. The lock: the 19 packages ring adds

Measured on 2026-11-22 in a scratch workspace: the root workspace of surf (`Cargo.toml`, the
crates and `generated/rust/crates` as on the branch of this story) copied, plus one crate
`ring-scratch` whose one dependency is `ring = "=0.17.14"`. `cargo generate-lockfile --offline`
printed `Locking 19 packages to highest Rust 1.99.1 compatible versions`; `grep -c "^source = "
Cargo.lock` printed `19`, the 19 of the epic section "Measured: the packages ring adds". The
versions are the newest the local registry index held that day; a lock resolved later may take
newer patch releases of the eight, and the allow-list follows the lock that `story:tls-lock-allow`
commits, not this table. The dependents are those `cargo tree --offline -i <name> --target all
-e normal,build --depth 1` printed; `surf-tls` stands where the scratch crate stood.

Every source is `registry+https://github.com/rust-lang/crates.io-index`.

| package | version | checksum | depended on by | compiled for surf's targets |
|---|---|---|---|---|
| cc | 1.11.1 | `4ff2a8126fff0f8c839080d619675a585e61720bb12d2baceae45769fd66f155` | ring (build) | yes, for the build machine: the build script of ring |
| cfg-if | 1.0.5 | `4e7648175b45a9a48536d676f68d918270699102aa8dab5496df06904c914600` | getrandom, ring | yes |
| find-msvc-tools | 0.1.15 | `1c66915cc5391097de7a2b0b972e8505aa33c0df401fd3a6811dbdb32881ba48` | cc | yes, for the build machine |
| getrandom | 0.2.17 | `ff2abc00be7fca6ebc474524697ae276ad847ad0a6b3faa4bcb027e9a4614ad0` | ring | yes |
| libc | 0.2.191 | `65cd87c9a6d60536580b316ad985726104730a5151c99845f2490428ef91240a` | getrandom (Unix); ring for aarch64 Apple (`Cargo.toml:263`) and for aarch64 and arm Linux and Android (`Cargo.toml:267`), so for `aarch64-unknown-linux-musl` and `aarch64-apple-darwin` directly | yes |
| ring | 0.17.14 | `a4689e6c2294d81e88dc6261c768b63bc4fcdb852be6d1352498b114f61383b7` | surf-tls | yes |
| shlex | 2.0.1 | `f8fadd59c855ef2080decdef8ff161eb6661b86933c9d82e5ba29dc602a55aba` | cc | yes, for the build machine |
| untrusted | 0.9.0 | `8ecb6da28b8a351d773b68d5825ac39017e680750f980f3a1a85cd8dd28a47c1` | ring | yes |
| wasi | 0.11.1+wasi-snapshot-preview1 | `ccf3ec651a847eb01de73ccad15eb7d99f80485de043efb2f370cd654f4ea44b` | getrandom (`target_os = "wasi"`) | no |
| windows-sys | 0.52.0 | `282be5f36a8ce781fad8c8ae18fa3f9beff57ec1b52cb3de0789201425d9a33d` | ring (aarch64 Windows) | no |
| windows-targets | 0.52.6 | `9b724f72796e036ab90c1021d4780d4d3d648aca59e491e6b98e725b84e99973` | windows-sys | no |
| windows_aarch64_gnullvm | 0.52.6 | `32a4622180e7a0ec044bb555404c800bc9fd9ec262ec147edd5989ccd0c02cd3` | windows-targets | no |
| windows_aarch64_msvc | 0.52.6 | `09ec2a7bb152e2252b53fa7803150007879548bc709c039df7627cabbd05d469` | windows-targets | no |
| windows_i686_gnu | 0.52.6 | `8e9b5ad5ab802e97eb8e295ac6720e509ee4c243f69d781394014ebfe8bbfa0b` | windows-targets | no |
| windows_i686_gnullvm | 0.52.6 | `0eee52d38c090b3caa76c563b86c3a4bd71ef1a819287c19d586d7334ae8ed66` | windows-targets | no |
| windows_i686_msvc | 0.52.6 | `240948bc05c5e7c6dabba28bf89d89ffce3e303022809e73deaefe4f6ec56c66` | windows-targets | no |
| windows_x86_64_gnu | 0.52.6 | `147a5c80aabfbf0c7d901cb5895d1de30ef2907eb21fbbab29ca94c5b08b1a78` | windows-targets | no |
| windows_x86_64_gnullvm | 0.52.6 | `24d5b23dc417412679681396f2b49f3de8c1473deb516bd34410872eff51ed0d` | windows-targets | no |
| windows_x86_64_msvc | 0.52.6 | `589f6da84c646204747d1270a2a5661ea66ed1cced2631d546fdfb155959f9ec` | windows-targets | no |

Compiled for each of the four release targets, and for `x86_64-unknown-linux-gnu` of the gate:
the same 8, cc, cfg-if, find-msvc-tools, getrandom, libc, ring, shlex and untrusted, as `cargo
tree --target <t> -e normal,build` printed for each of `x86_64-unknown-linux-musl`,
`aarch64-unknown-linux-musl`, `x86_64-apple-darwin`, `aarch64-apple-darwin` and
`x86_64-unknown-linux-gnu`. Three of them, cc, find-msvc-tools and shlex, are build
dependencies: they run on the build machine to compile ring's C and assembly and are not in a
shipped binary; five ship: cfg-if, getrandom, libc, ring, untrusted. The other 11 belong to
Windows (ring declares `windows-sys` 0.52 for aarch64 Windows only, `Cargo.toml:256-261`) and
WASI (getrandom declares `wasi` 0.11 for `target_os = "wasi"`, `Cargo.toml:115-116`); cargo
records them in the lock and never downloads or builds them for surf's targets.

These 19 are the lines of `taskfiles/lock-allow.txt`, one per package: name, version, source,
checksum, in the order of the table.

## 3. The toolchain per runner

What the forge runs today (`.github/workflows/release.yml`, job `package`): the toolchain of
`rust-toolchain.toml` (rustc 1.99.1) through `dtolnay/rust-toolchain` with the target, no C
compiler step, and `readelf -lW` refusing an `INTERP` header on Linux. `check.yml` runs on
`ubuntu-latest` for `x86_64-unknown-linux-gnu` and installs no C compiler.

How ring finds its C compiler: its build script compiles its C and assembly with cc 1.11.1,
which reads `CC_<target>` (the target with `-` or `_`), then `TARGET_CC`, then `CC`. Without
them, for a target other than the build machine's, it picks by the target
(`cc-1.11.1/src/lib.rs:4342-4350`): for `x86_64-unknown-linux-musl` the first of
`x86_64-linux-musl-gcc` and `musl-gcc` found on `PATH`, else the name
`x86_64-linux-musl-gcc` (`:5098`, `:5137-5162`); for `aarch64-unknown-linux-musl` always
`aarch64-linux-musl-gcc` (`:4954`); for the build machine's own target `cc`. On macOS the
compiler is `cc` of the Xcode command line tools.

The probe: the branch `probe/tls-ring`, never merged into `main`, holds
`.github/workflows/tls-probe.yml` and `probe/tls-ring/`, a crate outside the root workspace with
its own `Cargo.lock` (the same 19 packages, versions and checksums as item 2). Its `main`
verifies one fixed ECDSA P-256 signature made by `openssl dgst -sha256 -sign`, refuses it over
another message, runs one X25519 agreement between two keys drawn from `SystemRandom` and prints
`probe ok`. Seven jobs: the four release targets on their runners with the flags of the job
`package` (`-D warnings`, the two `--remap-path-prefix`, `-C strip=symbols`), `cargo build
--locked --release --target <t>`, `readelf -lW` on Linux, the binary run; each musl target a
second time without the compiler step (the negative controls); and `ubuntu-latest` with
`x86_64-unknown-linux-gnu` and `-D warnings`, `cargo build --locked`, as the gate builds. Each
job builds ring alone first, timed, with `CC_ENABLE_DEBUG_OUTPUT=1`, and prints the compiler
cc ran. The compiler step under test, for a musl target:

```yaml
      - name: The C compiler step for musl
        run: |
          set -euo pipefail
          sudo apt-get update -qq
          sudo apt-get install -y -qq --no-install-recommends musl-tools
          echo "CC_${TARGET//-/_}=musl-gcc" >> "$GITHUB_ENV"
```

The run: `tls-probe` run 45000324518 on `probe/tls-ring` at commit
`a82e69072b8696e5bcff3f45f03c27c9b22f7a04`, event `push`, 2026-11-22 07:54 to 07:55 UTC
(https://github.com/beyond10x/surf/actions/runs/45000324518). The run concludes `failure`
because of the two negative controls, as designed; the five build jobs conclude `success`.
Every job installed rustc 1.99.1 (`58cf8f9c3 2026-10-13`) through `dtolnay/rust-toolchain`. The
lines below are quoted from the job logs; the build times are the probe's own `date +%s` around
each step, whole seconds.

| job (runner) | conclusion | what the runner offers | compiler cc ran | ring build | probe build | binary | run |
|---|---|---|---|---|---|---|---|
| `release x86_64-unknown-linux-musl` (`ubuntu-22.04`, Linux 6.8.0-1070-azure) | `success` | `cc`, `gcc`: GCC 11.4.0; `clang` 14.0.0; after the step `musl-gcc` and `x86_64-linux-musl-gcc` (musl-tools 1.2.2-4, wrappers of the GCC 11.4.0) | `[ring 0.17.14] CC_x86_64_unknown_linux_musl = Some(musl-gcc)`, `running: "musl-gcc" "-O3" …` | `ring build: exit 0, 14 s` | `probe build: 1 s` | `ELF 64-bit LSB pie executable, x86-64, version 1 (SYSV), static-pie linked, … stripped`; `readelf -lW: no program interpreter, the binary is static` | `probe ok` |
| `release aarch64-unknown-linux-musl` (`ubuntu-22.04-arm`, Linux 6.8.0-1070-azure, aarch64) | `success` | `cc`, `gcc`: GCC 11.4.0; `clang` 14.0.0; after the step `musl-gcc` and `aarch64-linux-musl-gcc` (musl-tools) | `CC_aarch64_unknown_linux_musl = Some(musl-gcc)`, `running: "musl-gcc" "-O3" …` | `ring build: exit 0, 8 s` | `probe build: 0 s` | `ELF 64-bit LSB executable, ARM aarch64, version 1 (SYSV), statically linked, … stripped`; `readelf -lW: no program interpreter, the binary is static` | `probe ok` |
| `release x86_64-apple-darwin` (`macos-15-intel`, Darwin 24.6.0 x86_64) | `success` | `cc`, `gcc`, `clang`: Apple clang 17.0.0 (clang-1700.0.13.5); no musl compiler; no `CC` variable | `CC_x86_64_apple_darwin = None`, `running: "cc" …` | `ring build: exit 0, 16 s` | `probe build: 0 s` | `Mach-O 64-bit executable x86_64`; `otool -L`: `/usr/lib/libSystem.B.dylib` and `/usr/lib/libiconv.2.dylib` | `probe ok` |
| `release aarch64-apple-darwin` (`macos-15`, Darwin 24.6.0 arm64) | `success` | the same Apple clang 17.0.0 | `CC_aarch64_apple_darwin = None`, `running: "cc" …` | `ring build: exit 0, 12 s` | `probe build: 0 s` | `Mach-O 64-bit executable arm64`; `otool -L`: `/usr/lib/libSystem.B.dylib` and `/usr/lib/libiconv.2.dylib` | `probe ok` |
| `gate x86_64-unknown-linux-gnu` (`ubuntu-latest`, Linux 7.0.0-1020-azure) | `success` | `cc`, `gcc`: GCC 15.2.0; `clang` 21.1.8; no musl compiler | `CC_x86_64_unknown_linux_gnu = None`, `running: "cc" "-O0" …` | `ring build: exit 0, 13 s` | `probe build: 0 s` | `ELF 64-bit LSB pie executable, x86-64, … dynamically linked, interpreter /lib64/ld-linux-x86-64.so.2`, as every gate binary | `probe ok` |
| `negative control x86_64-unknown-linux-musl without the compiler step` (`ubuntu-22.04`) | `failure` | GCC 11.4.0, clang 14.0.0; `musl-gcc: not found`, `x86_64-linux-musl-gcc: not found`; no `CC` variable | `CC_x86_64_unknown_linux_musl = None`, `TARGET_CC = None`, `running: "x86_64-linux-musl-gcc" …` | `ring build: exit 101, 3 s` | not run | none | none |
| `negative control aarch64-unknown-linux-musl without the compiler step` (`ubuntu-22.04-arm`) | `failure` | GCC 11.4.0, clang 14.0.0; `musl-gcc: not found`, `aarch64-linux-musl-gcc: not found`; no `CC` variable | `CC_aarch64_unknown_linux_musl = None`, `TARGET_CC = None`, `running: "aarch64-linux-musl-gcc" …` | `ring build: exit 101, 1 s` | not run | none | none |

The compiler errors of the two negative controls, as their logs print them:

```
error: failed to run custom build command for `ring v0.17.14`
  error occurred in cc-rs: failed to find tool "x86_64-linux-musl-gcc": No such file or directory (os error 2)
```

```
error: failed to run custom build command for `ring v0.17.14`
  error occurred in cc-rs: failed to find tool "aarch64-linux-musl-gcc": No such file or directory (os error 2)
```

Neither log holds a line with `INTERP`, and the two musl release jobs print the static line. The
compiler step itself took 20 s on `ubuntu-22.04` and 23 s on `ubuntu-22.04-arm` (the log's
timestamps from its start to the `musl-gcc --version` line), most of it `apt-get update`.

What follows, per runner:

1. **The two musl targets need a compiler step.** Without it cc looks for
   `x86_64-linux-musl-gcc` and `aarch64-linux-musl-gcc` exactly as its source says (above), finds
   neither on the image, and ring's build fails; the job `package` of `release.yml` as it is
   would fail the same way once `surf-tls` depends on ring (inferred: the same images, no step). With `musl-tools` installed both names exist (the
   package installs `musl-gcc` and the target-named wrapper on both architectures), so the
   `CC_<target>` line is not strictly needed; it stays because it is what the run tested and it
   names the compiler instead of leaving it to cc's search. The result is static, as the release
   requires.
2. **The two macOS targets need no step**: Apple clang 17.0.0 of the image builds ring's C and
   assembly. The probe links to two libraries, `/usr/lib/libSystem.B.dylib` and
   `/usr/lib/libiconv.2.dylib`, both system libraries that every macOS installs, so the binary
   runs without anything installed beside it. Whether the `surf` binary of `main` already links
   `libiconv` was not compared: no `otool` record of an earlier release run was at hand.
   `story:tls-release` compares it in its first release run.
3. **`ubuntu-latest` of the gate needs no step**: its GCC 15.2.0 builds ring at opt-level 0 in
   13 s.

The lines for the job `package` of `.github/workflows/release.yml`, a step after
`dtolnay/rust-toolchain` and before the build (`story:tls-release` adds it):

```yaml
      - name: Install a C compiler that targets musl, for ring
        if: contains(matrix.target, '-linux-musl')
        env:
          TARGET: ${{ matrix.target }}
        run: |
          set -euo pipefail
          sudo apt-get update -qq
          sudo apt-get install -y -qq --no-install-recommends musl-tools
          echo "CC_${TARGET//-/_}=musl-gcc" >> "$GITHUB_ENV"
```

`check.yml` needs no line.

The workstation, measured on 2026-11-22 with rustc 1.99.1 (the toolchain `stable` of the
workstation, which is 1.99.1 and has the musl target installed), building the probe crate in the
scratch directory:

| build | compiler cc ran | binary | ring's build script (its C and assembly) | the job |
|---|---|---|---|---|
| `x86_64-unknown-linux-gnu`, dev profile, `RUSTFLAGS=-D warnings` (as the gate builds) | `cc` (GCC 16.2.1); no variable set | dynamically linked, as every gate binary | 16.6 s | 35 s, ring and its 7 packages |
| `x86_64-unknown-linux-musl`, release, the flags of `package` | `x86_64-linux-musl-gcc`, found on `PATH` (the musl package of the workstation installs it) | `static-pie linked`, `readelf -lW` shows no program interpreter | 59.6 s | 101 s |
| the same, with `CC_x86_64_unknown_linux_musl=gcc` (the glibc GCC) | `gcc` | `static-pie linked`, no program interpreter, and the same BuildID as the build above (`1ecc998b…`) | 12.6 s | 40 s |

Each binary printed `probe ok`. The times are from the timestamps cc and cargo write under
`target/` (the build script started, its `root-output` written, `libring` written) and from
the pool's `ran` line; the pool held each job in its queue for 9 to 23 minutes before, which is
not counted. The machine was loaded, so the times vary by a factor of about 4 between runs of
the same work (59.6 s and 12.6 s above).

What the third build shows: ring's C and assembly need no musl headers; the glibc GCC makes the
same objects. The kept evidence is the equal BuildID of the two binaries, which the build log
records (`file` printed `BuildID[sha1]=1ecc998b…` for both). The unit also ran `cmp` on the two
binaries before it removed the build output; it reported them identical, but that output was not
kept in a log. So on x86_64, setting
`CC_x86_64_unknown_linux_musl=gcc` without installing anything is a candidate step beside
`musl-tools`; the probe tests `musl-tools`, and the negative controls show what cc does with
neither.

## 4. The cost of the primitives in the test profile

Measured on 2026-11-22 on the workstation, loaded by other builds, through `slot`: a scratch
crate with `ring = "=0.17.14"`, `debug = false` for `dev` and `test` in its
`.cargo/config.toml`, an integration test run by `cargo test --test timings -- --nocapture`,
so the test profile: opt-level 0 for ring's Rust code and for the C and assembly that cc
compiles (cc takes the profile's opt-level). Each operation is called once untimed, then 5
times, each call timed with `std::time::Instant`; the table gives the median of the 5 and the
smallest and largest. Keys: RSA 2048, P-256, P-384 and Ed25519 keys written by `openssl genpkey`
and read by ring from PKCS#8; the signatures made by ring over a message of 130 bytes (the size
of a CertificateVerify content with SHA-256). An agreement is the whole client side: a key
generated from `SystemRandom`, its public key computed, the shared secret agreed with a fixed
peer key. A seal is one 16 KiB record sealed in place with a 5-byte additional data. HKDF is one
expand of 32 bytes from a pseudorandom key extracted before.

| operation | opt-level 0: median (smallest, largest), ms | ring at opt-level 3: median (smallest, largest), ms |
|---|---|---|
| RSA-2048 PKCS#1 v1.5 SHA-256 verification | 0.094 (0.093, 0.096) | 0.032 (0.030, 0.041) |
| RSA-2048 PSS SHA-256 verification | 0.119 (0.118, 0.120) | 0.034 (0.032, 0.035) |
| ECDSA P-256 SHA-256 verification | 0.102 (0.098, 0.104) | 0.073 (0.070, 0.083) |
| ECDSA P-384 SHA-384 verification | **58.919** (37.923, 71.586) | 16.662 (1.109, 29.900) |
| Ed25519 verification | 0.412 (0.406, 0.419) | 0.053 (0.051, 0.059) |
| X25519 agreement | 0.415 (0.403, 51.584) | 0.096 (0.090, 0.104) |
| P-256 agreement | 0.135 (0.121, 49.204) | 0.081 (0.076, 0.097) |
| AES-128-GCM seal of 16 KiB | 0.006 (0.006, 0.007) | 0.004 (0.004, 0.004) |
| ChaCha20-Poly1305 seal of 16 KiB | 0.010 (0.009, 0.011) | 0.009 (0.008, 0.009) |
| HKDF-SHA256 expand of 32 bytes | 0.004 (0.004, 0.004) | 0.000 (0.000, 0.001) |

Three more runs of the same test, built once, both variants one after the other in one job of
the pool, the verifications only (medians of 5, ms):

| run | RSA PKCS#1 | RSA PSS | P-256 | P-384 at opt-level 0 (smallest, largest) | P-384, ring at opt-level 3 (smallest, largest) |
|---|---|---|---|---|---|
| 2 | 0.094 / 0.031 | 0.124 / 0.030 | 0.114 / 0.082 | 18.070 (14.266, 76.036) | 2.803 (1.354, 4.997) |
| 3 | 0.091 / 0.032 | 0.114 / 0.038 | 0.095 / 0.088 | 24.329 (19.833, 34.375) | 15.894 (1.244, 24.116) |
| 4 | 0.097 / 0.033 | 0.116 / 0.036 | 0.104 / 0.081 | 5.643 (2.736, 6.803) | 1.122 (1.097, 1.191) |

(the first three columns: opt-level 0 / ring at opt-level 3.)

What the numbers say. RSA-2048, ECDSA P-256 and Ed25519 verification cost about 0.1 to 0.4 ms
at opt-level 0, steady across runs; the AEAD seals and HKDF are below 0.01 ms. P-384 is the one
slow operation, and the one whose numbers vary with the machine's load (the agreements of the
first run show single calls of about 50 ms too, so a call can lose a time slice of that size).
Over the four runs, P-384 verification:

| | medians of the 4 runs | smallest call | largest call | 100 checks, the bound of item 9.4, at the largest median | at the largest call |
|---|---|---|---|---|---|
| opt-level 0 | 5.6 to 58.9 ms | 2.7 ms | 76.0 ms | 5.9 s | 7.6 s |
| ring at opt-level 3 | 1.1 to 16.7 ms | 1.1 ms | 29.9 ms | 1.7 s | 3.0 s |

Only one of the four opt-level 0 medians, the first, is over the story's 50 ms; the other three
are 18.1, 24.3 and 5.6 ms. So the rule of the story alone does not carry the line. The ratio
does: run by run the opt-level 0 median is 1.5 to 6.4 times the opt-level 3 median (3.5, 6.4,
1.5, 5.0), the smallest calls 2.5 times (2.7 against 1.1 ms), and RSA verification about 3
times (0.09 to 0.12 against 0.03 to 0.04 ms). The worst case of the path bound falls from 5.9 s
to 1.7 s at the largest medians, and the Wycheproof P-384 file of item 10 (504 vectors) from at
most 29.7 s, half of the 60 s a test may run, to at most 8.4 s (estimates, each vector counted as
one verification at the largest median).

So the line for `.cargo/config.toml` (`story:tls-lock-allow` writes it):

```toml
[profile.dev.package.ring]
opt-level = 3
```

It also sets the opt-level cc compiles ring's C and assembly with, as the second column shows.
`profile.test` inherits it from `profile.dev`; the column was measured with exactly this line
given as `--config 'profile.dev.package.ring.opt-level=3'` to `cargo test`. Its cost is ring's
build at opt-level 3 in the gate's build: on the loaded machine the first build of the scratch
tests (the 8 packages and the tests, opt-level 0) ran 50 s in the pool, and the rebuild with the
line, which compiles ring and its build script again and nothing else, ran 61 s.

The line changes no shipped binary: it sets the `dev` profile, which `cargo build`, `cargo test`
and `cargo nextest` of the gate use; the release binaries are built with `--release`, whose
profile compiles every package, ring included, at opt-level 3 already.

## 5. Random sources

The shipped path draws from `ring::rand::SystemRandom`, which is getrandom 0.2.17 over the
operating system's source: on Linux the `getrandom` system call, with `/dev/urandom` when the
kernel has no such call (`getrandom-0.2.17/src/lib.rs:280-305`,
`src/linux_android_with_fallback.rs:9-17`), the device `surf-secret` reads
(`crates/secret/src/lib.rs:22`); on macOS `getentropy` (`src/lib.rs:241-248`). `SecureRandom` is sealed (`ring-0.17.14/src/rand.rs:23`): no
random source of surf's own can be written, so every draw goes through `SystemRandom` or, in
tests, the fixed sources of `ring::test::rand`.

Those fixed sources are exported but deprecated, "internal API that will be removed"
(`src/lib.rs:171-177`, `src/deprecated_test.rs:38-46`): `FixedByteRandom` fills every draw with
one byte; `FixedSliceRandom` copies its bytes into a draw of exactly their length and panics on
any other length (`src/testutil.rs:495-499`); `FixedSliceSequenceRandom` answers each draw with
the next slice. They are named only in `crypto.rs`, under `#[allow(deprecated)]`, behind the
cargo feature `test-tooling` (item 6), and a `FixedSliceRandom` is given a buffer of the exact
length of the draw it serves. The draw lengths, read in ring: an X25519 or P-256 private key 32
bytes (`agreement::EphemeralPrivateKey::generate(alg, rng)`, `src/agreement.rs:106-109`); an
ECDSA key pair draws its nonce key when it is made, 32 bytes for P-256 and 48 for P-384
(`src/ec/suite_b/ecdsa/signing.rs:163`, `:349-360`), and each signature draws the block length
of the digest less the nonce key, 32 bytes for P-256 and 80 for P-384, and mixes them with the
key and the digest (`:172-190`, `:305-330`); RSA-PSS draws a salt of the digest's length
(`src/rsa/padding/pss.rs:73-80`, `:217`). So one `FixedSliceRandom` of 32 bytes serves a P-256
key pair and all its signatures; P-384 needs `FixedByteRandom` or a sequence.

What follows: the RFC 8448 trace and each recorded flight are replayed with their own private
key bytes, so no test double of the shared secret is needed; every Wycheproof `x25519` and
`ecdh_secp256r1` vector runs with its private key and its shared secret is compared; every
certificate ring signs under a fixed source is reproducible byte for byte.

The scratch test that shows it, run once on 2026-11-22 (`cargo test --test acceptance5 --
--nocapture`, the scratch crate of item 4):

1. **X25519 of RFC 8448 section 3 from its bytes.** The client's private key of the trace
   (`49af42ba…ea5005`) given to `EphemeralPrivateKey::generate(&X25519, …)` through a
   `FixedSliceRandom` of those 32 bytes, then agreed with the server's public key of the trace:

   ```
   x25519 public key: 99381de5…9aaf2c
   x25519 shared secret: 8bd4054f…89492d
   ```

   Shortened here to the first 8 and the last 6 hex digits; the test printed the whole values
   and compared them with the whole values of RFC 8448 section 3. The public key equals the
   trace's client public key (`rfc8448.txt:165-166`), and the shared secret equals the IKM of the
   trace's `{server} extract secret "handshake"` (`rfc8448.txt:262-268`), where both stand in
   full.
2. **ECDSA P-256, one message signed twice under one fixed source.** The P-256 key of `openssl
   genpkey` read by `EcdsaKeyPair::from_pkcs8` and both signatures made with one
   `FixedSliceRandom` of 32 bytes of 0x5a:

   ```
   ecdsa p256 first:  304402206ddec6162ed1fb08ac5bd791d79b3831cb8e3e167f2a4ff0f26c65a0c817ac2302200bf1d3530daf02d73ac8f3d25b75f888028f7ae6115fa70049360c727495803e
   ecdsa p256 second: 304402206ddec6162ed1fb08ac5bd791d79b3831cb8e3e167f2a4ff0f26c65a0c817ac2302200bf1d3530daf02d73ac8f3d25b75f888028f7ae6115fa70049360c727495803e
   ecdsa p256 under SystemRandom equal: false
   ecdsa p384 under FixedByteRandom equal: true
   ```

   The two signatures are equal byte for byte, a second key pair made from the same file under the same source
   signs the same bytes, and the signature verifies; under `SystemRandom` two signatures differ;
   P-384 under a `FixedByteRandom` is reproducible too.
3. **The Ed25519 key of `openssl genpkey -algorithm ed25519`** (PKCS#8 v1, 48 bytes):

   ```
   ed25519 from_pkcs8: Err(KeyRejected("VersionNotSupported"))
   ed25519 from_pkcs8_maybe_unchecked: Ok, public key a69f2e53…20c744
   ed25519 public key of openssl pkey -pubout: a69f2e53…20c744
   ```

   Shortened here as above; the test compared the whole 32 bytes, which were equal. The key was
   made for this test and discarded, so its whole value stands nowhere.

`test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in
0.02s`.

## 6. The crate layout and its owners

`surf-tls` (`crates/tls`, `#![forbid(unsafe_code)]`), its files and the story that writes each:

| file | content | written by |
|---|---|---|
| `crypto.rs` | the seam: the one module that names ring | `story:tls-lock-allow` |
| `error.rs` | the public `Error` (below) | `story:tls-lock-allow` |
| `der.rs`, `x509/`, `pem.rs`, `base64.rs` | the DER reader, the X.509 model, PEM and base64 | `story:tls-x509` |
| `verify.rs` | `LeafKey`, `TrustAnchor`, the trait `ServerCertVerifier`, `CertificateRejection` | `story:tls-x509` |
| `path.rs`, `names.rs` | the implementation of `ServerCertVerifier` over a set of `TrustAnchor`s; host names (RFC 9525) | `story:tls-x509-path` |
| `anchors/` | the bundled list of item 11 | `story:tls-anchors` |
| `record.rs`, `key_schedule.rs`, `transcript.rs` | the record layer, the key schedule, the transcript hash | `story:tls-record` |
| `handshake/`, `connection.rs` | the handshake and the one connection layer | `story:tls-handshake` |

The verifier interface: the trait `ServerCertVerifier` takes the certificates the server sent,
the host and the time, and answers the `LeafKey` or a `CertificateRejection` of kind
`NotTrusted(check)` with a check of item 7, `Expired` or `NameMismatch`. The connection layer is
a client over any `Read + Write`, given a host, a `&dyn ServerCertVerifier` and the time.
`PathVerifier` of `path.rs` is the one implementation the crate ships; the handshake calls the
trait once, with the `cert_data` of each CertificateEntry in the order sent.

A host whose last label is a number in the URL Standard's sense (all ASCII digits, or `0x`/`0X`
followed by hex digits or nothing) is not a DNS name: `0x7f000001`, `127.0.0.0x1`, `1.2.3.0X`
and `example.0x` match no dNSName; `example.x0` and `0x.example` stay DNS names.

The host is normalised once, where the connection is made (`Connection::new` and
`Connection::with_random`, through `names::normalise`): one trailing dot removed, ASCII letters
lowered. That one value is sent in `server_name` (none for an IP address), given to
`ServerCertVerifier::verify`, and reported by `Connection::host`. `127.0.0.1.` is the address
127.0.0.1, as `surf-url` reads it (the URL Standard's IPv4 parser drops one empty last part), so
it carries no `server_name` and is matched against iPAddress entries.

KeyUpdate (RFC 8446 section 4.6.3): an `update_requested` is answered by one KeyUpdate of
`update_not_requested` before the next record of data the client writes, however many came; a
read writes nothing; `close` with an answer owed sends `close_notify` under the current key
without it.

The public `Error` of `error.rs`, a closed list:

- `TimedOut`; `ConnectionClosed` (before the handshake ended); `Truncated` (after it, without
  `close_notify`); `Io(ErrorKind)`.
- `VersionUnsupported { detail }`.
- `AlertReceived { description, before_server_hello }`.
- `AlertSent { description, violation }`, with `violation` one of: `RecordOverflow`,
  `BadRecordMac`, `UnknownContentType`, `EmptyInnerPlaintext`, `MessageTooLarge { declared,
  bound }`, `UnexpectedMessage { expected, got }`, `EmptyCertificate`, `TooManyCertificates`,
  `UnofferedSuite` (also a TLS 1.2 suite in a TLS 1.3 ServerHello or HelloRetryRequest: surf
  offers none), `UnofferedVersion` (a `supported_versions` in a ServerHello or
  HelloRetryRequest that holds anything but 0x0304), `UnofferedExtension`, `UnofferedGroup`,
  `UnofferedAlpn`, `SecondHelloRetryRequest`, `SuiteChangedAfterRetry` (a ServerHello after a
  HelloRetryRequest that selects another suite than it did, RFC 8446 section 4.1.4),
  `HelloRetryRequestNoChange` (a HelloRetryRequest that would not change the hello, the same
  section), `MissingExtension` (a HelloRetryRequest without `supported_versions`, a
  ServerHello without `key_share`, a CertificateRequest without `signature_algorithms`; alert
  `missing_extension`; a ServerHello without `supported_versions` is TLS 1.2),
  `SessionIdMismatch` (a session id echo other than ours), `CompressionNotNull`,
  `UnofferedKeyShare` (a ServerHello share in a group the client sent no share for),
  `KeyShareGroupChanged` (after a HelloRetryRequest, a ServerHello share in another group than
  it selected), `BadKeyShare` (a share the agreement refuses), `ExtensionNotAllowedHere` (an
  extension in a message the table of RFC 8446 section 4.2 does not allow it in),
  `RepeatedExtension` (two extensions of one type in a block; `DuplicateExtension` is the error
  of the X.509 reader), `NonEmptyRequestContext` (a `certificate_request_context` that is not
  empty in a CertificateRequest or Certificate of the handshake), `UnofferedSignatureScheme` (a
  CertificateVerify scheme not offered, not fitting the key, RSA PKCS#1 or SHA-1),
  `BadKeyUpdate` (a `request_update` other than 0 or 1), `Malformed { message }` (alert
  `decode_error`), and from `story:tls-record` `InterleavedHandshake`,
  `HandshakeSpansKeyChange`, `EmptyContent` (a handshake or alert record with no content),
  `BadAlertRecord` (an alert record that is not one alert of two bytes), `UnprotectedRecord`
  (a record without protection where protection is required: application data before the
  read keys are set, a handshake or alert record after), `Downgrade`,
  `UnexpectedChangeCipherSpec`, `BadCertificateVerify`, `BadFinished`, and from
  `story:tls-handshake` `TooManyRecordsWithoutData { bound }` (more records in a row than item
  9 allows that give the caller no application data; alert `unexpected_message`).
- `SequenceExhausted`, and from `story:tls-record` `KeyLimitReached` (item 9.6).
- `Certificate(CertificateRejection)`, which `story:tls-x509` adds. A `CertificateRejection`
  sends `unknown_ca` (`NoPathToAnchor`), `certificate_expired` (`Expired`) or `bad_certificate`
  (every other).

The list follows the tables "RFC 8446 client obligations" and "Messages the client receives" of
`story:tls-handshake` and "RFC 8446 record obligations" of `story:tls-record`.

The seam, `crypto.rs`: the operations of the epic (AEAD seal and open for the three suites,
SHA-256 and SHA-384, HKDF, X25519 and P-256 agreement, verification of the schemes of item 14),
and `REFUSED`, the signature algorithms it refuses by name: every MD5 or SHA-1 signature, RSA
keys under 2048 bits. ring exports no SHA-1 signing encoding (`src/rsa/padding.rs:20-24`, the SHA-1 PKCS#1 encoding is `pub(super)`,
for legacy verification only), refuses to sign with an RSA modulus under 2048 bits
(`src/rsa/keypair.rs:283`), and has AES-128-GCM and AES-256-GCM only, no AES-192
(`src/aead/algorithm.rs:149-163`).

Behind the cargo feature `test-tooling` (off by default; turned on by `surf-tls-test-certs`,
`surf-tls-test-server` and the dev-dependency of `surf-tls` on itself): the signing from PKCS#8
that ring offers, the fixed random sources of item 5, the RFC 8448 trace mode of
`story:tls-handshake`, and `TEST_TOOLING_MARKER`, the fixed string `surf-tls test-tooling, never
in a release`, which `Connection` reads through `std::hint::black_box` when the feature is on,
so that a binary that connects carries it exactly then. A release build never compiles them.

The signing algorithms the cases of `story:tls-test-certs` need, and the ring function that
makes each (all from a PKCS#8 key; the RSA, P-256, P-384 and Ed25519 keys of `openssl genpkey`
were read by these functions in the scratch crate of item 4):

| scheme (signature of a case) | cases | ring |
|---|---|---|
| rsa_pkcs1_sha256 (sha256WithRSAEncryption) | `ok-rsa2048`, and every other case an RSA issuer signs with this scheme | `RsaKeyPair::from_pkcs8`, `sign(&RSA_PKCS1_SHA256, …)` |
| rsa_pkcs1_sha384, rsa_pkcs1_sha512 | `ok-rsa-pkcs1-sha384`, `ok-rsa-pkcs1-sha512` | the same with `RSA_PKCS1_SHA384`, `RSA_PKCS1_SHA512` |
| rsa_pss_rsae_sha256, sha384, sha512 | `ok-rsa-pss`, `ok-rsa-pss-sha384`, `ok-rsa-pss-sha512` | the same with `RSA_PSS_SHA256`, `RSA_PSS_SHA384`, `RSA_PSS_SHA512`; the salt is the digest's length, as the RFC 4055 parameters of those cases say |
| ecdsa_secp256r1_sha256 | `ok-ecdsa-p256`, and every other case a P-256 issuer signs | `EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_ASN1_SIGNING, …)`, `sign` |
| ecdsa_secp384r1_sha384 | `ok-ecdsa-p384` | `EcdsaKeyPair::from_pkcs8(&ECDSA_P384_SHA384_ASN1_SIGNING, …)`, `sign` |
| ed25519 | `ok-ed25519` | `Ed25519KeyPair::from_pkcs8_maybe_unchecked`, which reads the PKCS#8 v1 that `openssl genpkey` writes where `from_pkcs8` wants v2 (`ed25519/signing.rs:79-111`; item 5), `sign` |

The cases ring cannot make: a SHA-1 signature, an MD5 signature and an RSA-1024 key. They are
made once by `openssl` in the lane `slow` and committed as bytes (item 10); the gate never runs
`openssl`.

The public modules of `surf-tls`: `crypto`, `error`, `der`, `x509`, `pem`, `verify`, `path`,
`anchors`, `record`, `key_schedule`, `transcript` and `connection`; `handshake` is private but
for `handshake::trace` under `test-tooling`. `record.rs`, `key_schedule.rs` and
`transcript.rs` serve either side of a connection (keys and direction given by the caller), so
`crates/tls-test-server` builds the server from them and from `crypto` and writes its own
server messages; it reads these modules and writes nothing under `crates/tls/src`, and a need it
finds there is a finding for the owner of the file. The test tooling crates
`surf-tls-test-certs`, `surf-tls-conformance` and `surf-tls-test-server` are not shipped. In a
wave of more than one unit, the coordinating agent writes the `mod` lines of
`crates/tls/src/lib.rs` and `crates/tls/tests/main.rs`. Each later story of the epic names a
file of this item only as its owner writes it.

## 7. The failure reasons and their detail

Five reasons join `surf.net.FailureReason`: `CertificateNotTrusted`, `CertificateExpired`,
`CertificateNameMismatch`, `TlsVersionUnsupported`, `TlsHandshakeFailed`. A failure is shown as
`page failed <reason> "<address>"`, and the detail below is the text printed after it.

The checks of `NotTrusted(check)`, a closed list; `INDEX` of `story:tls-test-certs` and
`CertificateRejection` of `story:tls-x509` take these twelve names and no other. Each gives
`CertificateNotTrusted`; every check but `NoPathToAnchor` sends `bad_certificate`.

| check | condition | alert surf sends | detail |
|---|---|---|---|
| `Unreadable` | the leaf refused by the X.509 reader: bad DER, a version 1 leaf, `AlgorithmMismatch` and the other errors of the reader; a certificate after the leaf the reader refuses, when no path is found (below); the anchor's certificate refused by the reader for its extensions, or with an authorityKeyIdentifier holding authorityCertIssuer or authorityCertSerialNumber or a keyIdentifier other than its subjectKeyIdentifier; a leaf with an empty subject whose subjectAltName is not critical (RFC 5280 section 4.2.1.6) | `bad_certificate` | `Unreadable: certificate <n> of the server: <error of the reader>`, and when certificates were left out `; <k> certificates left out as unreadable` (`; 1 certificate left out as unreadable`) |
| `NoPathToAnchor` | no path from the leaf to a trust anchor | `unknown_ca` | `NoPathToAnchor: no path to a trust anchor` |
| `BadSignature` | a signature on the path does not verify | `bad_certificate` | `BadSignature: the signature of certificate <n> does not verify` |
| `IssuerNotCa` | an issuer without basicConstraints cA, with basicConstraints not critical, or with an empty subject | `bad_certificate` | `IssuerNotCa: certificate <n> issues but is no CA` |
| `PathTooLong` | more intermediates than item 9 allows, or a pathLenConstraint broken | `bad_certificate` | `PathTooLong: <k> intermediates, at most <bound>` or `PathTooLong: pathLenConstraint of certificate <n> broken` |
| `CandidateLimit` | a limit of the path search of item 9.4 reached: 100 signature checks or 25,000 candidates considered | `bad_certificate` | `CandidateLimit: <bound> <signature checks or candidates considered>` |
| `KeyUsage` | keyUsage without keyCertSign on an issuer, or without digitalSignature on the leaf | `bad_certificate` | `KeyUsage: certificate <n> lacks <keyCertSign or digitalSignature>` |
| `UnknownCriticalExtension` | a critical extension the reader does not know; authorityKeyIdentifier or subjectKeyIdentifier marked critical (RFC 5280 sections 4.2.1.1 and 4.2.1.2) | `bad_certificate` | `UnknownCriticalExtension: <OID> in certificate <n>` |
| `NameConstraint` | a name outside a nameConstraints of an issuer or of the anchor, whether the extension is marked critical or not (one not marked critical is read and enforced like a critical one, not refused); nameConstraints present in a leaf that is no CA (RFC 5280 section 4.2.1.10); the name-constraint work of item 9.4 exhausted | `bad_certificate` | `NameConstraint: <name> outside the constraints of certificate <n>` |
| `NoServerAuth` | extKeyUsage without serverAuth on the leaf, or with neither serverAuth nor anyExtendedKeyUsage on an intermediate (below) | `bad_certificate` | `NoServerAuth: the leaf is not for TLS servers`, or `NoServerAuth: certificate <n> is not for TLS servers` for an intermediate |
| `WeakAlgorithm` | an MD5 or SHA-1 signature, decided before any verification | `bad_certificate` | `WeakAlgorithm: <algorithm> in certificate <n>` |
| `WeakKey` | an RSA key under 2048 bits, decided before any verification | `bad_certificate` | `WeakKey: RSA <bits> bits in certificate <n>` |

`<n>` counts the certificates as the server sent them, the leaf 0. Every certificate keeps its
sent position; one sent twice is a candidate once, at its first position, and every `<n>` in a
refusal is the position as sent. The anchor is numbered after them: the count of certificates
sent.

The first certificate (the leaf) must be readable; an unreadable certificate after it is left
out of the candidates and does not refuse the chain (RFC 8446 section 4.4.2: certificates after
the leaf that the path does not need may be ignored). When no path is found and at least one
certificate was left out, the refusal is `NotTrusted(Unreadable)` naming the first one left out
by its sent position and its reader error, without bytes of it, and how many were left out,
before any other reason. Reading costs one pass over each certificate sent, so the cost stays
bounded by the message size.

extKeyUsage is held on every certificate of the path below the anchor. The leaf needs
serverAuth in an extKeyUsage it has; an intermediate that has an extKeyUsage without serverAuth
and without anyExtendedKeyUsage refuses the path with `NoServerAuth`, naming that certificate's
sent position; an intermediate without the extension passes. The anchor's own extKeyUsage is
not read. A refusal ends that path only: the search goes on to another issuer of the same
subject (a cross-signed intermediate).

Every other condition, each with one reason:

| condition | reason | alert surf sends | detail |
|---|---|---|---|
| any check of the table above, among them a bad certificate (`BadSignature`), a certificate the reader refuses (`Unreadable`) and a weak algorithm (`WeakAlgorithm`, `WeakKey`) | `CertificateNotTrusted` | as above | as above |
| outside the validity, either side, of any certificate on the path | `CertificateExpired` | `certificate_expired` | `certificate <n> valid from <notBefore> to <notAfter>, checked at <time>` |
| no `subjectAltName` entry of the leaf matches the host (no Common Name is read) | `CertificateNameMismatch` | `bad_certificate` | `no subjectAltName entry matches <host>` |
| (a) a `protocol_version` alert before any ServerHello | `TlsVersionUnsupported` | none | `the server sent protocol_version before its ServerHello` |
| (b) a `handshake_failure` alert before any ServerHello | `TlsHandshakeFailed` | none | `the server sent handshake_failure before its ServerHello; a server limited to TLS 1.2 answers this way` |
| (c) a ServerHello that selects TLS 1.2 or below (no `supported_versions`) without a downgrade value, any suite | `TlsVersionUnsupported` | `protocol_version` | `the server selected <version>` |
| a ServerHello of TLS 1.2 or below with either downgrade value of RFC 8446 section 4.1.3 (`DOWNGRD` and 0x01 or 0x00): a downgrade to TLS 1.2 | `TlsHandshakeFailed` | `illegal_parameter` | `illegal_parameter: the ServerHello random holds a downgrade value` |
| a ServerHello whose `supported_versions` holds anything but 0x0304 | `TlsHandshakeFailed` | `illegal_parameter` | `illegal_parameter: the ServerHello selects <version>` |
| a handshake message that declares more bytes than item 9 allows (an oversized handshake message) | `TlsHandshakeFailed` | `illegal_parameter` | `illegal_parameter: a <message> of <declared> bytes, at most <bound>` |
| every other alert received or protocol violation (`AlertReceived`, `AlertSent` with any other violation, `SequenceExhausted`, `KeyLimitReached`) | `TlsHandshakeFailed` | the alert of the violation, or none | `<alert>: <violation>` for an alert surf sent, `the server sent <alert>` for one it received |
| a server that never finishes the handshake or the response (`TimedOut`) | `Unreachable` | none | as for HTTP today |
| the connection closed before the handshake ended (`ConnectionClosed`), a truncated record among them | `Unreachable` | none | as for HTTP today |
| the connection closed after the handshake without `close_notify` (`Truncated`) | `Unreachable` | none | as for HTTP today |
| an error of the socket (`Io`) | `Unreachable` | none | as for HTTP today |

`Unreachable` keeps its meaning of `spec/domains/net.yaml`: "it ended before a response was
complete".

Revocation is not checked: a revoked certificate that has not expired loads. The documents say
so in those words where `https:` is described.

## 8. The extra anchors

`surf --ca-file <path> <url>` and `surf-agent open --ca-file <path> [<url>]`. The client reads
the file and its bytes travel in the `open` request; not an environment variable, because a
`surfd` started by an earlier `open` does not see the environment of a later caller. The file
holds at most 32 KiB and 64 certificates; every block is a `CERTIFICATE`, every certificate a
CA (basicConstraints cA). The reader of the file is `surf_tls::anchors::Anchors::with_extra_pem`;
it refuses the file whole with an `AnchorError`: `NoCertificate` (no PEM block),
`NotACertificate { index, label }` (another label; its content is not decoded), `BadPem {
index }`, `BadCertificate { index, error }` (the reader's error), `NotACa { index }`,
`TooLarge` (over 32,768 bytes, before any block is read) or `TooMany` (over 64 blocks, before
any is decoded). A certificate the reader refuses is not taken as name and key from this file.
Why 32 KiB: an `open` request holds at most `surf_agent::wire::
REQUEST_MAX`, 64 KiB with its `end` line (`crates/agent/src/wire.rs:86`), and an `arg` value
travels quoted (`surf_answer::quote`, `crates/answer/src/lib.rs:221`), which writes a newline
as `\x0a`, 4 bytes; a PEM file of 64-character lines grows by 3 bytes per line, so 32,768 bytes
quote to at most 34,283 and leave the rest of the request its room. A file whose request would
still be over `REQUEST_MAX` (very short lines) is refused by `Request::encode` as any request
over it is.

`surf --anchors [--ca-file <path>]` prints `anchors <n> bundled <yyyy-mm-dd> sha256:<64 hex>`,
the count, date and sha256 of the bundled list of item 11, and with a file a second line
`anchors <m> file <path>`; exit 0.

## 9. Bounds

1. **The largest handshake message: 65,535 bytes** (2^16 - 1), the bound rustls 0.23.46 holds
   (`MAX_HANDSHAKE_SIZE = 0xffff`, `src/msgs/deframer/handshake.rs:376`). Reason: a handshake
   message is reassembled in memory before it is parsed, so this is what one message may cost;
   a certificate chain of the web fits far inside it, and the largest case of
   `story:tls-test-certs`, `candidate-limit` with 50 certificates, must fit too, so that the
   path bound below and not this one stops it. A message that declares more is refused at its
   4-byte header (`story:tls-record`): `illegal_parameter`, `MessageTooLarge { declared,
   bound }`. Why `illegal_parameter` and not `decode_error`, which rustls sends for the same
   case (`rustls-0.23.46/src/error.rs:213-219`, every `InvalidMessage` but three maps to
   `DecodeError`): RFC 8446 section 6.2 gives `decode_error` "for errors where the message does
   not conform to the formal protocol syntax" (`rfc8446.txt:4959-4962`) and `illegal_parameter`
   "for errors which conform to the formal protocol syntax but are otherwise incorrect"
   (`:4945-4948`). A declared length up to 2^24 - 1 conforms to the syntax (the length is a
   `uint24`); it is wrong only against surf's own bound, so `illegal_parameter`.
   A HelloRetryRequest cookie is at most 65,116 bytes (`messages::MAX_COOKIE`): the second
   ClientHello echoes it in `extensions<8..2^16-1>` beside its other extensions at their
   largest (413 bytes with a 255-byte `server_name` and a secp256r1 share) and the 6 bytes
   around the cookie. A longer one is `illegal_parameter`, `MessageTooLarge { declared,
   bound: 65116 }`, before any second hello is built.
2. **The most certificates in a Certificate message: 64.** Reason: more than the 50 of
   `candidate-limit`, so that the path builder sees that case whole. The length bound above alone
   would not bound the count usefully: a `CertificateEntry` is at least 6 bytes (3 bytes of
   `cert_data` length, at least 1 byte of data, 2 bytes of `extensions` length), and the message
   body has 4 bytes before the list (a 1-byte empty `certificate_request_context` and the 3-byte
   list length), so (65,535 - 4) / 6 gives at most 10,921 entries, each to be parsed. One more
   than 64 is `bad_certificate`, `TooManyCertificates`.
3. **The most intermediates on a path: 6**, rustls-webpki's `MAX_SUB_CA_COUNT`
   (`src/verify_cert.rs:873` of 0.103.17). Reason: the web's chains have one or two
   intermediates; a longer path is either an attack on the search or a mistake. `path-too-long`
   of `story:tls-test-certs` has 7. More is `NotTrusted(PathTooLong)`.
4. **The path search and its limits: 100 signature checks, 25,000 candidates considered and
   250,000 name-constraint comparisons**, per validation. The search (`story:tls-x509-path`,
   `path.rs`) is depth-first from the leaf. At each step, for the certificate at the end of the
   path, its candidate issuers are the trust anchors and then the certificates the server sent,
   in the order sent, whose subject equals the certificate's issuer name (byte for byte as DER;
   the reader normalises nothing); a certificate already on the path is no candidate. Key
   identifiers order the candidates, they filter none: an issuer whose subjectKeyIdentifier
   equals the child's authorityKeyIdentifier (both present) is tried first, the others after it
   in the order met. The issuer name alone decides whether a certificate is a candidate. Counts
   with this rule: the same-subject dead ends stay 39 at n=20 and 79 at n=40; the cycle 42 and
   98; every one verifies 514 and 1462; candidate-limit considered 104 to 147 (bound 153),
   without anchor 4108 to 4366.
   - **Each candidate looked at charges the counter `considered`**, one per name comparison,
     whether the names match or not. Limit 25,000.
   - **Before the search descends into a candidate, surf verifies the candidate's signature
     over the certificate** (after the policy of `REFUSED`, which costs no verification). Each
     verification charges the counter `signatures`, whether it succeeds or not. Limit 100, the
     number rustls-webpki keeps (`signatures: 100`, `rustls-webpki-0.103.17/src/verify_cert.rs:355-360`,
     taken there from the remedy of Go's CVE-2018-16875).
   - **Each subtree of a nameConstraints visited charges the counter
     `name_constraint_comparisons`.** One unit is one subtree visited for one subjectAltName
     entry, of any form, permitted or excluded, plus one unit per subtree for the one pass that
     finds whether a directoryName subtree is present; a leaf with n entries under m subtrees
     costs m + n * m. Limit 250,000, the number rustls-webpki keeps
     (`name_constraint_comparisons: 250_000`, `rustls-webpki-0.103.17/src/verify_cert.rs`).
     Reaching it refuses the path with `NotTrusted(NameConstraint)`, detail "the work of 250000
     name comparisons"; it never accepts for want of budget.
   - A candidate whose signature does not verify is not descended into. A signature whose
     scheme the seam does not support for the key (a P-384 key with an ecdsa-with-SHA256
     signature, say) counts as one that does not verify (`BadSignature`). A candidate that is an
     anchor and verifies ends the path: the other checks of item 7 run on it (validity,
     basicConstraints and pathLenConstraint, keyUsage, extKeyUsage, nameConstraints); if they
     pass, the path is the answer, else the search goes on with the next candidate. A candidate
     that would be the 7th intermediate is not descended into (item 9.3).

   Why this order, the other one than rustls-webpki's: webpki descends into every name match and
   verifies the signatures only when a path reaches an anchor
   (`rustls-webpki-0.103.17/src/verify_cert.rs:94`), so it needs a second budget for the descents
   themselves (`build_chain_calls: 200_000`, `:143`, `:364`). Verifying first cuts every branch
   whose link is false at its first step, so a descent happens only after a successful
   verification: at most 100 descents, and with at most 6 intermediates and at most 118 bundled
   anchors, 64 anchors of an extra file and 64 certificates sent, at most 101 steps of at most 246
   comparisons each, 24,846. So `considered` cannot reach its limit with today's sets; it bounds
   the work if a set grows.

   What the search answers when it ends without a path, in this order: a certificate left out
   as unreadable (item 7): `NotTrusted(Unreadable)`, the first of them with the count; else a
   limit reached (either counter): `NotTrusted(CandidateLimit)`; else a candidate cut by item 9.3:
   `NotTrusted(PathTooLong)`; else a candidate refused by `REFUSED`: `NotTrusted(WeakAlgorithm)`
   or `NotTrusted(WeakKey)`; else a name match whose signature failed: `NotTrusted(BadSignature)`;
   else a complete path that failed a check of item 7: the check of the last such path; else
   `NotTrusted(NoPathToAnchor)`. No check name is added to the 12 of item 7.

   The worst time, from the costs of item 4 with the line of item 4: 100 verifications at the
   largest median, 16.7 ms for P-384, 1.7 s (3.0 s at the largest single call, 29.9 ms); with
   RSA-2048 or P-256 keys about 10 ms (100 × 0.03 to 0.09 ms); 25,000 name comparisons of a few
   hundred bytes each, not measured, estimated under 25 ms (under 1 µs each). So the hostile cases
   of the limit, which must end in under 1 s (`story:tls-x509-path` acceptance 5), use P-256 or
   RSA keys: `candidate-limit` of `story:tls-test-certs` (48 intermediates with one subject and
   different keys: every candidate matches by name and one or two verify, so ending the
   search would need more than 47 verifications at each of its levels and the limit of 100 is
   reached before it ends) and the case of item 10 that never reaches an anchor.
5. **One HelloRetryRequest.** RFC 8446 section 4.1.4: "If a client receives a second
   HelloRetryRequest in the same connection [...] it MUST abort the handshake with an
   "unexpected_message" alert" (`rfc8446.txt:1837-1840`): `unexpected_message`,
   `SecondHelloRetryRequest`.
   `Transcript::hello_retry_request` refuses a second call with the same violation and leaves
   the transcript as it was (`story:tls-record`); the handshake refuses the message before it.
6. **The most records under one AES-GCM write key: 23,726,566.** RFC 8446 section 5.5 limits
   AES-GCM to 2^24.5 full-size records under one key (`rfc8446.txt:4696-4700`); 2^24.5 is
   23,726,566.4, rounded down. The record layer counts the records it writes under each write
   key (`record::AES_GCM_RECORD_LIMIT`, `write_limit`); the write that would pass the limit is
   refused before a byte of it is written, with the error `KeyLimitReached`, and the layer is
   closed after it. The layer sends no KeyUpdate of its own: a caller that may write that many
   records sends one before. ChaCha20-Poly1305 has no such limit (section 5.5: "the record
   sequence number would wrap before the safety limit is reached"): its keys are bounded by the
   sequence number alone (`SequenceExhausted`). Reading has no record limit: the limit guards
   the keys surf encrypts with, and the read sequence still cannot wrap. Why a count and no
   automatic update: a KeyUpdate is a handshake message of `story:tls-handshake`, which the
   record layer does not write on its own. The connection sends it: a KeyUpdate
   (`update_not_requested`) when fewer than 2 records remain under the write key, before the
   next record of data, so `write_data` never meets the layer's refusal at
   `AES_GCM_RECORD_LIMIT`.
7. **The most records in a row without application data: 32.** Empty application data,
   `user_canceled`, NewSessionTicket and KeyUpdate records give the caller nothing; the 33rd
   in a row ends the connection with `unexpected_message`, `TooManyRecordsWithoutData { bound:
   32 }`. A record with at least one byte of data starts the count again; handshake messages
   that share a record count as one record. The bound counts records, never time
   (`connection::MAX_RECORDS_WITHOUT_DATA`).

## 10. The test design, per material

Every material is vendored with a `SOURCE` file of the form of
`crates/html-conformance/data/*/SOURCE` (where it came from, the commit or date, the licence,
the files and their sha256), and a fast check in the gate compares each file with its digest.
The sizes below were read on 2026-11-22: each file fetched once with `curl` into the scratch
directory of the unit that wrote this note, counted with `wc -c` and hashed with `sha256sum`;
the vector and case counts read from the files with `jq`. The vendoring story pins the same
commit or takes a newer one and writes its own numbers into `SOURCE`. The times are estimates
from the costs of item 4, not runs.

1. **Wycheproof** (C2SP, Apache-2.0): a subset in `crates/tls/tests/data/wycheproof/`, vendored
   by `story:tls-lock-allow`, from
   `https://raw.githubusercontent.com/C2SP/wycheproof/<commit>/testvectors_v1/<file>`, commit
   `65ca494727a52944c204441c860b039e6786b99b` (2026-11-11, the head of `main` on 2026-11-22).
   Fourteen files, 3,027,947 bytes, 4,163 vectors:

   | file | bytes | vectors | sha256 |
   |---|---|---|---|
   | `aes_gcm_test.json` | 213,177 | 316 (103 of them AES-192) | `985e5ecc172e181eaf49e89508b9470dcf478002eb7e8559c707eb42dc97dfe7` |
   | `chacha20_poly1305_test.json` | 241,127 | 325 | `fe61d25f90e1bde4461d00eafe61049e5f29bd999f36b766df9cda90906ad53d` |
   | `x25519_test.json` | 253,890 | 518 | `35c3f5231cf25cc640b524d403461deee9e49441d5d915a3a25b2c8ff5adbe7d` |
   | `ecdh_secp256r1_test.json` | 418,368 | 612 | `cdb8bd5d1206fddb6618c69ffa18f303b4752caba321d348d4aacae3f20cbec4` |
   | `ecdsa_secp256r1_sha256_test.json` | 327,156 | 484 | `182db4f3e230f6f9fa9f800d2a614dede30284b8e8438bbfe1171905402e9332` |
   | `ecdsa_secp384r1_sha384_test.json` | 375,442 | 504 | `8a5b3ae1760975143414811f13588c24d951d9d8c904195087ba327591dfe9cc` |
   | `ed25519_test.json` | 172,628 | 210 | `ea80ffebac085e21cb9d3e903e7fa2b448d96702da5b31f061d0b83b430b74b3` |
   | `hkdf_sha256_test.json` | 92,505 | 86 | `bb2b462a38b251cb52a2aede706d6d4b62b26864f4e80c95497507ddb07c5f1e` |
   | `hkdf_sha384_test.json` | 116,155 | 83 | `69ff6ea3657bb9c1b8cdffbbb4e7832353d08fd15c0d9997b03f7a6b180e3678` |
   | `rsa_signature_2048_sha256_test.json` | 211,075 | 259 | `94a917b01ff50fb874cfc05bf29b4af44868d944a6558201cf18380da93fb393` |
   | `rsa_signature_2048_sha384_test.json` | 203,297 | 258 | `c571c105d261c0ff588a2888a529f152563fb3b77894b7620d4e4f8f934f2c1d` |
   | `rsa_signature_2048_sha512_test.json` | 207,663 | 259 | `16ea24b039905d054bdb6004f5fd179374e150b7b6d73a7ba654b6d00eab12ef` |
   | `rsa_pss_2048_sha256_mgf1_32_test.json` | 85,280 | 108 | `7f6efafc160f4816b96cbf1c12188a31051d7e3f001e27505d9edb5f2a0e325c` |
   | `rsa_pss_2048_sha384_mgf1_48_test.json` | 110,184 | 141 | `66d464778b0b2f683a1d1a20e94f77e9472b8cc393b45f423fa08a48e677063c` |

   The `testvectors_v1` directory holds no RSA-PSS file for 2048 bits with SHA-512 and a salt
   of 64 bytes; rsa_pss_rsae_sha512 is covered by `ok-rsa-pss-sha512` of
   `story:tls-test-certs` alone. The 103 AES-192 vectors of `aes_gcm_test` are a named skip,
   ring has no AES-192 (item 6); the test prints their count. Within the budget (estimate):
   the P-384 file is the costly one, 504 verifications, at most 8.4 s at the largest median of
   item 4 with its line (29.7 s without it); the other 3,659 vectors at most 0.42 ms each, the
   costliest operation of item 4 but P-384 at opt-level 0, at most 1.6 s. Each file is one test,
   all under 60 s, and the whole set under 10 s of the 120 s step.
2. **RFC 8448** in `crates/tls/tests/data/rfc8448/`: `https://www.rfc-editor.org/rfc/rfc8448.txt`,
   159,343 bytes, sha256 `6564d1376d1ec744fc7a9993da15ebc1b9be361908b166091f47ef605c537fba`;
   sections 3, 5 and 7, replayed with their private keys (item 5). Its RSA server key is 1024
   bits, so ring cannot check its CertificateVerify; the trace tests the key schedule, the record
   layer and the parsing.
3. **x509-limbo** (C2SP, Apache-2.0) in `crates/tls/tests/data/x509-limbo/`:
   `https://raw.githubusercontent.com/C2SP/x509-limbo/<commit>/limbo.json`, commit
   `b12423782a88abead01141fa6582254e3eb49e74` (2026-11-17, the head of `main` on 2026-11-22),
   41,805,636 bytes, sha256 `56799cefd65fc9e0bdb9cb0f97fce6734533b45df3de4fc17fdc7b7b79654fca`,
   `version` 1, 9,807 cases (935 `SUCCESS`, 8,872 `FAILURE`; 9,797 `SERVER`, 10 `CLIENT`; by
   family bettertls 9,572, rfc5280 109, webpki 56, crl 22, online 14, pathlen 13, pathological
   11, rfc9881 6, cve 3, invalid 1). It is over 20 MiB, so `story:tls-x509` chooses every case
   not of the `bettertls` family (235 cases, 1,999,896 bytes as compact JSON) and the
   `bettertls` cases in file order up to 20 MiB in all: at this commit the first 4,909
   (18,971,513 bytes), sizes of the cases as upstream has them. Of the cases chosen it then drops
   the field `peer_certificate_key` (the peer's private key, D158) and leaves out the case
   `online::facebook.com` (D161: a secret scanner's token pattern matches a run of its leaf's
   base64). So 5,143 cases are kept and 4,664 left out: `limbo-other.json` holds 234 cases in
   1,934,876 bytes, `limbo-bettertls-1.json` to `-4.json` the 4,909 of `bettertls`. `SOURCE`
   states the sizes, the counts, what is dropped and the jq that makes the files again. Within the budget (estimate): the file holds 39,975 certificates for its
   9,807 cases, about 4 per case; of 400 leaf certificates sampled (200 of each part), 392 hold
   P-256 keys, 2 RSA-2048 and 4 ML-DSA-44. So a case costs a few verifications at about 0.1 ms
   each, and the 5,143 cases about 2.5 s at opt-level 0 (5,143 × 4 × 0.12 ms); the 11
   `pathological` cases are held to the limits of item 9.4, at most 100 verifications each, 0.1
   s for P-256 keys. Reading 21 MB of JSON and its PEM is not estimated here; `story:tls-x509`
   measures it, and the case runner fits the 60 s and the 120 s with it.
4. **The generated certificates** in `crates/tls-test-certs/certs/`: made by
   `surf-tls-test-certs` through ring under a fixed source (reproducible, item 5), committed
   with the digest of their sources; the cases ring cannot make (a SHA-1 signature, an MD5
   signature, an RSA-1024 key, item 6) made once by `openssl` in the lane `slow` and committed
   as bytes. The gate never runs `openssl`. Beside `candidate-limit`, one more case for the
   limits of item 9.4: `candidate-limit-no-anchor`, 48 intermediates with one subject and one
   P-256 key, each issued by that subject and signed with that key, and a leaf issued by the
   same subject; neither that key's root nor any of the 48 is an anchor. Every candidate
   matches and verifies, no path reaches an anchor, the search descends to 6 intermediates and
   back, and the limit of 100 signature checks ends it: `NotTrusted(CandidateLimit)`, in about
   9 ms of verification with the line of item 4 (100 × 0.09 ms), far under its 1 s.
5. **The recorded flights** in `crates/tls/tests/data/flights/`: server flights of an
   independent implementation recorded on 127.0.0.1, pinned by sha256; recorded data, not a
   generated product. The lane `slow` records again and compares outcomes, not bytes, since the
   independent server draws its own randomness.

The repository's secret scanner (gitleaks, run on every commit the organization's bot makes)
meets the committed test vectors that hold key material: the RFC 8448 traces, x509-limbo and the
private keys of the test certificates. Its rule `generic-api-key` refused the first commit of
this note, for three lines that put the word key or secret beside 64 hex digits (an X25519
public key and shared secret of RFC 8448 and an Ed25519 public key); they are shortened in item
5. The scanner was run on this document only, not on any of the materials above. So each story
that vendors such data (`story:tls-record`, `story:tls-handshake`, `story:tls-test-certs`,
`story:tls-x509`) runs the scanner on its data before the commit
(`gitleaks detect --no-git --source <dir> --redact --no-banner` with the organization's
binary). The first of them fixes the form of those files for all four (binary files, or hex with
no such keyword on the line) and states it here through the coordinating agent. If no form
passes, it is a question to the operator, since the exceptions of the gates policy are his.

BoGo, BoringSSL's TLS suite, is not used: it needs Go and a long run. The lanes: `check` runs
`tls-certs-check` and `tls-flights-check` (digests, no build, no `openssl`) and
`tls-x509-conformance` (the x509-limbo counts); the task `slow` runs `tls-certs-slow` (the test
certificates made again and compared byte for byte) and `tls-flights-slow` (the flights
recorded again and replayed, compared by outcome). `slow` needs `openssl` 3.4 or later; no
workflow runs it, because the forge's runner pins no `openssl` and three of the certificates
are bytes `openssl` writes. It is run by hand after a change to the sources of either
generator.

## 11. U2 in detail: the bundled anchors

The file is curl's extract of Mozilla's root store, the `cacert.pem` that
`https://curl.se/docs/caextract.html` publishes: PEM, checked daily, converted when Mozilla's store
changes, with a `.sha256` beside it. Read on 2026-11-22 from `https://curl.se/ca/cacert.pem`:
200,116 bytes, `Last-Modified: Thu, 05 Nov 2026 04:12:01 GMT`, its header line "Certificate data
from Mozilla as of: Thu Nov  5 04:12:01 2026 GMT", 118 `BEGIN CERTIFICATE` blocks (of which
117 become anchors; e-Szigno TLS Root CA 2023, a P-521 key signed with ecdsa-with-SHA512,
gives none: the reader knows no P-521 key and ring 0.17.14 verifies none), sha256
`427c064216761a1e72a8b30f0d954d08cb6e4435a82091fb865afd20c8497285`, the value
`https://curl.se/ca/cacert.pem.sha256` publishes for it (that file is one line,
`<sha256>  cacert.pem`, 77 bytes; the `SHA256:` line in the list's own header is the hash of
Mozilla's `certdata.txt` it was converted from, not of the list). It lives in
`crates/tls/src/anchors/` with a `SOURCE`, written by `story:tls-anchors` when it takes the
file, that holds the address, the date of the header line, the count, and the sha256 of the list
itself (`sha256sum cacert.pem`), checked equal to the published `.sha256` line when the file is
taken and by the gate's digest check after; `surf --anchors` prints the date, the count and
that sha256 (item 8). A root
the certificate reader refuses (a version 1 root, for one) is taken as its name and key, as RFC
5280 section 6.1.1 (d) defines a trust anchor, only when a walk over it shows it holds no
nameConstraints: a version 1 or 2 certificate, or a version 3 certificate whose extension list
can be walked to its end (each extension's OBJECT IDENTIFIER read, no value interpreted) and
names no nameConstraints. A refused root that names nameConstraints, or whose extension list
cannot be walked, gives no anchor: `TrustAnchor::from_der` returns the reader's error (D164). A list older than 90 days is refreshed before a
release (`docs/releasing.md`). `task anchors-refresh` prints each title the new list adds or
removes (a title is the line curl writes above each certificate) and refuses, changing
nothing, a list that removes more than 10 titles (`ANCHORS_REMOVED_MAX`, default 10).

Licences, for `NOTICE`: the list is Mozilla's root store, published by Mozilla under the
Mozilla Public License 2.0 (not read again for this note: `story:tls-anchors` quotes the licence
line of curl's page into `SOURCE` when it takes the file); ring 0.17.14 is `Apache-2.0 AND ISC` (`Cargo.toml:160`, its files `LICENSE`,
`LICENSE-BoringSSL` and `LICENSE-other-bits`); of the packages that ship with it (item 2)
untrusted 0.9.0 is ISC and cfg-if, getrandom and libc are `MIT OR Apache-2.0`.

## 12. TLS 1.2

The ClientHello offers the three TLS 1.3 suites and nothing else: RFC 8446 section 9.3, a client
supports every parameter it advertises (`rfc8446.txt:5834-5835`). The coordinating agent took
six TLS 1.2 suites in on 2026-11-22 and reversed it the same day after the fifth critic round
(item 1). A server limited to TLS 1.2 is reported by the three kinds of item 7: (a)
`protocol_version` before any ServerHello, (b) `handshake_failure` before any ServerHello, (c) a
ServerHello below TLS 1.3. surf never continues a TLS 1.2 handshake in this epic.

The recording, on 2026-11-22 on the workstation: `openssl s_server -tls1_2 -www -naccept 1` of
OpenSSL 3.6.5 (29 September 2026) on 127.0.0.1 with a self-signed P-256 certificate, and as the
client `openssl s_client -tls1_3` with the three TLS 1.3 suites, the groups X25519 and P-256, the
six schemes of `signature_algorithms` (item 14), `-servername localhost`, `-alpn http/1.1` and
`-no_ticket`, both with `-msg`.

The client wrote one record, `16 03 01 00 e0`, a ClientHello of 224 bytes: `legacy_version`
0x0303, a legacy session id of 32 bytes, the cipher suites `13 01 13 02 13 03` and nothing else,
and the extensions `server_name` (`localhost`), `ec_point_formats`, `supported_groups` (x25519,
secp256r1), ALPN (`http/1.1`), `encrypt_then_mac`, `extended_master_secret`,
`signature_algorithms` (`04 03 05 03 08 07 08 04 08 05 08 06`), `supported_versions` (`03 04`
only), `psk_key_exchange_modes`, `key_share` (one x25519 share) and `compress_certificate`. It
differs from surf's hello in the five extensions OpenSSL adds (`ec_point_formats`,
`encrypt_then_mac`, `extended_master_secret`, `psk_key_exchange_modes`,
`compress_certificate`) and the one it leaves out (`signature_algorithms_cert`); the suites,
the versions and the groups are surf's.

The server's whole answer was 7 bytes, one record before any ServerHello:

```
<<< TLS 1.2, RecordHeader [length 0005]
    15 03 03 00 02
<<< TLS 1.3, Alert [length 0002], fatal protocol_version
    02 46
```

The server logged `tls_early_post_process_client_hello:unsupported protocol` and the client
`SSL handshake has read 7 bytes and written 229 bytes`. This is kind (a): `protocol_version`
before any ServerHello, `TlsVersionUnsupported`. The recording shows nothing else; kinds (b) and
(c) were not produced by this server. The logs stay in the scratch directory of the unit that
recorded them and are not committed.

Telling kind (b) apart from other handshake failures needs TLS 1.2 itself, the next epic (U3).

## 13. The clock

Validity is checked against a time the caller passes. `surf` and the session pass the system
clock. Library tests pass a fixed time, `2026-11-22T00:00:00Z`; a limbo case without
`validation_time` runs at `LIMBO_TIME`, the `commit-date` of the pinned limbo commit as its
`SOURCE` records it (`story:tls-x509`, `story:tls-x509-path`). Tests through a binary run on the
system clock, with certificates valid until 2125 and an expired one whose end lies in 2001.

## 14. RFC 8446 section 9: what surf offers against each requirement on a client

- 9.1 TLS_AES_128_GCM_SHA256 (MUST): offered; TLS_AES_256_GCM_SHA384 and
  TLS_CHACHA20_POLY1305_SHA256 (SHOULD): offered. Met.
- 9.1 signatures (MUST): rsa_pkcs1_sha256 for certificates, rsa_pss_rsae_sha256 for
  CertificateVerify and for certificates (the reader of `story:tls-x509` takes the RSASSA-PSS
  AlgorithmIdentifier of RFC 4055 with SHA-256, SHA-384 or SHA-512, MGF1 of the same hash and a
  salt of the digest length, case `ok-rsa-pss` of `story:tls-test-certs`),
  ecdsa_secp256r1_sha256: all verified by the seam and offered. `signature_algorithms`
  (CertificateVerify): ecdsa_secp256r1_sha256, ecdsa_secp384r1_sha384, ed25519,
  rsa_pss_rsae_sha256, rsa_pss_rsae_sha384, rsa_pss_rsae_sha512; `signature_algorithms_cert`:
  the same plus rsa_pkcs1_sha256, rsa_pkcs1_sha384, rsa_pkcs1_sha512. Met.
- 9.1 key exchange: secp256r1 (MUST) in `supported_groups`, reached by a HelloRetryRequest;
  X25519 (SHOULD) with a share in the first hello. Met.
- 9.2 extensions (MUST implement, MUST send when applicable): `supported_versions`, `cookie`
  (echoed), `signature_algorithms`, `signature_algorithms_cert`, `supported_groups`,
  `key_share`, `server_name` (sent when the host is a name; RFC 6066 allows no IP literal
  there; the name without a trailing dot, in ASCII lower case, the value the verifier is given,
  item 6). No `pre_shared_key` or `psk_key_exchange_modes`, as no PSK is offered. The
  ClientHello holds both `signature_algorithms` and `supported_groups`, and `key_share` with
  `supported_groups`. Met.
- 9.3 a client supports every parameter it advertises. Shown for each: the three suites and both
  groups complete a handshake with the test server, x25519 directly and secp256r1 after a
  HelloRetryRequest, with `server_name` and ALPN `http/1.1` (`story:tls-test-server` acceptance
  1); each of the six schemes of `signature_algorithms` completes a handshake with the server
  signing its CertificateVerify with it (acceptance 2 there); each of the nine schemes of
  `signature_algorithms_cert` signs the leaf of a case that path validation accepts
  (`ok-rsa2048`, `ok-rsa-pkcs1-sha384`, `ok-rsa-pkcs1-sha512`, `ok-rsa-pss`,
  `ok-rsa-pss-sha384`, `ok-rsa-pss-sha512`, `ok-ecdsa-p256`, `ok-ecdsa-p384`, `ok-ed25519` of
  `story:tls-test-certs`, `story:tls-x509-path` acceptance 1); the `cookie` is copied
  (`story:tls-handshake`). Met since the six TLS 1.2 suites were taken out (item 12).
- 9.3 a client ignores unknown extensions in a CertificateRequest or NewSessionTicket: met
  (`story:tls-handshake`).

No requirement of sections 9.1 to 9.3 is deviated from. A deviation found later is a line here,
named, and goes to the coordinating agent for `host-operator`.
