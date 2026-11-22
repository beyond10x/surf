//! The scripted lane: every task of `tasks.txt`, one after the other, with the `surf-agent`,
//! `surfd` and `surf-session` of `SURF_BIN_DIR`, against one fixture server (a thread of this
//! test) and one `surfd` in one private `XDG_RUNTIME_DIR`. It writes the root
//! `.out/agent-tasks.txt` and holds each task to its row of `status.txt`. No model call; every
//! address is on 127.0.0.1; nothing is built.

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

use surf_agent_tasks::agent::{Agent, steps};
use surf_agent_tasks::format::{read_status, read_tasks};
use surf_agent_tasks::process::{self, KILL_BOUND, RuntimeDir};
use surf_agent_tasks::server::serve;
use surf_agent_tasks::{
    TaskResult, judge, report, root_report_file, sites_dir, status_file, tasks_file,
};
use surf_test_process::ended_within;

/// `SURF_BIN_DIR`, which the step `test` sets to the programs `task build` made.
fn bin_dir() -> PathBuf {
    let dir = std::env::var_os("SURF_BIN_DIR").map(PathBuf::from).expect(
        "SURF_BIN_DIR names the directory of surf-agent, surfd and surf-session (task test sets \
         it to target/debug after task build); this test builds nothing",
    );
    for program in ["surf-agent", "surfd", "surf-session"] {
        assert!(
            dir.join(program).is_file(),
            "{} is not there",
            dir.join(program).display()
        );
    }
    dir
}

#[test]
fn every_task() {
    let bin = bin_dir();
    let tasks = read_tasks(&fs::read_to_string(tasks_file()).unwrap()).unwrap();
    let status = read_status(&fs::read_to_string(status_file()).unwrap(), &tasks).unwrap();
    let dir = RuntimeDir::new(env!("CARGO_TARGET_TMPDIR").as_ref());
    let server = serve(sites_dir()).expect("the fixture server listens");
    let agent = Agent::new(bin.clone(), dir.runtime());
    let own = std::process::id();

    // What /proc shows after every call: the surfd processes of the directory, and any cargo
    // below this test.
    let mut surfd_samples: Vec<Vec<u32>> = Vec::new();
    let mut cargo_seen: BTreeSet<u32> = BTreeSet::new();
    let mut results = Vec::new();
    for task in &tasks {
        let steps = steps(&task.name).unwrap_or_else(|| panic!("no steps for {}", task.name));
        let before = server.accepted();
        let mut sample = || {
            let surfd = dir.processes_named("surfd");
            if !surfd.is_empty() || !surfd_samples.is_empty() {
                surfd_samples.push(surfd);
            }
            for pid in process::descendants(own) {
                if process::comm(pid).as_deref() == Some("cargo") {
                    cargo_seen.insert(pid);
                }
            }
        };
        let mut run = agent.run(
            &steps,
            &server.address(&task.name, &task.start),
            &mut sample,
        );
        run.connections = server.accepted() - before;
        let verdict = judge(task, &run);
        results.push(TaskResult {
            task: task.name.clone(),
            calls: run.calls,
            connections: run.connections,
            verdict,
        });
    }

    // Every session is closed by its task; the one surfd is ended here: it would end on its
    // own only after its idle exit of 600 s.
    let surfd: BTreeSet<u32> = surfd_samples.iter().flatten().copied().collect();
    for &pid in &surfd {
        process::signal_kill(pid);
    }
    let ended: Vec<Result<_, String>> = surfd
        .iter()
        .map(|&pid| ended_within(pid, KILL_BOUND))
        .collect();
    let left: Vec<(u32, Option<String>)> = dir
        .processes()
        .into_iter()
        .map(|pid| (pid, process::comm(pid)))
        .collect();

    let outcome = report(&results, &status, &root_report_file());
    let written = fs::read_to_string(root_report_file()).expect("the report is written");
    print!("{written}");

    assert_eq!(surfd.len(), 1, "surfd processes seen: {surfd:?}");
    assert!(
        surfd_samples.iter().all(|sample| sample.len() == 1),
        "a sample without exactly one surfd: {surfd_samples:?}"
    );
    assert!(
        cargo_seen.is_empty(),
        "cargo below the test: {cargo_seen:?}"
    );
    for result in ended {
        result.expect("surfd ended after SIGKILL");
    }
    assert!(left.is_empty(), "processes of the run left: {left:?}");
    assert_eq!(
        written.lines().count(),
        1 + tasks.len() + written.matches("\nraise: ").count(),
        "{written}"
    );
    if let Err(error) = outcome {
        panic!("{error}");
    }
    drop(dir);
}
