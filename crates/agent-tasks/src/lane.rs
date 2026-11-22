//! The scripted lane: every task of `tasks.txt`, one after the other, with the `surf-agent` of
//! one directory, against one fixture server (a thread of this process) and one `surfd` in one
//! private `XDG_RUNTIME_DIR`.
//!
//! A run removes its report file first and writes it last. After the last task every session
//! is closed by its task; the run sends SIGKILL to the `surfd` it found (a `surfd` that `open`
//! started ends on its own only after its idle exit of 600 s) and waits for it, at most
//! [`KILL_BOUND`], then removes the private directory. What `/proc` showed is returned, for the
//! caller to hold the run to.

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use surf_test_process::ended_within;

use crate::agent::{Agent, steps};
use crate::format::{read_status, read_tasks};
use crate::judge::judge;
use crate::process::{self, KILL_BOUND, RuntimeDir};
use crate::report::{Lane, TaskResult, report};
use crate::server::serve;
use crate::{sites_dir, status_file, tasks_file};

/// What a lane run is given.
pub struct Config {
    /// The directory of `surf-agent`, `surfd` and `surf-session`.
    pub bin: PathBuf,
    /// Where the private directory goes when the caller's `XDG_RUNTIME_DIR` is not usable.
    pub fallback: PathBuf,
    /// The bound of one `surf-agent` call ([`crate::agent::CALL_BOUND`] in the gate).
    pub call_bound: Duration,
    /// The lane the report names.
    pub lane: Lane,
    /// The report's file.
    pub out: PathBuf,
}

/// What a lane run did and what `/proc` showed.
pub struct LaneRun {
    /// The judged results, in the order of `tasks.txt`.
    pub results: Vec<TaskResult>,
    /// What [`report`] returned.
    pub outcome: Result<(), String>,
    /// The `surfd` processes of the private directory after every call, from the first call
    /// after which one was there.
    pub surfd_samples: Vec<Vec<u32>>,
    /// Every process whose `comm` is `cargo` seen below this process.
    pub cargo_seen: BTreeSet<u32>,
    /// The wait for each `surfd` after SIGKILL.
    pub ended: Vec<Result<Duration, String>>,
    /// The processes of the private directory left after that, with their `comm`.
    pub left: Vec<(u32, Option<String>)>,
    /// The private directory, removed at the end of the run.
    pub directory: PathBuf,
}

/// Run every task and write the report.
pub fn run(config: &Config) -> LaneRun {
    let _ = fs::remove_file(&config.out);
    let tasks = read_tasks(&fs::read_to_string(tasks_file()).expect("tasks.txt"))
        .unwrap_or_else(|error| panic!("{error}"));
    let status = read_status(
        &fs::read_to_string(status_file()).expect("status.txt"),
        &tasks,
    )
    .unwrap_or_else(|error| panic!("{error}"));
    let dir = RuntimeDir::new(&config.fallback);
    let directory = dir.path().to_path_buf();
    let server = serve(sites_dir()).expect("the fixture server listens");
    let agent = Agent::new(config.bin.clone(), dir.runtime()).with_call_bound(config.call_bound);
    let own = std::process::id();

    let mut surfd_samples: Vec<Vec<u32>> = Vec::new();
    let mut cargo_seen = BTreeSet::new();
    let mut results = Vec::new();
    for task in &tasks {
        let steps = steps(&task.name).unwrap_or_else(|| panic!("no steps for {}", task.name));
        let before = server.accepted();
        // One pass over /proc that reads each `comm` only: the environment of a surfd, and the
        // parents of a cargo, are read for those alone (reading every process's stat after each
        // call took most of the run's time on a loaded host).
        let mut sample = || {
            let mut surfd = Vec::new();
            for pid in process::pids() {
                match process::comm(pid).as_deref() {
                    Some("surfd") if dir.holds(pid) => surfd.push(pid),
                    Some("cargo") if process::descends_from(pid, own) => {
                        cargo_seen.insert(pid);
                    }
                    _ => {}
                }
            }
            if !surfd.is_empty() || !surfd_samples.is_empty() {
                surfd_samples.push(surfd);
            }
        };
        let mut run = agent.run(
            &steps,
            &server.address(&task.name, &task.start),
            &mut sample,
        );
        run.connections = server.accepted() - before;
        results.push(TaskResult {
            task: task.name.clone(),
            calls: run.calls,
            connections: run.connections,
            verdict: judge(task, &run),
        });
    }

    let mut surfd: BTreeSet<u32> = surfd_samples.iter().flatten().copied().collect();
    surfd.extend(dir.processes_named("surfd"));
    for &pid in &surfd {
        process::signal_kill(pid);
    }
    let ended = surfd
        .iter()
        .map(|&pid| ended_within(pid, KILL_BOUND))
        .collect();
    let left = dir
        .processes()
        .into_iter()
        .map(|pid| (pid, process::comm(pid)))
        .collect();
    drop(dir);

    let outcome = report(&results, &status, &config.lane, &config.out);
    LaneRun {
        results,
        outcome,
        surfd_samples,
        cargo_seen,
        ended,
        left,
        directory,
    }
}
