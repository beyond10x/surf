---
title: Principles
description: The rules surf is built by.
---

# Principles

surf aims to be a compliant, minimal and secure browser. These are the rules that follow from
that.

1. **Specification first.** No behaviour exists in code that the specification under
   [`spec/`](https://github.com/beyond10x/surf/tree/main/spec) does not state. A gap is reported,
   not filled in quietly.
2. **Complete means complete.** Work stays marked incomplete until it passes the standard's own
   tests. Skipped cases are counted and listed by name.
3. **Isolation over speed.** A component is a security boundary. A design that needs two
   components to share memory is refused.
4. **No third-party code in what ships.** The Rust standard library and the operating system are
   allowed. Every outside package of the lock file is on the allow-list
   `taskfiles/lock-allow.txt` (19 of its 49 packages), and the lock check fails on one that is
   not. One exception: audited cryptographic primitives for TLS come from ring 0.17.14,
   reviewed as a dependency in the rustls audit of Cure53, 2020
   ([the report](https://cure53.de/pentest-report_rustls.pdf)). Only the crate `surf-tls` may
   depend on it, and the checks pin its packages by name, version and checksum. The binaries
   built from `main` use it; no release does yet.
5. **No unsafe code.** Every hand-written crate forbids it. The exceptions are in tests: three
   counting allocators that measure what a fetch holds, what a parse allocates and what the
   TLS code allocates.
6. **Text from the web stays text.** Anything from a page, a server or an address is escaped
   before it is printed, so a page cannot forge a line of surf's output.
7. **A passing test must be able to fail.** Tests are tried against planted defects. Over the
   whole suite (`task conform-slow`), a navigation storage that drops every write fails 50 of
   the 196 scenarios; the gate runs each of the 29 planted defects over the first 3 scenarios
   that notice it.
8. **Small.** A standard is followed for what surf does, not implemented whole.

## Compliant, minimal, secure

- **Compliant.** Where a web standard covers a behaviour, the specification follows it: the
  WHATWG HTML, Fetch, URL, DOM, MIME Sniffing and Encoding standards, the W3C CSS specifications,
  ECMA-262 for script, and RFC 9110 (HTTP) and RFC 8446 (TLS 1.3). The cross-browser test suite
  for them is web-platform-tests.
- **Minimal.** A feature is specified only when a step of the plan needs it.
- **Secure.** Isolation wins over speed and memory.

Compliance is claimed per section of a standard. A section is marked complete only when every
requirement in it is specified and the tests the standard's suite has for that section pass
(web-platform-tests; test262 for ECMA-262). Everything else is marked incomplete.
