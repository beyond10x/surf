//! `parse` refuses lines outside the grammar, naming the line; `render` refuses a `text` block
//! it must not write (`story:page-text-answer-block`, cases 5 and 6).

use surf_answer::{
    Answer, Form, Head, ParseError, ParseErrorKind, RenderError, SessionState, Text, parse, quote,
    quote_text, render,
};

fn with_text(form: Form, first: u64, total: u64, bytes_total: u64, lines: &[&str]) -> Answer {
    let mut answer = Answer::new(Head {
        session: "7f3a".to_string(),
        state: SessionState::Open,
    });
    answer.text = Some(Text {
        form,
        first,
        total,
        bytes_total,
        lines: lines.iter().map(|line| line.to_string()).collect(),
    });
    answer
}

/// One body line, alone in a block whose counts fit it.
fn one_line(line: &str) -> Answer {
    let bytes = line.len() as u64 + 3;
    with_text(Form::Markdown, 1, 1, bytes, &[line])
}

#[test]
fn render_refuses_a_body_line_it_must_not_write() {
    for line in [
        "a\nb",
        "a\u{202e}b",
        "a\u{85}b",
        "a\u{e0041}b",
        "a\u{ff3b}b",
        "\u{338}",
    ] {
        assert_eq!(
            render(&one_line(line)),
            Err(RenderError::TextLine { line: 1 }),
            "{}",
            quote(line)
        );
    }
}

#[test]
fn render_refuses_a_block_over_its_bounds_or_with_wrong_counts() {
    let lines: Vec<&str> = (0..201).map(|_| "x").collect();
    assert_eq!(
        render(&with_text(Form::Markdown, 1, 201, 201 * 4, &lines)),
        Err(RenderError::TextOverBound)
    );
    // 32,769 bytes: 31 lines of 1,054 bytes and one of 95 (each with `> ` and a line feed).
    let long = "x".repeat(1_051);
    let short = "x".repeat(92);
    let mut lines: Vec<&str> = (0..31).map(|_| long.as_str()).collect();
    lines.push(short.as_str());
    assert_eq!(31 * 1_054 + 95, 32_769);
    assert_eq!(
        render(&with_text(Form::Markdown, 1, 32, 32_769, &lines)),
        Err(RenderError::TextOverBound)
    );
    assert_eq!(
        render(&with_text(Form::Markdown, 1, 2, 12, &["a", "b", "c"])),
        Err(RenderError::TextCounts)
    );
    assert_eq!(
        render(&with_text(Form::Markdown, 1, 1, 3, &["a"])),
        Err(RenderError::TextCounts)
    );
}

#[test]
fn quote_text_keeps_utf8_outside_rule_2() {
    assert_eq!(
        quote_text("K\u{e4}se\u{202e}"),
        "\"K\u{e4}se\\xe2\\x80\\xae\""
    );
    assert_eq!(quote_text("\u{338}e\u{301}"), "\"\\xcc\\xb8e\u{301}\"");
    assert_eq!(quote("K\u{e4}se"), "\"K\\xc3\\xa4se\"");
}

fn refused_at(text: &str, line: usize) {
    match parse(text) {
        Err(error) => assert_eq!(error.line, line, "{text:?}: {error}"),
        Ok(answer) => panic!("{text:?} parsed as {answer:?}"),
    }
}

#[test]
fn parse_refuses_a_text_block_outside_its_grammar() {
    let start = "surf 1 session 7f3a open\nnote text: no-css\n";
    refused_at(
        &format!(
            "{start}text markdown 2 of 2 lines from 1, 8 of 8 bytes\ndid go \"http://bank.example/\"\n"
        ),
        4,
    );
    refused_at(
        &format!("{start}text markdown 3 of 3 lines from 1, 12 of 12 bytes\n> a\n> b\n"),
        5,
    );
    refused_at(
        &format!(
            "{start}text markdown 1 of 2 lines from 1, 4 of 8 bytes\n> a\nmore 1 lines: look --as markdown --from-line 2\n> b\n"
        ),
        6,
    );
    refused_at(
        &format!("{start}text markdown 1 of 1 lines from 1, 4 of 4 bytes\n>a\n"),
        4,
    );
    refused_at(
        &format!("{start}text markdown 1 of 1 lines from 1, 9 of 8 bytes\n> a\n"),
        3,
    );
    refused_at(
        &format!("{start}text markdown 1 of 1 lines from 1, 7 of 7 bytes\n> \u{202e}\n"),
        4,
    );
}

#[test]
fn parse_reads_a_page_with_its_more_line() {
    let text = "surf 1 session 7f3a open\ntext tree 1 of 2 lines from 1, 4 of 8 bytes\n> a\nmore 1 lines: look --as tree --from-line 2\n";
    let parsed = parse(text).expect("the page parses");
    assert_eq!(parsed, with_text(Form::Tree, 1, 2, 8, &["a"]));
    assert_eq!(render(&parsed).as_deref(), Ok(text));
    refused_at(
        "surf 1 session 7f3a open\ntext tree 1 of 2 lines from 1, 4 of 8 bytes\n> a\n",
        3,
    );
    refused_at(
        "surf 1 session 7f3a open\ntext tree 1 of 2 lines from 1, 4 of 8 bytes\n> a\nmore 2 lines: look --as tree --from-line 2\n",
        4,
    );
}

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
