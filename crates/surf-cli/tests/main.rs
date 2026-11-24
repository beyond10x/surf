//! The integration tests of `surf-cli`, one test binary: one module per file of `tests/`.
//! A new test file gets its `mod` line here (AGENTS.md, Layout).

#![forbid(unsafe_code)]

mod adversary_document_command;
mod adversary_tls_cli;
mod cli;
mod https;
mod output_streams;
mod page;
mod redirects;
mod review1_cli;
mod review1_output;
mod review2_output;
mod secreview_tls_cli;
