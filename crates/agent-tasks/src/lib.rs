//! Agent tasks: fixture sites served on 127.0.0.1, a question each and the expected answer, solved
//! with `surf-agent` and nothing else. Test tooling, not shipped.
//!
//! - `tasks.txt` holds one block per task and `status.txt` one line per task, read by
//!   [`format`]. No count of tasks is written anywhere: every count is read from `tasks.txt`.
//! - `sites/<task>/` holds the fixture files of each task, served by [`server`] under
//!   `/<task>/`, one server for every task.
//! - [`agent`] is the scripted agent of the gate's lane: the steps of each task and the
//!   `surf-agent` calls that take them, every handle found by its label in an answer.
//! - [`judge`] decides a task from what the agent read, and [`report`] writes the run's file
//!   and holds each task to the state `status.txt` lists for it.
//! - [`process`] finds the processes of a private `XDG_RUNTIME_DIR` through `/proc` and ends
//!   them.
//!
//! The model lane runs outside the gate: `taskfiles/agent-tasks-model.sh` with the program
//! `agent-tasks-serve` of this crate.

#![forbid(unsafe_code)]

pub mod agent;
pub mod format;
pub mod judge;
pub mod process;
pub mod report;
pub mod server;

pub use judge::{Run, StepFailure, Verdict, judge};
pub use report::{TaskResult, report};

use std::path::PathBuf;

/// `tasks.txt` of this crate.
pub fn tasks_file() -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/tasks.txt"))
}

/// `status.txt` of this crate.
pub fn status_file() -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/status.txt"))
}

/// `sites/` of this crate: one directory per task.
pub fn sites_dir() -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/sites"))
}

/// The run's file of the gate's lane, the root `.out/agent-tasks.txt`: only `tasks::every_task`
/// writes it, and `task agent-tasks-report` prints it.
pub fn root_report_file() -> PathBuf {
    PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../.out/agent-tasks.txt"
    ))
}
