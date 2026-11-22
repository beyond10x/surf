//! The integration tests of `surf-session`, one test binary: one module per file of `tests/`.
//! A new test file gets its `mod` line here (AGENTS.md, Layout).

#![forbid(unsafe_code)]

mod adversary_document_text;
mod adversary_events;
mod adversary_follow;
mod adversary_parse;
mod adversary_values;
mod adversary_view;
mod built;
mod common;
mod core;
mod e2e;
mod events;
mod fill_pick;
mod follow;
mod follow_refusals;
mod form_rows;
mod memory;
mod redirect;
mod view;
