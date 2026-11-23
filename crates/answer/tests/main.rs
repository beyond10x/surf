//! The integration tests of `surf-answer`, one test binary: one module per file of `tests/`.
//! A new test file gets its `mod` line here (AGENTS.md, Layout).

#![forbid(unsafe_code)]

mod adversary_follow;
mod back_forward;
mod cut;
mod forging;
mod golden;
mod note_parser;
mod now;
mod parse;
mod press;
mod unquote;
