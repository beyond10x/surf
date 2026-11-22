//! The integration tests of `surf-url`, one test binary: one module per file of `tests/`.
//! A new test file gets its `mod` line here (AGENTS.md, Layout).

#![forbid(unsafe_code)]

mod form_urlencoded;
mod limits;
mod urltestdata;
