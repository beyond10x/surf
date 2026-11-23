//! What `story:agent-change-and-log` adds to the format: the `changed` block in its three forms,
//! each against its golden file and read back to its typed value, and the lines the renderer and
//! the parser refuse.

use surf_answer::{
    Answer, CHANGED_TEXT_CHARS_MAX, CHANGED_TEXTS_MAX, Change, Changed, ControlChange, Cut, Did,
    Head, Page, ParseErrorKind, Path, PathEntry, RenderError, SessionState, VALUE_CHARS_MAX, Value,
    parse, render,
};

fn golden(name: &str) -> String {
    let path = format!("{}/tests/golden/{name}.txt", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{path}: {error}"))
}

fn entry(sequence: u64, command: &str, handle: Option<&str>, argument: Option<&str>) -> PathEntry {
    PathEntry {
        sequence,
        command: command.to_string(),
        handle: handle.map(str::to_string),
        argument: argument.map(str::to_string),
        argument_cut: false,
    }
}

fn answer(did: &str, handle: &str, address: &str, title: &str, path: Vec<PathEntry>) -> Answer {
    let mut answer = Answer::new(Head {
        session: "7f3a".to_string(),
        state: SessionState::Open,
    });
    answer.did = Some(Did::Action {
        command: did.to_string(),
        argument: Some(handle.to_string()),
    });
    answer.page = Some(Page::Loaded {
        status: 200,
        media_type: "text/html".to_string(),
        address: address.to_string(),
        title: Some(title.to_string()),
    });
    answer.path = Some(Path {
        total: path.len() as u64,
        entries: path,
    });
    answer
}

fn text(value: &str) -> Value {
    Value::Text(Cut::of(value, VALUE_CHARS_MAX))
}

fn label(value: &str) -> Cut {
    Cut::of(value, VALUE_CHARS_MAX)
}

fn controls() -> Answer {
    let mut typed = answer(
        "pick",
        "f7",
        "http://example.org/form",
        "Form",
        vec![
            entry(1, "open", None, None),
            entry(2, "go", None, Some("http://example.org/form")),
            entry(3, "pick", Some("f7"), None),
        ],
    );
    let change = |handle: &str, change| ControlChange {
        handle: handle.to_string(),
        change,
    };
    typed.changed = Some(Changed::Controls(vec![
        change(
            "f1",
            Change::Value {
                before: text(""),
                after: text("ada"),
            },
        ),
        change(
            "f2",
            Change::Value {
                before: Value::Empty,
                after: Value::Set,
            },
        ),
        change(
            "f5",
            Change::Selected {
                before: label("Red"),
                after: label("Green"),
            },
        ),
        change(
            "f6",
            Change::Checked {
                before: true,
                after: false,
            },
        ),
        change(
            "f7",
            Change::Checked {
                before: false,
                after: true,
            },
        ),
    ]));
    typed
}

fn page() -> Answer {
    let mut typed = answer(
        "press",
        "b1",
        "http://example.org/session",
        "Welcome",
        vec![
            entry(1, "open", None, None),
            entry(2, "press", Some("b1"), None),
        ],
    );
    let mut texts: Vec<Cut> = (1..=9)
        .map(|n| Cut::of(&format!("Text {n}"), CHANGED_TEXT_CHARS_MAX))
        .collect();
    texts.push(Cut::of(&"0123456789".repeat(20), CHANGED_TEXT_CHARS_MAX));
    typed.changed = Some(Changed::Page {
        from: Some("http://example.org/login".to_string()),
        to: Some("http://example.org/session".to_string()),
        texts,
        more: 5,
    });
    typed
}

fn none() -> Answer {
    let mut typed = Answer::new(Head {
        session: "7f3a".to_string(),
        state: SessionState::Open,
    });
    typed.token =
        Some("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string());
    typed.did = Some(Did::Action {
        command: "open".to_string(),
        argument: None,
    });
    typed.page = Some(Page::None);
    typed.path = Some(Path {
        total: 1,
        entries: vec![entry(1, "open", None, None)],
    });
    typed.changed = Some(Changed::Controls(Vec::new()));
    typed
}

#[test]
fn the_three_forms_render_their_golden_text_and_read_back() {
    for (name, typed) in [
        ("changed-controls", controls()),
        ("changed-page", page()),
        ("changed-none", none()),
    ] {
        let text = render(&typed).unwrap_or_else(|error| panic!("{name}: {error}"));
        assert_eq!(text, golden(name), "{name}");
        assert_eq!(parse(&text), Ok(typed), "{name}");
    }
}

#[test]
fn a_page_after_page_none_reads_back() {
    let text = "surf 1 session 7f3a open\npage 200 \"text/html\" \"http://example.org/\"\n\
                changed page none -> \"http://example.org/\"\n  + text \"Hello\"\nnow 0 of 0\n";
    let typed = parse(text).unwrap_or_else(|error| panic!("{error}: {text}"));
    assert_eq!(
        typed.changed,
        Some(Changed::Page {
            from: None,
            to: Some("http://example.org/".to_string()),
            texts: vec![Cut::of("Hello", CHANGED_TEXT_CHARS_MAX)],
            more: 0,
        })
    );
    assert_eq!(render(&typed).unwrap(), text);
}

#[test]
fn the_renderer_refuses_what_it_could_not_read_back() {
    let mut eleven = page();
    if let Some(Changed::Page { texts, more, .. }) = &mut eleven.changed {
        texts.push(Cut::of("one more", CHANGED_TEXT_CHARS_MAX));
        *more = 0;
    }
    assert_eq!(render(&eleven), Err(RenderError::ChangedTexts));
    let mut more_with_fewer = page();
    if let Some(Changed::Page { texts, .. }) = &mut more_with_fewer.changed {
        texts.pop();
    }
    assert_eq!(render(&more_with_fewer), Err(RenderError::ChangedTexts));
    let mut long = page();
    if let Some(Changed::Page { texts, .. }) = &mut long.changed {
        texts[0] = Cut {
            kept: "x".repeat(CHANGED_TEXT_CHARS_MAX + 1),
            cut: false,
        };
    }
    assert_eq!(
        render(&long),
        Err(RenderError::CutLength {
            field: "changed text"
        })
    );
    let mut handle = controls();
    if let Some(Changed::Controls(changes)) = &mut handle.changed {
        changes[0].handle = "F 1".to_string();
    }
    assert!(matches!(
        render(&handle),
        Err(RenderError::NotAWord {
            field: "changed handle",
            ..
        })
    ));
    assert_eq!(CHANGED_TEXTS_MAX, 10);
}

#[test]
fn the_parser_refuses_lines_outside_the_block() {
    let head = "surf 1 session 7f3a open\npage none\n";
    for (lines, line) in [
        // One `~` line short of its count.
        ("changed 2\n  ~ f1 checked false -> true\n", 5),
        // A `~` line after `changed 0`.
        ("changed 0\n  ~ f1 checked false -> true\n", 4),
        // A word that is not a change.
        ("changed 1\n  ~ f1 visible false -> true\n", 4),
        ("changed 1\n  ~ f1 checked no -> true\n", 4),
        ("changed 1\n  ~ f1 value plain -> \"a\"\n", 4),
        ("changed 01\n", 3),
        ("changed page \"a\" \"b\"\n", 3),
        ("changed page nowhere -> \"b\"\n", 3),
        // `more` with fewer than 10 texts, `more 0`, and an unquoted text.
        ("changed page \"a\" -> \"b\"\n  + text more 5\n", 4),
        ("changed page \"a\" -> \"b\"\n  + text Hello\n", 4),
    ] {
        let text = format!("{head}{lines}");
        let error = parse(&text).expect_err(&text);
        assert_eq!(
            (error.line, error.kind),
            (line, ParseErrorKind::OutsideGrammar),
            "{text}"
        );
    }
    let mut texts = String::new();
    for n in 1..=10 {
        texts.push_str(&format!("  + text \"Text {n}\"\n"));
    }
    for (last, line) in [("  + text more 0\n", 14), ("  + text \"Text 11\"\n", 14)] {
        let text = format!("{head}changed page \"a\" -> \"b\"\n{texts}{last}");
        let error = parse(&text).expect_err(&text);
        assert_eq!(
            (error.line, error.kind),
            (line, ParseErrorKind::OutsideGrammar),
            "{text}"
        );
    }
}
