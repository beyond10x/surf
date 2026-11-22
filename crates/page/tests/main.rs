//! The integration tests of `surf-page`, one test binary: one module per file of `tests/`.
//! A new test file gets its `mod` line here (AGENTS.md, Layout).

#![forbid(unsafe_code)]

mod adversary_follow;
mod adversary_select;
mod adversary_values;
mod base_url;
mod elements;
mod label;
mod tree_tables;
mod tree_template;
