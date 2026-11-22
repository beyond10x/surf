//! The tests of `surf-test-process`: the decision on `/proc/<pid>/stat` lines without a process
//! (`lines`), and the reads and the bounded wait on real children (`children`). The children are
//! `examples/sleeper.rs` of this package, not a program of the machine.

#![forbid(unsafe_code)]

mod children;
mod lines;
