//! The integration tests of `surf-view`, one test binary: one module per file of `tests/`.
//! A new test file gets its `mod` line here (AGENTS.md, Layout).

#![forbid(unsafe_code)]

mod adversary_view;
mod http;
mod json;
mod prototype;
mod stream_bounds;
mod web;
