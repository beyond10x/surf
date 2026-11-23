//! Cases 1 to 8 and 12 to 18 of `story:agent-session-events`: what the session process sends
//! `surfd` per action, its action, its events and its answer, and what `surfd` keeps of it. The
//! cases that read the log run `surfd` in the test process through `Daemon::start`, with the
//! built `surf-session` as the session program, a page server the test starts, and requests over
//! `surf_agent::wire` (decision D29). Cases 17 and 18 run the mapping alone, against the events
//! `spec/` declares.

#![forbid(unsafe_code)]

use crate::common;

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use std::sync::mpsc::RecvTimeoutError;
use std::thread;
use std::time::Duration;

use common::{Opened, PageServer, TestDir, surfd_in_process, wire_as, wire_open};
use surf::{MemoryBrowser, SystemEvent};
use surf_answer::Page;
use surf_ports::{Caller, ComponentPorts};
use surf_types::agent::{
    ActionCommand, ActionId, ActionRecorded, FillRequested, Go, NavigationRequested, OpenSession,
    PickRequested, PressRequested, RecordAction, SessionClosed, SessionId,
    SessionNavigationChanged, SessionOpened,
};
use surf_types::document::{
    CheckednessSet, DocumentId, DocumentParsed, ElementAdded, ElementId, EncodingName, FormAdded,
    FormId, FormSubmitted, OptionAdded, OptionId, OptionSelected, ParseLimit, ParsingStarted,
    SubmissionAbandoned, SubmissionEndReason, SubmissionId, SubmissionStarted, ValueSet,
};
use surf_types::navigation::{
    NavigationCancelled, NavigationCommitted, NavigationFailed, NavigationStarted, NavigationState,
};
use surf_types::net::{
    FailureReason, FetchFailed, MediaType, Method, RequestId, RequestStarted, ResponseReceived,
    Status,
};
use surf_types::primitives::Uuid;
use surf_types::render::{FrameId, FramePainted};
use surf_types::shell::{
    OpenTab, OpenTabOutcome, OpenWindow, OpenWindowOutcome, TabClosed, TabId, TabOpened, WindowId,
    WindowOpened,
};
use surf_types::url::Url;
use surfd::link::EntryKind;
use surfd::log::{Entry, OlderThanKept};
use surfd::{Daemon, State};

/// The entries of `open` without an address: the action entry, its four events, the answer
/// entry. Measured by this unit (story: counted from `spec/` as 6).
const OPEN_ENTRIES: usize = 6;

/// The entries of `go` to a served HTML page: the action entry, its ten events, the answer
/// entry. Measured by this unit (story: counted from `spec/` as 11); since
/// `story:html-document-port` the parser step adds `surf.document.DocumentParsed` (the page has
/// no actionable element, so no `surf.document.ElementAdded`).
const GO_ENTRIES: usize = 12;

// --- Reading entries ---------------------------------------------------------------------------

/// The kept entries of `session`.
fn kept(daemon: &Daemon, session: &Opened) -> Vec<Entry> {
    let log = daemon
        .session_entries(&session.id)
        .expect("the session is listed");
    assert!(!log.log_dropped, "the log was dropped");
    log.entries
}

/// The qualified name of an event entry.
fn event_name(entry: &Entry) -> &str {
    assert_eq!(entry.kind, EntryKind::Event, "{entry:?}");
    entry
        .text
        .strip_prefix("event ")
        .and_then(|rest| rest.trim_end().split(' ').next())
        .expect("an event line")
}

/// The fields of an event line (without its line feed): each name and its quoted field, `None`
/// for a field with no value; `None` for the whole when the line says `unmapped`.
fn fields(line: &str) -> Option<Vec<(String, Option<String>)>> {
    let rest = line.strip_prefix("event ").expect("an event line");
    let (_, mut rest) = rest.split_once(' ').unwrap_or((rest, ""));
    if rest == "unmapped" {
        return None;
    }
    let mut fields = Vec::new();
    while !rest.is_empty() {
        let (name, after) = rest.split_once('=').expect("a field");
        let (value, next) = match after.strip_prefix('"') {
            Some(inside) => {
                let end = inside.find('"').expect("a closing quote") + 2;
                (Some(after[..end].to_string()), &after[end..])
            }
            None => (None, after),
        };
        fields.push((name.to_string(), value));
        rest = next.strip_prefix(' ').unwrap_or(next);
    }
    Some(fields)
}

/// The quoted field `name` of an event entry.
fn field(entry: &Entry, name: &str) -> Option<String> {
    fields(entry.text.trim_end())
        .expect("a mapped event")
        .into_iter()
        .find(|(field, _)| field == name)
        .and_then(|(_, value)| value)
}

/// The entries of the one action that starts at `entries[from]`: the action entry, every event
/// entry after it, and the answer entry when it follows; the next action entry ends it.
fn action_at(entries: &[Entry], from: usize) -> &[Entry] {
    assert_eq!(entries[from].kind, EntryKind::Action, "{:?}", entries[from]);
    let mut end = from + 1;
    while end < entries.len() && entries[end].kind == EntryKind::Event {
        end += 1;
    }
    if end < entries.len() && entries[end].kind == EntryKind::Answer {
        end += 1;
    }
    &entries[from..end]
}

/// The event names of `entries` in order.
fn names(entries: &[Entry]) -> Vec<String> {
    entries
        .iter()
        .filter(|entry| entry.kind == EntryKind::Event)
        .map(|entry| event_name(entry).to_string())
        .collect()
}

/// The answer lines of `text` without the `token` line.
fn without_token(text: &str) -> String {
    text.split_inclusive('\n')
        .filter(|line| !line.starts_with("token "))
        .collect()
}

/// `go <url>` as the holder, which must answer status 0.
fn go(dir: &TestDir, session: &Opened, url: &str) -> String {
    let answer = wire_as(dir, session, "go", &[("url", url)]);
    assert_eq!(answer.status, 0, "go {url}: {answer:?}");
    answer.text
}

// --- The reference ------------------------------------------------------------------------------

/// The names `System::published()` lists in a `surf::MemoryBrowser` the test drives itself with
/// the commands the session issues for `open` and then `go <url>`, over the same transport: the
/// names of the `open` and those of the `go`.
fn reference_names(url: &str) -> (Vec<String>, Vec<String>) {
    let id = "0b0e4f5a-6c7d-4e8f-9a0b-1c2d3e4f5a6b";
    let session_id = SessionId(Uuid(id.to_string()));
    let caller = Caller::new();
    let mut browser = MemoryBrowser::from_ports(
        ComponentPorts::with_caller(caller.clone()),
        surf_session::fetch,
    );
    let window_id = match browser.shell_mut().open_window(OpenWindow {}).unwrap() {
        OpenWindowOutcome::Opened { window_opened } => window_opened.window_id,
    };
    browser.run().unwrap();
    let tab_id = match browser
        .shell_mut()
        .open_tab(OpenTab {
            window_id: window_id.clone(),
        })
        .unwrap()
    {
        OpenTabOutcome::Opened { tab_opened } => tab_opened.tab_id,
        other => panic!("{other:?}"),
    };
    browser.run().unwrap();
    caller.set(None);
    browser
        .agent_mut()
        .open_session(OpenSession {
            session_id: session_id.clone(),
            window_id,
            tab_id,
        })
        .unwrap();
    browser
        .agent_mut()
        .record_action(RecordAction {
            session_id: session_id.clone(),
            sequence: 1,
            command: ActionCommand::Open,
            url: None,
            handle: None,
            label: None,
        })
        .unwrap();
    browser.run().unwrap();
    let after_open = browser.system().published().len();
    caller.set(Some(session_id.clone()));
    browser
        .agent_mut()
        .go(Go {
            session_id: session_id.clone(),
            url: Url(url.to_string()),
        })
        .unwrap();
    browser
        .agent_mut()
        .record_action(RecordAction {
            session_id,
            sequence: 2,
            command: ActionCommand::Go,
            url: Some(Url(url.to_string())),
            handle: None,
            label: None,
        })
        .unwrap();
    browser.run().unwrap();
    let published: Vec<String> = browser
        .system()
        .published()
        .iter()
        .map(|event| event.name().to_string())
        .collect();
    (
        published[..after_open].to_vec(),
        published[after_open..].to_vec(),
    )
}

// --- The cases -----------------------------------------------------------------------------------

#[test]
fn case_01_entries_follow_the_actions_in_the_order_the_system_published() {
    let dir = TestDir::new();
    let pages = PageServer::start();
    let daemon = surfd_in_process(&dir, |_| {});
    let url = pages.url("/a");
    let (reference_open, reference_go) = reference_names(&url);

    let session = wire_open(&dir, None);
    let open = kept(&daemon, &session);
    assert_eq!(open.len(), OPEN_ENTRIES, "the entries of open: {open:#?}");
    go(&dir, &session, &url);
    let entries = kept(&daemon, &session);
    assert_eq!(
        entries.len(),
        OPEN_ENTRIES + GO_ENTRIES,
        "the entries of open and go: {entries:#?}"
    );
    assert_eq!(&entries[..OPEN_ENTRIES], open.as_slice(), "open, unchanged");

    let first = action_at(&entries, 0);
    assert_eq!(first.len(), OPEN_ENTRIES);
    assert_eq!(first[0].text, "action 1 open\n");
    assert_eq!(names(first), reference_open, "the events of open");
    assert_eq!(first.last().unwrap().kind, EntryKind::Answer);

    let second = action_at(&entries, OPEN_ENTRIES);
    assert_eq!(second.len(), GO_ENTRIES);
    assert_eq!(
        second[0].text,
        format!("action 2 go {}\n", surf_answer::quote(&url))
    );
    assert_eq!(names(second), reference_go, "the events of go");
    assert_eq!(second.last().unwrap().kind, EntryKind::Answer);

    let indices: Vec<u64> = entries.iter().map(|entry| entry.index).collect();
    let expected: Vec<u64> = (1..=entries.len() as u64).collect();
    assert_eq!(indices, expected, "per-session numbers from 1");
    assert!(entries.iter().all(|entry| entry.session == session.id));

    // `open <url>`: action open, its events, action go, its events, one answer entry.
    let other = wire_open(&dir, Some(&url));
    let entries = kept(&daemon, &other);
    assert_eq!(entries.len(), OPEN_ENTRIES + GO_ENTRIES - 1, "{entries:#?}");
    let open = action_at(&entries, 0);
    assert_eq!(open[0].text, "action 1 open\n");
    assert_eq!(open.len(), OPEN_ENTRIES - 1, "no answer entry after open");
    assert_eq!(names(open), reference_open);
    let go = action_at(&entries, OPEN_ENTRIES - 1);
    assert_eq!(
        go[0].text,
        format!("action 2 go {}\n", surf_answer::quote(&url))
    );
    assert_eq!(names(go), reference_go);
    assert_eq!(go.last().unwrap().kind, EntryKind::Answer);
    assert_eq!(
        entries
            .iter()
            .filter(|entry| entry.kind == EntryKind::Answer)
            .count(),
        1
    );
    let indices: Vec<u64> = entries.iter().map(|entry| entry.index).collect();
    assert_eq!(indices, (1..=entries.len() as u64).collect::<Vec<_>>());
}

#[test]
fn case_02_the_answer_entry_is_the_lines_the_client_printed() {
    let dir = TestDir::new();
    let pages = PageServer::start();
    let daemon = surfd_in_process(&dir, |_| {});
    let session = wire_open(&dir, None);
    let url = pages.url("/a");
    let printed = go(&dir, &session, &url);
    let entries = kept(&daemon, &session);
    let answer = entries.last().unwrap();
    assert_eq!(answer.kind, EntryKind::Answer);
    assert_eq!(answer.text, printed, "byte for byte");
    let parsed = surf_answer::parse(&answer.text).expect("the answer entry parses");
    assert_eq!(
        parsed.page,
        Some(Page::Loaded {
            status: 200,
            media_type: "text/html".to_string(),
            address: url,
            title: None,
        })
    );
}

#[test]
fn token_in_no_entry() {
    let dir = TestDir::new();
    let pages = PageServer::start();
    let daemon = surfd_in_process(&dir, |_| {});
    let sessions = [wire_open(&dir, None), wire_open(&dir, None)];
    for session in &sessions {
        let entries = kept(&daemon, session);
        let answer = entries.last().unwrap();
        assert_eq!(answer.kind, EntryKind::Answer);
        assert!(session.text.contains("\ntoken "), "{}", session.text);
        assert_eq!(
            answer.text,
            without_token(&session.text),
            "every line the client printed but the token line"
        );
    }
    for session in &sessions {
        go(&dir, session, &pages.url("/a"));
        assert_eq!(wire_as(&dir, session, "look", &[]).status, 0);
        assert_eq!(wire_as(&dir, session, "close", &[]).status, 0);
    }
    for session in &sessions {
        assert!(daemon.wait_sessions(Duration::from_secs(10), |list| {
            list.get(&session.id)
                .is_some_and(|listed| listed.state == State::Closed)
        }));
    }
    for session in &sessions {
        let entries = kept(&daemon, session);
        assert!(entries.len() > OPEN_ENTRIES + GO_ENTRIES, "{entries:#?}");
        for entry in &entries {
            for holder in &sessions {
                assert!(
                    !entry.text.contains(&holder.token),
                    "a token is in an entry of {}: {entry:?}",
                    session.id
                );
            }
        }
    }
}

#[test]
fn case_04_two_sessions_at_once_keep_their_own_entries() {
    let dir = TestDir::new();
    let pages = PageServer::start();
    let daemon = surfd_in_process(&dir, |_| {});
    let a = wire_open(&dir, None);
    let b = wire_open(&dir, None);
    thread::scope(|scope| {
        for (session, path) in [(&a, "/a"), (&b, "/b")] {
            let dir = &dir;
            let url = pages.url(path);
            scope.spawn(move || go(dir, session, &url));
        }
    });
    let mut numbers = Vec::new();
    for (session, other, path) in [(&a, &b, "/a"), (&b, &a, "/b")] {
        let entries = kept(&daemon, session);
        assert_eq!(entries.len(), OPEN_ENTRIES + GO_ENTRIES);
        let own = surf_answer::quote(&session.id);
        for entry in &entries {
            assert_eq!(entry.session, session.id);
            assert!(!entry.text.contains(&other.id), "{entry:?}");
            if entry.kind == EntryKind::Event
                && let Some(value) = fields(entry.text.trim_end())
                    .unwrap()
                    .into_iter()
                    .find(|(name, _)| name == "session_id")
                    .and_then(|(_, value)| value)
            {
                assert_eq!(value, own, "{entry:?}");
            }
        }
        assert!(
            entries[OPEN_ENTRIES]
                .text
                .contains(&surf_answer::quote(&pages.url(path)))
        );
        let own_numbers: Vec<u64> = entries.iter().map(|entry| entry.number).collect();
        assert!(
            own_numbers.windows(2).all(|pair| pair[0] < pair[1]),
            "{own_numbers:?}"
        );
        numbers.extend(own_numbers);
    }
    let unique: BTreeSet<u64> = numbers.iter().copied().collect();
    assert_eq!(
        unique.len(),
        numbers.len(),
        "a number used twice: {numbers:?}"
    );
}

#[test]
fn case_05_look_adds_no_entry() {
    let dir = TestDir::new();
    let daemon = surfd_in_process(&dir, |_| {});
    let session = wire_open(&dir, None);
    let before = kept(&daemon, &session);
    assert_eq!(before.len(), OPEN_ENTRIES);
    let newest = daemon.newest_entry();
    assert_eq!(wire_as(&dir, &session, "look", &[]).status, 0);
    assert_eq!(kept(&daemon, &session), before);
    assert_eq!(daemon.newest_entry(), newest);

    // The daemon leaked, so that only the drop of the directory, which no helper marked, can
    // stop the session: it finds it among the test process's descendants.
    let pid = daemon
        .sessions()
        .get(&session.id)
        .expect("the session is listed")
        .pid;
    std::mem::forget(daemon);
    drop(dir);
    assert!(
        !surf_test_process::alive(pid),
        "the session {pid} runs after the drop"
    );
}

/// `text` with each UUID in it replaced by `#<n>`, n the order of its first appearance in
/// `seen`, which the calls of one session share.
fn numbered(text: &str, seen: &mut Vec<String>) -> String {
    let is_uuid = |word: &[u8]| {
        word.len() == 36
            && word.iter().enumerate().all(|(at, b)| match at {
                8 | 13 | 18 | 23 => *b == b'-',
                _ => b.is_ascii_digit() || (b'a'..=b'f').contains(b),
            })
    };
    let bytes = text.as_bytes();
    let mut out = String::new();
    let mut at = 0;
    while at < bytes.len() {
        if at + 36 <= bytes.len() && is_uuid(&bytes[at..at + 36]) {
            let uuid = &text[at..at + 36];
            let n = match seen.iter().position(|known| known == uuid) {
                Some(n) => n,
                None => {
                    seen.push(uuid.to_string());
                    seen.len() - 1
                }
            };
            out.push_str(&format!("#{n}"));
            at += 36;
        } else {
            let c = text[at..].chars().next().unwrap();
            out.push(c);
            at += c.len_utf8();
        }
    }
    out
}

/// Case 10 of `story:page-text-look`: a call with a form records what the same call records
/// without it.
#[test]
fn a_call_with_a_form_records_the_same() {
    let dir = TestDir::new();
    let pages = PageServer::start();
    let daemon = surfd_in_process(&dir, |_| {});
    let (a, b) = (pages.url("/a"), pages.url("/b"));
    let mut paths = Vec::new();
    let mut sessions = Vec::new();
    for with_form in [true, false] {
        let form = |value: &'static str| -> Vec<(&'static str, &'static str)> {
            if with_form {
                vec![("form", value)]
            } else {
                Vec::new()
            }
        };
        let mut args = vec![("url", a.as_str())];
        args.extend(form("tree"));
        let answer = common::wire_call(&dir, &common::request("open", None, None, &args));
        assert_eq!(answer.status, 0, "{answer:?}");
        let session = common::opened(&answer.text);
        let mut args = vec![("url", b.as_str())];
        args.extend(form("markdown"));
        let answer = wire_as(&dir, &session, "go", &args);
        assert_eq!(answer.status, 0, "{answer:?}");
        let answer = wire_as(&dir, &session, "look", &form("tree"));
        assert_eq!(answer.status, 0, "{answer:?}");
        assert_eq!(answer.text.contains("\nnote text: no-css\n"), with_form);
        paths.push(common::path_block(&answer.text));
        sessions.push(session);
    }
    assert_eq!(paths[0], paths[1], "the path lines");
    let with = kept(&daemon, &sessions[0]);
    let without = kept(&daemon, &sessions[1]);
    assert_eq!(with.len(), without.len(), "{with:#?}\n{without:#?}");
    let (mut seen_with, mut seen_without) = (Vec::new(), Vec::new());
    let mut answers = 0;
    for (one, other) in with.iter().zip(&without) {
        assert_eq!((one.index, one.kind), (other.index, other.kind));
        let mut text = one.text.clone();
        if one.kind == EntryKind::Answer {
            answers += 1;
            let at = text.find("note text: ").expect("a note text line");
            assert!(text[at..].contains("\ntext "), "{text}");
            text.truncate(at);
        }
        assert_eq!(
            numbered(&text, &mut seen_with),
            numbered(&other.text, &mut seen_without),
            "entry {}",
            one.index
        );
    }
    assert_eq!(answers, 2, "the answer entries of open and go");
}

#[test]
fn case_06_a_refused_command_adds_no_entry() {
    let dir = TestDir::new();
    let pages = PageServer::start();
    let daemon = surfd_in_process(&dir, |_| {});
    let session = wire_open(&dir, None);
    let other = wire_open(&dir, None);
    let before = kept(&daemon, &session);
    assert_eq!(before.len(), OPEN_ENTRIES);
    let newest = daemon.newest_entry();
    assert_eq!(newest, 2 * OPEN_ENTRIES as u64);
    let wrong = Opened {
        id: session.id.clone(),
        token: other.token.clone(),
        text: String::new(),
    };
    for (command, args) in [
        ("go", vec![("url", pages.url("/a"))]),
        ("close", vec![]),
        ("look", vec![]),
    ] {
        let args: Vec<(&str, &str)> = args.iter().map(|(n, v)| (*n, v.as_str())).collect();
        let answer = wire_as(&dir, &wrong, command, &args);
        assert_eq!(answer.status, 1, "{command}: {answer:?}");
        assert!(answer.text.ends_with("did refused token\n"), "{answer:?}");
    }
    assert_eq!(kept(&daemon, &session), before);
    assert_eq!(daemon.newest_entry(), newest);
}

/// A page of `size` bytes: an HTML start, then upper-case letters from G to Z, none of which a
/// UUID, an address of the page server or an event name holds eight in a row.
fn letters_page(size: usize) -> Vec<u8> {
    let mut body = b"<!doctype html><p>".to_vec();
    let mut state: u32 = 0x9e37_79b9;
    while body.len() < size {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        body.push(b'G' + (state % 20) as u8);
    }
    body
}

#[test]
fn case_07_a_body_is_kept_as_its_length() {
    let body = letters_page(5000);
    assert_eq!(body.len(), 5000);
    let dir = TestDir::new();
    let pages = PageServer::with_body(body.clone());
    let daemon = surfd_in_process(&dir, |_| {});
    let session = wire_open(&dir, None);
    go(&dir, &session, &pages.url("/a"));
    let entries = kept(&daemon, &session);
    for name in [
        "surf.net.ResponseReceived",
        "surf.navigation.NavigationCommitted",
    ] {
        let found: Vec<&Entry> = entries
            .iter()
            .filter(|entry| entry.kind == EntryKind::Event && event_name(entry) == name)
            .collect();
        assert_eq!(found.len(), 1, "{name}");
        assert_eq!(
            field(found[0], "body"),
            Some(surf_answer::quote("5000 bytes")),
            "{name}"
        );
    }
    // Every 8 bytes of the page's letters, put in a set once; each window of each entry is
    // looked up in it.
    let letters: std::collections::BTreeSet<&[u8]> = body[18..].windows(8).collect();
    for entry in &entries {
        assert!(
            !entry
                .text
                .as_bytes()
                .windows(8)
                .any(|part| letters.contains(part)),
            "an entry holds bytes of the page: {entry:?}"
        );
    }
}

#[test]
fn case_08_a_page_of_6_mib_leaves_the_session_open() {
    let body = letters_page(6 * 1024 * 1024);
    let dir = TestDir::new();
    let pages = PageServer::with_body(body);
    let daemon = surfd_in_process(&dir, |_| {});
    let session = wire_open(&dir, None);
    let text = go(&dir, &session, &pages.url("/big"));
    assert!(
        text.starts_with(&format!("surf 1 session {} open\n", session.id)),
        "{text}"
    );
    assert_eq!(
        daemon
            .sessions()
            .get(&session.id)
            .map(|listed| listed.state),
        Some(State::Open)
    );
    assert_eq!(wire_as(&dir, &session, "look", &[]).status, 0);
    let entries = kept(&daemon, &session);
    for name in [
        "surf.net.ResponseReceived",
        "surf.navigation.NavigationCommitted",
    ] {
        let entry = entries
            .iter()
            .find(|entry| entry.kind == EntryKind::Event && event_name(entry) == name)
            .unwrap_or_else(|| panic!("no {name}"));
        assert_eq!(
            field(entry, "body"),
            Some(surf_answer::quote("6291456 bytes"))
        );
    }
}

#[test]
fn case_12_entries_after_a_number() {
    let dir = TestDir::new();
    let pages = PageServer::start();
    let daemon = surfd_in_process(&dir, |_| {});
    let a = wire_open(&dir, None);
    let b = wire_open(&dir, None);
    go(&dir, &a, &pages.url("/a"));
    go(&dir, &b, &pages.url("/b"));
    let mut all: Vec<Entry> = kept(&daemon, &a);
    all.extend(kept(&daemon, &b));
    all.sort_by_key(|entry| entry.number);
    let second = all[1].number;
    assert_eq!(daemon.entries_after(second), Ok(all[2..].to_vec()));

    // The entries of one session bound to those of its `go`: entries 1 to 6, its `open`, go.
    let dir = TestDir::new();
    let daemon = surfd_in_process(&dir, |config| config.log.entries_per_session = GO_ENTRIES);
    let session = wire_open(&dir, None);
    go(&dir, &session, &pages.url("/a"));
    let log = daemon.session_entries(&session.id).unwrap();
    assert_eq!(log.dropped, OPEN_ENTRIES as u64);
    assert_eq!(log.entries[0].number, OPEN_ENTRIES as u64 + 1);
    assert_eq!(
        daemon.entries_after(OPEN_ENTRIES as u64),
        Ok(log.entries.clone())
    );
    assert_eq!(
        daemon.entries_after(OPEN_ENTRIES as u64 - 1),
        Err(OlderThanKept)
    );
}

#[test]
fn case_13_a_subscriber_receives_the_entries_of_a_go() {
    let dir = TestDir::new();
    let pages = PageServer::start();
    let daemon = surfd_in_process(&dir, |_| {});
    let session = wire_open(&dir, None);
    let last = kept(&daemon, &session).last().unwrap().number;
    let mut reader = daemon.subscribe_after(last).expect("a subscription");
    go(&dir, &session, &pages.url("/a"));
    let mut received = Vec::new();
    while received.len() < GO_ENTRIES {
        received.push(
            reader
                .next_within(Duration::from_secs(10))
                .expect("an entry of the go"),
        );
    }
    assert_eq!(
        reader.next_within(Duration::from_millis(200)),
        Err(RecvTimeoutError::Timeout)
    );
    assert_eq!(received, kept(&daemon, &session)[OPEN_ENTRIES..].to_vec());
}

#[test]
fn case_14_a_subscriber_that_never_reads_is_disconnected() {
    // The shipped queue is the story's number, not the crate's constant; the case runs with a
    // queue of 24 entries, so that 3 go pass it.
    assert_eq!(surfd::log::READER_QUEUE, 1024);
    const QUEUE: usize = 24;
    let dir = TestDir::new();
    let pages = PageServer::start();
    let daemon = surfd_in_process(&dir, |config| config.log.reader_queue = QUEUE);
    let session = wire_open(&dir, None);
    let start = daemon.newest_entry();
    let mut stalled = daemon.subscribe_after(start).unwrap();
    let gos = QUEUE / GO_ENTRIES + 1;
    for _ in 0..gos {
        go(&dir, &session, &pages.url("/a"));
    }
    assert!(
        daemon.newest_entry() - start > QUEUE as u64,
        "{gos} go commands kept {} entries",
        daemon.newest_entry() - start
    );
    let mut read = 0;
    loop {
        match stalled.next_within(Duration::from_secs(1)) {
            Ok(_) => read += 1,
            Err(RecvTimeoutError::Disconnected) => break,
            Err(RecvTimeoutError::Timeout) => panic!("not disconnected after {read} entries"),
        }
    }
    assert_eq!(read, QUEUE, "the queue held");

    let last = daemon.newest_entry();
    let mut reader = daemon.subscribe_after(last).unwrap();
    go(&dir, &session, &pages.url("/b"));
    let mut received = Vec::new();
    while received.len() < GO_ENTRIES {
        received.push(reader.next_within(Duration::from_secs(10)).unwrap());
    }
    assert_eq!(received.first().map(|entry| entry.number), Some(last + 1));
    assert_eq!(
        daemon
            .sessions()
            .get(&session.id)
            .map(|listed| listed.state),
        Some(State::Open)
    );
}

#[test]
fn case_15_an_address_is_kept_quoted_in_one_line() {
    let dir = TestDir::new();
    let pages = PageServer::start();
    let daemon = surfd_in_process(&dir, |_| {});
    let session = wire_open(&dir, None);
    let address = pages.url("/x\n\"\u{202e}y");
    let answer = wire_as(&dir, &session, "go", &[("url", &address)]);
    assert_eq!(answer.status, 0, "{answer:?}");
    let entries = kept(&daemon, &session);
    let started: Vec<&Entry> = entries
        .iter()
        .filter(|entry| {
            entry.kind == EntryKind::Event && event_name(entry) == "surf.net.RequestStarted"
        })
        .collect();
    assert_eq!(started.len(), 1);
    let entry = started[0];
    assert_eq!(entry.text.matches('\n').count(), 1, "one line: {entry:?}");
    assert!(entry.text.ends_with('\n'));
    let quoted = field(entry, "url").expect("a url");
    assert_eq!(quoted, surf_answer::quote(&address));
    assert!(
        quoted.contains("\\x0a") && quoted.contains("\\x22") && quoted.contains("\\xe2\\x80\\xae")
    );
    assert_eq!(surf_answer::unquote(&quoted).unwrap(), address);
}

#[test]
fn case_16_entries_stay_readable_after_close_and_after_a_kill() {
    let dir = TestDir::new();
    let pages = PageServer::start();
    let daemon = surfd_in_process(&dir, |_| {});
    let closed = wire_open(&dir, None);
    go(&dir, &closed, &pages.url("/a"));
    assert_eq!(wire_as(&dir, &closed, "close", &[]).status, 0);
    let before = kept(&daemon, &closed);
    assert!(before.len() > OPEN_ENTRIES + GO_ENTRIES, "{before:#?}");
    assert!(daemon.wait_sessions(Duration::from_secs(10), |list| {
        list.get(&closed.id)
            .is_some_and(|listed| listed.state == State::Closed)
    }));
    assert_eq!(kept(&daemon, &closed), before);

    let killed = wire_open(&dir, None);
    go(&dir, &killed, &pages.url("/b"));
    let before = kept(&daemon, &killed);
    assert_eq!(before.len(), OPEN_ENTRIES + GO_ENTRIES);
    let pid = daemon.sessions().get(&killed.id).unwrap().pid;
    common::signal_kill(pid);
    assert!(daemon.wait_sessions(Duration::from_secs(10), |list| {
        list.get(&killed.id)
            .is_some_and(|listed| listed.state == State::Died)
    }));
    assert_eq!(kept(&daemon, &killed), before);
}

// --- The mapping ---------------------------------------------------------------------------------

/// The events `spec/domains/*.yaml` declare: each qualified name and its fields, name and type,
/// in the order declared.
fn spec_events() -> BTreeMap<String, Vec<(String, String)>> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec/domains");
    let mut events = BTreeMap::new();
    for entry in fs::read_dir(&dir).unwrap() {
        let text = fs::read_to_string(entry.unwrap().path()).unwrap();
        let mut in_events = false;
        let mut current: Option<(String, Vec<(String, String)>)> = None;
        for line in text.lines() {
            if !line.is_empty() && !line.starts_with(' ') && !line.starts_with('#') {
                in_events = line == "events:";
                events.extend(current.take());
                continue;
            }
            if !in_events {
                continue;
            }
            if let Some(name) = line.strip_prefix("  - name: ") {
                events.extend(current.take());
                current = Some((name.trim().to_string(), Vec::new()));
            } else if let Some(name) = line.strip_prefix("      - name: ") {
                if let Some((_, fields)) = current.as_mut() {
                    fields.push((name.trim().to_string(), String::new()));
                }
            } else if let Some(kind) = line.strip_prefix("        type: ")
                && let Some((_, fields)) = current.as_mut()
                && let Some(last) = fields.last_mut()
            {
                last.1 = kind.trim().to_string();
            }
        }
        events.extend(current.take());
    }
    events
}

fn uuid(n: u8) -> Uuid {
    Uuid(format!("00000000-0000-4000-8000-0000000000{n:02x}"))
}

const BODY: &[u8] = b"q#~";
const ADDRESS: &str = "http://example.org/a b";
/// A title holding a double quote, a backslash and a character outside ASCII.
const TITLE: &str = "Say \"hi\" \\ caf\u{e9}";
/// A link label holding a double quote and a character outside ASCII (`agent-links-spec`).
const LABEL: &str = "Sign \"in\" \u{e9}";
/// The media type of a request body (`agent-request-body-spec`).
const FORM: &str = "application/x-www-form-urlencoded";
/// What an agent typed into a control (`agent-forms-spec`): its entry gives its length only.
const TYPED: &str = "s3cret";

/// Every `SystemEvent` variant this mapping covers, one value each; a story that maps an event
/// adds it here.
fn covered() -> Vec<SystemEvent> {
    let url = || Url(ADDRESS.to_string());
    vec![
        SystemEvent::ActionRecorded(ActionRecorded {
            action_id: ActionId(uuid(1)),
            session_id: SessionId(uuid(2)),
            sequence: 7,
            command: ActionCommand::Follow,
            url: None,
            handle: Some("l4".to_string()),
            label: Some(LABEL.to_string()),
        }),
        SystemEvent::NavigationRequested(NavigationRequested {
            session_id: SessionId(uuid(2)),
            tab_id: TabId(uuid(3)),
            url: url(),
            handle: Some("l4".to_string()),
            label: Some(LABEL.to_string()),
        }),
        SystemEvent::SessionClosed(SessionClosed {
            session_id: SessionId(uuid(2)),
            window_id: WindowId(uuid(4)),
            tab_id: TabId(uuid(3)),
        }),
        SystemEvent::SessionNavigationChanged(SessionNavigationChanged {
            navigation_id: RequestId(uuid(5)),
            state: NavigationState::Committed,
            sessions: 1,
        }),
        SystemEvent::SessionOpened(SessionOpened {
            session_id: SessionId(uuid(2)),
            window_id: WindowId(uuid(4)),
            tab_id: TabId(uuid(3)),
        }),
        SystemEvent::DocumentParsed(DocumentParsed {
            document_id: DocumentId(uuid(6)),
            url: url(),
            navigation_id: RequestId(uuid(5)),
            title: TITLE.to_string(),
            encoding: EncodingName("windows-1252".to_string()),
            unsupported_encoding: Some(EncodingName("EUC-KR".to_string())),
            stopped_by: Some(ParseLimit::Depth),
        }),
        SystemEvent::ElementAdded(ElementAdded {
            element_id: ElementId(uuid(8)),
            document_id: DocumentId(uuid(6)),
            position: 3,
        }),
        SystemEvent::ParsingStarted(ParsingStarted {
            document_id: DocumentId(uuid(6)),
            body: BODY.to_vec(),
        }),
        SystemEvent::NavigationCancelled(NavigationCancelled {
            navigation_id: RequestId(uuid(5)),
            tab_id: TabId(uuid(3)),
        }),
        SystemEvent::NavigationCommitted(NavigationCommitted {
            navigation_id: RequestId(uuid(5)),
            tab_id: TabId(uuid(3)),
            url: url(),
            status: Status(404),
            media_type: MediaType("text/html".to_string()),
            body: BODY.to_vec(),
        }),
        SystemEvent::NavigationFailed(NavigationFailed {
            navigation_id: RequestId(uuid(5)),
            tab_id: TabId(uuid(3)),
            url: url(),
            reason: FailureReason::TooManyRedirects,
        }),
        SystemEvent::NavigationStarted(NavigationStarted {
            navigation_id: RequestId(uuid(5)),
            tab_id: TabId(uuid(3)),
            url: url(),
            method: Method::Post,
            body: Some(BODY.to_vec()),
            body_type: Some(MediaType(FORM.to_string())),
        }),
        SystemEvent::FetchFailed(FetchFailed {
            request_id: RequestId(uuid(5)),
            url: url(),
            reason: FailureReason::Unreachable,
        }),
        SystemEvent::RequestStarted(RequestStarted {
            request_id: RequestId(uuid(5)),
            url: url(),
            method: Method::Post,
            body: Some(BODY.to_vec()),
            body_type: Some(MediaType(FORM.to_string())),
        }),
        SystemEvent::ResponseReceived(ResponseReceived {
            request_id: RequestId(uuid(5)),
            url: url(),
            status: Status(404),
            media_type: MediaType("text/html".to_string()),
            body: BODY.to_vec(),
        }),
        SystemEvent::FramePainted(FramePainted {
            frame_id: FrameId(uuid(7)),
            document_id: DocumentId(uuid(6)),
            width: 800,
            height: 600,
        }),
        SystemEvent::TabClosed(TabClosed {
            tab_id: TabId(uuid(3)),
            window_id: WindowId(uuid(4)),
        }),
        SystemEvent::TabOpened(TabOpened {
            tab_id: TabId(uuid(3)),
            window_id: WindowId(uuid(4)),
        }),
        SystemEvent::WindowOpened(WindowOpened {
            window_id: WindowId(uuid(4)),
        }),
        SystemEvent::FillRequested(FillRequested {
            session_id: SessionId(uuid(2)),
            element_id: ElementId(uuid(8)),
            value: TYPED.to_string(),
            handle: Some("l4".to_string()),
        }),
        SystemEvent::PickRequested(PickRequested {
            session_id: SessionId(uuid(2)),
            element_id: ElementId(uuid(8)),
            option_id: Some(OptionId(uuid(10))),
            handle: Some("l4".to_string()),
        }),
        SystemEvent::PressRequested(PressRequested {
            session_id: SessionId(uuid(2)),
            element_id: ElementId(uuid(8)),
            handle: Some("l4".to_string()),
        }),
        SystemEvent::FormAdded(FormAdded {
            form_id: FormId(uuid(9)),
            document_id: DocumentId(uuid(6)),
            position: 3,
        }),
        SystemEvent::OptionAdded(OptionAdded {
            option_id: OptionId(uuid(10)),
            select_id: ElementId(uuid(8)),
            position: 3,
        }),
        SystemEvent::ValueSet(ValueSet {
            element_id: ElementId(uuid(8)),
        }),
        SystemEvent::CheckednessSet(CheckednessSet {
            element_id: ElementId(uuid(8)),
            checked: true,
        }),
        SystemEvent::OptionSelected(OptionSelected {
            option_id: OptionId(uuid(10)),
            select_id: ElementId(uuid(8)),
        }),
        SystemEvent::SubmissionStarted(SubmissionStarted {
            submission_id: SubmissionId(uuid(11)),
            submitter: ElementId(uuid(8)),
        }),
        SystemEvent::FormSubmitted(FormSubmitted {
            submission_id: SubmissionId(uuid(11)),
            tab_id: TabId(uuid(3)),
            url: url(),
            method: Method::Post,
            body: Some(BODY.to_vec()),
            body_type: Some(MediaType(FORM.to_string())),
        }),
        SystemEvent::SubmissionAbandoned(SubmissionAbandoned {
            submission_id: SubmissionId(uuid(11)),
            reason: SubmissionEndReason::InvalidControls,
        }),
    ]
}

/// The value each field of [`covered`] maps to, read back through `surf_answer::unquote`; `None`
/// for an absent optional value.
fn expected_value(event: &str, field: &str) -> Option<String> {
    let text = |s: &str| Some(s.to_string());
    let id = |n: u8| Some(uuid(n).0);
    match (event, field) {
        (_, "action_id") => id(1),
        (_, "session_id") => id(2),
        (_, "tab_id") => id(3),
        (_, "window_id") => id(4),
        (_, "navigation_id" | "request_id") => id(5),
        (_, "document_id") => id(6),
        (_, "frame_id") => id(7),
        (_, "element_id" | "select_id" | "submitter") => id(8),
        (_, "form_id") => id(9),
        (_, "option_id") => id(10),
        (_, "submission_id") => id(11),
        (_, "position") => text("3"),
        ("surf.document.CheckednessSet", "checked") => text("true"),
        ("surf.document.SubmissionAbandoned", "reason") => text("InvalidControls"),
        ("surf.agent.FillRequested", "value") => text("6 bytes"),
        ("surf.document.DocumentParsed", "title") => text(TITLE),
        ("surf.document.DocumentParsed", "encoding") => text("windows-1252"),
        ("surf.document.DocumentParsed", "unsupported_encoding") => text("EUC-KR"),
        ("surf.document.DocumentParsed", "stopped_by") => text("Depth"),
        ("surf.agent.ActionRecorded", "sequence") => text("7"),
        ("surf.agent.ActionRecorded", "command") => text("follow"),
        (_, "handle") => text("l4"),
        (_, "label") => text(LABEL),
        ("surf.agent.ActionRecorded", "url") => None,
        (_, "url") => text(ADDRESS),
        ("surf.agent.SessionNavigationChanged", "state") => text("Committed"),
        ("surf.agent.SessionNavigationChanged", "sessions") => text("1"),
        ("surf.navigation.NavigationFailed", "reason") => text("TooManyRedirects"),
        ("surf.net.FetchFailed", "reason") => text("Unreachable"),
        (_, "status") => text("404"),
        (_, "media_type") => text("text/html"),
        (_, "method") => text("Post"),
        (_, "body_type") => text(FORM),
        (_, "body") => text("3 bytes"),
        (_, "width") => text("800"),
        (_, "height") => text("600"),
        _ => panic!("no expected value for {event}.{field}"),
    }
}

#[test]
fn case_17_every_event_maps_with_all_its_fields() {
    let spec = spec_events();
    let covered = covered();
    let names: BTreeSet<String> = covered
        .iter()
        .map(|event| event.name().to_string())
        .collect();
    assert_eq!(names.len(), covered.len(), "a variant listed twice");
    // No command publishes `surf.agent.EventsLeftOut`: the session writes it
    // (`entries::left_out_line`), and `spec/` declares its name and field.
    let left_out = "surf.agent.EventsLeftOut";
    assert_eq!(
        names,
        spec.keys()
            .filter(|name| *name != left_out)
            .cloned()
            .collect::<BTreeSet<_>>(),
        "the variants covered and the events of spec/"
    );
    assert_eq!(
        names.len(),
        30,
        "the events of spec/ after story:agent-forms-spec"
    );
    assert_eq!(spec.len(), 31, "and surf.agent.EventsLeftOut");
    let line = surf_session::entries::left_out_line(3);
    assert!(line.starts_with(&format!("event {left_out} ")), "{line}");
    let mapped = fields(&line).expect("a mapped event");
    let declared: Vec<&str> = spec[left_out].iter().map(|(f, _)| f.as_str()).collect();
    let written: Vec<&str> = mapped.iter().map(|(f, _)| f.as_str()).collect();
    assert_eq!(written, declared, "the fields of {left_out}");
    assert_eq!(mapped[0].1.as_deref(), Some("\"3\""));
    assert!(surfd::link::Entry::line(&line).is_some(), "{line}");
    for field in [
        ("surf.net.ResponseReceived", "url"),
        ("surf.navigation.NavigationCommitted", "url"),
    ] {
        assert!(
            spec[field.0].iter().any(|(name, _)| name == field.1),
            "{field:?} of agent-redirect-spec"
        );
    }
    for event in [
        "surf.navigation.NavigationStarted",
        "surf.net.RequestStarted",
    ] {
        for field in ["method", "body", "body_type"] {
            assert!(
                spec[event].iter().any(|(name, _)| name == field),
                "{event}.{field} of agent-request-body-spec"
            );
        }
    }
    for event in &covered {
        let line = surf_session::entries::event_line(event);
        assert!(!line.contains(TYPED), "a typed value in an entry: {line}");
    }
    for event in &covered {
        let name = event.name();
        let line = surf_session::entries::event_line(event);
        assert!(line.starts_with(&format!("event {name}")), "{line}");
        let mapped = fields(&line).unwrap_or_else(|| panic!("{name} is unmapped: {line}"));
        let declared: Vec<&str> = spec[name].iter().map(|(field, _)| field.as_str()).collect();
        let written: Vec<&str> = mapped.iter().map(|(field, _)| field.as_str()).collect();
        assert_eq!(
            written, declared,
            "the fields of {name} in the order of spec/"
        );
        for (field, value) in &mapped {
            let read = value
                .as_ref()
                .map(|quoted| surf_answer::unquote(quoted).expect("a quoted field"));
            assert_eq!(read, expected_value(name, field), "{name}.{field}: {line}");
        }
        assert!(
            surfd::link::Entry::line(&line).is_some(),
            "an event line of the link: {line}"
        );
    }
    // A `Get` without a body: the method, then `body` and `body_type` with no value.
    let pairs = |list: &[(&str, Option<&str>)]| -> Vec<(String, Option<String>)> {
        list.iter()
            .map(|(name, value)| (name.to_string(), value.map(str::to_string)))
            .collect()
    };
    let get = SystemEvent::NavigationStarted(NavigationStarted {
        navigation_id: RequestId(uuid(5)),
        tab_id: TabId(uuid(3)),
        url: Url(ADDRESS.to_string()),
        method: Method::Get,
        body: None,
        body_type: None,
    });
    assert_eq!(
        read_back(&get),
        pairs(&[
            ("navigation_id", Some(uuid(5).0.as_str())),
            ("tab_id", Some(uuid(3).0.as_str())),
            ("url", Some(ADDRESS)),
            ("method", Some("Get")),
            ("body", None),
            ("body_type", None),
        ])
    );
    let get = SystemEvent::RequestStarted(RequestStarted {
        request_id: RequestId(uuid(5)),
        url: Url(ADDRESS.to_string()),
        method: Method::Get,
        body: None,
        body_type: None,
    });
    assert_eq!(
        read_back(&get),
        pairs(&[
            ("request_id", Some(uuid(5).0.as_str())),
            ("url", Some(ADDRESS)),
            ("method", Some("Get")),
            ("body", None),
            ("body_type", None),
        ])
    );
}

#[test]
fn byte_fields_are_lengths() {
    // A story that adds a field of type Bytes or Optional<Bytes> adds it here.
    const BYTES_FIELDS: [&str; 6] = [
        "surf.net.ResponseReceived.body",
        "surf.navigation.NavigationCommitted.body",
        "surf.document.ParsingStarted.body",
        "surf.navigation.NavigationStarted.body",
        "surf.net.RequestStarted.body",
        "surf.document.FormSubmitted.body",
    ];
    let bytes = |kind: &str| kind == "Bytes" || kind == "Optional<Bytes>";
    let spec = spec_events();
    let covered = covered();
    let mut declared: Vec<String> = Vec::new();
    for event in &covered {
        for (field, kind) in &spec[event.name()] {
            if bytes(kind) {
                declared.push(format!("{}.{field}", event.name()));
            }
        }
    }
    declared.sort();
    let mut listed: Vec<String> = BYTES_FIELDS.iter().map(|f| f.to_string()).collect();
    listed.sort();
    assert_eq!(declared, listed, "the fields of type Bytes");
    for event in &covered {
        let line = surf_session::entries::event_line(event);
        for (field, kind) in &spec[event.name()] {
            if !bytes(kind) {
                continue;
            }
            let mapped = fields(&line).unwrap_or_else(|| panic!("unmapped: {line}"));
            let value = mapped
                .iter()
                .find(|(name, _)| name == field)
                .and_then(|(_, value)| value.clone())
                .unwrap_or_else(|| panic!("no {field}: {line}"));
            assert_eq!(value, surf_answer::quote("3 bytes"), "{line}");
            assert!(!line.contains("q#~"), "the bytes are in {line}");
        }
    }
}

/// The fields of the event line of `event`, each read back through `surf_answer::unquote`.
fn read_back(event: &SystemEvent) -> Vec<(String, Option<String>)> {
    let line = surf_session::entries::event_line(event);
    assert!(
        surfd::link::Entry::line(&line).is_some(),
        "an event line of the link: {line}"
    );
    fields(&line)
        .unwrap_or_else(|| panic!("unmapped: {line}"))
        .into_iter()
        .map(|(name, value)| {
            let read = value.map(|quoted| surf_answer::unquote(&quoted).expect("a quoted field"));
            (name, read)
        })
        .collect()
}

/// Named case of `story:html-document-spec`: the three events of the document, each field read
/// back as written, the optional ones present and absent.
#[test]
fn document_events_map_with_their_fields() {
    let pairs = |list: &[(&str, Option<&str>)]| -> Vec<(String, Option<String>)> {
        list.iter()
            .map(|(name, value)| (name.to_string(), value.map(str::to_string)))
            .collect()
    };
    let document = "6a1b2c3d-0000-4000-8000-000000000001";
    let navigation = "6a1b2c3d-0000-4000-8000-000000000002";
    let element = "6a1b2c3d-0000-4000-8000-000000000003";
    let id = |text: &str| Uuid(text.to_string());

    let started = SystemEvent::ParsingStarted(ParsingStarted {
        document_id: DocumentId(id(document)),
        body: b"<title>x</title>".to_vec(),
    });
    assert_eq!(
        read_back(&started),
        pairs(&[("document_id", Some(document)), ("body", Some("16 bytes"))])
    );

    let added = SystemEvent::ElementAdded(ElementAdded {
        element_id: ElementId(id(element)),
        document_id: DocumentId(id(document)),
        position: 11,
    });
    assert_eq!(
        read_back(&added),
        pairs(&[
            ("element_id", Some(element)),
            ("document_id", Some(document)),
            ("position", Some("11")),
        ])
    );

    let parsed = |title: &str, unsupported: Option<&str>, stopped_by: Option<ParseLimit>| {
        SystemEvent::DocumentParsed(DocumentParsed {
            document_id: DocumentId(id(document)),
            url: Url(ADDRESS.to_string()),
            navigation_id: RequestId(id(navigation)),
            title: title.to_string(),
            encoding: EncodingName("UTF-8".to_string()),
            unsupported_encoding: unsupported.map(|name| EncodingName(name.to_string())),
            stopped_by,
        })
    };
    let line = surf_session::entries::event_line(&parsed(TITLE, None, None));
    assert!(
        line.contains(&surf_answer::quote(TITLE)) && !line.contains(TITLE),
        "the title is quoted: {line}"
    );
    assert_eq!(
        read_back(&parsed(TITLE, Some("EUC-KR"), Some(ParseLimit::Depth))),
        pairs(&[
            ("document_id", Some(document)),
            ("url", Some(ADDRESS)),
            ("navigation_id", Some(navigation)),
            ("title", Some(TITLE)),
            ("encoding", Some("UTF-8")),
            ("unsupported_encoding", Some("EUC-KR")),
            ("stopped_by", Some("Depth")),
        ])
    );
    assert_eq!(
        read_back(&parsed("", None, None)),
        pairs(&[
            ("document_id", Some(document)),
            ("url", Some(ADDRESS)),
            ("navigation_id", Some(navigation)),
            ("title", Some("")),
            ("encoding", Some("UTF-8")),
            ("unsupported_encoding", None),
            ("stopped_by", None),
        ])
    );
}

/// A label of 10,000 bytes from a page: a line feed, a `"` and a bidirectional control among
/// its first 100 bytes, and a two-byte character across byte 1,024, so the cut falls at 1,023.
fn long_label() -> String {
    let mut label = String::from("Sign\n\"in\"\u{202e}");
    while label.len() < 1023 {
        label.push('a');
    }
    label.push('\u{e9}');
    while label.len() < 10_000 {
        label.push('b');
    }
    assert_eq!(label.len(), 10_000);
    label
}

/// The `label` field of the event line of `event` and the field after it, read back.
fn label_and_next(event: &SystemEvent) -> (Option<String>, Option<(String, Option<String>)>) {
    let read = read_back(event);
    let at = read
        .iter()
        .position(|(name, _)| name == "label")
        .unwrap_or_else(|| panic!("no label: {read:?}"));
    (read[at].1.clone(), read.get(at + 1).cloned())
}

/// The event line of `event`, whose label is [`long_label`]: `label` quotes its first 1,023
/// bytes, the last character boundary at or below 1,024, and `label_bytes="10000"` follows it;
/// no line feed or bidirectional control is written as itself.
fn assert_cut(event: &SystemEvent, label: &str) {
    let (written, next) = label_and_next(event);
    let written = written.expect("a label");
    assert_eq!(written.len(), 1023, "the label's length after the cut");
    assert!(
        written == label[..1023],
        "the label is the first 1,023 bytes"
    );
    assert_eq!(
        next,
        Some(("label_bytes".to_string(), Some("10000".to_string())))
    );
    let line = surf_session::entries::event_line(event);
    assert!(
        !line.contains('\n') && !line.contains('\u{202e}'),
        "a raw line feed or bidirectional control"
    );
}

/// Story values-and-labels-before-fill, acceptance 3: the label of `ActionRecorded` is cut at
/// `TITLE_ENTRY_BYTES` with `label_bytes` after it, as a title is.
#[test]
fn a_recorded_label_is_cut_at_the_entry_bound() {
    let max = surf_session::entries::TITLE_ENTRY_BYTES;
    assert_eq!(max, 1024);
    let recorded = |label: &str| {
        SystemEvent::ActionRecorded(ActionRecorded {
            action_id: ActionId(uuid(1)),
            session_id: SessionId(uuid(2)),
            sequence: 7,
            command: ActionCommand::Follow,
            url: None,
            handle: Some("l4".to_string()),
            label: Some(label.to_string()),
        })
    };
    let label = long_label();
    assert_cut(&recorded(&label), &label);

    let whole = "c".repeat(max);
    let (written, next) = label_and_next(&recorded(&whole));
    assert_eq!(written.as_deref(), Some(whole.as_str()));
    assert_eq!(next, None);
    assert!(
        !surf_session::entries::event_line(&recorded(&whole)).contains("label_bytes"),
        "a whole label has no label_bytes"
    );

    // The label a `Follow` carries in its `NavigationRequested`, cut the same way.
    let requested = SystemEvent::NavigationRequested(NavigationRequested {
        session_id: SessionId(uuid(2)),
        tab_id: TabId(uuid(3)),
        url: Url(ADDRESS.to_string()),
        handle: Some("l4".to_string()),
        label: Some(label.clone()),
    });
    assert_cut(&requested, &label);
}

/// The field `name` of the event line of `event` and the field after it, read back.
fn field_and_next(
    event: &SystemEvent,
    name: &str,
) -> (Option<String>, Option<(String, Option<String>)>) {
    let read = read_back(event);
    let at = read
        .iter()
        .position(|(field, _)| field == name)
        .unwrap_or_else(|| panic!("no {name}: {read:?}"));
    (read[at].1.clone(), read.get(at + 1).cloned())
}

/// The adversary's finding on story values-and-labels-before-fill: a client supplies the handle
/// of a `follow`, `fill`, `pick` or `press`, up to the 64 KiB of a request, and the entry carries
/// it. A handle of 10,000 bytes ([`long_label`]) is cut as a label is: `handle` quotes its first
/// 1,023 bytes and `handle_bytes="10000"` follows it, so the line is at most the line of a
/// two-byte handle, one quoted cut field (4 × 1,024 + 2 bytes) and its `handle_bytes`. A handle of
/// 1,024 bytes is whole and has no `handle_bytes`.
fn assert_handle_cut(event_with: impl Fn(&str) -> SystemEvent) {
    let max = surf_session::entries::TITLE_ENTRY_BYTES;
    let handle = long_label();
    let long = event_with(&handle);
    let (written, next) = field_and_next(&long, "handle");
    let written = written.expect("a handle");
    assert_eq!(written.len(), 1023, "the handle's length after the cut");
    assert!(
        written == handle[..1023],
        "the handle is the first 1,023 bytes"
    );
    assert_eq!(
        next,
        Some(("handle_bytes".to_string(), Some("10000".to_string())))
    );
    let line = surf_session::entries::event_line(&long);
    let short = surf_session::entries::event_line(&event_with("l4"));
    let bound = short.len() + 4 * max + 2 + r#" handle_bytes="10000""#.len();
    assert!(
        line.len() <= bound,
        "an entry of {} bytes over its bound of {bound}",
        line.len()
    );
    assert!(
        !line.contains('\n') && !line.contains('\u{202e}'),
        "a raw line feed or bidirectional control"
    );

    let whole = "c".repeat(max);
    let (written, _) = field_and_next(&event_with(&whole), "handle");
    assert_eq!(written.as_deref(), Some(whole.as_str()));
    assert!(
        !surf_session::entries::event_line(&event_with(&whole)).contains("handle_bytes"),
        "a whole handle has no handle_bytes"
    );
}

#[test]
fn a_recorded_handle_is_cut_at_the_entry_bound() {
    assert_handle_cut(|handle| {
        SystemEvent::ActionRecorded(ActionRecorded {
            action_id: ActionId(uuid(1)),
            session_id: SessionId(uuid(2)),
            sequence: 7,
            command: ActionCommand::Follow,
            url: None,
            handle: Some(handle.to_string()),
            label: Some(LABEL.to_string()),
        })
    });
}

#[test]
fn a_requested_navigations_handle_is_cut_at_the_entry_bound() {
    assert_handle_cut(|handle| {
        SystemEvent::NavigationRequested(NavigationRequested {
            session_id: SessionId(uuid(2)),
            tab_id: TabId(uuid(3)),
            url: Url(ADDRESS.to_string()),
            handle: Some(handle.to_string()),
            label: Some(LABEL.to_string()),
        })
    });
}

#[test]
fn a_requested_fills_handle_is_cut_at_the_entry_bound() {
    assert_handle_cut(|handle| {
        SystemEvent::FillRequested(FillRequested {
            session_id: SessionId(uuid(2)),
            element_id: ElementId(uuid(8)),
            value: TYPED.to_string(),
            handle: Some(handle.to_string()),
        })
    });
}

#[test]
fn a_requested_picks_handle_is_cut_at_the_entry_bound() {
    assert_handle_cut(|handle| {
        SystemEvent::PickRequested(PickRequested {
            session_id: SessionId(uuid(2)),
            element_id: ElementId(uuid(8)),
            option_id: Some(OptionId(uuid(10))),
            handle: Some(handle.to_string()),
        })
    });
}

#[test]
fn a_requested_press_handle_is_cut_at_the_entry_bound() {
    assert_handle_cut(|handle| {
        SystemEvent::PressRequested(PressRequested {
            session_id: SessionId(uuid(2)),
            element_id: ElementId(uuid(8)),
            handle: Some(handle.to_string()),
        })
    });
}
