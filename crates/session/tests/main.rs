//! The integration tests of `surf-session`, one test binary: one module per file of `tests/`.
//! A new test file gets its `mod` line here (AGENTS.md, Layout).

#![forbid(unsafe_code)]

mod adversary_anchors;
mod adversary_back_forward;
mod adversary_controls;
mod adversary_change_log;
mod adversary_document_text;
mod adversary_events;
mod adversary_fill_pick;
mod adversary_follow;
mod adversary_parse;
mod adversary_press;
mod adversary_values;
mod adversary_view;
mod back_forward;
mod built;
mod change_and_log;
mod common;
mod core;
mod e2e;
mod events;
mod fill_pick;
mod follow;
mod follow_refusals;
mod form_rows;
mod https;
mod memory;
mod press;
mod redirect;
mod view;
