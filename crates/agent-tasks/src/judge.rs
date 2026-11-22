//! The judge: whether a task passed, from what the scripted agent did and read.

use crate::format::{Match, Task};

/// The first step of a task that did not go through.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StepFailure {
    /// A `surf-agent` call exited other than 0.
    Exit {
        /// Its exit status.
        status: i32,
        /// Its command word.
        command: String,
    },
    /// No line of the `now` block of the last `look` has the label.
    NoNowLine {
        /// The label looked for.
        label: String,
    },
}

/// What the scripted agent did for one task.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Run {
    /// Every `surf-agent` call of the task, `close` included.
    pub calls: usize,
    /// The connections the fixture server accepted while the task ran.
    pub connections: usize,
    /// The first step that did not go through.
    pub failure: Option<StepFailure>,
    /// The answers of every `look --as markdown` call, in order.
    pub answers: Vec<String>,
}

/// The judged result of a task.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// It passed.
    Pass,
    /// It failed, with the reason.
    Fail(String),
}

impl StepFailure {
    /// The reason the report writes.
    pub fn reason(&self) -> String {
        match self {
            StepFailure::Exit { status, command } => format!("exit {status} on {command}"),
            StepFailure::NoNowLine { label } => format!("no now line labelled \"{label}\""),
        }
    }
}

/// The text of the target page: the body lines of every text block of `answers`, each
/// without its leading `> ` or `>`, read through [`surf_answer::parse`]. An answer that does
/// not parse adds no line.
pub fn page_text(answers: &[String]) -> Vec<String> {
    answers
        .iter()
        .filter_map(|answer| surf_answer::parse(answer).ok()?.text)
        .flat_map(|text| text.lines)
        .collect()
}

/// Judge one task: it passes when every step went through, the answer is found by the task's
/// kind of match in the text of the target page, and the fixture server accepted exactly the
/// task's connections. Otherwise it fails with the first of these reasons.
pub fn judge(task: &Task, run: &Run) -> Verdict {
    if let Some(failure) = &run.failure {
        return Verdict::Fail(failure.reason());
    }
    let lines = page_text(&run.answers);
    let answer = task.answer.as_str();
    match &task.matcher {
        Match::Text => {
            if !lines.join("\n").contains(answer) {
                return Verdict::Fail(format!("answer \"{answer}\" not in the text"));
            }
        }
        Match::Row(cell) => {
            let in_a_row = lines.iter().any(|line| {
                line.starts_with('|') && line.contains(cell.as_str()) && line.contains(answer)
            });
            if !in_a_row {
                return Verdict::Fail(format!(
                    "answer \"{answer}\" in no table row with \"{cell}\""
                ));
            }
        }
    }
    if run.connections != task.connections {
        return Verdict::Fail(format!(
            "{} connections, expected {}",
            run.connections, task.connections
        ));
    }
    Verdict::Pass
}
