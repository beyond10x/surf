//! Adversary cases against the entries of the link (`surfd::link`, "The entries").

use surfd::link::{self, Answer};

const TOKEN: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

/// A link answer with `entries` as its block and `text` as its answer lines.
fn link_answer(entries: &str, text: &str) -> Vec<u8> {
    format!(
        "{}\nstatus 0\nstate open\nentries {}\n{entries}answer {}\n{text}",
        link::VERSION_LINE,
        entries.len(),
        text.len()
    )
    .into_bytes()
}

/// `crates/surfd/src/link.rs`, "The entries": an answer entry is `<m bytes: the answer lines
/// without the token line>`, and `surfd` "checks the entries against these rules"; D21: the
/// token is not kept. An answer entry holding the `token` line passes the check.
#[test]
fn adversary_an_answer_entry_holding_the_token_line_breaks_the_link() {
    let id = "aaaaaaaa-0000-4000-8000-000000000001";
    let text = format!("surf 1 session {id} open\ntoken {TOKEN}\ndid open\npage none\n");
    let entries = format!("action 1 open\nanswer-entry {}\n{text}", text.len());
    let decoded = Answer::decode(&link_answer(&entries, &text));
    assert!(
        decoded.is_err(),
        "an answer entry with the token line was accepted: {:?}",
        decoded.map(|answer| answer.entries)
    );
}

/// D20: the answer entry is the answer lines the session rendered for the client, as the link
/// carries them; `surfd` "checks the entries against the link rules". An answer entry that is not
/// the answer lines (without the token line) passes the check.
#[test]
fn adversary_an_answer_entry_other_than_the_answer_lines_breaks_the_link() {
    let id = "aaaaaaaa-0000-4000-8000-000000000001";
    let text = format!("surf 1 session {id} open\ndid go \"http://a/\"\npage none\n");
    let other = format!("surf 1 session {id} open\ndid go \"http://b/\"\npage none\n");
    let entries = format!(
        "action 2 go \"http://a/\"\nanswer-entry {}\n{other}",
        other.len()
    );
    let decoded = Answer::decode(&link_answer(&entries, &text));
    assert!(
        decoded.is_err(),
        "an answer entry unlike the answer lines was accepted: {:?}",
        decoded.map(|answer| answer.entries)
    );
}

/// The answer of `go` with a `text` block whose answer lines end in `last`, as its answer lines
/// and its answer entry.
fn text_answer(page: &str, last: &str) -> Vec<u8> {
    let id = "aaaaaaaa-0000-4000-8000-000000000001";
    let text = format!(
        "surf 1 session {id} open\ndid go \"http://a/\"\n{page}\npath 1 of 1\n  1 go \
         \"http://a/\"\nnote text: no-css\ntext markdown 1 of 1 lines from 1, {} of {} bytes\n\
         {last}\n",
        last.len() + 1,
        last.len() + 1
    );
    let entries = format!(
        "action 1 go \"http://a/\"\nanswer-entry {}\n{text}",
        text.len()
    );
    link_answer(&entries, &text)
}

/// Case 11 of `story:page-text-look` (decision D1 B): a `>` line of a `text` block may hold UTF-8
/// characters above U+007F that `surf_answer::escaped_in_text` does not name; every other line
/// stays printable ASCII, and a `>` line holding a character the rule escapes breaks the link.
#[test]
fn adversary_utf_8_only_in_a_body_line_and_only_what_is_not_escaped() {
    let page = "page 200 \"text/html\" \"http://a/\"";
    let decoded = Answer::decode(&text_answer(page, "> K\u{e4}se"));
    let answer = decoded.expect("U+00E4 in a body line passes the link");
    assert!(answer.text.contains("> K\u{e4}se\n"));
    assert!(
        answer
            .entries
            .last()
            .unwrap()
            .text
            .contains("> K\u{e4}se\n")
    );

    let page_line = "page 200 \"text/html\" \"http://a/\" \"K\u{e4}se\"";
    let breaking = [
        (page_line, "> Kase".to_string()),
        (page, "> \u{202e}fdp.exe".to_string()),
        (page, "> a\u{85}b".to_string()),
        (page, "> a\u{7f}b".to_string()),
        (page, "K\u{e4}se".to_string()),
    ];
    for (page, last) in &breaking {
        let decoded = Answer::decode(&text_answer(page, last));
        assert!(
            decoded.is_err(),
            "accepted: {page:?} {last:?}: {:?}",
            decoded.map(|answer| answer.text)
        );
    }
    // In the answer entry and in the answer lines alike: C3 followed by a byte that does not
    // continue it.
    let mut invalid = text_answer(page, "> K\u{e4}se");
    for at in 0..invalid.len() - 1 {
        if invalid[at] == 0xc3 {
            invalid[at + 1] = b'(';
        }
    }
    assert!(
        Answer::decode(&invalid).is_err(),
        "bytes that are not UTF-8"
    );
}
