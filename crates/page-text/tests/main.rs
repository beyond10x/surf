//! The integration tests of `surf-page-text`, one test binary: one module per file of
//! `tests/cases/`, each declared here with its `#[path]` line, and the shared pages of
//! `tests/common/` (AGENTS.md, Layout).

#![forbid(unsafe_code)]

mod common;
#[path = "cases/outline.rs"]
mod outline;
