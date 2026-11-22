//! The rule of the report on judged results written here, each into a file under
//! `CARGO_TARGET_TMPDIR`: a task listed `pass` that fails fails the run after the file is
//! written, a task listed `pending` that passes is raised. The root `.out/agent-tasks.txt` is
//! left alone.

use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;

use surf_agent_tasks::format::{read_status, read_tasks};
use surf_agent_tasks::report::Lane;
use surf_agent_tasks::{TaskResult, Verdict, report, root_report_file, status_file, tasks_file};

/// The judged results of case 1 of the story on base A.
fn base_a() -> Vec<TaskResult> {
    let fail = |task: &str, reason: &str| TaskResult {
        task: task.to_string(),
        calls: 3,
        connections: 1,
        verdict: Verdict::Fail(reason.to_string()),
    };
    vec![
        fail("article", "exit 2 on look"),
        fail("table", "exit 2 on look"),
        fail("link", "no now line labelled \"Opening hours\""),
        fail("search", "no now line labelled \"Search\""),
    ]
}

/// The lane the reports of these tests name.
fn lane() -> Lane {
    Lane {
        name: "gate".to_string(),
        binaries: PathBuf::from("/opt/surf/bin"),
    }
}

fn out_file(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("agent-tasks-{name}-{}.txt", std::process::id()))
}

fn modified(path: &PathBuf) -> Option<SystemTime> {
    fs::metadata(path).and_then(|m| m.modified()).ok()
}

fn status_with(article: &str) -> String {
    fs::read_to_string(status_file())
        .unwrap()
        .lines()
        .map(|line| {
            if line.starts_with("article ") {
                format!("{article}\n")
            } else {
                format!("{line}\n")
            }
        })
        .collect()
}

#[test]
fn a_listed_pass_that_fails_fails() {
    let tasks = read_tasks(&fs::read_to_string(tasks_file()).unwrap()).unwrap();
    let status = read_status(&status_with("article pass"), &tasks).unwrap();
    let out = out_file("listed-pass");
    let _ = fs::remove_file(&out);
    let root = root_report_file();
    let before = modified(&root);

    let outcome = report(&base_a(), &status, &lane(), &out);

    assert_eq!(
        outcome,
        Err("agent-tasks: task article is listed pass and failed: exit 2 on look".to_string())
    );
    let written = fs::read_to_string(&out).expect("the file is written");
    assert_eq!(
        written,
        "agent-tasks: 4 tasks, 0 pass, 4 fail, 0 skipped, lane gate, binaries /opt/surf/bin\n\
         task article fail 3 calls 1 connections: exit 2 on look\n\
         task table fail 3 calls 1 connections: exit 2 on look\n\
         task link fail 3 calls 1 connections: no now line labelled \"Opening hours\"\n\
         task search fail 3 calls 1 connections: no now line labelled \"Search\"\n"
    );
    assert_eq!(modified(&root), before, "the root file is not touched");
    let _ = fs::remove_file(&out);
}

#[test]
fn a_pending_task_that_passes_is_raised() {
    let tasks = read_tasks(&fs::read_to_string(tasks_file()).unwrap()).unwrap();
    let status = read_status(&status_with("article pending story:page-text-look"), &tasks).unwrap();
    let mut results = base_a();
    results[0].verdict = Verdict::Pass;
    let out = out_file("pending-pass");
    let _ = fs::remove_file(&out);
    let root = root_report_file();
    let before = modified(&root);

    let outcome = report(&results, &status, &lane(), &out);

    assert_eq!(outcome, Ok(()));
    let written = fs::read_to_string(&out).expect("the file is written");
    assert!(
        written.starts_with(
            "agent-tasks: 4 tasks, 1 pass, 3 fail, 0 skipped, lane gate, binaries /opt/surf/bin\n"
        ),
        "{written}"
    );
    assert!(
        written.contains("\ntask article pass 3 calls 1 connections\n"),
        "{written}"
    );
    assert!(written.ends_with("\nraise: article\n"), "{written}");
    assert_eq!(written.matches("raise:").count(), 1, "{written}");
    assert_eq!(modified(&root), before, "the root file is not touched");
    let _ = fs::remove_file(&out);
}
