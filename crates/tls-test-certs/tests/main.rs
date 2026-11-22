//! The integration tests of `surf-tls-test-certs`, one test binary: one module per file of
//! `tests/` (AGENTS.md, Layout). They read the committed files of `certs/`; only
//! `certs::reproducible` builds the cases, in memory. No test runs `openssl`, generates a key or
//! reads the clock.

#![forbid(unsafe_code)]

mod adversary;
mod certs;
mod check;
