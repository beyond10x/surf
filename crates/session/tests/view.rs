//! Cases 13, 14, 15, 16 and 21 of `story:view-surfd-serves`: the live view over the real
//! session process. `surfd` runs in the test process (`surfd_in_process`, decision D29) with the
//! built `surf-session`, a page server the test starts, and the built `surf-agent` driving it.
//! No case writes down how many events a command has: each compares what a route answers with
//! `Daemon::session_entries`.

#![forbid(unsafe_code)]

use crate::common;

use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use common::{Opened, PageServer, TestDir, agent_call, agent_open, stderr, surfd_in_process};
use surfd::Daemon;
use surfd::link::{self, EntryKind};

/// How long a case waits for one event of the stream.
const ALLOWED: Duration = Duration::from_secs(10);

// --- view-url and the routes ------------------------------------------------------------------

pub(crate) struct ViewUrl {
    port: u16,
    token: String,
}

pub(crate) fn view_url(dir: &TestDir) -> ViewUrl {
    let line = fs::read_to_string(dir.surf().join("view-url")).expect("view-url");
    let rest = line
        .strip_prefix("http://127.0.0.1:")
        .expect("http://127.0.0.1:");
    let (port, rest) = rest.split_once("/#").expect("/#");
    ViewUrl {
        port: port.parse().expect("a port"),
        token: rest.strip_suffix('\n').expect("a line feed").to_string(),
    }
}

/// The 200 body of `GET <target>` with the view token.
pub(crate) fn body(view: &ViewUrl, target: &str) -> Vec<u8> {
    let mut stream = TcpStream::connect(("127.0.0.1", view.port)).expect("the view listens");
    stream.set_read_timeout(Some(ALLOWED)).unwrap();
    let request = format!(
        "GET {target} HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nAuthorization: Bearer {}\r\n\r\n",
        view.port, view.token
    );
    stream.write_all(request.as_bytes()).unwrap();
    let mut answer = Vec::new();
    stream.read_to_end(&mut answer).unwrap();
    assert!(
        answer.starts_with(b"HTTP/1.1 200 OK\r\n"),
        "{target}: {}",
        String::from_utf8_lossy(&answer)
    );
    let at = answer
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .expect("a head")
        + 4;
    answer[at..].to_vec()
}

fn json_of(bytes: &[u8]) -> json::Value {
    json::parse(std::str::from_utf8(bytes).expect("UTF-8"))
}

fn detail(view: &ViewUrl, id: &str) -> json::Value {
    json_of(&body(view, &format!("/api/sessions/{id}")))
}

fn summary(view: &ViewUrl, id: &str) -> json::Value {
    json_of(&body(view, "/api/sessions"))
        .get("sessions")
        .array()
        .iter()
        .find(|summary| summary.get("id").str() == id)
        .unwrap_or_else(|| panic!("{id} is not listed"))
        .clone()
}

// --- The stream -------------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub(crate) struct Event {
    id: u64,
    pub(crate) event: String,
    pub(crate) data: String,
}

impl Event {
    fn json(&self) -> json::Value {
        json::parse(&self.data)
    }
}

/// An open `GET /events`, read by a thread of its own that keeps every byte it read; closed
/// when dropped.
pub(crate) struct Events {
    socket: TcpStream,
    received: Receiver<Event>,
    bytes: Arc<Mutex<Vec<u8>>>,
    reader: Option<JoinHandle<()>>,
}

impl Events {
    /// Open the stream and read its head: the server writes it once the reader is subscribed.
    pub(crate) fn open(view: &ViewUrl) -> Events {
        let mut socket = TcpStream::connect(("127.0.0.1", view.port)).expect("the view listens");
        let request = format!(
            "GET /events HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nAuthorization: Bearer {}\r\n\r\n",
            view.port, view.token
        );
        socket.write_all(request.as_bytes()).unwrap();
        socket.set_read_timeout(Some(ALLOWED)).unwrap();
        let mut reader = BufReader::new(socket.try_clone().unwrap());
        let bytes = Arc::new(Mutex::new(Vec::new()));
        let mut line = String::new();
        loop {
            line.clear();
            reader.read_line(&mut line).unwrap();
            bytes.lock().unwrap().extend_from_slice(line.as_bytes());
            if line == "\r\n" {
                break;
            }
        }
        reader.get_ref().set_read_timeout(None).unwrap();
        let (sender, received) = mpsc::channel();
        let kept = Arc::clone(&bytes);
        let thread = thread::spawn(move || {
            let (mut id, mut event, mut data) = (None, None, None);
            let mut line = String::new();
            loop {
                line.clear();
                match reader.read_line(&mut line) {
                    Ok(0) | Err(_) => return,
                    Ok(_) => {}
                }
                kept.lock().unwrap().extend_from_slice(line.as_bytes());
                let text = line.strip_suffix('\n').unwrap_or(&line);
                if text.is_empty() {
                    if let (Some(id), Some(event), Some(data)) =
                        (id.take(), event.take(), data.take())
                    {
                        let _ = sender.send(Event { id, event, data });
                    }
                } else if let Some(value) = text.strip_prefix("id: ") {
                    id = Some(value.parse().expect("a numeric id"));
                } else if let Some(value) = text.strip_prefix("event: ") {
                    event = Some(value.to_string());
                } else if let Some(value) = text.strip_prefix("data: ") {
                    data = Some(value.to_string());
                }
            }
        });
        Events {
            socket,
            received,
            bytes,
            reader: Some(thread),
        }
    }

    fn event(&self) -> Event {
        match self.received.recv_timeout(ALLOWED) {
            Ok(event) => event,
            Err(RecvTimeoutError::Timeout) => panic!("no event within {ALLOWED:?}"),
            Err(RecvTimeoutError::Disconnected) => panic!("the stream ended"),
        }
    }

    /// Every event until one for which `last` holds, that one included.
    pub(crate) fn until(&self, last: impl Fn(&Event) -> bool) -> Vec<Event> {
        let mut events = Vec::new();
        loop {
            let event = self.event();
            let done = last(&event);
            events.push(event);
            if done {
                return events;
            }
        }
    }

    /// Every event until the `session` event of `id`, which must follow an answer entry of it.
    fn through_answer_of(&self, id: &str) -> Vec<Event> {
        self.until(|event| event.event == "session" && event.json().get("id").str() == id)
    }

    pub(crate) fn bytes(&self) -> Vec<u8> {
        self.bytes.lock().unwrap().clone()
    }
}

impl Drop for Events {
    fn drop(&mut self) {
        let _ = self.socket.shutdown(Shutdown::Both);
        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }
    }
}

// --- Driving the session ----------------------------------------------------------------------

fn agent_go(dir: &TestDir, session: &Opened, url: &str) -> String {
    let output = agent_call(dir, &["go", url], &session.id, &session.token);
    assert!(
        matches!(output.status.code(), Some(0 | 1)),
        "go {url}: {}",
        stderr(&output)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// A port on 127.0.0.1 that nothing listens on: one the system gave and the test let go.
fn closed_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("a port");
    listener.local_addr().unwrap().port()
}

/// The entries of the detail of `id`, each checked against the kept entry of the log with the
/// same place: number, session, index, `kept_at`, kind, and for an action its command and
/// argument, for an event its name.
fn entries_as_kept(daemon: &Daemon, view: &ViewUrl, id: &str) -> Vec<json::Value> {
    let detail = detail(view, id);
    assert_eq!(detail.get("unreadable").number(), 0);
    let entries = detail.get("entries").array().to_vec();
    let kept = daemon.session_entries(id).expect("listed").entries;
    assert_eq!(
        entries.len(),
        kept.len(),
        "the detail lists every kept entry"
    );
    for (at, (entry, kept)) in entries.iter().zip(&kept).enumerate() {
        assert_eq!(entry.get("number").number(), kept.number);
        assert_eq!(entry.get("session").str(), id);
        assert_eq!(
            entry.get("index").number(),
            at as u64 + 1,
            "index from 1, no gap"
        );
        assert_eq!(entry.get("kept_at").number(), kept.kept_at);
        let kind = entry.get("kind").str();
        match kept.kind {
            EntryKind::Action => {
                assert_eq!(kind, "action");
                let line = link::read_action(&kept.text).expect("an action line");
                assert_eq!(entry.get("command").str(), line.command);
                let argument = line
                    .argument
                    .map(|field| surf_answer::unquote(field).unwrap());
                assert_eq!(entry.get("argument").opt_str(), argument.as_deref());
            }
            EntryKind::Event => {
                assert_eq!(kind, "event");
                let line = link::read_event(&kept.text).expect("an event line");
                assert_eq!(entry.get("name").str(), line.name);
            }
            EntryKind::Answer => assert_eq!(kind, "answer"),
        }
    }
    let numbers: Vec<u64> = entries
        .iter()
        .map(|entry| entry.get("number").number())
        .collect();
    assert!(
        numbers.windows(2).all(|pair| pair[0] < pair[1]),
        "{numbers:?}"
    );
    entries
}

fn kinds(entries: &[json::Value]) -> Vec<&str> {
    entries
        .iter()
        .map(|entry| entry.get("kind").str())
        .collect()
}

fn path_actions(answer: &json::Value) -> Vec<(u64, String, Option<String>)> {
    answer
        .get("path")
        .get("actions")
        .array()
        .iter()
        .map(|action| {
            (
                action.get("sequence").number(),
                action.get("command").str().to_string(),
                action.get("argument").opt_str().map(str::to_string),
            )
        })
        .collect()
}

// --- The cases --------------------------------------------------------------------------------

#[test]
fn case_13_the_detail_lists_open_and_go_in_the_order_of_d19() {
    let dir = TestDir::new();
    let pages = PageServer::start();
    let daemon = surfd_in_process(&dir, |_| {});
    let a = pages.url("/a");
    let b = pages.url("/b");
    let session = agent_open(&dir, Some(&a));
    agent_go(&dir, &session, &b);
    let view = view_url(&dir);
    let entries = entries_as_kept(&daemon, &view, &session.id);
    let kinds = kinds(&entries);

    let actions: Vec<usize> = (0..kinds.len())
        .filter(|&at| kinds[at] == "action")
        .collect();
    let answers: Vec<usize> = (0..kinds.len())
        .filter(|&at| kinds[at] == "answer")
        .collect();
    assert_eq!(actions.len(), 3, "{kinds:?}");
    assert_eq!(answers.len(), 2, "{kinds:?}");
    let (open, go_a, go_b) = (actions[0], actions[1], actions[2]);
    assert_eq!(open, 0);
    assert_eq!(
        (
            entries[open].get("sequence").number(),
            entries[open].get("command").str()
        ),
        (1, "open")
    );
    assert_eq!(entries[open].get("argument"), &json::Value::Null);
    assert_eq!(
        (
            entries[go_a].get("sequence").number(),
            entries[go_a].get("command").str(),
            entries[go_a].get("argument").str()
        ),
        (2, "go", a.as_str())
    );
    assert_eq!(
        (
            entries[go_b].get("sequence").number(),
            entries[go_b].get("command").str(),
            entries[go_b].get("argument").str()
        ),
        (3, "go", b.as_str())
    );
    // open's events, go <a>'s events, open's one answer entry, go <b>'s events, its answer.
    assert!(go_a > open + 1, "open has events");
    assert!(kinds[open + 1..go_a].iter().all(|&kind| kind == "event"));
    assert_eq!(
        answers[0],
        go_b - 1,
        "the answer of open right before go <b>"
    );
    assert!(answers[0] > go_a + 1, "go <a> has events");
    assert!(
        kinds[go_a + 1..answers[0]]
            .iter()
            .all(|&kind| kind == "event")
    );
    assert_eq!(answers[1], entries.len() - 1, "the answer of go <b> last");
    assert!(answers[1] > go_b + 1, "go <b> has events");
    assert!(
        kinds[go_b + 1..answers[1]]
            .iter()
            .all(|&kind| kind == "event")
    );

    let open_answer = &entries[answers[0]];
    let page = open_answer.get("page");
    assert_eq!(page.get("kind").str(), "loaded");
    assert_eq!(page.get("address").str(), a);
    assert_eq!(
        path_actions(open_answer),
        [
            (1, "open".to_string(), None),
            (2, "go".to_string(), Some(a.clone()))
        ]
    );
    let go_answer = &entries[answers[1]];
    assert_eq!(go_answer.get("page").get("kind").str(), "loaded");
    assert_eq!(go_answer.get("page").get("address").str(), b);
}

#[test]
fn case_14_a_stream_receives_a_go_then_the_session_event() {
    let dir = TestDir::new();
    let pages = PageServer::start();
    let daemon = surfd_in_process(&dir, |_| {});
    let session = agent_open(&dir, None);
    let view = view_url(&dir);
    let events = Events::open(&view);
    agent_go(&dir, &session, &pages.url("/a"));
    let received = events.through_answer_of(&session.id);

    let entries = entries_as_kept(&daemon, &view, &session.id);
    let go = entries
        .iter()
        .position(|entry| {
            entry.get("kind").str() == "action" && entry.get("sequence").number() == 2
        })
        .expect("the action entry of go");
    let expected = &entries[go..];
    let (session_event, streamed) = received.split_last().unwrap();
    assert_eq!(streamed.len(), expected.len(), "{received:?}");
    for (event, entry) in streamed.iter().zip(expected) {
        assert_eq!(event.event, "entry");
        assert_eq!(event.id, entry.get("number").number(), "id: is the number");
        assert_eq!(&event.json(), entry, "the entry as the detail writes it");
    }
    assert_eq!(expected.last().unwrap().get("kind").str(), "answer");
    assert_eq!(session_event.event, "session");
    assert_eq!(
        session_event.id,
        expected.last().unwrap().get("number").number(),
        "the session event has the answer entry's id"
    );
    assert_eq!(session_event.json().get("state").str(), "open");
}

#[test]
fn case_15_a_go_to_a_closed_port_fails_unreachable() {
    let dir = TestDir::new();
    let daemon = surfd_in_process(&dir, |_| {});
    let session = agent_open(&dir, None);
    let view = view_url(&dir);
    let events = Events::open(&view);
    let url = format!("http://127.0.0.1:{}/", closed_port());
    let printed = agent_go(&dir, &session, &url);
    assert!(printed.contains("page failed Unreachable"), "{printed}");
    let received = events.through_answer_of(&session.id);
    let expected = format!(r#"{{"kind":"failed","reason":"Unreachable","address":"{url}"}}"#);
    let streamed = received
        .iter()
        .rfind(|event| event.event == "entry" && event.json().get("kind").str() == "answer")
        .expect("the answer entry on the stream");
    assert!(streamed.data.contains(&expected), "{}", streamed.data);
    let entries = entries_as_kept(&daemon, &view, &session.id);
    let answer = entries.last().unwrap();
    assert_eq!(answer.get("page"), &json::parse(&expected));
    assert_eq!(
        summary(&view, &session.id).get("page"),
        &json::parse(&expected)
    );
    assert_eq!(
        received.last().unwrap().json().get("page"),
        &json::parse(&expected)
    );
}

#[test]
fn case_16_no_session_token_is_in_any_answer_or_event() {
    let dir = TestDir::new();
    let pages = PageServer::start();
    let _daemon = surfd_in_process(&dir, |_| {});
    let view = view_url(&dir);
    let events = Events::open(&view);
    let first = agent_open(&dir, Some(&pages.url("/a")));
    agent_go(&dir, &first, &pages.url("/b"));
    let second = agent_open(&dir, None);
    agent_go(
        &dir,
        &second,
        &format!("http://127.0.0.1:{}/", closed_port()),
    );
    events.through_answer_of(&second.id);
    let mut bytes = events.bytes();
    bytes.extend(body(&view, "/api/sessions"));
    bytes.extend(body(&view, &format!("/api/sessions/{}", first.id)));
    bytes.extend(body(&view, &format!("/api/sessions/{}", second.id)));
    assert!(bytes.len() > 1_000, "something was read");
    for token in [&first.token, &second.token] {
        assert_eq!(token.len(), 64);
        assert!(
            !bytes
                .windows(token.len())
                .any(|window| window == token.as_bytes()),
            "a session token in what the view answered"
        );
    }
}

#[test]
fn case_21_an_address_is_written_as_its_exact_value() {
    let dir = TestDir::new();
    let pages = PageServer::start();
    let daemon = surfd_in_process(&dir, |_| {});
    let session = agent_open(&dir, None);
    let address = pages.url("/x\"y\\zé\nw");
    agent_go(&dir, &session, &address);
    let view = view_url(&dir);

    let listed = summary(&view, &session.id);
    assert_eq!(listed.get("address").str(), address);
    let entries = entries_as_kept(&daemon, &view, &session.id);
    let answer = entries.last().unwrap();
    assert_eq!(answer.get("kind").str(), "answer");
    assert_eq!(answer.get("page").get("address").str(), address);
    let started: Vec<&json::Value> = entries
        .iter()
        .filter(|entry| {
            entry.get("kind").str() == "event"
                && entry.get("name").str() == "surf.net.RequestStarted"
        })
        .collect();
    assert_eq!(started.len(), 1);
    assert_eq!(started[0].get("fields").get("url").str(), address);
    for value in [
        listed.get("address").str(),
        answer.get("page").get("address").str(),
        started[0].get("fields").get("url").str(),
    ] {
        assert!(
            !value.contains("\\x"),
            "an escape of the link left: {value:?}"
        );
    }
}

/// The bytes of each entry of a detail body as the view wrote them: an entry starts with
/// `{"number":`, which no string of an entry holds (the writer escapes `"`), and the last one
/// ends before the `]}` that closes the detail.
fn entry_bytes(body: &[u8]) -> Vec<&[u8]> {
    const START: &[u8] = b"{\"number\":";
    let starts: Vec<usize> = (0..body.len())
        .filter(|&at| body[at] == b'{' && body[at..].starts_with(START))
        .collect();
    let close = body.len() - b"]}".len();
    assert!(body[close..].starts_with(b"]}"), "a detail ends with ]}}");
    starts
        .iter()
        .enumerate()
        .map(|(at, &start)| match starts.get(at + 1) {
            Some(&next) => &body[start..next - 1],
            None => &body[start..close],
        })
        .collect()
}

/// `story:page-step-holds-under-load` (1a): the page test `an entry over 1 MiB` goes once to an
/// address of 60,000 bytes and has the view draw that answer; its size and the four actions of
/// three such goes are measured here, from the detail as the view writes it, and not in the page
/// test. One go: the answer entry is over 1,048,576 bytes, since the address is written three
/// times (the page line, the path and `changed`), each `<` as `<`. Three goes: open and
/// three goes in the detail, and the last answer over 1 MiB too.
#[test]
fn large_entry_one_go_makes_an_answer_over_1_mib_and_three_make_four_actions() {
    const MIB: usize = 1_048_576;
    let dir = TestDir::new();
    let pages = PageServer::start();
    let _daemon = surfd_in_process(&dir, |_| {});
    let session = agent_open(&dir, None);
    let address = pages.url(&format!("/first/a.html?{}", "<".repeat(60_000)));
    let view = view_url(&dir);
    // Whether the head of an entry, where `kind` is written after the numbers and the session,
    // holds `text`.
    let holds = |entry: &[u8], text: &[u8]| {
        entry[..entry.len().min(256)]
            .windows(text.len())
            .any(|window| window == text)
    };
    // The detail's entries as written: the end of each action entry from its command on (the
    // test's JSON reader takes seconds over an address of 60,000 bytes in a debug build), and
    // the size of the last entry, an answer.
    let read = |view: &ViewUrl| -> (Vec<String>, usize) {
        let body = body(view, &format!("/api/sessions/{}", session.id));
        let entries = entry_bytes(&body);
        let actions = entries
            .iter()
            .filter(|entry| holds(entry, br#","kind":"action","#))
            .map(|entry| {
                let text = std::str::from_utf8(entry).expect("UTF-8");
                let at = text.find(r#""command":"#).expect("a command");
                text[at..].to_string()
            })
            .collect();
        let last = entries.last().expect("an entry");
        assert!(
            holds(last, br#","kind":"answer","#),
            "the last entry is an answer"
        );
        (actions, last.len())
    };
    // The address as the view writes it: its one character to escape is `<`.
    let go = format!(
        r#""command":"go","argument":"{}"}}"#,
        address.replace('<', "\\u003c")
    );
    let open = r#""command":"open","argument":null}"#.to_string();

    agent_go(&dir, &session, &address);
    let (actions, first) = read(&view);
    assert_eq!(actions, vec![open.clone(), go.clone()]);
    assert!(first > MIB, "the answer of one go is {first} bytes");

    agent_go(&dir, &session, &address);
    agent_go(&dir, &session, &address);
    let (actions, last) = read(&view);
    assert_eq!(actions, vec![open, go.clone(), go.clone(), go]);
    assert!(last > MIB, "the answer of the third go is {last} bytes");
}

/// A JSON reader for the tests: the values `surf-view` writes (no fraction, no exponent, no
/// sign), every escape of RFC 8259 read.
mod json {
    #[derive(Clone, Debug, PartialEq)]
    pub enum Value {
        Null,
        Bool(bool),
        Number(u64),
        Str(String),
        Array(Vec<Value>),
        Object(Vec<(String, Value)>),
    }

    impl Value {
        pub fn get(&self, name: &str) -> &Value {
            match self {
                Value::Object(members) => members
                    .iter()
                    .find(|(member, _)| member == name)
                    .map(|(_, value)| value)
                    .unwrap_or_else(|| panic!("no member {name} in {self:?}")),
                other => panic!("not an object: {other:?}"),
            }
        }

        pub fn str(&self) -> &str {
            match self {
                Value::Str(text) => text,
                other => panic!("not a string: {other:?}"),
            }
        }

        pub fn opt_str(&self) -> Option<&str> {
            match self {
                Value::Null => None,
                other => Some(other.str()),
            }
        }

        pub fn number(&self) -> u64 {
            match self {
                Value::Number(number) => *number,
                other => panic!("not a number: {other:?}"),
            }
        }

        pub fn array(&self) -> &[Value] {
            match self {
                Value::Array(items) => items,
                other => panic!("not an array: {other:?}"),
            }
        }
    }

    pub fn parse(text: &str) -> Value {
        let chars: Vec<char> = text.chars().collect();
        let mut at = 0;
        let value = value(&chars, &mut at);
        assert_eq!(at, chars.len(), "text after the value");
        value
    }

    fn value(chars: &[char], at: &mut usize) -> Value {
        match chars[*at] {
            'n' => literal(chars, at, "null", Value::Null),
            't' => literal(chars, at, "true", Value::Bool(true)),
            'f' => literal(chars, at, "false", Value::Bool(false)),
            '"' => Value::Str(string(chars, at)),
            '[' => {
                *at += 1;
                let mut items = Vec::new();
                if chars[*at] == ']' {
                    *at += 1;
                    return Value::Array(items);
                }
                loop {
                    items.push(value(chars, at));
                    let next = chars[*at];
                    *at += 1;
                    match next {
                        ',' => {}
                        ']' => return Value::Array(items),
                        other => panic!("{other:?} in an array"),
                    }
                }
            }
            '{' => {
                *at += 1;
                let mut members = Vec::new();
                if chars[*at] == '}' {
                    *at += 1;
                    return Value::Object(members);
                }
                loop {
                    let name = string(chars, at);
                    assert_eq!(chars[*at], ':');
                    *at += 1;
                    members.push((name, value(chars, at)));
                    let next = chars[*at];
                    *at += 1;
                    match next {
                        ',' => {}
                        '}' => return Value::Object(members),
                        other => panic!("{other:?} in an object"),
                    }
                }
            }
            '0'..='9' => {
                let start = *at;
                while *at < chars.len() && chars[*at].is_ascii_digit() {
                    *at += 1;
                }
                let digits: String = chars[start..*at].iter().collect();
                Value::Number(digits.parse().expect("a number"))
            }
            other => panic!("{other:?} starts no value"),
        }
    }

    fn literal(chars: &[char], at: &mut usize, word: &str, value: Value) -> Value {
        let end = *at + word.chars().count();
        let found: String = chars[*at..end].iter().collect();
        assert_eq!(found, word);
        *at = end;
        value
    }

    fn hex4(chars: &[char], at: &mut usize) -> u32 {
        let digits: String = chars[*at..*at + 4].iter().collect();
        *at += 4;
        u32::from_str_radix(&digits, 16).expect("four hexadecimal digits")
    }

    fn string(chars: &[char], at: &mut usize) -> String {
        assert_eq!(chars[*at], '"');
        *at += 1;
        let mut out = String::new();
        loop {
            let c = chars[*at];
            *at += 1;
            match c {
                '"' => return out,
                '\\' => {
                    let escape = chars[*at];
                    *at += 1;
                    match escape {
                        '"' => out.push('"'),
                        '\\' => out.push('\\'),
                        '/' => out.push('/'),
                        'b' => out.push('\u{8}'),
                        'f' => out.push('\u{c}'),
                        'n' => out.push('\n'),
                        'r' => out.push('\r'),
                        't' => out.push('\t'),
                        'u' => {
                            let first = hex4(chars, at);
                            let code = if (0xd800..0xdc00).contains(&first) {
                                assert_eq!(chars[*at..*at + 2], ['\\', 'u']);
                                *at += 2;
                                let low = hex4(chars, at);
                                0x10000 + ((first - 0xd800) << 10) + (low - 0xdc00)
                            } else {
                                first
                            };
                            out.push(char::from_u32(code).expect("a scalar value"));
                        }
                        other => panic!("the escape \\{other}"),
                    }
                }
                c if u32::from(c) < 0x20 => panic!("a raw control character in a string"),
                c => out.push(c),
            }
        }
    }
}
