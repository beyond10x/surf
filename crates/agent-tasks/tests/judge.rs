//! The judge on answers written or built here, no process: a changed answer, a page the agent
//! did not read to its end, and a table flattened to one paragraph each fail.

use std::fs;

use surf_agent_tasks::format::{Task, read_tasks};
use surf_agent_tasks::{Run, StepFailure, Verdict, judge, tasks_file};

use surf_answer::{Answer, Form, Head, SessionState, render, text_page};

fn task(name: &str) -> Task {
    let text = fs::read_to_string(tasks_file()).unwrap();
    read_tasks(&text)
        .unwrap()
        .into_iter()
        .find(|task| task.name == name)
        .unwrap_or_else(|| panic!("no task {name}"))
}

/// A `look --as markdown` answer whose text block is `lines` from line `from`.
fn answer(lines: &[String], from: u64) -> String {
    let mut answer = Answer::new(Head {
        session: "4b0f6d2e-8c1a-4e5b-9f3d-2a7c6e1b0d94".to_string(),
        state: SessionState::Open,
    });
    answer.text = Some(text_page(Form::Markdown, lines.iter().cloned(), from));
    render(&answer).expect("the answer renders")
}

fn run(answers: Vec<String>, connections: usize) -> Run {
    Run {
        calls: 2 + answers.len(),
        connections,
        failure: None,
        answers,
        opened: None,
    }
}

#[test]
fn a_changed_answer_fails() {
    let article_answer = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/answers/article.txt"
    ))
    .unwrap();
    surf_answer::parse(&article_answer).expect("the answer file is an answer");
    let mut article = task("article");
    assert_eq!(
        judge(&article, &run(vec![article_answer.clone()], 1)),
        Verdict::Pass
    );
    article.answer = "1933".to_string();
    assert_eq!(
        judge(&article, &run(vec![article_answer.clone()], 1)),
        Verdict::Fail("answer \"1933\" not in the text".to_string())
    );

    // The first failure wins: a step, then the answer, then the connections.
    let article = task("article");
    assert_eq!(
        judge(&article, &run(vec![article_answer.clone()], 2)),
        Verdict::Fail("2 connections, expected 1".to_string())
    );
    let mut failed = run(vec![], 1);
    failed.failure = Some(StepFailure::Exit {
        status: 2,
        command: "look".to_string(),
    });
    assert_eq!(
        judge(&article, &failed),
        Verdict::Fail("exit 2 on look".to_string())
    );
    failed.failure = Some(StepFailure::NoNowLine {
        label: "Opening hours".to_string(),
    });
    assert_eq!(
        judge(&article, &failed),
        Verdict::Fail("no now line labelled \"Opening hours\"".to_string())
    );

    // Paged: 260 lines, the answer on line 230, past the first page.
    let lines: Vec<String> = (1..=260)
        .map(|k| {
            if k == 230 {
                "The ferry leaves from Kestrel Bay.".to_string()
            } else {
                format!("line {k}")
            }
        })
        .collect();
    let first = answer(&lines, 1);
    let second = answer(&lines, 201);
    assert!(
        first.ends_with("more 60 lines: look --as markdown --from-line 201\n"),
        "{first}"
    );
    assert!(first.contains("\n> line 200\n"));
    assert!(!first.contains("\n> line 201\n"));
    assert!(!second.contains("\nmore "), "{second}");
    assert!(second.contains("\n> line 260\n"));
    let paged = &read_tasks(
        "task paged
server http
start /paged.html
target /paged.html
connections 1
commands open look close
question Where does the ferry leave from?
answer Kestrel Bay
",
    )
    .unwrap()[0];
    assert_eq!(
        judge(paged, &run(vec![first.clone()], 1)),
        Verdict::Fail("answer \"Kestrel Bay\" not in the text".to_string())
    );
    assert_eq!(judge(paged, &run(vec![first, second], 1)), Verdict::Pass);
}

#[test]
fn a_flattened_table_fails() {
    let table = task("table");
    let head = [
        "# Fruit prices".to_string(),
        String::new(),
        "Prices of the market stall this week, in euros per kilo.".to_string(),
        String::new(),
    ];
    let flat = [
        "Prices Fruit Price Apples 1.80 Pears 2.40 Plums 3.10 Cherries 6.50 Grapes 2.90 Lemons 0.60"
            .to_string(),
    ];
    let rows = |pears: &str, plums: &str| -> Vec<String> {
        vec![
            "| Fruit | Price |".to_string(),
            "| --- | --- |".to_string(),
            "| Apples | 1.80 |".to_string(),
            format!("| Pears | {pears} |"),
            format!("| Plums | {plums} |"),
            "| Cherries | 6.50 |".to_string(),
            "| Grapes | 2.90 |".to_string(),
            "| Lemons | 0.60 |".to_string(),
        ]
    };
    let reason = Verdict::Fail("answer \"2.40\" in no table row with \"Pears\"".to_string());

    let flattened: Vec<String> = head.iter().chain(&flat).cloned().collect();
    assert_eq!(judge(&table, &run(vec![answer(&flattened, 1)], 1)), reason);

    let as_rows: Vec<String> = head.iter().cloned().chain(rows("2.40", "3.10")).collect();
    let rendered = answer(&as_rows, 1);
    assert!(rendered.contains("\n> | Pears | 2.40 |\n"), "{rendered}");
    assert_eq!(judge(&table, &run(vec![rendered], 1)), Verdict::Pass);

    let other_row: Vec<String> = head.iter().cloned().chain(rows("3.10", "2.40")).collect();
    assert_eq!(judge(&table, &run(vec![answer(&other_row, 1)], 1)), reason);
}
