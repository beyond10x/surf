//! The integration tests of `surf-webidl`, one test binary: one module per file of `tests/`.
//! A new test file gets its `mod` line here (AGENTS.md, Layout).

#![forbid(unsafe_code)]

mod adversary;
mod adversary_2;
mod adversary_3;
mod adversary_4;
mod adversary_5;
mod depth;
mod extraction;
mod reader;
mod refusals;
mod same_object;
mod selection;
mod source;
mod vendored;
