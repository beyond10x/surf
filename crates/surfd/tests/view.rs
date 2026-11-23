//! Cases 10, 11, 12, 17, 18, 19, 20, 24 and 27 of `story:view-surfd-serves` and case 2 of
//! `story:view-controls-and-changes` (`case_02_controls_…`): the live view `surfd` serves on
//! 127.0.0.1. Every case but 27 runs `surfd` in the test process (`common::start`) with the
//! stand-in session, driven by the wire (`common::call`), and case 2 by the linked `surf-agent`
//! with the password on its standard input; case 27 runs the built `surfd`. Each case reads the
//! port and the token from `view-url` and asserts what a route answers.

use crate::common;

use std::fs;
use std::io::{BufRead, BufReader, ErrorKind, Read, Write};
use std::net::{Shutdown, TcpStream};
use std::os::unix::fs::PermissionsExt;
use std::process::{Output, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use common::*;
use surf_agent::wire::{self, Arg};
use surfd::{Daemon, State};

// --- view-url and the routes ------------------------------------------------------------------

/// The line of `view-url`, read into its port and token.
struct ViewUrl {
    line: String,
    port: u16,
    token: String,
}

fn view_url(dir: &TestDir) -> ViewUrl {
    let line = fs::read_to_string(dir.surf().join("view-url")).expect("view-url");
    let rest = line
        .strip_prefix("http://127.0.0.1:")
        .expect("http://127.0.0.1:");
    let (port, rest) = rest.split_once("/#").expect("/#");
    let token = rest.strip_suffix('\n').expect("a line feed").to_string();
    ViewUrl {
        port: port.parse().expect("a port"),
        token,
        line,
    }
}

/// `GET <target>` on the view, with `token` as the bearer when given: the status and the body.
fn get(port: u16, target: &str, token: Option<&str>) -> (u16, Vec<u8>) {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("the view listens");
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .unwrap();
    let mut request = format!("GET {target} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n");
    if let Some(token) = token {
        request.push_str(&format!("Authorization: Bearer {token}\r\n"));
    }
    request.push_str("\r\n");
    stream.write_all(request.as_bytes()).unwrap();
    let mut answer = Vec::new();
    stream.read_to_end(&mut answer).unwrap();
    let at = answer
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .expect("a head")
        + 4;
    let status = std::str::from_utf8(&answer[9..12])
        .unwrap()
        .parse()
        .unwrap();
    (status, answer[at..].to_vec())
}

/// The 200 body of `target`, read as JSON.
fn json_of(view: &ViewUrl, target: &str) -> json::Value {
    let (status, body) = get(view.port, target, Some(&view.token));
    assert_eq!(status, 200, "{target}: {}", String::from_utf8_lossy(&body));
    json::parse(std::str::from_utf8(&body).expect("UTF-8"))
}

fn sessions(view: &ViewUrl) -> json::Value {
    json_of(view, "/api/sessions")
}

fn detail(view: &ViewUrl, id: &str) -> json::Value {
    json_of(view, &format!("/api/sessions/{id}"))
}

/// The listed summary of `id`.
fn listed(list: &json::Value, id: &str) -> json::Value {
    list.get("sessions")
        .array()
        .iter()
        .find(|summary| summary.get("id").str() == id)
        .unwrap_or_else(|| panic!("{id} is not listed"))
        .clone()
}

// --- The stream -------------------------------------------------------------------------------

/// One event of the stream.
#[derive(Clone, Debug)]
struct Event {
    id: u64,
    event: String,
    data: String,
}

impl Event {
    fn json(&self) -> json::Value {
        json::parse(&self.data)
    }
}

enum Got {
    Event(Event),
    Ended,
    Nothing,
}

/// An open `GET /events`, read by a thread of its own; closed when dropped.
struct Events {
    socket: TcpStream,
    received: Receiver<Option<Event>>,
    reader: Option<JoinHandle<()>>,
}

impl Events {
    /// Open the stream, with `Last-Event-ID: <last>` when given, and read its head: the server
    /// writes it once the reader is subscribed.
    fn open(view: &ViewUrl, last: Option<u64>) -> Events {
        let mut socket = TcpStream::connect(("127.0.0.1", view.port)).expect("the view listens");
        let mut request = format!(
            "GET /events HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nAuthorization: Bearer {}\r\n",
            view.port, view.token
        );
        if let Some(last) = last {
            request.push_str(&format!("Last-Event-ID: {last}\r\n"));
        }
        request.push_str("\r\n");
        socket.write_all(request.as_bytes()).unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        let mut reader = BufReader::new(socket.try_clone().unwrap());
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        assert_eq!(line, "HTTP/1.1 200 OK\r\n");
        loop {
            line.clear();
            reader.read_line(&mut line).unwrap();
            if line == "\r\n" {
                break;
            }
        }
        reader.get_ref().set_read_timeout(None).unwrap();
        let (sender, received) = mpsc::channel();
        let thread = thread::spawn(move || {
            let mut id = None;
            let mut event = None;
            let mut data = None;
            let mut line = String::new();
            loop {
                line.clear();
                match reader.read_line(&mut line) {
                    Ok(0) | Err(_) => {
                        let _ = sender.send(None);
                        return;
                    }
                    Ok(_) => {}
                }
                let text = line.strip_suffix('\n').unwrap_or(&line);
                if text.is_empty() {
                    if let (Some(id), Some(event), Some(data)) =
                        (id.take(), event.take(), data.take())
                    {
                        let _ = sender.send(Some(Event { id, event, data }));
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
            reader: Some(thread),
        }
    }

    fn next(&self, within: Duration) -> Got {
        match self.received.recv_timeout(within) {
            Ok(Some(event)) => Got::Event(event),
            Ok(None) | Err(RecvTimeoutError::Disconnected) => Got::Ended,
            Err(RecvTimeoutError::Timeout) => Got::Nothing,
        }
    }

    /// The next event, which must come within [`ALLOWED`].
    fn event(&self) -> Event {
        match self.next(ALLOWED) {
            Got::Event(event) => event,
            Got::Ended => panic!("the stream ended"),
            Got::Nothing => panic!("no event within {ALLOWED:?}"),
        }
    }

    /// Every event until one for which `last` holds, that one included.
    fn until(&self, last: impl Fn(&Event) -> bool) -> Vec<Event> {
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
}

impl Drop for Events {
    fn drop(&mut self) {
        let _ = self.socket.shutdown(Shutdown::Both);
        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }
    }
}

// --- Driving the stand-in ---------------------------------------------------------------------

fn go(dir: &TestDir, session: &Opened, url: &str) -> wire::Answer {
    let mut go = request("go", Some(&session.id), Some(&session.token));
    go.args.push(Arg {
        name: "url".to_string(),
        value: url.to_string(),
    });
    call(dir, &go)
}

fn close(dir: &TestDir, daemon: &Daemon, session: &Opened) {
    let answer = call(
        dir,
        &request("close", Some(&session.id), Some(&session.token)),
    );
    assert_eq!(answer.status, 0, "close: {answer:?}");
    assert!(daemon.wait_sessions(ALLOWED, |list| {
        list.get(&session.id)
            .is_some_and(|listed| listed.state == State::Closed)
    }));
}

/// A writer the test reads back: where `view_line` goes.
#[derive(Clone, Default)]
struct Captured(Arc<Mutex<Vec<u8>>>);

impl Write for Captured {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// The numbers of the kept entries of `id`.
fn numbers(daemon: &Daemon, id: &str) -> Vec<u64> {
    daemon
        .session_entries(id)
        .expect("listed")
        .entries
        .iter()
        .map(|entry| entry.number)
        .collect()
}

// --- The cases --------------------------------------------------------------------------------

#[test]
fn case_10_view_url_is_mode_0600_and_goes_with_the_daemon() {
    let dir = TestDir::new();
    let captured = Captured::default();
    let mut config = config(&dir);
    config.view_line = Some(Box::new(captured.clone()));
    let daemon = start(config);
    let path = dir.surf().join("view-url");
    let metadata = fs::symlink_metadata(&path).expect("view-url");
    assert!(metadata.file_type().is_file(), "a regular file");
    assert_eq!(metadata.permissions().mode() & 0o7777, 0o600);
    let bytes = fs::read(&path).unwrap();
    let view = view_url(&dir);
    // No assertion prints the token on a failure.
    assert!(
        bytes == format!("http://127.0.0.1:{}/#{}\n", view.port, view.token).into_bytes(),
        "view-url is not http://127.0.0.1:<port>/#<token> and a line feed"
    );
    assert_eq!(view.token.len(), 64);
    assert!(
        view.token
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
        "64 lower-case hexadecimal digits"
    );
    assert!(
        *captured.0.lock().unwrap() == bytes,
        "view_line did not get the bytes of view-url"
    );

    let (status, body) = get(view.port, "/", None);
    assert_eq!(status, 200);
    let index = surf_view::page_files()
        .into_iter()
        .find(|file| file.name == "index.html")
        .unwrap();
    assert_eq!(body, index.bytes);

    drop(daemon);
    assert!(
        fs::symlink_metadata(&path).is_err(),
        "view-url is left after the daemon"
    );
}

#[test]
fn case_11_two_open_sessions_are_listed_with_the_newest_number() {
    let dir = TestDir::new();
    let daemon = start(config(&dir));
    dir.set("address", "http://a.example/one");
    let first = open(&dir);
    dir.set("address", "http://a.example/two");
    let second = open(&dir);
    let view = view_url(&dir);
    let list = sessions(&view);
    let summaries = list.get("sessions").array();
    assert_eq!(summaries.len(), 2);
    for (session, address) in [
        (&first, "http://a.example/one"),
        (&second, "http://a.example/two"),
    ] {
        let summary = listed(&list, &session.id);
        assert_eq!(summary.get("state").str(), "open");
        assert_eq!(summary.get("address").str(), address);
    }
    let highest = [&first, &second]
        .iter()
        .flat_map(|session| {
            detail(&view, &session.id)
                .get("entries")
                .array()
                .iter()
                .map(|entry| entry.get("number").number())
                .collect::<Vec<_>>()
        })
        .max()
        .unwrap();
    assert_eq!(list.get("newest").number(), highest);
    assert_eq!(highest, daemon.newest_entry());
}

#[test]
fn case_12_a_closed_session_is_listed_closed_and_its_stream_says_so_last() {
    let dir = TestDir::new();
    let daemon = start(config(&dir));
    let session = open(&dir);
    let view = view_url(&dir);
    let events = Events::open(&view, None);
    close(&dir, &daemon, &session);

    let summary = listed(&sessions(&view), &session.id);
    assert_eq!(summary.get("state").str(), "closed");
    assert!(matches!(summary.get("ended_at"), json::Value::Number(_)));
    let entries = detail(&view, &session.id).get("entries").array().to_vec();
    let close_at = entries
        .iter()
        .position(|entry| {
            entry.get("kind").str() == "action" && entry.get("command").str() == "close"
        })
        .expect("the close action entry");
    assert_eq!(entries[close_at + 1].get("kind").str(), "answer");
    assert_eq!(
        close_at + 2,
        entries.len(),
        "the answer entry of close is the last"
    );

    events.until(|event| {
        event.event == "session"
            && event.json().get("id").str() == session.id
            && event.json().get("state").str() == "closed"
    });
    // A barrier: the next session's entries come after anything the stream still had to say
    // of the first one.
    let next = open(&dir);
    let after =
        events.until(|event| event.event == "session" && event.json().get("id").str() == next.id);
    assert!(
        after
            .iter()
            .all(|event| event.event != "session" || event.json().get("id").str() != session.id),
        "a session event of the closed session after its closed one: {after:?}"
    );
}

#[test]
fn case_17_a_stream_from_a_number_replays_or_resets() {
    // From the number of a go's action entry: its answer entry and the later entries.
    let dir = TestDir::new();
    let daemon = start(config(&dir));
    let session = open(&dir);
    assert_eq!(go(&dir, &session, "http://a.example/1").status, 0);
    assert_eq!(go(&dir, &session, "http://a.example/2").status, 0);
    let kept = daemon.session_entries(&session.id).unwrap().entries;
    let action = kept
        .iter()
        .find(|entry| entry.text == "action 2 go \"http://a.example/1\"\n")
        .expect("the first go's action entry")
        .number;
    let view = view_url(&dir);
    let events = Events::open(&view, Some(action));
    let replayed: Vec<Event> = (0..3).map(|_| events.event()).collect();
    let later: Vec<u64> = kept
        .iter()
        .map(|entry| entry.number)
        .filter(|&number| number > action)
        .collect();
    assert_eq!(later.len(), 3);
    assert_eq!(
        replayed.iter().map(|event| event.id).collect::<Vec<_>>(),
        later
    );
    assert!(
        replayed.iter().all(|event| event.event == "entry"),
        "a session event among the replay: {replayed:?}"
    );
    assert_eq!(replayed[0].json().get("kind").str(), "answer");
    assert!(matches!(
        events.next(Duration::from_millis(300)),
        Got::Nothing
    ));
    drop(events);
    drop(daemon);
    drop(dir);

    // log.entries_per_session 3 and 5 entries made (3 of open, 2 of one go): number 1 is older
    // than what is kept.
    let dir = TestDir::new();
    let mut settings = config(&dir);
    settings.log.entries_per_session = 3;
    let daemon = start(settings);
    let session = open(&dir);
    assert_eq!(go(&dir, &session, "http://a.example/0").status, 0);
    assert_eq!(daemon.newest_entry(), 5);
    let kept = daemon.session_entries(&session.id).unwrap();
    assert_eq!((kept.entries.len(), kept.dropped), (3, 2));
    let view = view_url(&dir);
    let events = Events::open(&view, Some(1));
    let first = events.event();
    assert_eq!((first.id, first.event.as_str()), (5, "reset"));
    assert_eq!(first.data, r#"{"newest":5}"#);
    drop(events);
    drop(daemon);
    drop(dir);

    // view_limits.queue_entries 8: 9 below the newest resets, 8 below replays 8 entries.
    let dir = TestDir::new();
    let mut settings = config(&dir);
    settings.view_limits.queue_entries = 8;
    let daemon = start(settings);
    let session = open(&dir);
    for at in 0..5 {
        assert_eq!(
            go(&dir, &session, &format!("http://a.example/{at}")).status,
            0
        );
    }
    let newest = daemon.newest_entry();
    assert_eq!(newest, 13);
    let view = view_url(&dir);
    let events = Events::open(&view, Some(newest - 9));
    let first = events.event();
    assert_eq!((first.id, first.event.as_str()), (13, "reset"));
    assert_eq!(first.data, r#"{"newest":13}"#);
    drop(events);
    let events = Events::open(&view, Some(newest - 8));
    let replayed: Vec<u64> = (0..8).map(|_| events.event().id).collect();
    assert_eq!(replayed, (6..=13).collect::<Vec<u64>>());
}

#[test]
fn case_18_the_first_closed_log_is_dropped_whole_and_the_rest_stays() {
    let dir = TestDir::new();
    let mut config = config(&dir);
    config.log.bytes_total = 1_000;
    let daemon = start(config);
    let first = open(&dir);
    close(&dir, &daemon, &first);
    let former = daemon.session_entries(&first.id).unwrap();
    assert!(!former.log_dropped, "the bound is passed too early");
    let former = former.entries.len() as u64;
    let second = open(&dir);
    close(&dir, &daemon, &second);
    let third = open(&dir);
    let mut gos = 0;
    while !daemon.session_entries(&first.id).unwrap().log_dropped {
        assert!(gos < 100, "the bound over all sessions is never passed");
        assert_eq!(
            go(&dir, &third, &format!("http://a.example/{gos}")).status,
            0
        );
        gos += 1;
    }

    let view = view_url(&dir);
    let dropped = detail(&view, &first.id);
    assert_eq!(dropped.get("log_dropped"), &json::Value::Bool(true));
    assert!(dropped.get("entries").array().is_empty());
    assert_eq!(dropped.get("dropped").number(), former);
    let summary = dropped.get("session");
    assert_eq!(summary.get("last_action_at"), &json::Value::Null);
    assert_eq!(summary.get("page"), &json::Value::Null);
    assert_eq!(summary.get("state").str(), "closed");

    let whole = detail(&view, &third.id);
    assert_eq!(whole.get("log_dropped"), &json::Value::Bool(false));
    assert_eq!(whole.get("dropped").number(), 0);
    let entries = whole.get("entries").array();
    assert_eq!(
        entries.len(),
        3 + 2 * gos,
        "open's three entries and two per go"
    );
    let numbers_seen: Vec<u64> = entries
        .iter()
        .map(|entry| entry.get("number").number())
        .collect();
    assert_eq!(numbers_seen, numbers(&daemon, &third.id));
    assert_eq!(
        detail(&view, &second.id).get("log_dropped"),
        &json::Value::Bool(false),
        "only the first-closed log is dropped"
    );
}

#[test]
fn case_19_a_title_is_listed_as_its_exact_value() {
    let dir = TestDir::new();
    let _daemon = start(config(&dir));
    let title = "a\"b\\cé";
    dir.set("title", title);
    let session = open(&dir);
    let view = view_url(&dir);
    let summary = listed(&sessions(&view), &session.id);
    assert_eq!(summary.get("title").str(), title);
}

#[test]
fn case_20_a_new_start_draws_a_new_token_and_the_old_one_is_refused() {
    let dir = TestDir::new();
    let daemon = start(config(&dir));
    let old = view_url(&dir);
    drop(daemon);
    let _daemon = start(config(&dir));
    let new = view_url(&dir);
    // No assertion prints a token on a failure.
    assert!(
        new.token != old.token,
        "the same view token after a new start"
    );
    assert!(new.line != old.line, "the same view-url after a new start");
    let (status, body) = get(new.port, "/api/sessions", Some(&old.token));
    assert_eq!(status, 401);
    assert_eq!(body, br#"{"error":"unauthorized"}"#);
    let (status, _) = get(new.port, "/api/sessions", Some(&new.token));
    assert_eq!(status, 200);
}

#[test]
fn case_24_a_late_answer_shows_as_before_until_its_entries_arrive() {
    let dir = TestDir::new();
    let mut config = config(&dir);
    config.session_answer_timeout = Duration::from_secs(1);
    let daemon = start(config);
    let session = open(&dir);
    let view = view_url(&dir);
    let before = get(
        view.port,
        &format!("/api/sessions/{}", session.id),
        Some(&view.token),
    );
    let highest = *numbers(&daemon, &session.id).iter().max().unwrap();
    let events = Events::open(&view, None);

    dir.set("behaviour", "stall");
    let hold = dir.hold();
    let refused = go(&dir, &session, "http://a.example/late");
    assert_eq!(refused.status, 3, "{refused:?}");
    assert_eq!(
        refused.text,
        refusal(&session.id, "open", "session-timeout")
    );
    let summary = listed(&sessions(&view), &session.id);
    assert_eq!(summary.get("state").str(), "open");
    assert_eq!(
        get(
            view.port,
            &format!("/api/sessions/{}", session.id),
            Some(&view.token)
        ),
        before,
        "the detail while the answer is late"
    );

    drop(hold);
    let received = events.until(|event| event.event == "session");
    let entries: Vec<&Event> = received
        .iter()
        .filter(|event| event.event == "entry")
        .collect();
    assert_eq!(entries.len(), 2, "{received:?}");
    assert_eq!(entries[0].json().get("kind").str(), "action");
    assert_eq!(
        entries[0].json().get("argument").str(),
        "http://a.example/late"
    );
    assert_eq!(entries[1].json().get("kind").str(), "answer");
    assert!(entries[0].id > highest && entries[1].id > entries[0].id);
    let last = received.last().unwrap();
    assert_eq!(
        last.id, entries[1].id,
        "the session event has the answer's number"
    );
    assert_eq!(last.json().get("id").str(), session.id);

    let after = detail(&view, &session.id);
    let kept: Vec<u64> = after
        .get("entries")
        .array()
        .iter()
        .map(|entry| entry.get("number").number())
        .collect();
    assert_eq!(&kept[kept.len() - 2..], [entries[0].id, entries[1].id]);
}

/// `surf-agent <words>` as `session`, with `input` on its standard input.
fn agent_with_input(dir: &TestDir, session: &Opened, input: &str, words: &[&str]) -> Output {
    let mut command = agent(dir);
    command
        .args(words)
        .env("SURF_SESSION", &session.id)
        .env("SURF_TOKEN", &session.token)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for _ in 0..50 {
        match command.spawn() {
            Err(error) if error.kind() == ErrorKind::ExecutableFileBusy => {
                thread::sleep(Duration::from_millis(20));
            }
            Err(error) => panic!("surf-agent: {error}"),
            Ok(mut child) => {
                let mut stdin = child.stdin.take().expect("a pipe");
                stdin.write_all(input.as_bytes()).unwrap();
                drop(stdin);
                return child.wait_with_output().unwrap();
            }
        }
    }
    panic!("the copied program stayed busy");
}

/// Case 2 of `story:view-controls-and-changes`: after `fill f2 -` with the password on standard
/// input and `press b1`, no byte of `/api/sessions/<id>`, of `/api/sessions` or of the stream
/// since before the session opened holds the password. The stand-in session receives it in the
/// request, as the session does, and writes its length (`12 bytes`) in its `FillRequested` entry.
#[test]
fn case_02_controls_a_filled_password_is_in_no_byte_of_the_detail_or_the_stream() {
    const SECRET: &str = "hunter2-view";
    let dir = TestDir::new();
    let _daemon = start(config(&dir));
    let view = view_url(&dir);
    let events = Events::open(&view, None);
    let session = open(&dir);
    let fill = agent_with_input(&dir, &session, SECRET, &["fill", "f2", "-"]);
    assert_eq!(fill.status.code(), Some(0), "{}", stderr(&fill));
    let press = agent_with_input(&dir, &session, "", &["press", "b1"]);
    assert_eq!(press.status.code(), Some(0), "{}", stderr(&press));
    assert!(
        dir.requests(&session.id).contains(SECRET),
        "the password reached the session through surfd"
    );

    let mut streamed = events.until(|event| {
        event.event == "entry" && event.data.contains(r#""changed":{"kind":"page""#)
    });
    streamed.push(events.event());
    assert_eq!(
        streamed.last().map(|event| event.event.as_str()),
        Some("session")
    );
    for event in &streamed {
        assert!(
            !event.data.contains(SECRET) && !event.event.contains(SECRET),
            "the stream holds the password in event {}",
            event.id
        );
    }
    assert!(
        streamed
            .iter()
            .any(|event| event.data.contains(r#""value":"12 bytes""#)),
        "the FillRequested entry with the length is on the stream"
    );

    let (status, detail) = get(
        view.port,
        &format!("/api/sessions/{}", session.id),
        Some(&view.token),
    );
    assert_eq!(status, 200);
    let detail = String::from_utf8(detail).expect("UTF-8");
    assert!(!detail.contains(SECRET), "the detail holds the password");
    assert!(detail.contains(r#""value":"12 bytes""#), "{detail}");
    assert!(
        detail.contains(r#""value":{"kind":"set","text":null,"cut":null}"#),
        "{detail}"
    );
    assert!(
        detail.contains(r#""changed":{"kind":"controls","lines":[{"handle":"f2","what":"value","#),
        "{detail}"
    );
    let (status, list) = get(view.port, "/api/sessions", Some(&view.token));
    assert_eq!(status, 200);
    assert!(
        !String::from_utf8_lossy(&list).contains(SECRET),
        "the list holds the password"
    );
}

#[test]
fn case_27_the_idle_exit_ends_the_stream_and_removes_view_url() {
    let dir = TestDir::new();
    let mut surfd = Surfd::start(&dir, &["--idle-exit", "1"], |_| {});
    assert_eq!(surfd.line, "ready\n");
    let view = view_url(&dir);
    let events = Events::open(&view, None);
    let status = surfd
        .ends_within(Duration::from_secs(5))
        .expect("surfd exits within 5 s with a stream open");
    assert!(status.success(), "{status:?}");
    // The first thing the stream gives is its end: no event comes before it.
    match events.next(ALLOWED) {
        Got::Ended => {}
        Got::Event(event) => panic!("an event without a session: {event:?}"),
        Got::Nothing => panic!("the stream's connection did not end"),
    }
    assert!(
        fs::symlink_metadata(dir.surf().join("view-url")).is_err(),
        "view-url is left"
    );
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
