//! A check of the report on results written here, into a file under `CARGO_TARGET_TMPDIR`: its
//! line after the task lines and before the `raise:` lines, counted in no number of the first
//! line, and a failed check failing the run after the file is written. No process is started.

use std::fs;
use std::path::PathBuf;

use surf_agent_tasks::format::{read_status, read_tasks};
use surf_agent_tasks::report::{CheckResult, Lane};
use surf_agent_tasks::{TaskResult, Verdict, report, status_file, tasks_file};

#[test]
fn a_failed_check_is_reported() {
    let tasks = read_tasks(&fs::read_to_string(tasks_file()).unwrap()).unwrap();
    let status = read_status(&fs::read_to_string(status_file()).unwrap(), &tasks).unwrap();
    let results: Vec<TaskResult> = tasks
        .iter()
        .map(|task| TaskResult {
            task: task.name.clone(),
            calls: 3,
            connections: 1,
            verdict: Verdict::Pass,
        })
        .collect();
    let checks = [CheckResult {
        check: "https-without-root".to_string(),
        verdict: Verdict::Fail("open answered page 200".to_string()),
    }];
    let lane = Lane {
        name: "gate".to_string(),
        binaries: PathBuf::from("/opt/surf/bin"),
    };
    let out = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("agent-tasks-check-{}.txt", std::process::id()));
    let _ = fs::remove_file(&out);

    let outcome = report(&results, &checks, &status, &lane, &out);

    let written = fs::read_to_string(&out).expect("the file is written");
    let _ = fs::remove_file(&out);
    let mut expected = format!(
        "agent-tasks: {0} tasks, {0} pass, 0 fail, 0 skipped, lane gate, binaries bin\n",
        tasks.len()
    );
    for task in &tasks {
        expected.push_str(&format!("task {} pass 3 calls 1 connections\n", task.name));
    }
    expected.push_str("check https-without-root fail: open answered page 200\n");
    assert_eq!(written, expected);
    assert_eq!(
        outcome,
        Err("agent-tasks: check https-without-root failed: open answered page 200".to_string())
    );
}
