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
