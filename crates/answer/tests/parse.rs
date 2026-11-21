//! `parse` refuses lines outside the grammar, naming the line.

use surf_answer::{ParseError, ParseErrorKind, parse};

#[test]
fn parse_refuses_line_outside_grammar() {
    assert_eq!(
        parse("surf 1 session 7f3a open\nnow 5\n"),
        Err(ParseError {
            line: 2,
            kind: ParseErrorKind::OutsideGrammar,
        })
    );
}

#[test]
fn parse_refuses_refusal_reason_not_in_list() {
    assert_eq!(
        parse("surf 1 session 7f3a open\ndid refused forbidden\n"),
        Err(ParseError {
            line: 2,
            kind: ParseErrorKind::UnknownRefusalReason,
        })
    );
}

#[test]
fn parse_refuses_answer_without_head() {
    assert_eq!(
        parse("page none\n"),
        Err(ParseError {
            line: 1,
            kind: ParseErrorKind::NoHead,
        })
    );
}
