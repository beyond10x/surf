//! A page's scripts on the script engine swell (`docs/design/scripts.md`): one agent per
//! runner, on a thread of its own whose stack is the agent's `min_stack_bytes()` plus a margin
//! for surf's own frames, under surf's limits, with the host hooks of the note's Q6. A source is
//! parsed and run to its end, and the runner answers how it ended with a word of the note's
//! closed list of ends (Q7). A runner of a document (`run_document`) also holds the generated
//! DOM bindings (`bindings`) over the document's tree, answered by surf's slices (`dom`): the
//! members of `reading` read the tree; the others throw `NotSupportedError` until their stories.
//!
//! swell comes in as a git dependency on a release tag (`crates/script/Cargo.toml`), and no
//! other crate of this repository may depend on it or on this crate but those
//! `taskfiles/lock-owners.txt` names (`task lock-check`).

#![forbid(unsafe_code)]

pub mod bindings;
pub mod dom;
mod runner;

pub use runner::{
    Document, MESSAGE_CHARS, Mode, Reason, Runner, RunnerError, RunnerLimits,
    SESSION_ENGINE_LIMITS, ScriptEnd, ScriptError, ScriptErrorKind, ScriptHost, Site,
    TEST_CLOCK_START, TEST_CLOCK_STEP, run, run_document,
};
