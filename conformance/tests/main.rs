//! The integration tests of `surf-conformance`, one test binary: one module per file of `tests/`.
//! A new test file gets its `mod` line here (AGENTS.md, Layout).

#![forbid(unsafe_code)]

mod adversary;
mod adversary_2;
mod adversary_noticed;
mod planted_defect;
mod review2_planted_defects;
mod two_of_each_entity;
