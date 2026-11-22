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
| RFC 8446 (TLS 1.3) | `rfc8446.txt`, August 2018 |
| RFC 8448 (example handshake traces) | `rfc8448.txt`, January 2019 |
| ring | 0.17.14, the crates.io package (checksum in item 2) |
| cc | 1.11.1, the crates.io package (checksum in item 2) |
| rustls, rustls-webpki | 0.23.46 and 0.103.17, the crates.io packages, for the bounds of item 9 only |
| OpenSSL | 3.6.5 of 29 September 2026, the `openssl` command of the workstation |

Line numbers below are those of these versions. The workstation is the shared build machine the
coordinating agent works on (x86_64 Linux, GCC 16.2.1, musl 1.2.6); its builds and tests run
through the machine's build pool (`slot`), which queues a job until a slot is free; the times
below are wall times of the running job, not of the queue.

## 1. The decisions U1, U2 and U3, and the TLS 1.2 suites

1. **U1, the source of the primitives: `ring` 0.17.14.** The operator, 2026-11-22 05:47 CET,
   relayed by the session `host-operator`: "A: ring 0.17.14". Of the four options of the epic
   it adds the fewest packages (item 2), covers every primitive TLS 1.3 needs, and is the source
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
| libc | 0.2.191 | `65cd87c9a6d60536580b316ad985726104730a5151c99845f2490428ef91240a` | getrandom (Unix), ring (aarch64 Apple only) | yes |
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
| `release x86_64-apple-darwin` (`macos-15-intel`, Darwin 24.6.0 x86_64) | `success` | `cc`, `gcc`, `clang`: Apple clang 17.0.0 (clang-1700.0.13.5); no musl compiler; no `CC` variable | `CC_x86_64_apple_darwin = None`, `running: "cc" …` | `ring build: exit 0, 16 s` | `probe build: 0 s` | `Mach-O 64-bit executable x86_64`, linked to `/usr/lib/libSystem.B.dylib` alone | `probe ok` |
| `release aarch64-apple-darwin` (`macos-15`, Darwin 24.6.0 arm64) | `success` | the same Apple clang 17.0.0 | `CC_aarch64_apple_darwin = None`, `running: "cc" …` | `ring build: exit 0, 12 s` | `probe build: 0 s` | `Mach-O 64-bit executable arm64`, linked to `/usr/lib/libSystem.B.dylib` alone | `probe ok` |
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
   assembly, and the binary links to `libSystem` alone, as surf's binary does today.
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
same objects, and the binary is byte for byte the one `musl-gcc` makes (`cmp` of the two
binaries). So on x86_64, setting
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
slow operation: its median at opt-level 0 was 58.9 ms in the first run, over the bound of 50 ms,
and 18.1, 24.3 and 5.6 ms in the three after; its smallest call fell to 2.7 ms when the machine
was quieter. Its spread is the machine's load, not the code (the agreements of the first run
show single calls of about 50 ms too); on the loaded machine the gate runs on, those numbers are
what the test step pays. With ring at opt-level 3 P-384 costs 1.1 ms on a quiet machine and its
medians were 1.1 to 16.7 ms, RSA verification about a third of its opt-level 0 time.

So, as the story's rule asks for a verification over 50 ms, the line for `.cargo/config.toml`
(`story:tls-lock-allow` writes it):

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
  (application data without protection), `Downgrade`, `UnexpectedChangeCipherSpec`,
  `BadCertificateVerify`, `BadFinished`.
- `SequenceExhausted`.
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
| `Unreadable` | a certificate the server sent that the X.509 reader refuses: bad DER, a version 1 leaf, `AlgorithmMismatch` and the other errors of the reader | `bad_certificate` | `Unreadable: certificate <n> of the server: <error of the reader>` |
| `NoPathToAnchor` | no path from the leaf to a trust anchor | `unknown_ca` | `NoPathToAnchor: no path to a trust anchor` |
| `BadSignature` | a signature on the path does not verify | `bad_certificate` | `BadSignature: the signature of certificate <n> does not verify` |
| `IssuerNotCa` | an issuer without basicConstraints cA | `bad_certificate` | `IssuerNotCa: certificate <n> issues but is no CA` |
| `PathTooLong` | more intermediates than item 9 allows, or a pathLenConstraint broken | `bad_certificate` | `PathTooLong: <k> intermediates, at most <bound>` or `PathTooLong: pathLenConstraint of certificate <n> broken` |
| `CandidateLimit` | the most path candidates of item 9 tried | `bad_certificate` | `CandidateLimit: <bound> candidates tried` |
| `KeyUsage` | keyUsage without keyCertSign on an issuer, or without digitalSignature on the leaf | `bad_certificate` | `KeyUsage: certificate <n> lacks <keyCertSign or digitalSignature>` |
| `UnknownCriticalExtension` | a critical extension the reader does not know | `bad_certificate` | `UnknownCriticalExtension: <OID> in certificate <n>` |
| `NameConstraint` | a name outside a nameConstraints of an issuer or of the anchor | `bad_certificate` | `NameConstraint: <name> outside the constraints of certificate <n>` |
| `NoServerAuth` | extKeyUsage without serverAuth on the leaf | `bad_certificate` | `NoServerAuth: the leaf is not for TLS servers` |
| `WeakAlgorithm` | an MD5 or SHA-1 signature, decided before any verification | `bad_certificate` | `WeakAlgorithm: <algorithm> in certificate <n>` |
| `WeakKey` | an RSA key under 2048 bits, decided before any verification | `bad_certificate` | `WeakKey: RSA <bits> bits in certificate <n>` |

`<n>` counts the certificates as the server sent them, the leaf 0.

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
| every other alert received or protocol violation (`AlertReceived`, `AlertSent` with any other violation, `SequenceExhausted`) | `TlsHandshakeFailed` | the alert of the violation, or none | `<alert>: <violation>` for an alert surf sent, `the server sent <alert>` for one it received |
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
CA (basicConstraints cA). Why 32 KiB: an `open` request holds at most `surf_agent::wire::
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
   bound }`.
2. **The most certificates in a Certificate message: 64.** Reason: more than the 50 of
   `candidate-limit`, so that the path builder sees that case whole; the length bound above
   alone would let some 13,000 near-empty entries be parsed one by one. One more is
   `bad_certificate`, `TooManyCertificates`.
3. **The most intermediates on a path: 6**, rustls-webpki's `MAX_SUB_CA_COUNT`
   (`src/verify_cert.rs:873` of 0.103.17). Reason: the web's chains have one or two
   intermediates; a longer path is either an attack on the search or a mistake. `path-too-long`
   of `story:tls-test-certs` has 7. More is `NotTrusted(PathTooLong)`.
4. **The most path candidates tried: 100**, counted as signature verifications of a candidate
   issuer over a certificate in one validation, the budget rustls-webpki keeps
   (`signatures: 100`, `src/verify_cert.rs:355-360`, taken there from the remedy of Go's
   CVE-2018-16875). Reason: the search over issuers with one subject and different keys is
   exponential in the depth; the bound makes its cost at most 100 verifications, which item 4
   prices in the test profile, with the line of item 4, at about 10 ms for RSA-2048 or P-256
   keys (100 × 0.03 to 0.09 ms) and 0.1 to 1.7 s for P-384 keys (100 × the medians 1.1 to
   16.7 ms). So the hostile case `candidate-limit` of `story:tls-test-certs`, which must end in
   under 1 s (`story:tls-x509-path` acceptance 5), uses P-256 or RSA keys for its 48
   intermediates. Reaching it is `NotTrusted(CandidateLimit)`.
5. **One HelloRetryRequest.** RFC 8446 section 4.1.4: "If a client receives a second
   HelloRetryRequest in the same connection [...] it MUST abort the handshake with an
   "unexpected_message" alert" (`rfc8446.txt:1837-1840`): `unexpected_message`,
   `SecondHelloRetryRequest`.

## 10. The test design, per material

Every material is vendored with a `SOURCE` file of the form of
`crates/html-conformance/data/*/SOURCE` (where it came from, the commit or date, the licence,
the files and their sha256), and a fast check in the gate compares each file with its digest.
The sizes of the upstream files were not measured for this note (the unit that wrote it had no
network); the story that vendors a material records the size in its `SOURCE` and in its
report.

1. **Wycheproof** (C2SP, Apache-2.0): a subset in `crates/tls/tests/data/wycheproof/`, vendored
   by `story:tls-lock-allow`: `aes_gcm_test`, `chacha20_poly1305_test`, `x25519_test`,
   `ecdh_secp256r1_test`, `ecdsa_secp256r1_sha256_test`, `ecdsa_secp384r1_sha384_test`,
   `ed25519_test`, `hkdf_sha256_test`, `hkdf_sha384_test`, and the RSA files for 2048 bits,
   `rsa_pss_2048_sha256_mgf1_32_test.json` among them. The AES-192 vectors of `aes_gcm_test`
   are a named skip, ring has no AES-192 (item 6); the test prints their count.
2. **RFC 8448** in `crates/tls/tests/data/rfc8448/`: sections 3, 5 and 7, replayed with their
   private keys (item 5). Its RSA server key is 1024 bits, so ring cannot check its
   CertificateVerify; the trace tests the key schedule, the record layer and the parsing.
3. **x509-limbo** (C2SP, Apache-2.0) in `crates/tls/tests/data/x509-limbo/`, at a pinned
   commit; 9,807 cases on 2026-11-22 (epic, "Testing within the budget"). When `limbo.json` at
   the pinned commit is over 20 MiB, `story:tls-x509` keeps every case not of the `bettertls`
   family and the `bettertls` cases in file order up to 20 MiB, and states in `SOURCE` the size,
   the count kept and the count left out.
4. **The generated certificates** in `crates/tls-test-certs/certs/`: made by
   `surf-tls-test-certs` through ring under a fixed source (reproducible, item 5), committed
   with the digest of their sources; the cases ring cannot make (a SHA-1 signature, an MD5
   signature, an RSA-1024 key, item 6) made once by `openssl` in the lane `slow` and committed
   as bytes. The gate never runs `openssl`.
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

BoGo, BoringSSL's TLS suite, is not used: it needs Go and a long run. The lanes: `tls-certs-check`
and `tls-flights-check` in `check`, `tls-certs-slow` and `tls-flights-slow` in `slow`.

## 11. U2 in detail: the bundled anchors

The file is curl's extract of Mozilla's root store, the `cacert.pem` that
`https://curl.se/docs/caextract.html` publishes: PEM, checked daily, converted when Mozilla's store
changes, with a `.sha256` beside it. On 2026-11-22 the coordinating agent read its date as
2026-11-05 and its count as 118 certificates (epic, U2). It lives in `crates/tls/src/anchors/`
with a `SOURCE` that holds its date, its count and the sha256 of its `.sha256` file, written by
`story:tls-anchors` when it takes the file; `surf --anchors` prints those three (item 8). A root
the certificate reader refuses (a version 1 root, for one) is taken as its name and key, as RFC
5280 section 6.1.1 (d) defines a trust anchor. A list older than 90 days is refreshed before a
release (`docs/releasing.md`).

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
  there). No `pre_shared_key` or `psk_key_exchange_modes`, as no PSK is offered. The
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
