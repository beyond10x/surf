//! Cuts. A field with a length limit is cut before escaping on a character boundary, ends in
//! `...`, and reads back to its kept part and the mark that it was cut (no block of step 1 has
//! such a field; the label of the `now` block is the first). And the pages of the `text` block
//! (`story:page-text-answer-block`, cases 2 to 4): at most 200 body lines and 32,768 bytes, a
//! `more` line when lines are left, and a line alone over the bound cut between escape units.

use surf_answer::{
    Answer, Cut, Form, Head, SessionState, Text, UnquoteError, parse, quote_cut, render, text_page,
    unquote_cut,
};

/// The lines of the text block as `render` writes them, without the head line; and a check
/// that they parse back to the same value.
fn block(text: Text) -> String {
    let mut answer = Answer::new(Head {
        session: "7f3a".to_string(),
        state: SessionState::Open,
    });
    answer.text = Some(text);
    let rendered = render(&answer).expect("the block renders");
    assert_eq!(
        parse(&rendered).as_ref(),
        Ok(&answer),
        "the block parses back"
    );
    rendered
        .strip_prefix("surf 1 session 7f3a open\n")
        .expect("the head line")
        .to_string()
}

fn header(text: &str) -> &str {
    text.lines().next().expect("a header")
}

fn last(text: &str) -> &str {
    text.lines().last().expect("a line")
}

fn body_count(text: &str) -> usize {
    text.lines().filter(|line| line.starts_with('>')).count()
}

#[test]
fn text_pages_by_200_lines() {
    let lines = || (0..450).map(|_| "xxxxxxxxxx".to_string());
    let first = block(text_page(Form::Markdown, lines(), 1));
    assert_eq!(
        header(&first),
        "text markdown 200 of 450 lines from 1, 2600 of 5850 bytes"
    );
    assert_eq!(body_count(&first), 200);
    assert_eq!(
        last(&first),
        "more 250 lines: look --as markdown --from-line 201"
    );

    let past = block(text_page(Form::Markdown, lines(), 451));
    assert_eq!(
        past,
        "text markdown 0 of 450 lines from 451, 0 of 5850 bytes\n"
    );
}

#[test]
fn text_pages_by_32768_bytes_to_the_end() {
    let line = "x".repeat(1_000);
    let lines = || (0..450).map(|_| line.clone());
    let first = block(text_page(Form::Markdown, lines(), 1));
    assert_eq!(
        header(&first),
        "text markdown 32 of 450 lines from 1, 32096 of 451350 bytes"
    );
    assert_eq!(
        last(&first),
        "more 418 lines: look --as markdown --from-line 33"
    );

    let mut from = 1;
    let mut pages = 0;
    let mut joined = Vec::new();
    let mut last_page = String::new();
    while from <= 450 {
        let text = text_page(Form::Markdown, lines(), from);
        from += text.lines.len() as u64;
        joined.extend(text.lines.clone());
        last_page = block(text);
        pages += 1;
    }
    assert_eq!(pages, 15);
    assert_eq!(
        last_page,
        format!(
            "text markdown 2 of 450 lines from 449, 2006 of 451350 bytes\n> {line}\n> {line}\n"
        )
    );
    assert_eq!(joined, lines().collect::<Vec<_>>());
}

#[test]
fn a_line_alone_over_the_bound_is_cut_between_units() {
    let lines = vec!["x".repeat(40_000), "y".to_string()];
    let page = block(text_page(Form::Markdown, lines.into_iter(), 1));
    let mut expected = String::from("text markdown 1 of 2 lines from 1, 32768 of 40007 bytes\n> ");
    expected.push_str(&"x".repeat(32_760));
    expected.push_str("[cut]\nmore 1 lines: look --as markdown --from-line 2\n");
    assert_eq!(page, expected);

    let escaped = format!("{}\\[{}", "x".repeat(32_759), "x".repeat(10_000));
    let text = text_page(Form::Markdown, std::iter::once(escaped), 1);
    assert_eq!(text.lines, vec![format!("{}[cut]", "x".repeat(32_759))]);

    let bytes = format!(
        "{}\\xe2\\x80\\xae{}",
        "x".repeat(32_758),
        "x".repeat(10_000)
    );
    let text = text_page(Form::Markdown, std::iter::once(bytes), 1);
    assert_eq!(text.lines, vec![format!("{}[cut]", "x".repeat(32_758))]);
}

#[test]
fn cut_field_reads_back_kept_part_and_mark() {
    let long = format!("{}\u{2028}tail", "é".repeat(59));
    let field = quote_cut(&long, 60);
    assert_eq!(
        field,
        format!("\"{}\\xe2\\x80\\xa8\"...", "\\xc3\\xa9".repeat(59))
    );
    assert_eq!(
        unquote_cut(&field, 60),
        Ok(Cut {
            kept: format!("{}\u{2028}", "é".repeat(59)),
            cut: true,
        })
    );

    let dots = "ends in dots...";
    assert_eq!(quote_cut(dots, 60), "\"ends in dots...\"");
    assert_eq!(
        unquote_cut(&quote_cut(dots, 60), 60),
        Ok(Cut {
            kept: dots.to_string(),
            cut: false,
        })
    );

    assert_eq!(unquote_cut("\"short\"...", 60), Err(UnquoteError::Length));
    assert_eq!(
        unquote_cut(&format!("\"{}\"", "a".repeat(61)), 60),
        Err(UnquoteError::Length)
    );
}
