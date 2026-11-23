//! The integration tests of `surf`, one test binary: one module per file of `tests/`.
//! A new test file gets its `mod` line here (AGENTS.md, Layout).

#![forbid(unsafe_code)]

mod adversary_forms;
mod common;
mod conform_totals;
mod escape;
mod forget_delivered;
mod forms;
mod generated_check_links;
mod generated_check_modes;
mod held;
mod library;
mod lock_check;
mod parse;
mod request_method;
mod review1_generated_check;
mod review1_library;
mod review2_generated_check;
mod slowest_tests;
