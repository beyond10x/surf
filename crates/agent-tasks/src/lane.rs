//! The scripted lane: every task of `tasks.txt`, one after the other, with the `surf-agent` of
//! one directory, against one fixture server (a thread of this process) and one `surfd` in one
//! private `XDG_RUNTIME_DIR`.
//!
//! A run removes its report file first and writes it last. After the last task every session
//! is closed by its task: the run waits, at most [`END_BOUND`], until no process of the
//! private directory but its `surfd` is left, since a session exits after `surfd` relayed its
//! answer to `close`. It then sends SIGKILL to the `surfd` it found (a `surfd` that `open`
//! started ends on its own only after its idle exit of 600 s), waits for it, at most
//! [`KILL_BOUND`], waits again, at most [`END_BOUND`], until no process of the directory is
//! left, and removes the private directory. What `/proc` showed is returned, for the caller to
//! hold the run to.
//!
//! A row whose `server` is `https` runs against the TLS server the caller gives
//! ([`Config::https`]): its start address is `<origin><start>`, its `open` is `open --ca-file
//! <root> <address>`, and its connections are those that server counts. Without one such a row
//! has no result and the report writes it `skipped`. With one, after the last task and in the
//! same `surfd`, the check `https-without-root` runs ([`without_root`]) on the start address of
//! the first such row.

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use surf_test_process::ended_within;

use crate::agent::{Agent, steps};
use crate::format::{Server, read_status, read_tasks};
use crate::judge::{Verdict, judge};
use crate::process::{self, END_BOUND, KILL_BOUND, RuntimeDir};
use crate::report::{CheckResult, Lane, TaskResult, report};
use crate::server::serve;
use crate::{sites_dir, status_file, tasks_file};

/// The name of the check of a session opened without the root of the TLS server.
pub const WITHOUT_ROOT: &str = "https-without-root";

/// The site of the rows whose `server` is `https`, on a TLS server the caller started.
pub struct Https {
    /// `https://localhost:<port>`: the start address of such a row is this and its `start`.
    pub origin: String,
    /// The file `open --ca-file` names: the root the server's chain leads to.
    pub root: PathBuf,
    /// The connections the server has accepted so far.
    pub accepted: Box<dyn Fn() -> usize>,
}

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
    /// The TLS server of the `https` rows; `None` skips them and the check.
    pub https: Option<Https>,
}

/// What a lane run did and what `/proc` showed.
pub struct LaneRun {
    /// The judged results, in the order of `tasks.txt`.
    pub results: Vec<TaskResult>,
    /// The results of the checks: `https-without-root` when [`Config::https`] was given and
    /// `tasks.txt` has an `https` row, else none.
    pub checks: Vec<CheckResult>,
    /// What [`report`] returned.
    pub outcome: Result<(), String>,
    /// The `surfd` processes of the private directory after every call, from the first call
    /// after which one was there.
    pub surfd_samples: Vec<Vec<u32>>,
    /// Every process whose `comm` is `cargo` seen below this process.
    pub cargo_seen: BTreeSet<u32>,
    /// The processes of the private directory but its `surfd` still there [`END_BOUND`] after
    /// the last task, with their `/proc/<pid>/stat`: a session its task did not close, or a
    /// process a call left.
    pub after_tasks: Vec<(u32, String)>,
    /// The wait for each `surfd` after SIGKILL.
    pub ended: Vec<Result<Duration, String>>,
    /// The processes of the private directory still there [`END_BOUND`] after that, with their
    /// `/proc/<pid>/stat`.
    pub left: Vec<(u32, String)>,
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
    let http_accepted = || server.accepted();
    let root = config.https.as_ref().map(|https| {
        https
            .root
            .to_str()
            .unwrap_or_else(|| panic!("{} is not UTF-8", https.root.display()))
    });
    for task in &tasks {
        let steps = steps(&task.name).unwrap_or_else(|| panic!("no steps for {}", task.name));
        let (start, ca_file, accepted): (String, Option<&str>, &dyn Fn() -> usize) =
            match (task.server, &config.https) {
                (Server::Http, _) => (
                    server.address(&task.name, &task.start),
                    None,
                    &http_accepted,
                ),
                (Server::Https, Some(https)) => (
                    format!("{}{}", https.origin, task.start),
                    root,
                    &*https.accepted,
                ),
                // No TLS server: the row has no result, and the report writes it skipped.
                (Server::Https, None) => continue,
            };
        let before = accepted();
        let mut run = agent.run_with_ca_file(&steps, &start, ca_file, &mut sample);
        run.connections = accepted() - before;
        results.push(TaskResult {
            task: task.name.clone(),
            calls: run.calls,
            connections: run.connections,
            verdict: judge(task, &run),
        });
    }
    let mut checks = Vec::new();
    if let Some(https) = &config.https
        && let Some(task) = tasks.iter().find(|task| task.server == Server::Https)
    {
        let address = format!("{}{}", https.origin, task.start);
        let verdict = match agent.open_look_close(&address, &mut sample) {
            Ok((opened, looked)) => without_root(&address, &opened, &looked),
            Err(reason) => Verdict::Fail(reason),
        };
        checks.push(CheckResult {
            check: WITHOUT_ROOT.to_string(),
            verdict,
        });
    }

    let mut surfd: BTreeSet<u32> = surfd_samples.iter().flatten().copied().collect();
    surfd.extend(dir.processes_named("surfd"));
    let after_tasks = dir.left_within(&surfd, END_BOUND);
    for &pid in &surfd {
        process::signal_kill(pid);
    }
    let ended = surfd
        .iter()
        .map(|&pid| ended_within(pid, KILL_BOUND))
        .collect();
    // A session still open ends when the end of surfd closes its standard input.
    let left = dir.left_within(&BTreeSet::new(), END_BOUND);
    drop(dir);

    let outcome = report(&results, &checks, &status, &config.lane, &config.out);
    LaneRun {
        results,
        checks,
        outcome,
        surfd_samples,
        cargo_seen,
        after_tasks,
        ended,
        left,
        directory,
    }
}

/// The check `https-without-root` on the answers of a session opened on `address` without
/// `--ca-file`: `opened`, that of `open`, and `looked`, that of `look --as markdown`. It passes
/// when `open` answered `page failed CertificateNotTrusted "<address>"` (a failed load is no
/// refusal: `open` exits 0) and `look` answered `note text: no-document`. Otherwise it fails
/// with `open answered <page line>` (the line up to its first quoted field, the whole line when
/// that part is the expected one), `open answered no page line` or `look answered no note text:
/// no-document`.
pub fn without_root(address: &str, opened: &str, looked: &str) -> Verdict {
    let expected = format!("page failed CertificateNotTrusted \"{address}\"");
    let unquoted = |line: &str| {
        line.split(' ')
            .take_while(|word| !word.starts_with('"'))
            .collect::<Vec<_>>()
            .join(" ")
    };
    match opened.lines().find(|line| line.starts_with("page ")) {
        None => Verdict::Fail("open answered no page line".to_string()),
        Some(line) if line != expected => {
            let shown = match unquoted(line) {
                short if short == unquoted(&expected) => line.to_string(),
                short => short,
            };
            Verdict::Fail(format!("open answered {shown}"))
        }
        Some(_) if !looked.lines().any(|line| line == "note text: no-document") => {
            Verdict::Fail("look answered no note text: no-document".to_string())
        }
        Some(_) => Verdict::Pass,
    }
}
