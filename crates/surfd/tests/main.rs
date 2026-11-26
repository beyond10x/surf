//! The integration tests of `surfd`, one test binary: one module per file of `tests/`.
//! A new test file gets its `mod` line here (AGENTS.md, Layout).

#![forbid(unsafe_code)]

mod adversary;
mod adversary_2;
mod adversary_controls;
mod adversary_link;
mod built;
mod common;
mod first_line;
mod hardening;
mod in_process;
mod log;
mod view;
