//! Adversary and fix round, story html-document-port: what the parser step adds to the entries of
//! a `go` (the `surf.document.DocumentParsed` and `surf.document.ElementAdded` entries) is bounded
//! whatever the page holds, so the entries of a request always fit the link, `go` answers and the
//! session stays open. Each case runs a real `surf-session` under an in-process `surfd` against a
//! page server on 127.0.0.1.

use crate::common;

use common::{PageServer, TestDir, surfd_in_process, wire_as, wire_open};
use surf_session::ELEMENT_ENTRIES_MAX;
use surf_session::entries::TITLE_ENTRY_BYTES;
use surfd::State;
use surfd::link::EntryKind;
use surfd::log::Entry;

/// The bytes of a `DocumentParsed` line beside its title field and its address field: the event's
/// name, its two identities, the encodings, the limit and the field names.
const OTHER_FIELDS_MAX: usize = 512;

/// `go` to a page serving `body`: the answer's status, whether the session is still listed open,
/// and the session's kept entries with the number of entries its log dropped.
fn go_to(body: Vec<u8>) -> (u8, bool, Vec<Entry>, u64, String) {
    let dir = TestDir::new();
    let pages = PageServer::with_body(body);
    let daemon = surfd_in_process(&dir, |_| {});
    let session = wire_open(&dir, None);
    let url = pages.url("/t");
    let answer = wire_as(&dir, &session, "go", &[("url", &url)]);
    let open = daemon
        .sessions()
        .get(&session.id)
        .map(|listed| listed.state)
        == Some(State::Open);
    let log = daemon.session_entries(&session.id).expect("listed");
    (answer.status, open, log.entries, log.dropped, url)
}

/// The kept `DocumentParsed` entry.
fn document_parsed(entries: &[Entry]) -> &Entry {
    entries
        .iter()
        .find(|entry| {
            entry
                .text
                .starts_with("event surf.document.DocumentParsed ")
        })
        .unwrap_or_else(|| panic!("no DocumentParsed entry among {} entries", entries.len()))
}

/// The smallest page found red before the fix: a windows-1252 page (no `meta`, so the default)
/// whose title is 700 KiB of the byte 0x80. Each is U+20AC, three bytes of UTF-8, written as
/// twelve bytes (`\xe2\x82\xac`) in a quoted field, so the title alone came to 8.4 MiB, over
/// `surfd::link::ENTRIES_MAX`, and `go` answered status 3 with the session died. The adversary's
/// page, 2.25 MiB of U+0001 (four bytes each), takes the same path.
#[test]
fn a_page_with_a_long_title_leaves_go_answering_and_the_session_open() {
    let mut body = b"<!doctype html><title>".to_vec();
    body.resize(body.len() + 700 * 1024, 0x80);
    body.extend_from_slice(b"</title>");
    let (status, open, entries, _, _) = go_to(body);
    assert_eq!(status, 0);
    assert!(open, "the session is open");
    let parsed = document_parsed(&entries);
    let title = format!(
        "title=\"{}\"",
        "\\xe2\\x82\\xac".repeat(TITLE_ENTRY_BYTES / 3)
    );
    assert!(parsed.text.contains(&title), "{}", &parsed.text[..300]);
    assert!(
        parsed
            .text
            .contains(&format!("title_bytes=\"{}\"", 3 * 700 * 1024)),
        "{}",
        &parsed.text[..300]
    );
}

#[test]
fn a_title_one_byte_over_the_cut_is_cut_in_the_entry() {
    let title = "a".repeat(TITLE_ENTRY_BYTES + 1);
    let body = format!("<!doctype html><title>{title}</title>").into_bytes();
    let (status, open, entries, _, _) = go_to(body);
    assert_eq!(status, 0);
    assert!(open, "the session is open");
    let parsed = document_parsed(&entries);
    let kept = format!(
        "title=\"{}\" title_bytes=\"{}\" ",
        &title[..TITLE_ENTRY_BYTES],
        TITLE_ENTRY_BYTES + 1
    );
    assert!(parsed.text.contains(&kept), "{}", parsed.text);

    // At the cut's length the title is whole and the line has no `title_bytes`.
    let title = "a".repeat(TITLE_ENTRY_BYTES);
    let body = format!("<!doctype html><title>{title}</title>").into_bytes();
    let (_, _, entries, _, _) = go_to(body);
    let parsed = document_parsed(&entries);
    assert!(parsed.text.contains(&format!("title=\"{title}\" ")));
    assert!(!parsed.text.contains("title_bytes="), "{}", parsed.text);
}

#[test]
fn a_title_of_control_characters_at_the_cut_stays_within_the_bound() {
    let mut body = b"<!doctype html><meta charset=utf-8><title>".to_vec();
    body.resize(body.len() + TITLE_ENTRY_BYTES, 0x01);
    body.extend_from_slice(b"</title>");
    let (status, open, entries, _, url) = go_to(body);
    assert_eq!(status, 0);
    assert!(open, "the session is open");
    let parsed = document_parsed(&entries);
    let title = format!("title=\"{}\"", "\\x01".repeat(TITLE_ENTRY_BYTES));
    assert!(parsed.text.contains(&title), "{}", parsed.text);
    let bound = 4 * TITLE_ENTRY_BYTES + 2 + surf_answer::quote(&url).len() + OTHER_FIELDS_MAX;
    assert!(
        parsed.text.len() <= bound,
        "{} bytes, bound {bound}",
        parsed.text.len()
    );
}

#[test]
fn a_page_with_more_links_than_a_request_carries_answers_and_keeps_the_session_open() {
    const LINKS: usize = 10_001;
    let mut body = b"<!doctype html><title>Links</title>".to_vec();
    for _ in 0..LINKS {
        body.extend_from_slice(b"<a href=/>x</a>");
    }
    let (status, open, entries, dropped, _) = go_to(body);
    assert_eq!(status, 0);
    assert!(open, "the session is open");
    assert_eq!(dropped, 0, "no entry dropped by the log's bounds");
    let added = entries
        .iter()
        .filter(|entry| entry.text.starts_with("event surf.document.ElementAdded "))
        .count();
    assert_eq!(added, ELEMENT_ENTRIES_MAX);
    let left_out: Vec<&Entry> = entries
        .iter()
        .filter(|entry| entry.text.starts_with("event surf.agent.EventsLeftOut "))
        .collect();
    assert_eq!(
        left_out
            .iter()
            .map(|entry| entry.text.as_str())
            .collect::<Vec<_>>(),
        [format!(
            "event surf.agent.EventsLeftOut count=\"{}\"\n",
            LINKS - ELEMENT_ENTRIES_MAX
        )]
    );
    document_parsed(&entries);
    assert!(
        entries
            .iter()
            .any(|entry| entry.text.starts_with("action 2 go ")),
        "the action entry of the go is kept"
    );
    assert_eq!(
        entries.last().map(|entry| entry.kind),
        Some(EntryKind::Answer)
    );
}
