//! The integration tests of `surf-webidl`, one test binary: one module per file of `tests/`.
//! A new test file gets its `mod` line here (AGENTS.md, Layout).

#![forbid(unsafe_code)]

mod adversary;
mod adversary_2;
mod depth;
mod extraction;
mod reader;
mod refusals;
mod selection;
mod source;
mod vendored;
