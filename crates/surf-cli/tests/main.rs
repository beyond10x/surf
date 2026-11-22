//! The integration tests of `surf-cli`, one test binary: one module per file of `tests/`.
//! A new test file gets its `mod` line here (AGENTS.md, Layout).

#![forbid(unsafe_code)]

mod cli;
mod output_streams;
mod page;
mod redirects;
mod review1_cli;
mod review1_output;
mod review2_output;
