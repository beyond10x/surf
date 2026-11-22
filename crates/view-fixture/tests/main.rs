//! The integration tests of `surf-view-fixture`, one test binary: one module per file of `tests/`.
//! A new test file gets its `mod` line here (AGENTS.md, Layout).

#![forbid(unsafe_code)]

mod acceptance;
mod adversary_commands;
mod fixture;
