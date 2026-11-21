//! Cases 1 to 8 of `story:view-page-prototype`, against the example program `prototype` that
//! `cargo test` builds (`target/<profile>/examples/prototype`, found from this test's own
//! executable). Each case starts the program, reads its one line, speaks to it over
//! `std::net::TcpStream` with the token of that line, and kills it at its end.

#![forbid(unsafe_code)]

use std::fs;
use std::io::{BufRead, BufReader, ErrorKind, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::OnceLock;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant, SystemTime};

const WAIT: Duration = Duration::from_secs(10);
const NINE: [&str; 9] = [
    "dom.js",
    "index.html",
    "page.js",
    "sessions.js",
    "state.js",
    "stream.js",
    "timeline.js",
    "view.css",
    "view.js",
];

// ---------------------------------------------------------------------------------------------
// The program.

fn web() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("web")
}

/// The newest modification time of the files the example is built from.
fn newest_source() -> SystemTime {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut newest = SystemTime::UNIX_EPOCH;
    for dir in [crate_dir.join("src"), crate_dir.join("web")] {
        for entry in fs::read_dir(&dir).expect("read a source directory") {
            let modified = entry
                .expect("directory entry")
                .metadata()
                .and_then(|m| m.modified())
                .expect("modification time");
            newest = newest.max(modified);
        }
    }
    let example = fs::metadata(crate_dir.join("examples/prototype.rs"))
        .and_then(|m| m.modified())
        .expect("examples/prototype.rs");
    newest.max(example)
}

/// Whether the example at `path` exists and is not older than a file it is built from.
fn fresh(path: &Path) -> bool {
    fs::metadata(path)
        .and_then(|m| m.modified())
        .is_ok_and(|built| built >= newest_source())
}

/// The example that `cargo test` built, beside this test's directory
/// (`target/<profile>/examples/prototype`). `cargo test -p surf-view` builds it; `cargo test
/// --test prototype` alone does not (measured with cargo 1.99.1), so when it is missing or
/// older than a file it is built from, this builds it once with the cargo running these tests,
/// into the same directory. No case runs an old program.
fn program() -> PathBuf {
    static BUILT: OnceLock<PathBuf> = OnceLock::new();
    BUILT
        .get_or_init(|| {
            let exe = std::env::current_exe().expect("the test's executable");
            let profile_dir = exe
                .parent()
                .and_then(Path::parent)
                .expect("target/<profile>");
            let path = profile_dir.join("examples/prototype");
            if fresh(&path) {
                return path;
            }
            let profile = match profile_dir.file_name().and_then(|name| name.to_str()) {
                Some("debug") => "dev",
                Some(other) => other,
                None => panic!("no profile directory above {}", exe.display()),
            };
            let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
            let status = Command::new(env!("CARGO"))
                .args([
                    "build",
                    "--quiet",
                    "--example",
                    "prototype",
                    "--profile",
                    profile,
                ])
                .arg("--manifest-path")
                .arg(&manifest)
                .stdin(Stdio::null())
                .status()
                .expect("start cargo");
            assert!(status.success(), "building the example failed: {status}");
            assert!(
                fresh(&path),
                "{} is still missing or older than its sources after the build",
                path.display()
            );
            path
        })
        .clone()
}

/// A running example, killed when dropped.
struct Running {
    child: Child,
    lines: Receiver<String>,
    line: String,
    port: u16,
    token: String,
}

impl Running {
    fn start() -> Self {
        let mut child = Command::new(program())
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("start the example");
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
        let mut running = Self {
            child,
            lines,
            line: String::new(),
            port: 0,
            token: String::new(),
        };
        let line = match running.lines.recv_timeout(Duration::from_secs(5)) {
            Ok(line) => line,
            Err(error) => panic!("no line from the example within 5 s: {error:?}"),
        };
        let (port, token) = address(&line).unwrap_or_else(|| {
            panic!("the line is not http://127.0.0.1:<port>/#<64 hex>: {line:?}")
        });
        running.line = line;
        running.port = port;
        running.token = token;
        running
    }

    fn running(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(None))
    }

    fn get(&self, target: &str) -> Answer {
        get(self.port, target, Some(&self.token), None)
    }
}

impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// `http://127.0.0.1:<port>/#` and 64 lower-case hexadecimal digits: the port and the token.
fn address(line: &str) -> Option<(u16, String)> {
    let rest = line.strip_prefix("http://127.0.0.1:")?;
    let (port, token) = rest.split_once("/#")?;
    if port.is_empty() || !port.bytes().all(|b| b.is_ascii_digit()) || port.starts_with('0') {
        return None;
    }
    let port: u16 = port.parse().ok()?;
    let hex = token.len() == 64
        && token
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
    hex.then(|| (port, token.to_string()))
}

// ---------------------------------------------------------------------------------------------
// HTTP over a TcpStream.

struct Answer {
    status: u16,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl Answer {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }

    fn text(&self) -> &str {
        std::str::from_utf8(&self.body).expect("a UTF-8 body")
    }
}

fn request(port: u16, target: &str, token: Option<&str>, last_event_id: Option<&str>) -> Vec<u8> {
    let mut text = format!("GET {target} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n");
    if let Some(token) = token {
        text.push_str(&format!("Authorization: Bearer {token}\r\n"));
    }
    if let Some(id) = last_event_id {
        text.push_str(&format!("Last-Event-ID: {id}\r\n"));
    }
    text.push_str("\r\n");
    text.into_bytes()
}

fn get(port: u16, target: &str, token: Option<&str>, last_event_id: Option<&str>) -> Answer {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    stream.set_read_timeout(Some(WAIT)).expect("read timeout");
    stream
        .write_all(&request(port, target, token, last_event_id))
        .expect("send the request");
    let mut bytes = Vec::new();
    stream.read_to_end(&mut bytes).expect("read the answer");
    split(&bytes)
}

fn split(bytes: &[u8]) -> Answer {
    let end = bytes
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .expect("the end of the head");
    let head = std::str::from_utf8(&bytes[..end]).expect("an ASCII head");
    let mut lines = head.split("\r\n");
    let status_line = lines.next().expect("a status line");
    let status = status_line
        .split(' ')
        .nth(1)
        .and_then(|code| code.parse().ok())
        .expect("a status code");
    let headers = lines
        .map(|line| {
            let (name, value) = line.split_once(':').expect("a header line");
            (name.to_string(), value.trim().to_string())
        })
        .collect();
    Answer {
        status,
        headers,
        body: bytes[end + 4..].to_vec(),
    }
}

/// One server-sent event.
#[derive(Debug)]
struct Event {
    id: u64,
    name: String,
    data: String,
}

/// An open `/events` stream.
struct Stream {
    stream: TcpStream,
    buffer: Vec<u8>,
}

impl Stream {
    fn open(running: &Running, last_event_id: Option<&str>) -> Self {
        let mut stream = TcpStream::connect(("127.0.0.1", running.port)).expect("connect");
        stream
            .set_read_timeout(Some(Duration::from_millis(200)))
            .expect("read timeout");
        stream
            .write_all(&request(
                running.port,
                "/events",
                Some(&running.token),
                last_event_id,
            ))
            .expect("send the request");
        let mut opened = Self {
            stream,
            buffer: Vec::new(),
        };
        let deadline = Instant::now() + WAIT;
        let end = loop {
            if let Some(end) = opened.buffer.windows(4).position(|w| w == b"\r\n\r\n") {
                break end;
            }
            assert!(opened.fill(deadline), "no head of /events within {WAIT:?}");
        };
        let answer = split(&opened.buffer[..end + 4]);
        assert_eq!(answer.status, 200, "/events answers 200");
        assert_eq!(answer.header("Content-Type"), Some("text/event-stream"));
        opened.buffer.drain(..end + 4);
        opened
    }

    /// Reads more bytes; false once `deadline` passed or the stream ended.
    fn fill(&mut self, deadline: Instant) -> bool {
        let mut chunk = [0u8; 8192];
        while Instant::now() < deadline {
            match self.stream.read(&mut chunk) {
                Ok(0) => return false,
                Ok(n) => {
                    self.buffer.extend_from_slice(&chunk[..n]);
                    return true;
                }
                Err(error)
                    if matches!(
                        error.kind(),
                        ErrorKind::WouldBlock | ErrorKind::TimedOut | ErrorKind::Interrupted
                    ) => {}
                Err(_) => return false,
            }
        }
        false
    }

    /// The next event before `deadline`, comment blocks skipped.
    fn next(&mut self, deadline: Instant) -> Option<Event> {
        loop {
            if let Some(end) = self.buffer.windows(2).position(|w| w == b"\n\n") {
                let block: Vec<u8> = self.buffer.drain(..end + 2).collect();
                let block = String::from_utf8(block).expect("UTF-8 events");
                let mut id = None;
                let mut name = None;
                let mut data = None;
                for line in block.lines() {
                    if line.starts_with(':') {
                        continue;
                    }
                    if let Some(value) = line.strip_prefix("id: ") {
                        id = Some(value.parse().expect("a numeric id"));
                    } else if let Some(value) = line.strip_prefix("event: ") {
                        name = Some(value.to_string());
                    } else if let Some(value) = line.strip_prefix("data: ") {
                        data = Some(value.to_string());
                    } else if !line.is_empty() {
                        panic!("a line outside the framing: {line:?}");
                    }
                }
                match (id, name, data) {
                    (Some(id), Some(name), Some(data)) => return Some(Event { id, name, data }),
                    (None, None, None) => continue,
                    other => panic!("an incomplete event: {other:?}"),
                }
            }
            if !self.fill(deadline) {
                return None;
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// A JSON reader that keeps the order of members.

#[derive(Clone, Debug, PartialEq)]
enum Json {
    Null,
    Bool(bool),
    Number(u64),
    Text(String),
    List(Vec<Json>),
    Object(Vec<(String, Json)>),
}

impl Json {
    fn parse(text: &str) -> Json {
        let mut reader = JsonReader {
            bytes: text.as_bytes(),
            at: 0,
        };
        let value = reader.value();
        assert_eq!(reader.at, text.len(), "text after the JSON value: {text}");
        value
    }

    fn get(&self, name: &str) -> &Json {
        match self {
            Json::Object(members) => members
                .iter()
                .find(|(n, _)| n == name)
                .map(|(_, v)| v)
                .unwrap_or_else(|| panic!("no member {name} in {self:?}")),
            _ => panic!("not an object: {self:?}"),
        }
    }

    fn names(&self) -> Vec<&str> {
        match self {
            Json::Object(members) => members.iter().map(|(n, _)| n.as_str()).collect(),
            _ => panic!("not an object: {self:?}"),
        }
    }

    fn list(&self) -> &[Json] {
        match self {
            Json::List(items) => items,
            _ => panic!("not a list: {self:?}"),
        }
    }

    fn text(&self) -> &str {
        match self {
            Json::Text(text) => text,
            _ => panic!("not a string: {self:?}"),
        }
    }

    fn number(&self) -> u64 {
        match self {
            Json::Number(number) => *number,
            _ => panic!("not a number: {self:?}"),
        }
    }
}

struct JsonReader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl JsonReader<'_> {
    fn peek(&self) -> u8 {
        *self.bytes.get(self.at).expect("more JSON")
    }

    fn expect(&mut self, text: &str) {
        assert!(
            self.bytes[self.at..].starts_with(text.as_bytes()),
            "expected {text} at {}",
            self.at
        );
        self.at += text.len();
    }

    fn value(&mut self) -> Json {
        match self.peek() {
            b'n' => {
                self.expect("null");
                Json::Null
            }
            b't' => {
                self.expect("true");
                Json::Bool(true)
            }
            b'f' => {
                self.expect("false");
                Json::Bool(false)
            }
            b'"' => Json::Text(self.string()),
            b'[' => {
                self.at += 1;
                let mut items = Vec::new();
                if self.peek() == b']' {
                    self.at += 1;
                    return Json::List(items);
                }
                loop {
                    items.push(self.value());
                    match self.peek() {
                        b',' => self.at += 1,
                        b']' => {
                            self.at += 1;
                            return Json::List(items);
                        }
                        other => panic!("unexpected {:?} in a list", other as char),
                    }
                }
            }
            b'{' => {
                self.at += 1;
                let mut members = Vec::new();
                if self.peek() == b'}' {
                    self.at += 1;
                    return Json::Object(members);
                }
                loop {
                    let name = self.string();
                    self.expect(":");
                    members.push((name, self.value()));
                    match self.peek() {
                        b',' => self.at += 1,
                        b'}' => {
                            self.at += 1;
                            return Json::Object(members);
                        }
                        other => panic!("unexpected {:?} in an object", other as char),
                    }
                }
            }
            b'0'..=b'9' => {
                let start = self.at;
                while self.bytes.get(self.at).is_some_and(u8::is_ascii_digit) {
                    self.at += 1;
                }
                let digits = std::str::from_utf8(&self.bytes[start..self.at]).expect("digits");
                Json::Number(digits.parse().expect("a number that fits"))
            }
            other => panic!("unexpected {:?}", other as char),
        }
    }

    fn string(&mut self) -> String {
        self.expect("\"");
        let mut text = String::new();
        loop {
            let rest = std::str::from_utf8(&self.bytes[self.at..]).expect("UTF-8");
            let c = rest.chars().next().expect("a closed string");
            self.at += c.len_utf8();
            match c {
                '"' => return text,
                '\\' => {
                    let escape = self.peek();
                    self.at += 1;
                    match escape {
                        b'"' => text.push('"'),
                        b'\\' => text.push('\\'),
                        b'/' => text.push('/'),
                        b'n' => text.push('\n'),
                        b'r' => text.push('\r'),
                        b't' => text.push('\t'),
                        b'b' => text.push('\u{8}'),
                        b'f' => text.push('\u{c}'),
                        b'u' => {
                            let hex = std::str::from_utf8(&self.bytes[self.at..self.at + 4])
                                .expect("hex digits");
                            self.at += 4;
                            let unit = u32::from_str_radix(hex, 16).expect("hex digits");
                            text.push(char::from_u32(unit).expect("not a surrogate"));
                        }
                        other => panic!("unknown escape {:?}", other as char),
                    }
                }
                c => {
                    assert!(c >= ' ', "a raw control character in a string");
                    text.push(c);
                }
            }
        }
    }
}

const SUMMARY: [&str; 8] = [
    "id",
    "state",
    "address",
    "title",
    "opened_at",
    "last_action_at",
    "ended_at",
    "page",
];

fn sessions(running: &Running) -> (Answer, Json) {
    let answer = running.get("/api/sessions");
    assert_eq!(answer.status, 200, "/api/sessions answers 200");
    let json = Json::parse(answer.text());
    (answer, json)
}

fn assert_no_raw_markup(what: &str, text: &str) {
    for c in ['<', '>', '&'] {
        assert!(!text.contains(c), "{what} holds a raw {c}: {text}");
    }
}

// ---------------------------------------------------------------------------------------------
// The cases.

#[test]
fn case_01_prints_one_address_line_and_keeps_running() {
    let mut running = Running::start();
    assert!(address(&running.line).is_some());
    thread::sleep(Duration::from_secs(2));
    assert!(
        running.running(),
        "the example ended within 2 s of its line"
    );
    match running.lines.recv_timeout(Duration::from_millis(10)) {
        Err(RecvTimeoutError::Timeout) => {}
        other => panic!("a second line or a closed output: {other:?}"),
    }
}

#[test]
fn case_02_root_answers_index_html() {
    let running = Running::start();
    let answer = get(running.port, "/", None, None);
    assert_eq!(answer.status, 200);
    assert_eq!(
        answer.header("Content-Type"),
        Some("text/html; charset=utf-8")
    );
    let file = fs::read(web().join("index.html")).expect("read index.html");
    assert_eq!(answer.body, file, "/ answers the bytes of web/index.html");
}

#[test]
fn case_03_each_page_file_answers_under_its_name_with_its_type() {
    let files = surf_view::page_files();
    let mut names: Vec<&str> = files.iter().map(|file| file.name.as_str()).collect();
    names.sort_unstable();
    assert_eq!(
        names, NINE,
        "page_files() holds the nine files of the story"
    );
    let mut on_disk: Vec<String> = fs::read_dir(web())
        .expect("read web/")
        .map(|entry| {
            entry
                .expect("entry")
                .file_name()
                .into_string()
                .expect("UTF-8")
        })
        .collect();
    on_disk.sort();
    assert_eq!(on_disk, NINE, "web/ holds the nine files and no other");

    let running = Running::start();
    for file in &files {
        let expected_type = match file.name.rsplit_once('.').map(|(_, ext)| ext) {
            Some("html") => "text/html; charset=utf-8",
            Some("css") => "text/css; charset=utf-8",
            Some("js") => "text/javascript; charset=utf-8",
            other => panic!("a page file of type {other:?}"),
        };
        assert_eq!(file.content_type, expected_type, "{}", file.name);
        let target = if file.name == "index.html" {
            "/".to_string()
        } else {
            format!("/{}", file.name)
        };
        let answer = get(running.port, &target, None, None);
        assert_eq!(answer.status, 200, "{target}");
        assert_eq!(
            answer.header("Content-Type"),
            Some(expected_type),
            "{target}"
        );
        let bytes = fs::read(web().join(&file.name)).expect("read the page file");
        assert_eq!(answer.body, bytes, "{target} answers its file's bytes");
        assert_eq!(
            file.bytes, bytes,
            "page_files() holds {}'s bytes",
            file.name
        );
    }
}

#[test]
fn case_04_session_list_holds_three_sessions_in_three_states() {
    let running = Running::start();
    let (answer, json) = sessions(&running);
    assert_eq!(answer.header("Content-Type"), Some("application/json"));
    assert!(
        answer.text().starts_with("{\"newest\":"),
        "{}",
        answer.text()
    );
    assert_eq!(json.names(), ["newest", "sessions"]);
    let list = json.get("sessions").list();
    assert_eq!(list.len(), 3, "three sessions");
    let mut states: Vec<&str> = list.iter().map(|s| s.get("state").text()).collect();
    states.sort_unstable();
    assert_eq!(states, ["closed", "died", "open"]);
    for summary in list {
        assert_eq!(
            summary.names(),
            SUMMARY,
            "the members of a summary, in order"
        );
    }
}

#[test]
fn case_05_each_session_detail_holds_its_entries_in_the_order_of_d19() {
    let running = Running::start();
    let (_, json) = sessions(&running);
    for summary in json.get("sessions").list() {
        let id = summary.get("id").text();
        let answer = running.get(&format!("/api/sessions/{id}"));
        assert_eq!(answer.status, 200, "/api/sessions/{id}");
        assert_eq!(answer.header("Content-Type"), Some("application/json"));
        let detail = Json::parse(answer.text());
        assert_eq!(
            detail.names(),
            ["session", "dropped", "log_dropped", "unreadable", "entries"]
        );
        assert_eq!(detail.get("session").get("id").text(), id);
        let entries = detail.get("entries").list();
        assert!(!entries.is_empty(), "{id} has entries");
        let mut last = 0;
        let mut kinds = String::new();
        for entry in entries {
            assert_eq!(
                entry.names()[..5],
                ["number", "session", "index", "kept_at", "kind"],
                "the first members of an entry"
            );
            assert_eq!(entry.get("session").text(), id);
            let number = entry.get("number").number();
            assert!(number > last, "number increases: {number} after {last}");
            last = number;
            kinds.push(match entry.get("kind").text() {
                "action" => 'a',
                "event" => 'e',
                "answer" => 'r',
                other => panic!("an entry of kind {other}"),
            });
        }
        // `action`, any number of `event`, `answer`, repeated.
        let mut rest = kinds.as_str();
        while !rest.is_empty() {
            rest = rest
                .strip_prefix('a')
                .unwrap_or_else(|| panic!("{id}: no action entry where one starts: {kinds}"));
            rest = rest.trim_start_matches('e');
            rest = rest
                .strip_prefix('r')
                .unwrap_or_else(|| panic!("{id}: an action without its answer: {kinds}"));
        }
    }
}

#[test]
fn case_06_stream_without_last_event_id_delivers_live_entries_and_sessions() {
    let running = Running::start();
    let (_, json) = sessions(&running);
    let newest = json.get("newest").number();
    let mut stream = Stream::open(&running, None);
    let deadline = Instant::now() + Duration::from_secs(3);
    let mut first_entry = None;
    let mut answer = None;
    let mut session_after_answer = false;
    while let Some(event) = stream.next(deadline) {
        match event.name.as_str() {
            "entry" => {
                if first_entry.is_none() {
                    first_entry = Some(event.id);
                }
                let entry = Json::parse(&event.data);
                assert_eq!(entry.get("number").number(), event.id);
                if entry.get("kind").text() == "answer" {
                    answer = Some(event.id);
                }
            }
            "session" => {
                if answer == Some(event.id) {
                    let summary = Json::parse(&event.data);
                    assert_eq!(summary.names(), SUMMARY);
                    session_after_answer = true;
                    break;
                }
            }
            other => panic!("an event {other} on a live stream"),
        }
    }
    let first = first_entry.expect("an entry within 3 s");
    assert!(
        first > newest,
        "the first entry {first} is above newest {newest}"
    );
    assert!(
        session_after_answer,
        "a session event with the id of an answer entry within 3 s"
    );
}

#[test]
fn case_07_stream_from_zero_starts_at_one() {
    let running = Running::start();
    let mut stream = Stream::open(&running, Some("0"));
    let event = stream
        .next(Instant::now() + Duration::from_secs(3))
        .expect("an event within 3 s");
    assert_eq!(event.name, "entry");
    assert_eq!(event.id, 1);
}

#[test]
fn case_08_markup_in_values_is_written_escaped() {
    let running = Running::start();
    let (answer, json) = sessions(&running);
    assert_no_raw_markup("/api/sessions", answer.text());
    let died = json
        .get("sessions")
        .list()
        .iter()
        .find(|s| s.get("state").text() == "died")
        .expect("the died session");
    let title = died.get("title").text();
    assert!(
        title.contains("<script>alert(1)</script>") && title.contains('"'),
        "the died session's title: {title}"
    );
    assert!(
        answer
            .text()
            .contains("\\u003cscript\\u003ealert(1)\\u003c/script\\u003e"),
        "<script> is written as \\u003cscript\\u003e"
    );
    assert!(
        answer.text().contains("\\u0022"),
        "\" is written as \\u0022"
    );

    let mut newest = 0;
    for summary in json.get("sessions").list() {
        let id = summary.get("id").text();
        let detail = running.get(&format!("/api/sessions/{id}"));
        assert_no_raw_markup(&format!("/api/sessions/{id}"), detail.text());
        for entry in Json::parse(detail.text()).get("entries").list() {
            newest = newest.max(entry.get("number").number());
        }
    }

    // Every kept entry from 1, and live entries and sessions after them.
    let mut stream = Stream::open(&running, Some("0"));
    let deadline = Instant::now() + Duration::from_secs(4);
    let mut events = 0;
    let mut live_session = false;
    while let Some(event) = stream.next(deadline) {
        assert_no_raw_markup(&format!("the data of event {}", event.id), &event.data);
        events += 1;
        if event.name == "session" && event.id > newest {
            live_session = true;
            break;
        }
    }
    assert!(events as u64 > newest, "the replay and a live event");
    assert!(live_session, "a live session event within 4 s");
}
