//! The forging matrix: each of 10 forging values rendered into each of the 5 outside fields of
//! step 1 and the 4 of `story:agent-follow-links` (the label and the target of a `now` line,
//! resolved and unresolved, and the label of a `path` line). Every answer holds only bytes from U+0020 to U+007E and line feeds, has exactly the
//! line count of the same answer with a plain value, and `parse` gives back exactly the values
//! given. The body line of the `text` block is the sixth field: through `quote_text` every
//! value renders and holds no character rule 2 escapes; written raw, a value that could end its
//! line is refused by `render`.

use surf_answer::{
    Answer, Cut, Did, Element, Form, Head, LABEL_CHARS_MAX, Note, Now, Page, Path, PathEntry,
    SessionState, TARGET_CHARS_MAX, Target, Text, TextNote, Visibility, escaped_at_run_start,
    escaped_in_text, parse, quote_text, quote_text_cut, render,
};

/// The 10 forging values, each with a name for the failure message.
fn forging_values() -> Vec<(&'static str, String)> {
    let mixed = ['a', '"', '\\', '\n', '\r', '\u{2028}', '\u{202e}', 'é', ' '];
    let long: String = mixed.iter().cycle().take(10_000).collect();
    assert_eq!(long.chars().count(), 10_000);
    vec![
        ("line feed", "\n".to_string()),
        ("carriage return", "\r".to_string()),
        ("U+2028", "\u{2028}".to_string()),
        ("U+2029", "\u{2029}".to_string()),
        ("bidirectional control U+202E", "\u{202e}".to_string()),
        ("double quote", "\"".to_string()),
        ("backslash", "\\".to_string()),
        ("quote then now 5", "\" now 5".to_string()),
        (
            "beginning page",
            "page 200 \"text/html\" \"http://forged.example/\"".to_string(),
        ),
        ("10,000 characters", long),
    ]
}

fn plain_loaded(media_type: &str, address: &str, title: &str, argument: &str) -> Answer {
    let mut answer = Answer::new(Head {
        session: "7f3a".to_string(),
        state: SessionState::Open,
    });
    answer.did = Some(Did::Action {
        command: "go".to_string(),
        argument: Some("http://example.org/".to_string()),
    });
    answer.page = Some(Page::Loaded {
        status: 200,
        media_type: media_type.to_string(),
        address: address.to_string(),
        title: Some(title.to_string()),
    });
    answer.path = Some(Path {
        total: 2,
        entries: vec![
            PathEntry {
                sequence: 1,
                command: "open".to_string(),
                handle: None,
                argument: None,
                argument_cut: false,
            },
            PathEntry {
                sequence: 2,
                command: "go".to_string(),
                handle: None,
                argument: Some(argument.to_string()),
                argument_cut: false,
            },
        ],
    });
    answer
}

fn plain_failed(address: &str) -> Answer {
    let mut answer = plain_loaded("text/html", "http://example.org/", "Home", "plain");
    answer.page = Some(Page::Failed {
        reason: "Unreachable".to_string(),
        address: address.to_string(),
    });
    answer
}

/// Render the answer for every forging value, check the three properties, and return how many
/// round trips gave back exactly the answer given.
fn matrix(field: &str, answer_with: impl Fn(&str) -> Answer) -> usize {
    let plain = render(&answer_with("plain")).expect("the plain answer renders");
    let plain_lines = plain.matches('\n').count();
    let mut equal = 0;
    for (name, value) in forging_values() {
        let given = answer_with(&value);
        let text = render(&given).expect("a forging answer renders");
        assert!(
            text.bytes()
                .all(|b| b == b'\n' || (0x20..=0x7e).contains(&b)),
            "{field}, {name}: a byte outside U+0020 to U+007E and line feed"
        );
        assert_eq!(
            text.matches('\n').count(),
            plain_lines,
            "{field}, {name}: the line count differs from the plain answer's"
        );
        let parsed = parse(&text).unwrap_or_else(|error| panic!("{field}, {name}: {error}"));
        assert_eq!(parsed, given, "{field}, {name}");
        equal += 1;
    }
    println!("{field}: {equal} of 10 round trips equal to the values given");
    equal
}

#[test]
fn forging_page_address() {
    let equal = matrix("page address", |value| {
        plain_loaded("text/html", value, "Home", "plain")
    });
    assert_eq!(equal, 10);
}

#[test]
fn forging_page_media_type() {
    let equal = matrix("page media type", |value| {
        plain_loaded(value, "http://example.org/", "Home", "plain")
    });
    assert_eq!(equal, 10);
}

#[test]
fn forging_page_title() {
    let equal = matrix("page title", |value| {
        plain_loaded("text/html", "http://example.org/", value, "plain")
    });
    assert_eq!(equal, 10);
}

#[test]
fn forging_page_failed_address() {
    let equal = matrix("page failed address", plain_failed);
    assert_eq!(equal, 10);
}

/// The plain answer with a `text` block of three body lines, the middle one given.
fn with_body_line(line: &str) -> Answer {
    let mut answer = plain_loaded("text/html", "http://example.org/", "Home", "plain");
    answer.notes = vec![Note::Text(TextNote::NoCss)];
    let lines = vec!["before".to_string(), line.to_string(), "after".to_string()];
    let bytes = lines.iter().map(|line| line.len() as u64 + 3).sum();
    answer.text = Some(Text {
        form: Form::Tree,
        first: 1,
        total: 3,
        bytes_total: bytes,
        lines,
    });
    answer
}

/// A text answer holds no character rule 2 escapes, its line count is the plain one's, and it
/// parses back to the value given.
fn text_matrix(field: &str, answer_with: impl Fn(&str) -> Answer) -> (usize, usize) {
    let plain = render(&answer_with("plain")).expect("the plain answer renders");
    let plain_lines = plain.matches('\n').count();
    let (mut equal, mut refused) = (0, 0);
    for (name, value) in forging_values() {
        let given = answer_with(&value);
        let Ok(text) = render(&given) else {
            refused += 1;
            continue;
        };
        assert!(
            text.chars().all(|c| c == '\n' || !escaped_in_text(c)),
            "{field}, {name}: a character rule 2 escapes"
        );
        assert!(
            text.split('\n')
                .all(|line| !line.chars().next().is_some_and(escaped_at_run_start)),
            "{field}, {name}: a line starts with a combining mark"
        );
        assert_eq!(
            text.matches('\n').count(),
            plain_lines,
            "{field}, {name}: the line count differs from the plain answer's"
        );
        let parsed = parse(&text).unwrap_or_else(|error| panic!("{field}, {name}: {error}"));
        assert_eq!(parsed, given, "{field}, {name}");
        equal += 1;
    }
    println!("{field}: {equal} of 10 round trips equal, {refused} refused");
    (equal, refused)
}

#[test]
fn forging_text_body_line_through_quote_text() {
    // The 10,000 characters quote to about 49,000 bytes, over the block's bound of 32,768:
    // `render` refuses that block (`text_page` would cut the line).
    let (equal, refused) = text_matrix("text body line by quote_text", |value| {
        with_body_line(&format!("  p {}", quote_text(value)))
    });
    assert_eq!((equal, refused), (9, 1));
    let (equal, refused) = text_matrix("text body line by quote_text_cut", |value| {
        with_body_line(&format!("  p {}", quote_text_cut(value, 80)))
    });
    assert_eq!((equal, refused), (10, 0));
}

#[test]
fn forging_text_body_line_raw_is_refused_or_kept_in_its_line() {
    // Line feed, carriage return, U+2028, U+2029, U+202E and the long value (it holds a line
    // feed) are refused; the double quote, the backslash and the two forged lines stay inside
    // their body line behind `> `.
    let (equal, refused) = text_matrix("text body line raw", with_body_line);
    assert_eq!((equal, refused), (4, 6));
}

#[test]
fn forging_path_argument() {
    let equal = matrix("path argument", |value| {
        plain_loaded("text/html", "http://example.org/", "Home", value)
    });
    assert_eq!(equal, 10);
}

/// The plain answer with a `now` block of two link lines, the first with the label and target
/// given, cut at their limits as the session cuts them, and `note visibility:`.
fn with_now(label: &str, target: Target) -> Answer {
    let mut answer = plain_loaded("text/html", "http://example.org/", "Home", "plain");
    answer.now = Some(Now {
        total: 2,
        elements: vec![
            Element {
                handle: "l1".to_string(),
                kind: "link".to_string(),
                visibility: Visibility::Unknown,
                label: Cut::of(label, LABEL_CHARS_MAX),
                target,
            },
            Element {
                handle: "l2".to_string(),
                kind: "link".to_string(),
                visibility: Visibility::Hidden,
                label: Cut::of("after", LABEL_CHARS_MAX),
                target: Target::Resolved(Cut::of("/after", TARGET_CHARS_MAX)),
            },
        ],
    });
    answer.notes = vec![Note::Visibility];
    answer
}

#[test]
fn forging_now_label() {
    let equal = matrix("now label", |value| {
        with_now(value, Target::Resolved(Cut::of("/", TARGET_CHARS_MAX)))
    });
    assert_eq!(equal, 10);
}

#[test]
fn forging_now_target() {
    let equal = matrix("now target", |value| {
        with_now("plain", Target::Resolved(Cut::of(value, TARGET_CHARS_MAX)))
    });
    assert_eq!(equal, 10);
}

#[test]
fn forging_now_target_unresolved() {
    let equal = matrix("now target unresolved", |value| {
        with_now(
            "plain",
            Target::Unresolved(Cut::of(value, TARGET_CHARS_MAX)),
        )
    });
    assert_eq!(equal, 10);
}

#[test]
fn forging_path_label() {
    let equal = matrix("path label", |value| {
        let mut answer = plain_loaded("text/html", "http://example.org/", "Home", "plain");
        let label = Cut::of(value, LABEL_CHARS_MAX);
        if let Some(path) = &mut answer.path {
            path.entries[1] = PathEntry {
                sequence: 2,
                command: "follow".to_string(),
                handle: Some("l1".to_string()),
                argument: Some(label.kept),
                argument_cut: label.cut,
            };
        }
        answer
    });
    assert_eq!(equal, 10);
}
