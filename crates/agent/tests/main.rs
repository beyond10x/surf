//! The integration tests of `surf-agent`, one test binary: one module per file of `tests/`.
//! A new test file gets its `mod` line here (AGENTS.md, Layout).

#![forbid(unsafe_code)]

mod adversary_ca_file;
mod adversary_fill_pick;
mod adversary_skill_definition;
mod ca_file;
mod common;
mod drift;
mod help_for_agent;
mod secreview_refusal_text;
mod stand_in;
mod start;
mod token;
mod usage;
