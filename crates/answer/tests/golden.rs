//! One golden answer per block form of step 1, compared byte for byte with
//! `tests/golden/<form>.txt`, and one round trip per golden file: `parse` of the file gives the
//! typed value of its golden test, and `render` of that value gives the file again.

use surf_answer::{
    Answer, Did, Head, Note, Page, Path, PathEntry, RefusalReason, SessionState, parse, render,
};

fn golden(name: &str) -> String {
    let path = format!("{}/tests/golden/{name}.txt", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{path}: {error}"))
}

fn head(state: SessionState) -> Answer {
    Answer::new(Head {
        session: "7f3a".to_string(),
        state,
    })
}

fn head_open() -> Answer {
    head(SessionState::Open)
}

fn head_died() -> Answer {
    head(SessionState::Died)
}

fn head_unknown() -> Answer {
    Answer::new(Head {
        session: "0b5e".to_string(),
        state: SessionState::Unknown,
    })
}

fn token() -> Answer {
    let mut answer = head_open();
    answer.token =
        Some("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string());
    answer
}

fn did() -> Answer {
    let mut answer = head_open();
    answer.did = Some(Did::Action {
        command: "go".to_string(),
        argument: Some("http://example.org/".to_string()),
    });
    answer
}

fn did_refused() -> Answer {
    let mut answer = head(SessionState::Closed);
    answer.did = Some(Did::Refused(RefusalReason::Closed));
    answer
}

fn page() -> Answer {
    let mut answer = head_open();
    answer.page = Some(Page::Loaded {
        status: 200,
        media_type: "text/html".to_string(),
        address: "http://example.org/login".to_string(),
        title: Some("Sign in".to_string()),
    });
    answer
}

fn page_none() -> Answer {
    let mut answer = head_open();
    answer.page = Some(Page::None);
    answer
}

fn page_failed() -> Answer {
    let mut answer = head_open();
    answer.page = Some(Page::Failed {
        reason: "Unreachable".to_string(),
        address: "http://127.0.0.1:9/".to_string(),
    });
    answer
}

fn path() -> Answer {
    let mut answer = head(SessionState::Closed);
    answer.path = Some(Path {
        total: 4,
        entries: vec![
            PathEntry {
                sequence: 2,
                command: "go".to_string(),
                argument: Some("http://example.org/".to_string()),
            },
            PathEntry {
                sequence: 3,
                command: "go".to_string(),
                argument: Some("http://example.org/login".to_string()),
            },
            PathEntry {
                sequence: 4,
                command: "close".to_string(),
                argument: None,
            },
        ],
    });
    answer
}

fn note_parser() -> Answer {
    let mut answer = head_open();
    answer.notes = vec![Note::Parser(vec![
        "tables".to_string(),
        "foreign-content".to_string(),
    ])];
    answer
}

/// `render` of the typed value equals the golden file byte for byte.
fn assert_golden(name: &str, answer: Answer) {
    let rendered = render(&answer).expect("the golden value renders");
    assert_eq!(rendered.as_bytes(), golden(name).as_bytes(), "{name}");
}

/// `parse` of the golden file gives the typed value, and `render` of that gives the file.
fn assert_round_trip(name: &str, answer: Answer) {
    let text = golden(name);
    let parsed = parse(&text).expect("the golden file parses");
    assert_eq!(parsed, answer, "{name}");
    let rendered = render(&parsed).expect("the parsed value renders");
    assert_eq!(rendered.as_bytes(), text.as_bytes(), "{name}");
}

#[test]
fn golden_head() {
    assert_golden("head", head_open());
}

#[test]
fn golden_head_died() {
    assert_golden("head-died", head_died());
}

#[test]
fn golden_head_unknown() {
    assert_golden("head-unknown", head_unknown());
}

#[test]
fn golden_token() {
    assert_golden("token", token());
}

#[test]
fn golden_did() {
    assert_golden("did", did());
}

#[test]
fn golden_did_refused() {
    assert_golden("did-refused", did_refused());
}

#[test]
fn golden_page() {
    assert_golden("page", page());
}

#[test]
fn golden_page_none() {
    assert_golden("page-none", page_none());
}

#[test]
fn golden_page_failed() {
    assert_golden("page-failed", page_failed());
}

#[test]
fn golden_path() {
    assert_golden("path", path());
}

#[test]
fn golden_note_parser() {
    assert_golden("note-parser", note_parser());
}

#[test]
fn round_trip_head() {
    assert_round_trip("head", head_open());
}

#[test]
fn round_trip_head_died() {
    assert_round_trip("head-died", head_died());
}

#[test]
fn round_trip_head_unknown() {
    assert_round_trip("head-unknown", head_unknown());
}

#[test]
fn round_trip_token() {
    assert_round_trip("token", token());
}

#[test]
fn round_trip_did() {
    assert_round_trip("did", did());
}

#[test]
fn round_trip_did_refused() {
    assert_round_trip("did-refused", did_refused());
}

#[test]
fn round_trip_page() {
    assert_round_trip("page", page());
}

#[test]
fn round_trip_page_none() {
    assert_round_trip("page-none", page_none());
}

#[test]
fn round_trip_page_failed() {
    assert_round_trip("page-failed", page_failed());
}

#[test]
fn round_trip_path() {
    assert_round_trip("path", path());
}

#[test]
fn round_trip_note_parser() {
    assert_round_trip("note-parser", note_parser());
}
