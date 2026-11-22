//! The readers of `tasks.txt` and `status.txt`.
//!
//! `tasks.txt`: one block per task, blocks separated by one empty line, each line `<key>
//! <value>`, the keys in this order, each once: `task`, `server`, `start`, `target`,
//! `connections`, `commands`, `question`, `answer`, then `match`, optional. Anything else is
//! refused with its line number.
//!
//! `status.txt`: one line per task of `tasks.txt`, in the same order: `<task> pass`, or `<task>
//! pending <story id>` with one or more story ids separated by one space.

use std::fmt;

/// The server a task's fixture is served by.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Server {
    /// `http`: the fixture server of this crate.
    Http,
    /// `https`: the TLS test server (`story:agent-tasks-https`).
    Https,
}

impl Server {
    /// The word of `tasks.txt`.
    pub fn word(self) -> &'static str {
        match self {
            Server::Http => "http",
            Server::Https => "https",
        }
    }
}

/// How the judge looks for the answer: the block's `match` line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Match {
    /// `match text`, or no `match` line: the answer occurs in the page's text.
    Text,
    /// `match row <cell>`: a table row of the page's text holds both the cell and the answer.
    Row(String),
}

/// One block of `tasks.txt`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Task {
    /// `task`: the name, `[a-z]+`, unique.
    pub name: String,
    /// `server`.
    pub server: Server,
    /// `start`: the path the agent opens, on the task's site.
    pub start: String,
    /// `target`: the path of the page that holds the answer.
    pub target: String,
    /// `connections`: how many connections the fixture server accepts for the task.
    pub connections: usize,
    /// `commands`: the `surf-agent` command words the scripted agent issues, in order.
    pub commands: Vec<String>,
    /// `question`.
    pub question: String,
    /// `answer`: the expected answer.
    pub answer: String,
    /// `match`.
    pub matcher: Match,
    /// The number of the block's first line.
    pub line: usize,
}

/// Why `tasks.txt` was refused: the line, counted from 1, and what is wrong with it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FormatError {
    /// The line, counted from 1.
    pub line: usize,
    /// What is wrong.
    pub message: String,
}

impl fmt::Display for FormatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "tasks.txt line {}: {}", self.line, self.message)
    }
}

impl std::error::Error for FormatError {}

/// The state `status.txt` lists for a task.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Listed {
    /// `pass`: the task passes; a failure fails the run.
    Pass,
    /// `pending <story id> ...`: the stories that make it pass.
    Pending(Vec<String>),
}

/// One line of `status.txt`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Status {
    /// The task's name.
    pub task: String,
    /// Its listed state.
    pub listed: Listed,
}

/// The keys every block has, in their order; `match` may follow them.
const KEYS: [&str; 8] = [
    "task",
    "server",
    "start",
    "target",
    "connections",
    "commands",
    "question",
    "answer",
];

fn error(line: usize, message: impl Into<String>) -> FormatError {
    FormatError {
        line,
        message: message.into(),
    }
}

/// A name of a task or a command word: `[a-z]+`.
fn is_word(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|b| b.is_ascii_lowercase())
}

/// A story id: `story:` and `[a-z0-9-]+`.
fn is_story_id(text: &str) -> bool {
    text.strip_prefix("story:").is_some_and(|rest| {
        !rest.is_empty()
            && rest
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    })
}

/// Read `tasks.txt` to its blocks.
pub fn read_tasks(text: &str) -> Result<Vec<Task>, FormatError> {
    if text.is_empty() {
        return Err(error(1, "the file holds no block"));
    }
    let Some(body) = text.strip_suffix('\n') else {
        return Err(error(
            text.split('\n').count(),
            "the last line does not end in a line feed",
        ));
    };
    let lines: Vec<&str> = body.split('\n').collect();
    let mut tasks: Vec<Task> = Vec::new();
    let mut at = 0;
    loop {
        if lines[at].is_empty() {
            return Err(error(
                at + 1,
                "an empty line where a block starts: blocks are separated by one empty line",
            ));
        }
        let first = at;
        let mut values: Vec<&str> = Vec::with_capacity(KEYS.len());
        let mut matcher = Match::Text;
        let mut read = 0;
        while at < lines.len() && !lines[at].is_empty() {
            let number = at + 1;
            let (key, value) = lines[at]
                .split_once(' ')
                .filter(|(key, value)| !key.is_empty() && !value.is_empty())
                .ok_or_else(|| error(number, "expected `<key> <value>`"))?;
            if value.starts_with(' ') || value.ends_with(' ') {
                return Err(error(
                    number,
                    format!("the value of `{key}` has a space at an end"),
                ));
            }
            if read < KEYS.len() {
                if key != KEYS[read] {
                    return Err(error(
                        number,
                        format!("expected the key `{}`, found `{key}`", KEYS[read]),
                    ));
                }
                check_value(key, value, &tasks).map_err(|message| error(number, message))?;
                values.push(value);
            } else if read == KEYS.len() && key == "match" {
                matcher = read_match(value).map_err(|message| error(number, message))?;
            } else {
                return Err(error(
                    number,
                    format!("`{key}` after the last key of the block"),
                ));
            }
            read += 1;
            at += 1;
        }
        if read < KEYS.len() {
            return Err(error(
                at + 1,
                format!(
                    "the block of line {} ends before its `{}` line",
                    first + 1,
                    KEYS[read]
                ),
            ));
        }
        tasks.push(Task {
            name: values[0].to_string(),
            server: if values[1] == "http" {
                Server::Http
            } else {
                Server::Https
            },
            start: values[2].to_string(),
            target: values[3].to_string(),
            connections: values[4].parse().expect("checked to be a number"),
            commands: values[5].split(' ').map(str::to_string).collect(),
            question: values[6].to_string(),
            answer: values[7].to_string(),
            matcher,
            line: first + 1,
        });
        if at == lines.len() {
            return Ok(tasks);
        }
        at += 1;
        if at == lines.len() {
            return Err(error(at, "an empty line at the end of the file"));
        }
    }
}

/// Check the value of one of [`KEYS`].
fn check_value(key: &str, value: &str, earlier: &[Task]) -> Result<(), String> {
    let number = || {
        value.bytes().all(|b| b.is_ascii_digit())
            && !value.starts_with('0')
            && value.parse::<usize>().is_ok()
    };
    match key {
        "task" if !is_word(value) => Err(format!("the task name `{value}` is not of `[a-z]+`")),
        "task" if earlier.iter().any(|task| task.name == value) => {
            Err(format!("the task `{value}` is listed twice"))
        }
        "server" if value != "http" && value != "https" => Err(format!(
            "the server `{value}` is neither `http` nor `https`"
        )),
        "start" | "target" if !value.starts_with('/') || value.contains(' ') => Err(format!(
            "the {key} `{value}` is not a path that starts with `/`"
        )),
        "connections" if !number() => Err(format!(
            "the connections `{value}` are not a number of at least 1"
        )),
        "commands" if !value.split(' ').all(is_word) => {
            Err("the commands are not words of `[a-z]+` separated by one space".to_string())
        }
        _ => Ok(()),
    }
}

/// The value of a `match` line: `text`, or `row <cell>` with a cell of one or more characters
/// without a space or `|`.
fn read_match(value: &str) -> Result<Match, String> {
    if value == "text" {
        return Ok(Match::Text);
    }
    match value.strip_prefix("row ") {
        Some(cell) if !cell.is_empty() && !cell.contains([' ', '|']) => {
            Ok(Match::Row(cell.to_string()))
        }
        _ => Err(
            "expected `match text` or `match row <cell>`, the cell without a space or `|`"
                .to_string(),
        ),
    }
}

/// Read `status.txt` against the tasks of `tasks.txt`: one line per task, in their order.
pub fn read_status(text: &str, tasks: &[Task]) -> Result<Vec<Status>, String> {
    let mut rows: Vec<(usize, Status)> = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let number = index + 1;
        let bad = || {
            format!(
                "status.txt line {number}: expected `<task> pass` or `<task> pending <story id> \
                 ...`, found {line:?}"
            )
        };
        let mut words = line.split(' ');
        let task = words.next().filter(|word| is_word(word)).ok_or_else(bad)?;
        let listed = match words.next() {
            Some("pass") if words.next().is_none() => Listed::Pass,
            Some("pending") => {
                let ids: Vec<String> = words.map(str::to_string).collect();
                if ids.is_empty() || !ids.iter().all(|id| is_story_id(id)) {
                    return Err(bad());
                }
                Listed::Pending(ids)
            }
            _ => return Err(bad()),
        };
        rows.push((
            number,
            Status {
                task: task.to_string(),
                listed,
            },
        ));
    }
    for (number, row) in &rows {
        if !tasks.iter().any(|task| task.name == row.task) {
            return Err(format!(
                "status.txt line {number}: task `{}` is not a task of tasks.txt",
                row.task
            ));
        }
        if rows
            .iter()
            .filter(|(_, other)| other.task == row.task)
            .count()
            > 1
        {
            return Err(format!(
                "status.txt line {number}: task `{}` is listed twice",
                row.task
            ));
        }
    }
    for task in tasks {
        if !rows.iter().any(|(_, row)| row.task == task.name) {
            return Err(format!("status.txt: no line for task `{}`", task.name));
        }
    }
    for ((number, row), task) in rows.iter().zip(tasks) {
        if row.task != task.name {
            return Err(format!(
                "status.txt line {number}: task `{}` stands where tasks.txt has `{}`",
                row.task, task.name
            ));
        }
    }
    Ok(rows.into_iter().map(|(_, row)| row).collect())
}
