//! The integration tests of `surf-script`, one test binary: one module per file of `tests/`.
//! A new test file gets its `mod` line here (AGENTS.md, Layout).

#![forbid(unsafe_code)]

mod adversary;
mod adversary_2;
mod adversary_3;
mod adversary_4;
mod bindings;
mod dom_attributes;
mod dom_mutation;
mod dom_reading;
mod dom_selectors;
mod engine;
mod parse_run;
