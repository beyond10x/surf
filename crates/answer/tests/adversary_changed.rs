//! The adversary's cases on the `changed` block of `story:agent-change-and-log` (`cf9a6ba`): every
//! field of the block filled with values that hold line breaks, quotes, escapes, the mark of a
//! cut, the head of another line of the answer, and characters of two to four bytes at the edge
//! of each length limit. Each answer renders to one line per field and reads back to its typed
//! value and to the same bytes.

use surf_answer::{
    Answer, CHANGED_TEXT_CHARS_MAX, CHANGED_TEXTS_MAX, Change, Changed, ControlChange, Cut, Head,
    Page, Path, SessionState, VALUE_CHARS_MAX, Value, parse, render,
};

/// Values a page or an agent can put into a field of the block.
fn hostile() -> Vec<String> {
    let mut values: Vec<String> = [
        "",
        " ",
        "\n",
        "\r",
        "\r\n",
        "\u{2028}",
        "\u{202e}",
        "\u{0}",
        "\u{7f}",
        "\u{feff}",
        "\u{1f600}",
        "\"",
        "\\",
        "\\x41",
        "...",
        "\"...",
        "x\"...",
        "\" -> \"",
        "none",
        "\n  + text more 5",
        "\n  + text \"x\"",
        "\n  ~ f1 checked false -> true",
        "\nchanged 0",
        "\nnow 1 of 1",
        "\ndid go \"http://evil/\"",
        "(set)",
        "(empty)",
    ]
    .iter()
    .map(|value| value.to_string())
    .collect();
    for limit in [CHANGED_TEXT_CHARS_MAX, VALUE_CHARS_MAX] {
        for unit in ["a", "é", "日", "\u{1f600}", "\""] {
            for n in [limit - 1, limit, limit + 1] {
                values.push(unit.repeat(n));
            }
        }
    }
    values
}

fn head() -> Answer {
    let mut answer = Answer::new(Head {
        session: "7f3a".to_string(),
        state: SessionState::Open,
    });
    answer.page = Some(Page::None);
    answer.path = Some(Path {
        total: 0,
        entries: Vec::new(),
    });
    answer
}

/// `typed` renders to text whose `changed` block has `lines` lines, which reads back to `typed`
/// and renders again to the same bytes.
fn holds(typed: &Answer, lines: usize) {
    let text = render(typed).unwrap_or_else(|error| panic!("{error}: {typed:?}"));
    assert!(
        text.bytes()
            .all(|b| b == b'\n' || (b' '..=b'~').contains(&b)),
        "{text}"
    );
    let block = text
        .lines()
        .skip_while(|line| !line.starts_with("changed "))
        .count();
    assert_eq!(block, lines, "{text}");
    let parsed = parse(&text).unwrap_or_else(|error| panic!("{error}: {text}"));
    assert_eq!(&parsed, typed, "{text}");
    assert_eq!(render(&parsed).unwrap(), text);
}

#[test]
fn adversary_changed_page_fields_hold_hostile_values() {
    let values = hostile();
    for value in &values {
        for (from, to) in [
            (Some(value.clone()), None),
            (None, Some(value.clone())),
            (Some(value.clone()), Some(value.clone())),
        ] {
            let mut typed = head();
            typed.changed = Some(Changed::Page {
                from,
                to,
                texts: vec![Cut::of(value, CHANGED_TEXT_CHARS_MAX)],
                more: 0,
            });
            holds(&typed, 2);
        }
    }
    let mut typed = head();
    typed.changed = Some(Changed::Page {
        from: None,
        to: None,
        texts: values
            .iter()
            .take(CHANGED_TEXTS_MAX)
            .map(|value| Cut::of(value, CHANGED_TEXT_CHARS_MAX))
            .collect(),
        more: u64::MAX,
    });
    holds(&typed, 12);
}

#[test]
fn adversary_changed_control_fields_hold_hostile_values() {
    let values = hostile();
    for value in &values {
        let cut = Cut::of(value, VALUE_CHARS_MAX);
        let changes = vec![
            ControlChange {
                handle: "f1".to_string(),
                change: Change::Value {
                    before: Value::Text(cut.clone()),
                    after: Value::Set,
                },
            },
            ControlChange {
                handle: "f2".to_string(),
                change: Change::Value {
                    before: Value::Empty,
                    after: Value::Text(cut.clone()),
                },
            },
            ControlChange {
                handle: "f30".to_string(),
                change: Change::Selected {
                    before: cut.clone(),
                    after: Cut::of("", VALUE_CHARS_MAX),
                },
            },
            ControlChange {
                handle: "f4".to_string(),
                change: Change::Checked {
                    before: false,
                    after: true,
                },
            },
        ];
        let mut typed = head();
        typed.changed = Some(Changed::Controls(changes));
        holds(&typed, 5);
    }
}
