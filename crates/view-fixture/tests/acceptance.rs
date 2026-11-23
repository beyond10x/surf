//! The HTTP part of the live view's acceptance case (`epic:live-view`, `story:view-acceptance`):
//! cases 1 to 3, each named after its sentence, against a `surf-view-fixture` the test starts (a
//! page server, `surfd` in process, sessions) over `std::net::TcpStream`. The page part is
//! `view-tests/tests/acceptance.spec.mjs`. The answers are the ones `docs/view-stream.md` gives.
//! No fixed sleep: every wait is on a line, an answer or an event, bounded by [`WAIT`].

#![forbid(unsafe_code)]

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, Instant};

const WAIT: Duration = Duration::from_secs(10);

/// The running fixture, with the view's port, the view token and the page server's address.
/// Dropped: its standard input is closed, so it stops `surfd`, its sessions and the page server;
/// killed only when it has not ended within [`WAIT`].
struct Fixture {
    child: Child,
    stdin: Option<ChildStdin>,
    lines: Receiver<String>,
    port: u16,
    token: String,
    pages: String,
}

impl Fixture {
    fn start() -> Fixture {
        let bin = crate::common::bin_dir();
        for program in ["surf-agent", "surf-session"] {
            assert!(bin.join(program).is_file(), "{program} beside the fixture");
        }
        let mut child = Command::new(env!("CARGO_BIN_EXE_surf-view-fixture"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("start the fixture");
        let stdin = child.stdin.take();
        let stdout = child.stdout.take().expect("its standard output");
        let (sender, lines) = mpsc::channel();
        thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else { return };
                if sender.send(line).is_err() {
                    return;
                }
            }
        });
        let mut fixture = Fixture {
            child,
            stdin,
            lines,
            port: 0,
            token: String::new(),
            pages: String::new(),
        };
        let first = fixture.line();
        let view = member(&first, "view");
        let rest = view
            .strip_prefix("http://127.0.0.1:")
            .unwrap_or_else(|| panic!("not a view address: {view}"));
        let (port, token) = rest.split_once("/#").expect("a token fragment");
        fixture.port = port.parse().expect("a port");
        fixture.token = token.to_string();
        fixture.pages = member(&first, "pages");
        fixture
    }

    fn line(&self) -> String {
        self.lines
            .recv_timeout(WAIT)
            .unwrap_or_else(|error| panic!("no line from the fixture within {WAIT:?}: {error:?}"))
    }

    fn command(&mut self, words: &[&str]) -> String {
        let stdin = self.stdin.as_mut().expect("standard input open");
        writeln!(stdin, "{}", words.join("\t")).expect("send a command");
        stdin.flush().expect("flush");
        self.line()
    }

    /// `open`: the new session's identity.
    fn open(&mut self) -> String {
        let id = member(&self.command(&["open"]), "id");
        assert_eq!(id.len(), 36, "a session identity: {id}");
        id
    }

    /// `surf-agent go <url>` as session `id`, which must exit 0.
    fn go(&mut self, id: &str, url: &str) {
        assert_eq!(self.command(&["agent", id, "go", url]), "{\"status\":0}");
    }

    /// `GET <target>` with `host` and, when given, the token: the whole answer as text.
    fn get(&self, target: &str, host: &str, token: Option<&str>) -> Answer {
        let mut stream = TcpStream::connect(("127.0.0.1", self.port)).expect("connect");
        stream.set_read_timeout(Some(WAIT)).expect("read timeout");
        let mut request = format!("GET {target} HTTP/1.1\r\nHost: {host}\r\n");
        if let Some(token) = token {
            request.push_str(&format!("Authorization: Bearer {token}\r\n"));
        }
        request.push_str("\r\n");
        stream
            .write_all(request.as_bytes())
            .expect("send the request");
        let mut bytes = Vec::new();
        stream.read_to_end(&mut bytes).expect("read the answer");
        Answer::of(&String::from_utf8(bytes).expect("a UTF-8 answer"))
    }

    fn host(&self) -> String {
        format!("127.0.0.1:{}", self.port)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        drop(self.stdin.take());
        let deadline = Instant::now() + WAIT;
        while matches!(self.child.try_wait(), Ok(None)) {
            if Instant::now() >= deadline {
                let _ = self.child.kill();
                let _ = self.child.wait();
                return;
            }
            thread::sleep(Duration::from_millis(20));
        }
    }
}

/// One answer: its status, its head lines without the status line, and its body.
struct Answer {
    status: u16,
    head: Vec<String>,
    body: String,
}

impl Answer {
    fn of(text: &str) -> Answer {
        let (head, body) = text.split_once("\r\n\r\n").expect("a head and a body");
        let mut lines = head.split("\r\n");
        let status = lines
            .next()
            .and_then(|line| line.split(' ').nth(1))
            .and_then(|code| code.parse().ok())
            .expect("a status code");
        Answer {
            status,
            head: lines.map(str::to_string).collect(),
            body: body.to_string(),
        }
    }
}

/// The string value of member `name` in a one-line JSON object, up to its closing quote.
fn member(json: &str, name: &str) -> String {
    let start = json
        .find(&format!("\"{name}\":\""))
        .unwrap_or_else(|| panic!("no member {name} in {json}"))
        + name.len()
        + 4;
    let end = json[start..].find('"').expect("a closed string") + start;
    json[start..end].to_string()
}

/// The number after `"<name>":` at the start of `json`'s first such member.
fn number(json: &str, name: &str) -> u64 {
    let key = format!("\"{name}\":");
    let start = json
        .find(&key)
        .unwrap_or_else(|| panic!("no member {name} in {json}"))
        + key.len();
    let digits: String = json[start..]
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    digits.parse().expect("a number")
}

/// The JSON of each entry of a session detail, in its order: each starts at `{"number":`.
fn entries_of(detail: &str) -> Vec<String> {
    let starts: Vec<usize> = detail
        .match_indices("{\"number\":")
        .map(|(at, _)| at)
        .collect();
    let end = detail.len() - "]}".len();
    starts
        .iter()
        .enumerate()
        .map(|(i, &start)| {
            let stop = starts.get(i + 1).map_or(end, |next| next - 1);
            detail[start..stop].to_string()
        })
        .collect()
}

/// One event of the stream: its `id:`, its `event:` and its `data:`.
struct Event {
    id: u64,
    name: String,
    data: String,
}

/// Reads one event of `reader`, skipping comment lines.
fn next_event(reader: &mut impl BufRead) -> Event {
    let (mut id, mut name, mut data) = (None, None, None);
    loop {
        let mut line = String::new();
        let read = reader.read_line(&mut line).expect("a line of the stream");
        assert!(read > 0, "the stream ended");
        let line = line.trim_end_matches('\n');
        if line.is_empty() {
            if let (Some(id), Some(name), Some(data)) = (id.take(), name.take(), data.take()) {
                return Event { id, name, data };
            }
            continue;
        }
        if let Some(value) = line.strip_prefix("id: ") {
            id = Some(value.parse().expect("a numeric id"));
        } else if let Some(value) = line.strip_prefix("event: ") {
            name = Some(value.to_string());
        } else if let Some(value) = line.strip_prefix("data: ") {
            data = Some(value.to_string());
        }
    }
}

#[test]
fn api_sessions_lists_both_with_token_and_401_without() {
    let mut fixture = Fixture::start();
    let first = fixture.open();
    let second = fixture.open();
    let host = fixture.host();
    let token = fixture.token.clone();

    let with = fixture.get("/api/sessions", &host, Some(&token));
    assert_eq!(with.status, 200, "{}", with.body);
    for id in [&first, &second] {
        assert!(
            with.body
                .contains(&format!("{{\"id\":\"{id}\",\"state\":\"open\"")),
            "{id} listed open: {}",
            with.body
        );
    }
    assert_eq!(with.body.matches("{\"id\":").count(), 2, "{}", with.body);

    let without = fixture.get("/api/sessions", &host, None);
    assert_eq!(without.status, 401, "{}", without.body);
    assert!(
        without
            .head
            .iter()
            .any(|line| line == "WWW-Authenticate: Bearer"),
        "{:?}",
        without.head
    );
    assert_eq!(without.body, "{\"error\":\"unauthorized\"}");
}

#[test]
fn request_with_another_host_is_refused() {
    let mut fixture = Fixture::start();
    fixture.open();
    let token = fixture.token.clone();
    for host in [
        format!("localhost:{}", fixture.port),
        format!("127.0.0.1:{}", fixture.port.wrapping_add(1)),
        "example.org".to_string(),
    ] {
        let refused = fixture.get("/api/sessions", &host, Some(&token));
        assert_eq!(refused.status, 403, "Host: {host}: {}", refused.body);
        assert_eq!(refused.body, "{\"error\":\"host\"}", "Host: {host}");
    }
}

#[test]
fn events_deliver_a_go_in_order() {
    let mut fixture = Fixture::start();
    let id = fixture.open();
    let mut stream = TcpStream::connect(("127.0.0.1", fixture.port)).expect("connect");
    stream.set_read_timeout(Some(WAIT)).expect("read timeout");
    write!(
        stream,
        "GET /events HTTP/1.1\r\nHost: {}\r\nAuthorization: Bearer {}\r\n\r\n",
        fixture.host(),
        fixture.token
    )
    .expect("send the request");
    let mut reader = BufReader::new(stream);
    // The head: the server subscribes before it writes it, so the go below is live.
    let mut status = String::new();
    reader.read_line(&mut status).expect("the status line");
    assert_eq!(status, "HTTP/1.1 200 OK\r\n");
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).expect("a head line");
        if line == "\r\n" {
            break;
        }
    }

    let url = format!("{}/acceptance/start.html", fixture.pages);
    fixture.go(&id, &url);

    // The entries of the go, as they arrive, up to its answer entry; `session` events between.
    let mut streamed: Vec<String> = Vec::new();
    let mut last_id = 0;
    loop {
        let event = next_event(&mut reader);
        assert!(event.id >= last_id, "id {} after {last_id}", event.id);
        if event.name == "session" {
            continue;
        }
        assert_eq!(event.name, "entry", "{}", event.data);
        assert!(event.id > last_id, "entry id {} after {last_id}", event.id);
        assert_eq!(number(&event.data, "number"), event.id, "{}", event.data);
        last_id = event.id;
        let answer = event.data.contains("\"kind\":\"answer\"");
        streamed.push(event.data);
        if answer {
            break;
        }
    }

    let first = &streamed[0];
    assert!(
        first.contains(&format!(
            "\"kind\":\"action\",\"sequence\":2,\"command\":\"go\",\"argument\":\"{url}\"}}"
        )),
        "the go's action entry first: {first}"
    );
    let answer = &streamed[streamed.len() - 1];
    assert!(
        answer.contains(&format!(
            "\"page\":{{\"kind\":\"loaded\",\"status\":200,\"media_type\":\"text/html\",\"address\":\"{url}\""
        )),
        "the go's answer entry last, its page loaded: {answer}"
    );
    assert!(streamed.len() > 2, "events between: {}", streamed.len());
    for event in &streamed[1..streamed.len() - 1] {
        assert!(event.contains("\"kind\":\"event\""), "{event}");
    }

    // The detail lists the same entries, byte for byte, in the same order (D19), and none
    // between the go's action entry and its answer entry that the stream did not deliver.
    let detail = fixture.get(
        &format!("/api/sessions/{id}"),
        &fixture.host(),
        Some(&fixture.token),
    );
    assert_eq!(detail.status, 200, "{}", detail.body);
    let listed = entries_of(&detail.body);
    let at = listed
        .iter()
        .position(|entry| entry == first)
        .unwrap_or_else(|| panic!("the go's action entry in the detail: {}", detail.body));
    assert_eq!(
        listed[at..at + streamed.len()],
        streamed[..],
        "the stream's entries are the detail's, in its order"
    );
    assert_eq!(
        listed.len(),
        at + streamed.len(),
        "the go's answer ends the detail"
    );
}
