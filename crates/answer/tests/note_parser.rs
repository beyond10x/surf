//! The renderer refuses a `note parser` part outside `[a-z-]+` rather than write it.

use surf_answer::{Answer, Head, Note, RenderError, SessionState, render};

#[test]
fn note_parser_refuses_part_outside_word() {
    let mut answer = Answer::new(Head {
        session: "7f3a".to_string(),
        state: SessionState::Open,
    });
    answer.notes = vec![Note::Parser(vec![
        "tables".to_string(),
        "foreign content\nnow 1".to_string(),
    ])];
    assert_eq!(
        render(&answer),
        Err(RenderError::NotAWord {
            field: "note parser part",
            value: r#""foreign content\x0anow 1""#.to_string(),
        })
    );
}
