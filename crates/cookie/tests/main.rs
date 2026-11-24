//! The integration tests of `surf-cookie`, one test binary: one module per file of `tests/`.
//! A new test file gets its `mod` line here (AGENTS.md, Layout).

#![forbid(unsafe_code)]

mod adversary_jar;
mod bound;
mod hosts;
mod jar;
mod secreview2_chain;
mod secreview_jar;
mod shadow;
