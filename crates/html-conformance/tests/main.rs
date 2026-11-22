//! The integration tests of `surf-html-conformance`, one test binary: one module per file of `tests/`.
//! A new test file gets its `mod` line here (AGENTS.md, Layout).

#![forbid(unsafe_code)]

mod check;
mod entities_table;
mod inventory;
mod readers;
mod tree_format;
