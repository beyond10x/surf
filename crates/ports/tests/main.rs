//! The integration tests of `surf-ports`, one test binary: one module per file of `tests/`.
//! A new test file gets its `mod` line here (AGENTS.md, Layout).

#![forbid(unsafe_code)]

mod adversary_document_forms;
mod adversary_history;
mod agent;
mod history;
mod ports;
mod review1_ports;
