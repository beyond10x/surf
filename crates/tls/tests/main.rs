//! The integration tests of `surf-tls`, one test binary: one module per file of `tests/tls/`,
//! each declared with `#[path]`; the data lies in `tests/data/`. A new test file gets its two
//! lines here (AGENTS.md, Layout). The counting allocator of `tls/alloc.rs` is the one `unsafe`
//! under `crates/tls`, so this file denies `unsafe` and that module allows it on its one
//! `impl`; every other module forbids it.

#![deny(unsafe_code)]

#[path = "tls/alloc.rs"]
mod alloc;
#[path = "tls/record.rs"]
mod record;
#[path = "tls/rfc8448.rs"]
mod rfc8448;
#[path = "tls/seam.rs"]
mod seam;
