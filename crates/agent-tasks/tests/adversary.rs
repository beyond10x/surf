//! Adversary cases: the judge of `match row` on rows that hold the answer only as a substring
//! or that hold the whole table, and the model lane's driver on a model command that does not
//! exist and on argument words with a line feed.

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use surf_agent_tasks::format::{Task, read_tasks};
use surf_agent_tasks::{Run, Verdict, judge, tasks_file};

use surf_answer::{Answer, Form, Head, SessionState, render, text_page};

use crate::model_driver::{ONE_TASK, assert_one_task};

fn task(name: &str) -> Task {
    read_tasks(&fs::read_to_string(tasks_file()).unwrap())
        .unwrap()
        .into_iter()
        .find(|task| task.name == name)
        .unwrap()
}

fn markdown(lines: &[&str]) -> String {
    let mut answer = Answer::new(Head {
        session: "4b0f6d2e-8c1a-4e5b-9f3d-2a7c6e1b0d94".to_string(),
        state: SessionState::Open,
    });
    answer.text = Some(text_page(
        Form::Markdown,
        lines.iter().map(|line| line.to_string()),
        1,
    ));
    render(&answer).unwrap()
}

fn one_answer(lines: &[&str]) -> Run {
    Run {
        calls: 3,
        connections: 1,
        failure: None,
        answers: vec![markdown(lines)],
    }
}

#[test]
fn a_pears_row_whose_price_only_contains_the_answer_fails() {
    // Pears cost 12.40; 2.40 is no cell of the page.
    let rows = [
        "| Fruit | Price |",
        "| --- | --- |",
        "| Apples | 1.80 |",
        "| Pears | 12.40 |",
        "| Plums | 3.10 |",
    ];
    assert_ne!(judge(&task("table"), &one_answer(&rows)), Verdict::Pass);
}

#[test]
fn a_table_flattened_into_one_row_fails() {
    // The table of prices.html flattened into one cell of one row, as a layout table holds it.
    let rows = [
        "| Prices Fruit Price Apples 1.80 Pears 2.40 Plums 3.10 Cherries 6.50 Grapes 2.90 Lemons 0.60 |",
        "| --- |",
    ];
    assert_ne!(judge(&task("table"), &one_answer(&rows)), Verdict::Pass);
}

/// Runs the driver with `model_command`, over `tasks` (`AGENT_TASKS_FILE`) when given: its exit
/// status and the out file.
fn run_driver(model_command: &str, name: &str, tasks: Option<&str>) -> (Option<i32>, String) {
    let bin = std::env::var_os("SURF_BIN_DIR").map(PathBuf::from).unwrap();
    let manifest = env!("CARGO_MANIFEST_DIR");
    let out = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("adversary-{name}-{}.txt", std::process::id()));
    let _ = fs::remove_file(&out);
    let mut command = Command::new("sh");
    command
        .arg(format!("{manifest}/../../taskfiles/agent-tasks-model.sh"))
        .arg(&bin)
        .arg(&out)
        .env("AGENT_TASKS_SERVE", bin.join("agent-tasks-serve"))
        .env("AGENT_TASKS_MODEL_COMMAND", model_command)
        .env("AGENT_TASKS_MODEL", "stub")
        .current_dir(manifest)
        .stdin(Stdio::null());
    if let Some(tasks) = tasks {
        command.env("AGENT_TASKS_FILE", tasks);
    }
    let output = command.output().unwrap();
    let written = fs::read_to_string(&out).unwrap_or_default();
    let _ = fs::remove_file(&out);
    (output.status.code(), written)
}

#[test]
fn a_model_command_that_does_not_exist_fails_the_lane() {
    let (code, written) = run_driver("/nonexistent/adversary-model", "missing", None);
    assert_ne!(
        code,
        Some(0),
        "the lane exited 0 and wrote a count although no model ran:\n{written}"
    );
}

#[test]
fn a_call_with_a_line_feed_in_a_word_is_one_call() {
    let stub = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/adversary-newline-model.sh"
    );
    let (code, written) = run_driver(stub, "newline", Some(ONE_TASK));
    assert_eq!(code, Some(0), "{written}");
    assert_one_task(&written);
    let article = written
        .lines()
        .find(|line| line.starts_with("task article "))
        .unwrap_or("");
    assert_eq!(article, "task article fail 1 calls", "{written}");
}
