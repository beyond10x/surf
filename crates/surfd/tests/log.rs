//! Cases 9, 10 and 11 of `story:agent-session-events`: the log `surfd` keeps, driven alone
//! (`surfd::log::Log`), and case 11 once more through `Daemon::start` with the stand-in session;
//! case 23 of `story:view-surfd-serves`, an entry kept after its log was dropped whole.
//! Two more on the link: entries go and come back as they were sent, and a session whose
//! entries break the link's rules is ended and listed died.

use crate::common;

use std::time::Duration;

use common::*;
use surfd::link::{self, EntryKind};
use surfd::log::{Bounds, Log, OlderThanKept};
use surfd::{Config, State};

/// An action entry of exactly `size` bytes, its line feed included.
fn entry_of(size: usize) -> link::Entry {
    let prefix = "action 1 go \"";
    let line = format!("{prefix}{}\"", "a".repeat(size - prefix.len() - 2));
    let entry = link::Entry::line(&line).expect("an action line");
    assert_eq!(entry.text.len(), size);
    entry
}

const A: &str = "aaaaaaaa-0000-4000-8000-000000000001";
const B: &str = "bbbbbbbb-0000-4000-8000-000000000002";

#[test]
fn case_09_the_entry_bound_per_session_drops_the_oldest() {
    let mut log = Log::new(Bounds {
        entries_per_session: 10,
        ..Bounds::default()
    });
    for _ in 0..25 {
        log.keep(A, entry_of(100), 0);
    }
    let kept = log.session_entries(A).unwrap();
    assert_eq!(kept.entries.len(), 10);
    assert_eq!(kept.entries[0].index, 16);
    assert_eq!(kept.dropped, 15);
    assert!(!kept.log_dropped);

    let config = Config::new(None);
    assert_eq!(config.log.entries_per_session, 10_000);
    assert_eq!(config.log.bytes_per_session, 16_777_216);
    assert_eq!(config.log.bytes_total, 67_108_864);
    assert_eq!(config.log.reader_queue, 1_024);
}

#[test]
fn case_10_the_byte_bound_per_session_drops_the_oldest() {
    let mut log = Log::new(Bounds {
        bytes_per_session: 4_096,
        ..Bounds::default()
    });
    for _ in 0..10 {
        log.keep(A, entry_of(1_000), 0);
    }
    let kept = log.session_entries(A).unwrap();
    assert_eq!(kept.entries.len(), 4);
    assert_eq!(kept.entries[0].index, 7);
    assert_eq!(kept.dropped, 6);
}

#[test]
fn case_11_the_bound_over_all_sessions_drops_an_ended_log_whole() {
    let mut log = Log::new(Bounds {
        bytes_total: 8_000,
        ..Bounds::default()
    });
    for _ in 0..4 {
        log.keep(A, entry_of(1_000), 0);
    }
    log.end(A);
    for _ in 0..4 {
        log.keep(B, entry_of(1_000), 0);
    }
    let a = log.session_entries(A).unwrap();
    assert_eq!(
        (a.entries.len(), a.log_dropped),
        (4, false),
        "at 8,000 bytes"
    );

    log.keep(B, entry_of(1_000), 0);
    let a = log.session_entries(A).unwrap();
    assert!(a.log_dropped, "the log of A is reported dropped");
    assert!(a.entries.is_empty());
    assert_eq!(a.dropped, 4, "its 4 entries counted dropped");
    assert!(log.log_dropped(A));

    log.keep(B, entry_of(1_000), 0);
    let b = log.session_entries(B).unwrap();
    assert_eq!(b.entries.len(), 6, "B keeps all 6");
    assert_eq!(b.dropped, 0);
    assert!(!b.log_dropped);

    // Through `Daemon::start`, the bound over all sessions below the bytes of one session's
    // `open` entries: the stand-in's are its action line, one event line and its answer.
    let dir = TestDir::new();
    let mut config = config(&dir);
    config.log.bytes_total = 100;
    let daemon = start(config);
    let a = open(&dir);
    assert!(
        daemon.session_entries(&a.id).unwrap().dropped > 0,
        "open is over the bound"
    );
    let closed = call(&dir, &request("close", Some(&a.id), Some(&a.token)));
    assert_eq!(closed.status, 0, "{closed:?}");
    assert!(daemon.wait_sessions(ALLOWED, |list| {
        list.get(&a.id)
            .is_some_and(|session| session.state == State::Closed)
    }));
    // The stand-in exits as soon as it has written the answer to `close`, so `surfd` lists A
    // ended either before or after it keeps the entries of that answer; the log expects both
    // (`surfd::log`: an entry kept for a session whose log was dropped whole, "a `close` kept
    // after the session ended"). Kept before, they drop A's oldest entries and A ends within
    // the bound, and B's entries drop its log. Kept after, A has ended and they put the log
    // above the bound themselves, so A's log is dropped whole already. Either way the list
    // entry and the log say the same.
    let kept = daemon.session_entries(&a.id).unwrap();
    assert_eq!(
        daemon.sessions().get(&a.id).unwrap().log_dropped,
        kept.log_dropped,
        "{kept:?}"
    );
    if kept.log_dropped {
        assert!(kept.entries.is_empty(), "{kept:?}");
    }
    let b = open(&dir);
    let listed = daemon.sessions();
    assert!(
        listed.get(&a.id).unwrap().log_dropped,
        "the list entry of A"
    );
    assert!(!listed.get(&b.id).unwrap().log_dropped);
    assert!(daemon.session_entries(&a.id).unwrap().log_dropped);
}

/// Case 23 of `story:view-surfd-serves`: an entry kept for a session whose log was dropped
/// whole (a `close` kept after the session ended) is dropped at once.
#[test]
fn case_23_an_entry_kept_after_the_log_was_dropped_whole_is_dropped() {
    let mut log = Log::new(Bounds {
        bytes_total: 8_000,
        ..Bounds::default()
    });
    for _ in 0..4 {
        log.keep(A, entry_of(1_000), 0);
    }
    log.end(A);
    for _ in 0..5 {
        log.keep(B, entry_of(1_000), 0);
    }
    let a = log.session_entries(A).unwrap();
    assert!(a.log_dropped, "B's entries dropped the log of A");
    assert_eq!(a.dropped, 4);
    let before = log.entries_after(9).unwrap();
    assert!(before.is_empty(), "nothing above 9 yet");

    let mut reader = log.subscribe_after(log.newest()).unwrap();
    let number = log.keep(A, entry_of(100), 0);
    assert_eq!(number, 10, "it is given its number");
    let a = log.session_entries(A).unwrap();
    assert!(a.entries.is_empty(), "{a:?}");
    assert!(a.log_dropped);
    assert_eq!(a.dropped, 5, "one higher");
    assert_eq!(log.entries_after(9), Err(OlderThanKept));
    assert_eq!(log.entries_after(10), Ok(Vec::new()));
    let handed = reader.try_next().expect("handed to a live reader");
    assert_eq!((handed.number, handed.session.as_str()), (10, A));
    assert_eq!(log.newest_of(A, EntryKind::Action), None);
    assert_eq!(log.session_entries(B).unwrap().entries.len(), 5);
}

#[test]
fn entries_cross_the_link_as_they_were_sent() {
    let answer = link::Answer {
        status: 0,
        reason: None,
        state: link::SessionState::Open,
        address: None,
        title: None,
        entries: vec![
            link::Entry::line("action 2 go \"http://e.org/a b\"").unwrap(),
            link::Entry::line("event surf.agent.ActionRecorded action_id=\"x\" url=").unwrap(),
            link::Entry::line("event surf.render.FramePainted unmapped").unwrap(),
            link::Entry::answer("surf 1 session x open\ndid go \"a\"\n"),
        ],
        text: "surf 1 session x open\ndid go \"a\"\n".to_string(),
    };
    let bytes = answer.encode().unwrap();
    assert_eq!(link::Answer::decode(&bytes).unwrap(), answer);
    assert_eq!(answer.entries[1].kind, EntryKind::Event);

    let mut with_token = answer.clone();
    with_token.text = "surf 1 session x open\ntoken abc\ndid go \"a\"\n".to_string();
    let bytes = with_token.encode().unwrap();
    assert_eq!(link::Answer::decode(&bytes).unwrap(), with_token);
    let mut token_in_entry = with_token.clone();
    *token_in_entry.entries.last_mut().unwrap() = link::Entry::answer(&with_token.text);
    assert!(token_in_entry.encode().is_err());
    let mut other_entry = answer.clone();
    *other_entry.entries.last_mut().unwrap() = link::Entry::answer("surf 1 session x open\n");
    assert!(other_entry.encode().is_err());

    for line in [
        "event surf.net.RequestStarted url=http://e.org/",
        "event surf.net.RequestStarted url=\"a\"x",
        "event surf.net.RequestStarted  url=\"a\"",
        "event surf.net.RequestStarted URL=\"a\"",
        "action 01 open",
        "action 1 Open",
        "answer-entry 3",
    ] {
        assert_eq!(link::Entry::line(line), None, "{line}");
    }
    let mut out_of_order = answer.clone();
    out_of_order.entries.swap(0, 1);
    assert!(out_of_order.encode().is_err());
    let mut no_answer = answer.clone();
    no_answer.entries.pop();
    assert!(no_answer.encode().is_err());
}

#[test]
fn a_session_whose_entries_break_the_link_is_died() {
    let dir = TestDir::new();
    let daemon = start(config(&dir));
    let session = open(&dir);
    dir.set("behaviour", "bad-entries");
    let answer = look(&dir, &session);
    assert_eq!(answer.status, 3, "{answer:?}");
    assert_eq!(answer.text, refusal(&session.id, "died", "session-died"));
    assert!(daemon.wait_sessions(Duration::from_secs(5), |list| {
        list.get(&session.id)
            .is_some_and(|listed| listed.state == State::Died)
    }));
    let kept = daemon.session_entries(&session.id).unwrap();
    assert!(
        kept.entries
            .iter()
            .all(|entry| !entry.text.contains("url=http")),
        "{kept:?}"
    );
}
