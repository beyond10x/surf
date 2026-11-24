//! The integration tests of `surf-agent`, one test binary: one module per file of `tests/`.
//! A new test file gets its `mod` line here (AGENTS.md, Layout).

#![forbid(unsafe_code)]

mod adversary_fill_pick;
mod ca_file;
mod common;
mod drift;
mod stand_in;
mod start;
mod token;
mod usage;
