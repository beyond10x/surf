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
    /// A `surf-agent` call ran past its bound and was killed.
    Timeout {
        /// The bound.
        bound: std::time::Duration,
        /// Its command word.
        command: String,
    },
    /// A `surf-agent` call wrote more than the cap of its output and was killed.
    TooLarge {
        /// The cap, in bytes.
        cap: usize,
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
    /// The answer of `open`, when it exited 0.
    pub opened: Option<String>,
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
            StepFailure::Timeout { bound, command } => {
                format!("no answer within {} s on {command}", bound.as_secs_f64())
            }
            StepFailure::TooLarge { cap, command } => {
                format!("output over {cap} bytes on {command}")
            }
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

/// The cells of a table row of page-text-tables (`| Apple | 0.50 | 120 |`): a line that starts
/// and ends with `|`, split at every `|` that no `\` escapes, each cell without the spaces at
/// its ends. `None` for any other line.
pub fn row_cells(line: &str) -> Option<Vec<&str>> {
    let inner = line.strip_prefix('|')?.strip_suffix('|')?;
    // The last pipe is escaped when an odd number of backslashes stands directly before it.
    let backslashes = inner
        .bytes()
        .rev()
        .take_while(|&byte| byte == b'\\')
        .count();
    if backslashes % 2 == 1 {
        return None;
    }
    let mut cells = Vec::new();
    let mut start = 0;
    let mut escaped = false;
    for (at, character) in inner.char_indices() {
        match character {
            _ if escaped => escaped = false,
            '\\' => escaped = true,
            '|' => {
                cells.push(inner[start..at].trim_matches(' '));
                start = at + 1;
            }
            _ => {}
        }
    }
    cells.push(inner[start..].trim_matches(' '));
    Some(cells)
}

/// Whether `cells` is the delimiter row of a table: every cell dashes, with an optional `:` at
/// either end.
fn delimiter(cells: &[&str]) -> bool {
    cells.iter().all(|cell| {
        let dashes = cell.strip_prefix(':').unwrap_or(cell);
        let dashes = dashes.strip_suffix(':').unwrap_or(dashes);
        !dashes.is_empty() && dashes.bytes().all(|byte| byte == b'-')
    })
}

/// The body rows of the tables of `lines`, in the form the page output writes a table: a
/// header row, the delimiter row directly under it with as many cells, then the rows that
/// follow, each with the header's number of cells. A row with another number of cells, or any
/// other line, ends the table. A line with pipes outside such a table is no row.
pub fn table_rows(lines: &[String]) -> Vec<Vec<&str>> {
    let cells: Vec<Option<Vec<&str>>> = lines.iter().map(|line| row_cells(line)).collect();
    let mut rows = Vec::new();
    let mut at = 0;
    while at + 1 < cells.len() {
        let (Some(header), Some(under)) = (&cells[at], &cells[at + 1]) else {
            at += 1;
            continue;
        };
        if under.len() != header.len() || !delimiter(under) {
            at += 1;
            continue;
        }
        at += 2;
        while let Some(Some(row)) = cells.get(at) {
            if row.len() != header.len() {
                break;
            }
            rows.push(row.clone());
            at += 1;
        }
    }
    rows
}

/// Judge one task: it passes when every step went through, the answer is found by the task's
/// kind of match in the text of the target page, and the fixture server accepted exactly the
/// task's connections. Otherwise it fails with the first of these reasons.
///
/// `match text`: the answer occurs in the body lines joined with line feeds. `match row
/// <cell>`: one body row of a table ([`table_rows`]) has a cell that is exactly `<cell>` and
/// another that is exactly the answer, so `| Pears | 12.40 |`, a table flattened into one cell
/// and a table flattened into one row of cells do not pass.
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
            let in_a_row = table_rows(&lines).iter().any(|cells| {
                cells.iter().enumerate().any(|(named, &value)| {
                    value == cell.as_str()
                        && cells
                            .iter()
                            .enumerate()
                            .any(|(other, &value)| other != named && value == answer)
                })
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
