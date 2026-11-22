//! The integration tests of `surf-tls`, one test binary: one module per file of `tests/tls/`,
//! each declared with `#[path]`; the data lies in `tests/data/`. A new test file gets its two
//! lines here (AGENTS.md, Layout). The counting allocator of `tls/alloc.rs` is the one `unsafe`
//! under `crates/tls`, so this file denies `unsafe` and that module allows it on its one
//! `impl`; every other module forbids it.

#![deny(unsafe_code)]

#[path = "tls/alloc.rs"]
mod alloc;
#[path = "tls/der.rs"]
mod der;
#[path = "tls/limbo_parse.rs"]
mod limbo_parse;
#[path = "tls/pem.rs"]
mod pem;
#[path = "tls/seam.rs"]
mod seam;
#[path = "tls/x509_parse.rs"]
mod x509_parse;
