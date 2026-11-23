//! `tasks.txt` and `status.txt` read to their rows, and refused, naming the line or the task,
//! when they are not in their form; the `commands` line of each task against the scripted
//! agent's steps.

use std::fs;

use surf_agent_tasks::agent::steps;
use surf_agent_tasks::format::{Listed, Match, Server, read_status, read_tasks};
use surf_agent_tasks::{status_file, tasks_file};

const BLOCK: &str = "task paged
server http
start /paged.html
target /paged.html
connections 1
commands open look close
question Where does the ferry leave from?
answer Kestrel Bay
";

fn tasks_text() -> String {
    fs::read_to_string(tasks_file()).expect("tasks.txt")
}

fn status_text() -> String {
    fs::read_to_string(status_file()).expect("status.txt")
}

fn refused(text: &str) -> (usize, String) {
    match read_tasks(text) {
        Ok(tasks) => panic!("accepted: {tasks:?}"),
        Err(error) => (error.line, error.to_string()),
    }
}

#[test]
fn tasks_txt_is_read() {
    let tasks = read_tasks(&tasks_text()).expect("tasks.txt reads");
    assert!(!tasks.is_empty());
    let blocks = tasks_text().split("\n\n").count();
    assert_eq!(tasks.len(), blocks, "one task per block");
    let article = tasks.iter().find(|t| t.name == "article").expect("article");
    assert_eq!(article.server, Server::Http);
    assert_eq!(article.start, "/article.html");
    assert_eq!(article.target, "/article.html");
    assert_eq!(article.connections, 1);
    assert_eq!(article.commands, ["open", "look", "close"]);
    assert_eq!(article.question, "In which year was the bridge opened?");
    assert_eq!(article.answer, "1932");
    assert_eq!(article.matcher, Match::Text);
    let table = tasks.iter().find(|t| t.name == "table").expect("table");
    assert_eq!(table.matcher, Match::Row("Pears".to_string()));
    for task in &tasks {
        if task.name != "table" {
            assert_eq!(task.matcher, Match::Text, "{}", task.name);
        }
    }

    // Two blocks, the second starting at line 10.
    let two = format!("{BLOCK}\n{}", BLOCK.replace("task paged", "task second"));
    let read = read_tasks(&two).expect("two blocks");
    assert_eq!(read.len(), 2);
    assert_eq!(read[1].line, 10);
    assert_eq!(
        read_tasks(&format!("{BLOCK}match text\n")).unwrap()[0].matcher,
        Match::Text
    );

    // A block missing `answer`: the line where it is due is named.
    let no_answer = BLOCK.replace("answer Kestrel Bay\n", "");
    let (line, message) = refused(&no_answer);
    assert_eq!(line, 8, "{message}");
    assert!(message.contains("answer"), "{message}");
    let no_answer_then_match = BLOCK.replace("answer Kestrel Bay\n", "match text\n");
    let (line, message) = refused(&no_answer_then_match);
    assert_eq!(line, 8, "{message}");
    assert!(message.contains("answer"), "{message}");

    // `match row` without a cell, `match rows Pears`, a cell with a space or a `|`.
    for bad in [
        "match row",
        "match row ",
        "match rows Pears",
        "match row Pe ars",
        "match row Pe|ars",
        "match",
    ] {
        let (line, message) = refused(&format!("{BLOCK}{bad}\n"));
        assert_eq!(line, 9, "{bad:?}: {message}");
        assert!(message.starts_with("tasks.txt line 9: "), "{message}");
    }

    // Anything else, refused with its line number.
    let cases: &[(&str, usize)] = &[
        (&BLOCK.replace("server http", "server ftp"), 2),
        (&BLOCK.replace("task paged", "task Paged"), 1),
        (&BLOCK.replace("connections 1", "connections one"), 5),
        (
            &BLOCK.replace("commands open look close", "commands open  look"),
            6,
        ),
        (&BLOCK.replace("start /paged.html", "start paged.html"), 3),
        (&BLOCK.replace("target /paged.html\n", ""), 4),
        (&BLOCK.replace("question", "query"), 7),
        (&format!("{BLOCK}colour green\n"), 9),
        (&format!("{BLOCK}match text\nmatch text\n"), 10),
        (&format!("{BLOCK}\n\n{BLOCK}"), 10),
        (&format!("{BLOCK}\n"), 9),
        (&format!("\n{BLOCK}"), 1),
        (&format!("{BLOCK}\n{BLOCK}"), 10),
    ];
    for (text, expected) in cases {
        let (line, message) = refused(text);
        assert_eq!(line, *expected, "{message}\n{text}");
    }
    assert!(read_tasks("").is_err());
}

#[test]
fn status_lists_every_task() {
    let tasks = read_tasks(&tasks_text()).unwrap();
    let status = read_status(&status_text(), &tasks).expect("status.txt reads");
    assert_eq!(status.len(), tasks.len());
    for (row, task) in status.iter().zip(&tasks) {
        assert_eq!(row.task, task.name);
    }
    let search = status.iter().find(|s| s.task == "search").unwrap();
    assert_eq!(search.listed, Listed::Pass);

    let first = &tasks[0].name;
    let without: String = status_text()
        .lines()
        .filter(|line| line.split(' ').next() != Some(first.as_str()))
        .map(|line| format!("{line}\n"))
        .collect();
    let error = read_status(&without, &tasks).expect_err("a missing line");
    assert!(error.contains(&format!("`{first}`")), "{error}");

    let extra = format!("{}ferry pass\n", status_text());
    let error = read_status(&extra, &tasks).expect_err("an extra line");
    assert!(error.contains("`ferry`"), "{error}");

    let twice = format!("{}{first} pass\n", status_text());
    let error = read_status(&twice, &tasks).expect_err("a task twice");
    assert!(error.contains(&format!("`{first}`")), "{error}");

    let text = status_text();
    let mut lines: Vec<&str> = text.lines().collect();
    lines.swap(0, 1);
    let swapped: String = lines.iter().map(|line| format!("{line}\n")).collect();
    let error = read_status(&swapped, &tasks).expect_err("out of order");
    assert!(error.contains(&format!("`{}`", tasks[1].name)), "{error}");

    for bad in [
        format!("{first} passed"),
        format!("{first} pending"),
        format!("{first} pending page-text-look"),
        format!("{first} pending story:page-text-look  story:x"),
        format!("{first} pass story:x"),
    ] {
        let mut text = status_text();
        text = text
            .lines()
            .map(|line| {
                if line.split(' ').next() == Some(first.as_str()) {
                    format!("{bad}\n")
                } else {
                    format!("{line}\n")
                }
            })
            .collect();
        let error = read_status(&text, &tasks).expect_err(&bad);
        assert!(error.contains("line 1"), "{bad}: {error}");
    }
}

#[test]
fn commands_match_the_steps() {
    let tasks = read_tasks(&tasks_text()).unwrap();
    for task in &tasks {
        let steps = steps(&task.name).unwrap_or_else(|| panic!("no steps for {}", task.name));
        let words: Vec<&str> = steps.iter().map(|step| step.word()).collect();
        assert_eq!(words, task.commands, "{}", task.name);
        let named: Option<&[&str]> = match task.name.as_str() {
            "article" | "table" => Some(&["open", "look", "close"]),
            "link" => Some(&["open", "look", "follow", "look", "close"]),
            "search" => Some(&["open", "look", "fill", "press", "look", "close"]),
            _ => None,
        };
        if let Some(named) = named {
            assert_eq!(words, named, "{}", task.name);
        }
    }
}
