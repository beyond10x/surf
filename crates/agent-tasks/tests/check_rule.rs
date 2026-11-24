//! Adversary cases of the check `https-without-root`: its line in the report on results written
//! here (a passing check counted in no number of the first line and written before the `raise:`
//! lines; a task and the check that both fail; a failed check under `SURF_AGENT_TASKS=measure`),
//! the rule of [`lane::without_root`] on answers written here, and the second session of the
//! check driven against a stand-in `surf-agent` (a shell script; no `surfd`, no server).

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use surf_agent_tasks::agent::Agent;
use surf_agent_tasks::format::{Listed, Status};
use surf_agent_tasks::lane::{self, WITHOUT_ROOT};
use surf_agent_tasks::report::{CheckResult, Lane};
use surf_agent_tasks::{TaskResult, Verdict, report};

use surf_answer::{Answer, Head, SessionState, render};

const ADDRESS: &str = "https://localhost:41234/notice.html";

fn measuring() -> bool {
    std::env::var_os("SURF_AGENT_TASKS").is_some_and(|value| value == "measure")
}

fn lane_gate() -> Lane {
    Lane {
        name: "gate".to_string(),
        binaries: PathBuf::from("/opt/surf/bin"),
    }
}

fn row(task: &str, listed: Listed) -> Status {
    Status {
        task: task.to_string(),
        listed,
    }
}

fn result(task: &str, verdict: Verdict) -> TaskResult {
    TaskResult {
        task: task.to_string(),
        calls: 3,
        connections: 1,
        verdict,
    }
}

fn check(verdict: Verdict) -> CheckResult {
    CheckResult {
        check: WITHOUT_ROOT.to_string(),
        verdict,
    }
}

/// Write the report to a file of its own under `CARGO_TARGET_TMPDIR`: what it returned and the
/// file's text.
fn written(
    name: &str,
    results: &[TaskResult],
    checks: &[CheckResult],
    status: &[Status],
) -> (Result<(), String>, String) {
    let out = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "agent-tasks-check-rule-{name}-{}.txt",
        std::process::id()
    ));
    let _ = fs::remove_file(&out);
    let outcome = report(results, checks, status, &lane_gate(), &out);
    let text = fs::read_to_string(&out).expect("the file is written");
    let _ = fs::remove_file(&out);
    (outcome, text)
}

#[test]
fn a_passing_check_is_counted_in_no_number_and_comes_before_the_raises() {
    let status = [
        row("alpha", Listed::Pass),
        row("beta", Listed::Pending(vec!["story:x".to_string()])),
    ];
    let results = [
        result("alpha", Verdict::Pass),
        result("beta", Verdict::Pass),
    ];
    let (outcome, text) = written("pass", &results, &[check(Verdict::Pass)], &status);
    let raise = if measuring() { "" } else { "raise: beta\n" };
    assert_eq!(
        text,
        format!(
            "agent-tasks: 2 tasks, 2 pass, 0 fail, 0 skipped, lane gate, binaries bin\n\
             task alpha pass 3 calls 1 connections\n\
             task beta pass 3 calls 1 connections\n\
             check https-without-root pass\n\
             {raise}"
        )
    );
    assert_eq!(outcome, Ok(()));
}

#[test]
fn a_task_and_the_check_that_both_fail_write_both_lines_and_the_task_error() {
    let status = [row("alpha", Listed::Pass), row("beta", Listed::Pass)];
    let results = [
        result("alpha", Verdict::Pass),
        result("beta", Verdict::Fail("exit 2 on look".to_string())),
    ];
    let checks = [check(Verdict::Fail("open answered page 200".to_string()))];
    let (outcome, text) = written("both", &results, &checks, &status);
    assert_eq!(
        text,
        "agent-tasks: 2 tasks, 1 pass, 1 fail, 0 skipped, lane gate, binaries bin\n\
         task alpha pass 3 calls 1 connections\n\
         task beta fail 3 calls 1 connections: exit 2 on look\n\
         check https-without-root fail: open answered page 200\n"
    );
    let expected = if measuring() {
        Ok(())
    } else {
        Err("agent-tasks: task beta is listed pass and failed: exit 2 on look".to_string())
    };
    assert_eq!(outcome, expected);
}

/// A failed check: its line in both lanes; the error under the gate, none under
/// `SURF_AGENT_TASKS=measure`. [`a_failed_check_under_measure_returns_no_error`] runs this
/// test again in a process of its own with that variable.
#[test]
fn the_outcome_of_a_failed_check_follows_the_lane() {
    let status = [row("alpha", Listed::Pass)];
    let results = [result("alpha", Verdict::Pass)];
    let checks = [check(Verdict::Fail("open answered page 200".to_string()))];
    let (outcome, text) = written("lane", &results, &checks, &status);
    assert_eq!(
        text,
        "agent-tasks: 1 tasks, 1 pass, 0 fail, 0 skipped, lane gate, binaries bin\n\
         task alpha pass 3 calls 1 connections\n\
         check https-without-root fail: open answered page 200\n"
    );
    let expected = if measuring() {
        Ok(())
    } else {
        Err("agent-tasks: check https-without-root failed: open answered page 200".to_string())
    };
    assert_eq!(outcome, expected);
}

#[test]
fn a_failed_check_under_measure_returns_no_error() {
    let name = "check_rule::the_outcome_of_a_failed_check_follows_the_lane";
    let output = Command::new(std::env::current_exe().unwrap())
        .args([name, "--exact", "--test-threads=1"])
        .env("SURF_AGENT_TASKS", "measure")
        .stdin(Stdio::null())
        .output()
        .expect("the test binary runs");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success() && stdout.contains("test result: ok. 1 passed"),
        "exit {:?}\nstdout: {stdout}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn without_root_holds_the_whole_page_line_and_the_note() {
    let expected = format!("session s\npage failed CertificateNotTrusted \"{ADDRESS}\"\n");
    let note = "session s\nnote text: no-document\n";
    let other = "https://localhost:41234/other.html";
    let cases: [(String, &str, Verdict); 6] = [
        (expected.clone(), note, Verdict::Pass),
        (
            format!("page failed CertificateExpired \"{ADDRESS}\"\n"),
            note,
            Verdict::Fail("open answered page failed CertificateExpired".to_string()),
        ),
        (
            format!("page failed CertificateNotTrusted \"{other}\"\n"),
            note,
            Verdict::Fail(format!(
                "open answered page failed CertificateNotTrusted \"{other}\""
            )),
        ),
        (
            format!("page 200 \"{ADDRESS}\"\n"),
            note,
            Verdict::Fail("open answered page 200".to_string()),
        ),
        (
            expected.clone(),
            "session s\ntext markdown 1\n",
            Verdict::Fail("look answered no note text: no-document".to_string()),
        ),
        (
            "session s\n".to_string(),
            note,
            Verdict::Fail("open answered no page line".to_string()),
        ),
    ];
    for (opened, looked, verdict) in cases {
        assert_eq!(
            lane::without_root(ADDRESS, &opened, looked),
            verdict,
            "opened {opened:?}, looked {looked:?}"
        );
    }
}

/// A directory with a stand-in `surf-agent`: it appends its first word to `calls`, answers
/// `open` with `open_body` and exit `open_exit`, `look` with exit `look_exit`, `close` with 0.
fn stand_in(name: &str, open_body: &str, open_exit: i32, look_exit: i32) -> (PathBuf, PathBuf) {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "agent-tasks-check-rule-{name}-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("runtime")).unwrap();
    let calls = dir.join("calls");
    fs::write(dir.join("open-answer"), open_body).unwrap();
    let script = format!(
        "#!/bin/sh\n\
         printf '%s\\n' \"$1\" >> '{calls}'\n\
         case $1 in\n\
         \x20 open) /bin/cat '{answer}'; exit {open_exit} ;;\n\
         \x20 look) exit {look_exit} ;;\n\
         esac\n\
         exit 0\n",
        calls = calls.display(),
        answer = dir.join("open-answer").display(),
    );
    fs::write(dir.join("surf-agent"), script).unwrap();
    fs::set_permissions(dir.join("surf-agent"), fs::Permissions::from_mode(0o755)).unwrap();
    (dir, calls)
}

#[test]
fn a_check_whose_session_cannot_be_opened_fails_with_the_reason() {
    let (dir, calls) = stand_in("no-open", "", 3, 0);
    let agent = Agent::new(dir.clone(), dir.join("runtime"));
    let got = agent.open_look_close(ADDRESS, &mut || {});
    let logged = fs::read_to_string(&calls).unwrap_or_default();
    let _ = fs::remove_dir_all(&dir);
    assert_eq!(got, Err("exit 3 on open".to_string()));
    assert_eq!(logged, "open\n");
}

#[test]
fn the_check_session_is_closed_after_a_failed_look() {
    let mut answer = Answer::new(Head {
        session: "4b0f6d2e-8c1a-4e5b-9f3d-2a7c6e1b0d94".to_string(),
        state: SessionState::Open,
    });
    answer.token = Some("ab".repeat(32));
    let (dir, calls) = stand_in("failed-look", &render(&answer).unwrap(), 0, 4);
    let agent = Agent::new(dir.clone(), dir.join("runtime"));
    let got = agent.open_look_close(ADDRESS, &mut || {});
    let logged = fs::read_to_string(&calls).unwrap_or_default();
    let _ = fs::remove_dir_all(&dir);
    assert_eq!(got, Err("exit 4 on look".to_string()));
    assert_eq!(logged, "open\nlook\nclose\n");
}
