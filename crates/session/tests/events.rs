//! Cases 1 to 8 and 12 to 18 of `story:agent-session-events`: what the session process sends
//! `surfd` per action, its action, its events and its answer, and what `surfd` keeps of it. The
//! cases that read the log run `surfd` in the test process through `Daemon::start`, with the
//! built `surf-session` as the session program, a page server the test starts, and requests over
//! `surf_agent::wire` (decision D29). Cases 17 and 18 run the mapping alone, against the events
//! `spec/` declares.

#![forbid(unsafe_code)]

mod common;

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
    ActionCommand, ActionId, ActionRecorded, Go, NavigationRequested, OpenSession, RecordAction,
    SessionClosed, SessionId, SessionNavigationChanged, SessionOpened,
};
use surf_types::document::{DocumentId, DocumentParsed};
use surf_types::navigation::{
    NavigationCancelled, NavigationCommitted, NavigationFailed, NavigationStarted, NavigationState,
};
use surf_types::net::{
    FailureReason, FetchFailed, MediaType, RequestId, RequestStarted, ResponseReceived, Status,
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

/// The entries of `go` to a served HTML page: the action entry, its nine events, the answer
/// entry. Measured by this unit (story: counted from `spec/` as 11).
const GO_ENTRIES: usize = 11;

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
    let letters = &body[18..];
    for entry in &entries {
        for window in letters.windows(8) {
            assert!(
                !entry.text.as_bytes().windows(8).any(|part| part == window),
                "an entry holds bytes of the page: {entry:?}"
            );
        }
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
    // The story's number, not the crate's constant.
    const QUEUE: usize = 1024;
    let dir = TestDir::new();
    let pages = PageServer::start();
    let daemon = surfd_in_process(&dir, |_| {});
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

/// Every `SystemEvent` variant this mapping covers, one value each; a story that maps an event
/// adds it here.
fn covered() -> Vec<SystemEvent> {
    let url = || Url(ADDRESS.to_string());
    vec![
        SystemEvent::ActionRecorded(ActionRecorded {
            action_id: ActionId(uuid(1)),
            session_id: SessionId(uuid(2)),
            sequence: 7,
            command: ActionCommand::Go,
            url: None,
        }),
        SystemEvent::NavigationRequested(NavigationRequested {
            session_id: SessionId(uuid(2)),
            tab_id: TabId(uuid(3)),
            url: url(),
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
        }),
        SystemEvent::FetchFailed(FetchFailed {
            request_id: RequestId(uuid(5)),
            url: url(),
            reason: FailureReason::Unreachable,
        }),
        SystemEvent::RequestStarted(RequestStarted {
            request_id: RequestId(uuid(5)),
            url: url(),
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
        ("surf.agent.ActionRecorded", "sequence") => text("7"),
        ("surf.agent.ActionRecorded", "command") => text("go"),
        ("surf.agent.ActionRecorded", "url") => None,
        (_, "url") => text(ADDRESS),
        ("surf.agent.SessionNavigationChanged", "state") => text("Committed"),
        ("surf.agent.SessionNavigationChanged", "sessions") => text("1"),
        ("surf.navigation.NavigationFailed", "reason") => text("TooManyRedirects"),
        ("surf.net.FetchFailed", "reason") => text("Unreachable"),
        (_, "status") => text("404"),
        (_, "media_type") => text("text/html"),
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
    assert_eq!(
        names,
        spec.keys().cloned().collect::<BTreeSet<_>>(),
        "the variants covered and the events of spec/"
    );
    assert_eq!(
        names.len(),
        17,
        "the events on main when this story started"
    );
    for field in [
        ("surf.net.ResponseReceived", "url"),
        ("surf.navigation.NavigationCommitted", "url"),
    ] {
        assert!(
            spec[field.0].iter().any(|(name, _)| name == field.1),
            "{field:?} of agent-redirect-spec"
        );
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
}

#[test]
fn byte_fields_are_lengths() {
    // A story that adds a field of type Bytes adds it here.
    const BYTES_FIELDS: [&str; 2] = [
        "surf.net.ResponseReceived.body",
        "surf.navigation.NavigationCommitted.body",
    ];
    let spec = spec_events();
    let covered = covered();
    let mut declared: Vec<String> = Vec::new();
    for event in &covered {
        for (field, kind) in &spec[event.name()] {
            if kind == "Bytes" {
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
            if kind != "Bytes" {
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
