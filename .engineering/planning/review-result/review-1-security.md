---
format: aep.planning-md/3
id: review-result:review-1-security
kind: review-result
status: active
title: Security reviewer, review 1, waves 1 to 4 of epic headless-rust-target
relations:
- reviews: story:net-fetch-failure-reasons
- reviews: story:generated-rust-workspace
- reviews: story:http-fetch-library
- reviews: story:memory-ports
- reviews: story:headless-cli
revision: 2
---
```
unit: review-1 (security-reviewer), main at 3aaa3aaebc6d38baaf20815d1433d08128c9b946, tree surf-r1-sec
verdict: red
cases: executed 60→80, red 5
origin: introduced 6, pre-existing 0, undecided 0
wrote-outside-worktree: 10 paths under $HOME/.cache/scratch/b84c1d1b-86a1-4880-9b86-d11fa8d6a813/waves/review-1/security (listed in part 6)
needs-coordinator: no
```

All 7 invariants were checked. Two findings need a change: a drip-fed response keeps one fetch running with no deadline, and Unicode line and paragraph separators in the address pass through to both output streams. The other four are notes.

## 1. Diff stat

`git --no-pager diff --stat` is empty: no tracked file changed. `git status --short` shows three new files, all tests, left uncommitted:
```
?? crates/fetch/tests/review1_fetch.rs
?? crates/ports/tests/review1_ports.rs
?? crates/surf/tests/review1_output.rs
```

## 2. Cases added (each red case was run alone first; logs are in scratch as `alone-*.log`)

| id | sev | origin | file:line | what was measured | what reaches it | test |
|---|---|---|---|---|---|---|
| S1 | warning | introduced | crates/fetch/src/lib.rs:165 | `one fetch took 4.008987335s with a read timeout of 200ms and ended Ok(40)`: the server sent 1 byte every 100 ms | Any server `surf <url>` loads. With the default limits a fetch can last about 22.7 days on the header (64 KiB × 30 s) and about 16 years on the body (16 MiB × 30 s). Name resolution has no timeout, and the 10 s connect timeout applies once per resolved address. The story says "never a hang". | `review1_a_drip_fed_response_ends_within_a_bound_of_the_read_timeout` (review1_fetch.rs:414) |
| S2 | note | introduced | crates/fetch/src/lib.rs:400 | `body limit 65536 bytes, body buffer allocated 131040 bytes`. The docs at :36 claim "body limit plus a few KiB". | Any server using chunked encoding. A first piece of 4095 bytes makes growth double past the limit: with defaults, about 32 MiB for a 16 MiB limit. | `review1_body_allocation_stays_within_the_body_limit_plus_a_few_kib` (:454) |
| S3 | note | introduced | crates/fetch/src/lib.rs:616 | `panicked at crates/fetch/src/lib.rs:616:22: attempt to add with overflow` (`max - body.len() + 1`) | Only with `Limits { max_body_bytes: usize::MAX }` and a close-delimited body. Nothing found sets that: `surf` uses `Limits::default()`. | `review1_unlimited_body_with_a_close_delimited_response_does_not_panic` (:480) |
| S4 | note | introduced | crates/fetch/src/lib.rs:234 | `http://[127.0.0.1]:41043/` connected and sent `Host: [127.0.0.1]:41043`; `[localhost]` would also resolve | The address on the command line. It reaches the host named inside the brackets (not somewhere else), but sends a Host value RFC 3986 §3.2.2 does not allow. | `review1_bracketed_host_that_is_not_ipv6_is_refused_without_connecting` (:189) |
| S5 | warning | introduced | crates/surf/src/lib.rs:218 | U+2028, U+2029 and U+202E pass through `escape`. Splitting stdout on U+2028/U+2029 turns 3 printed rows into 6 forged `request 00000000-… Responded http://forged/` rows plus 3 reordered `‮gnp.exe` lines. stderr gets the same through main.rs:32. | The command-line address. Server bytes cannot reach it, because `surf-fetch` refuses non-ASCII. The code matches README.md:81, which promises to escape control characters (Cc) only. | `review1_unicode_line_separators_and_bidi_controls_cannot_split_or_reorder_a_row` (review1_output.rs:152) |

Green probes (13), each now passing:

**Fetch: address side**
- 32 hostile addresses are all refused before any connection is made: CR, LF, bare CR, TAB, NUL, DEL, U+0085, U+2028, non-ASCII, spaces in host or port, user info, `\@`, `http:///`, `http:\\`, ports 0, 00, 65536, port+65536 and 2^32, `+p`, `-p`, `p:p`, `0x…`, and unclosed or trailing `]`.
- The request bytes are exactly `GET <target> HTTP/1.1\r\nHost: <authority>\r\nConnection: close\r\n\r\n` for 6 addresses, including a `#@` fragment and percent-encoded CRLF.

**Fetch: response side**
- Endless 1xx responses, an endless header line, endless fields and endless obs-fold lines each end as malformed within the header limit.
- 10 bad `Content-Length` forms are malformed: negative, `+`, hex, list members that differ, empty, overflow, non-ASCII digit. `u64::MAX` is refused in under 400 ms.
- `Transfer-Encoding` overrides `Content-Length`. Chunked twice, other codings, an empty value and a folded `gzip` are all refused.
- A body of exactly the limit is accepted and one byte more is malformed, for length, close and chunked framing alike.
- 8 bare-CR placements are refused.
- Bad chunk sizes, a 1100-byte extension and an endless extension are malformed.

**Ports**
- Every identity is read into a fresh, zeroed 16-byte buffer (20 identities across 4 ports types).
- Exactly 122 source bits are carried, with no two bits landing in the same place.
- A short fill followed by an error gives an unmet obligation and no row.
- From `/dev/urandom`, over 2000 identities every free bit is set 700 to 1300 times; only bits 48–51 and 64–65 are fixed.

**Output**
- ESC, OSC, CSI, the 8-bit control set (C1), DEL, backspace and CRLF row forgery from the command line (scheme, host, path, `ftp:`, bare) reach neither stream.
- The same control bytes from a server (reason phrase, header fields, `Content-Type`, body, a non-HTTP reply, a bad chunk line) reach neither stream.

## 3. Suite run

Command: `RUSTFLAGS='-D warnings' cargo test --workspace --locked --no-fail-fast`. Exit 101.
```
error: 2 targets failed:
    `-p surf --test review1_output`
    `-p surf-fetch --test review1_fetch`
29 test suites: 75 passed, 5 failed, 0 ignored
```
The before count comes from a second run after my cases existed, with `-- --skip review1_` (all 20 of my cases deselected): exit 0, `29 test suites: 60 passed, 0 failed, 0 ignored`. That matches the 60 in docs/plan/wave-4.md.

## 4. Judgement finding

- **S6** (note, CONFIRMED, introduced), crates/ports/src/random.rs:71: `.map_err(|_| unmet_context(kind))` throws away the `io::Error`. The failure itself still reaches the caller: the existing test `failing_source_…` and my short-fill probe both show it, and the binary exits 3. But the reason (for example ENOENT or EMFILE) is lost, because the generated `UnmetObligation` holds only `&'static str`. One way to keep it is for `OsRandom` to hold its last error so the caller can report it.

Corrections to consider (I applied none):
- **S1:** add a whole-fetch deadline to `Limits`, and set each socket timeout to the smaller of the read timeout and the time left.
- **S2:** reserve against the remaining limit (`try_reserve_exact`), or document the doubling.
- **S3:** use `max.saturating_sub(len).saturating_add(1)`.
- **S4:** require the bracketed text to parse as an `Ipv6Addr`.
- **S5:** also escape U+2028, U+2029 and the bidi format characters U+202A–202E and U+2066–2069.

## 5. Reviewed and found sound

- **Invariant 1:** no input I tried made it panic. Every limit holds, apart from the deadline (S1) and the 2× buffer (S2). Header parsing uses checked or saturating arithmetic throughout (lib.rs:360-405, 453-510).
- **Invariant 2:** the address check covers bytes 0x21–0x7e over the whole address (lib.rs:216). Numeric hosts such as `2130706433` resolve the same way the WHATWG URL parser reads them. Not probed: IPv6 zone identifiers.
- **Invariant 3:** OS randomness is read with `read_exact`, so a short read is an error (random.rs:57). There is no fallback value.
- **Invariant 4:** port tables are private, and each storage trait is implemented only on its own ports type (ports.rs:124-206). The generated port keeps its `behaviors` field private (shell/src/lib.rs:34). `Browser::answer` touches only `self.system.net` and the transport (surf/src/lib.rs:467-497). This was cleared by reading the code; no case runs it.
- **Invariant 5:** see S5. The failure detail is safe: the scheme is printed with `{:?}` and all other details are ASCII or OS text.
- **Invariant 6:** there are no `source` lines in Cargo.lock, no `unsafe` in any `.rs` file, and `#![forbid(unsafe_code)]` is in all 4 hand-written crate roots and all 7 generated ones.
- **Invariant 7:** `set: [pipefail]` is on. A scratch probe showed go-task 3.54 stops a multi-line command at its first failure and fails `false | cat`. No step uses `|| true`, `status:` or `preconditions`.

Not done:
- The per-field memory cost of 64 KiB of tiny header lines was not measured.
- The 10 s connect timeout and resolver blocking were not tested, since no test may leave the machine.
- No IPv6 listener was available for testing literals over the wire.

## 6. Paths written outside the worktree

All under `$HOME/.cache/scratch/b84c1d1b-86a1-4880-9b86-d11fa8d6a813/waves/review-1/security/`:
- `alone-review1_a_drip_fed_response_ends_within_a_bound_of_the_read_timeout.log`
- `alone-review1_body_allocation_stays_within_the_body_limit_plus_a_few_kib.log`
- `alone-review1_unlimited_body_with_a_close_delimited_response_does_not_panic.log`
- `alone-review1_bracketed_host_that_is_not_ipv6_is_refused_without_connecting.log`
- `alone-review1_unicode_line_separators_and_bidi_controls_cannot_split_or_reorder_a_row.log`
- `suite-after.log`
- `suite-before.log`
- `taskprobe/Taskfile.yml`
- the directories `security/` and `security/taskprobe/`

Build output went to the worktree's `target/`. The lease was taken and released.

## 7. Findings

```findings
- file: crates/fetch/src/lib.rs
  line: 165
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: only a per-read timeout bounds a fetch, so a server dripping one byte per read timeout holds one fetch for up to (64 KiB + 16 MiB) x 30 s at the defaults, against the story's "never a hang"
- file: crates/fetch/src/lib.rs
  line: 400
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: amortised Vec growth lets a server choose chunk pieces so the body buffer reaches about twice max_body_bytes (131040 bytes for a 65536-byte limit), contrary to the documented "body limit plus a few KiB"
- file: crates/fetch/src/lib.rs
  line: 616
  category: boundary
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: with max_body_bytes = usize::MAX a close-delimited body whose first byte arrives after the header section overflows max - body.len() + 1 and panics in debug builds; no caller found sets that limit
- file: crates/fetch/src/lib.rs
  line: 234
  category: boundary
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: a bracketed host that is not an IPv6 address ([127.0.0.1], [localhost]) is accepted, connected to and sent as Host, where RFC 3986 section 3.2.2 allows only IPv6 or IPvFuture in brackets
- file: crates/surf/src/lib.rs
  line: 218
  category: integrity
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: escape leaves U+2028, U+2029 and bidi override characters as themselves, so a command-line address forges extra rows for a reader that splits lines the Unicode way and reorders the row in bidi terminals, on stdout and stderr
- file: crates/ports/src/random.rs
  line: 71
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the random source's io::Error is discarded when mapped to an unmet obligation, so the failure surfaces but its cause never reaches the operator
```
