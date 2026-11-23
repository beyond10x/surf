//! The 33 cases of `story:view-http-server`, each named after its number. Every case starts its
//! own server on 127.0.0.1 port 0 with a source of its own and speaks to it over
//! `std::net::TcpStream`. `docs/view-stream.md` is what the server answers; case 24 reads its
//! examples from that file.

#![forbid(unsafe_code)]

use std::collections::{HashMap, VecDeque};
use std::io::{ErrorKind, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use surf_secret::Token;
use surf_view::{
    Config, EventKind, Item, Limits, Next, PageFile, Reader, Server, Source, TooOld, serve,
};

const TOKEN: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const ID_A: &str = "5f0c2a9e-3b1d-4c6e-8a7f-1e2d3c4b5a69";
const ID_B: &str = "0b7d4e21-9c3a-4f58-b6e2-7a1d9c8e5f30";
const INDEX: &[u8] = b"<!doctype html>\n<title>surf</title>\n";
const VIEW_JS: &[u8] = b"export {};\n";
const VIEW_CSS: &[u8] = b"body{margin:0}\n";
const LIST: &[u8] =
    b"{\"newest\":0,\"sessions\":[{\"id\":\"5f0c2a9e-3b1d-4c6e-8a7f-1e2d3c4b5a69\"}]}";
const DETAIL: &[u8] = b"{\"session\":{\"id\":\"5f0c2a9e-3b1d-4c6e-8a7f-1e2d3c4b5a69\"}}";
const SECURITY: [&str; 5] = [
    "Content-Security-Policy: default-src 'none'; script-src 'self'; style-src 'self'; connect-src 'self'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'",
    "X-Content-Type-Options: nosniff",
    "Referrer-Policy: no-referrer",
    "Cache-Control: no-store",
    "Cross-Origin-Resource-Policy: same-origin",
];
const WAIT: Duration = Duration::from_secs(10);

// ---------------------------------------------------------------------------------------------
// The source the tests supply.

#[derive(Default)]
struct State {
    sessions: Vec<u8>,
    session: HashMap<String, Vec<u8>>,
    newest: u64,
    /// Entries numbered at most this were dropped: a reader from below it is too old.
    dropped_through: u64,
    kept: Vec<Item>,
    live: Vec<Sender<Item>>,
    calls: usize,
    read_from: Vec<u64>,
}

#[derive(Clone, Default)]
struct TestSource(Arc<Mutex<State>>);

impl TestSource {
    fn new() -> Self {
        let source = Self::default();
        {
            let mut state = source.state();
            state.sessions = LIST.to_vec();
            state.session.insert(ID_A.to_string(), DETAIL.to_vec());
        }
        source
    }

    fn state(&self) -> MutexGuard<'_, State> {
        self.0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Keeps entries `first..=last` with `data` naming their number, as if `1..first` were
    /// dropped.
    fn keep(&self, first: u64, last: u64) {
        let mut state = self.state();
        state.dropped_through = first - 1;
        for number in first..=last {
            state.kept.push(entry(number));
        }
        state.newest = last;
    }

    /// Yields `item` live to every reader; an `entry` is also kept for the replay.
    fn publish(&self, item: Item) {
        let mut state = self.state();
        state.newest = state.newest.max(item.number);
        if item.kind == EventKind::Entry {
            state.kept.push(item.clone());
        }
        state
            .live
            .retain(|sender| sender.send(item.clone()).is_ok());
    }

    /// Ends every reader: the server ends its stream.
    fn end(&self) {
        self.state().live.clear();
    }

    fn calls(&self) -> usize {
        self.state().calls
    }
}

impl Source for TestSource {
    fn sessions(&self) -> Vec<u8> {
        let mut state = self.state();
        state.calls += 1;
        state.sessions.clone()
    }

    fn session(&self, id: &str) -> Option<Vec<u8>> {
        let mut state = self.state();
        state.calls += 1;
        state.session.get(id).cloned()
    }

    fn newest(&self) -> u64 {
        let mut state = self.state();
        state.calls += 1;
        state.newest
    }

    fn read_from(&self, number: u64) -> Result<Box<dyn Reader>, TooOld> {
        let mut state = self.state();
        state.calls += 1;
        state.read_from.push(number);
        if number < state.dropped_through {
            return Err(TooOld);
        }
        let replay = state
            .kept
            .iter()
            .filter(|item| item.number > number)
            .cloned()
            .collect();
        let (sender, live) = mpsc::channel();
        state.live.push(sender);
        Ok(Box::new(TestReader { replay, live }))
    }
}

struct TestReader {
    replay: VecDeque<Item>,
    live: Receiver<Item>,
}

impl Reader for TestReader {
    fn next(&mut self, wait: Duration) -> Next {
        if let Some(item) = self.replay.pop_front() {
            return Next::Item(item);
        }
        match self.live.recv_timeout(wait) {
            Ok(item) => Next::Item(item),
            Err(RecvTimeoutError::Timeout) => Next::Idle,
            Err(RecvTimeoutError::Disconnected) => Next::End,
        }
    }
}

fn entry(number: u64) -> Item {
    Item {
        number,
        kind: EventKind::Entry,
        data: format!("{{\"number\":{number}}}").into_bytes(),
    }
}

// ---------------------------------------------------------------------------------------------
// Starting a server and speaking to it.

fn page_files() -> Vec<PageFile> {
    vec![
        PageFile {
            name: "index.html".to_string(),
            content_type: "text/html; charset=utf-8".to_string(),
            bytes: INDEX.to_vec(),
        },
        PageFile {
            name: "view.js".to_string(),
            content_type: "text/javascript; charset=utf-8".to_string(),
            bytes: VIEW_JS.to_vec(),
        },
        PageFile {
            name: "view.css".to_string(),
            content_type: "text/css; charset=utf-8".to_string(),
            bytes: VIEW_CSS.to_vec(),
        },
    ]
}

fn start_with(source: &TestSource, limits: Limits, files: Vec<PageFile>) -> (Server, u16) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind 127.0.0.1:0");
    let port = listener.local_addr().expect("local address").port();
    let config = Config {
        token: Token::from_hex(TOKEN).expect("token"),
        host: format!("127.0.0.1:{port}"),
        files,
        limits,
    };
    let server = serve(listener, config, source.clone()).expect("serve");
    (server, port)
}

fn start(source: &TestSource, limits: Limits) -> (Server, u16) {
    start_with(source, limits, page_files())
}

fn connect(port: u16) -> TcpStream {
    let stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    stream.set_read_timeout(Some(WAIT)).expect("read timeout");
    stream
}

/// Sends `request` and reads the answer to the end of the connection.
fn exchange(port: u16, request: &[u8]) -> Vec<u8> {
    let mut stream = connect(port);
    let _ = stream.write_all(request);
    read_to_close(&mut stream)
}

fn read_to_close(stream: &mut TcpStream) -> Vec<u8> {
    let mut answer = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        match stream.read(&mut chunk) {
            Ok(0) => return answer,
            Ok(n) => answer.extend_from_slice(&chunk[..n]),
            Err(error) if error.kind() == ErrorKind::Interrupted => continue,
            Err(error) if is_timeout(&error) => panic!("no end of the answer within {WAIT:?}"),
            Err(_) => return answer,
        }
    }
}

fn is_timeout(error: &std::io::Error) -> bool {
    matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut)
}

fn request(method: &str, target: &str, port: u16, token: Option<&str>) -> Vec<u8> {
    let mut text = format!("{method} {target} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n");
    if let Some(token) = token {
        text.push_str(&format!("Authorization: Bearer {token}\r\n"));
    }
    text.push_str("\r\n");
    text.into_bytes()
}

fn get(port: u16, target: &str, token: Option<&str>) -> Answer {
    parse(&exchange(port, &request("GET", target, port, token)))
}

fn raw(port: u16, text: &str) -> Answer {
    parse(&exchange(port, text.as_bytes()))
}

struct Answer {
    status: u16,
    /// The head's lines after the status line, without their CR LF.
    lines: Vec<String>,
    body: Vec<u8>,
}

impl Answer {
    fn header(&self, name: &str) -> Option<&str> {
        self.lines.iter().find_map(|line| {
            let (field, value) = line.split_once(": ")?;
            field.eq_ignore_ascii_case(name).then_some(value)
        })
    }

    fn names(&self) -> Vec<&str> {
        self.lines
            .iter()
            .map(|line| line.split_once(':').map_or(line.as_str(), |(name, _)| name))
            .collect()
    }

    fn refusal(&self, status: u16, word: &str) {
        assert_eq!(self.status, status, "status of {:?}", self.lines);
        assert_eq!(self.header("Content-Type"), Some("application/json"));
        assert_eq!(
            String::from_utf8_lossy(&self.body),
            format!("{{\"error\":\"{word}\"}}")
        );
    }
}

/// Splits an answer into status, head lines and body, and holds every answer of this file to
/// the rules over every answer: CR LF line ends, no `Access-Control-`, `Set-Cookie` or `Server`
/// line, the security lines, `Date` and `Connection: close`.
fn parse(bytes: &[u8]) -> Answer {
    let end = find(bytes, b"\r\n\r\n").unwrap_or_else(|| {
        panic!(
            "no head in the answer: {:?}",
            String::from_utf8_lossy(bytes)
        )
    });
    let head = std::str::from_utf8(&bytes[..end]).expect("head is UTF-8");
    let mut lines = head.split("\r\n");
    let status_line = lines.next().expect("status line");
    assert!(
        !head.replace("\r\n", "").contains('\n'),
        "a bare LF in the head"
    );
    let status: u16 = status_line
        .strip_prefix("HTTP/1.1 ")
        .and_then(|rest| rest.get(..3))
        .and_then(|code| code.parse().ok())
        .unwrap_or_else(|| panic!("status line {status_line:?}"));
    let lines: Vec<String> = lines.map(str::to_string).collect();
    for line in &lines {
        let lower = line.to_ascii_lowercase();
        assert!(!lower.starts_with("access-control-"), "{line}");
        assert!(!lower.starts_with("set-cookie"), "{line}");
        assert!(!lower.starts_with("server:"), "{line}");
    }
    for line in SECURITY {
        assert!(lines.iter().any(|have| have == line), "missing {line}");
    }
    assert_eq!(lines.last().map(String::as_str), Some("Connection: close"));
    let answer = Answer {
        status,
        lines,
        body: bytes[end + 4..].to_vec(),
    };
    let date = answer.header("Date").expect("Date line");
    check_date(date);
    answer
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

/// `date` is an IMF-fixdate within a minute of now, its weekday that of its date.
fn check_date(date: &str) {
    const DAYS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    assert_eq!(date.len(), 29, "{date:?}");
    let bytes = date.as_bytes();
    assert_eq!(&date[3..5], ", ");
    assert_eq!(&date[25..], " GMT");
    assert!(bytes[7] == b' ' && bytes[11] == b' ' && bytes[16] == b' ');
    assert!(bytes[19] == b':' && bytes[22] == b':');
    let number = |range: std::ops::Range<usize>| -> i64 {
        let text = &date[range];
        assert!(text.bytes().all(|b| b.is_ascii_digit()), "{date:?}");
        text.parse().expect("digits")
    };
    let day = number(5..7);
    let month = MONTHS
        .iter()
        .position(|name| *name == &date[8..11])
        .expect("month") as i64
        + 1;
    let year = number(12..16);
    let seconds = number(17..19) * 3600 + number(20..22) * 60 + number(23..25);
    // Days from the civil date, the inverse of what the server computes.
    let shifted = if month <= 2 { year - 1 } else { year };
    let era = shifted.div_euclid(400);
    let year_of_era = shifted - era * 400;
    let day_of_year = (153 * (if month > 2 { month - 3 } else { month + 9 }) + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    let days = era * 146_097 + day_of_era - 719_468;
    assert_eq!(
        &date[..3],
        DAYS[(days + 4).rem_euclid(7) as usize],
        "{date:?}"
    );
    let then = days * 86_400 + seconds;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_secs() as i64;
    assert!((now - then).abs() < 60, "{date:?} is not now");
}

fn wait_for(what: &str, mut condition: impl FnMut() -> bool) -> Duration {
    let started = Instant::now();
    while !condition() {
        assert!(started.elapsed() < WAIT, "not within {WAIT:?}: {what}");
        thread::sleep(Duration::from_millis(10));
    }
    started.elapsed()
}

// ---------------------------------------------------------------------------------------------
// Reading the stream.

struct Stream {
    stream: TcpStream,
    buffer: Vec<u8>,
}

impl Stream {
    /// Opens `/events` with the token, and `Last-Event-ID: <last>` when given.
    fn open(port: u16, last: Option<&str>) -> (Self, Answer) {
        let mut text = format!(
            "GET /events HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nAuthorization: Bearer {TOKEN}\r\n"
        );
        if let Some(last) = last {
            text.push_str(&format!("Last-Event-ID: {last}\r\n"));
        }
        text.push_str("\r\n");
        let mut stream = connect(port);
        stream.write_all(text.as_bytes()).expect("send");
        let mut reader = Self {
            stream,
            buffer: Vec::new(),
        };
        let end = reader.fill_until(b"\r\n\r\n");
        let head: Vec<u8> = reader.buffer.drain(..end + 4).collect();
        let answer = parse(&head);
        (reader, answer)
    }

    /// Reads until `needle` is in the buffer and returns where it starts.
    fn fill_until(&mut self, needle: &[u8]) -> usize {
        let mut chunk = [0u8; 65536];
        loop {
            if let Some(at) = find(&self.buffer, needle) {
                return at;
            }
            match self.stream.read(&mut chunk) {
                Ok(0) => panic!(
                    "the stream closed before {:?}: {:?}",
                    String::from_utf8_lossy(needle),
                    String::from_utf8_lossy(&self.buffer)
                ),
                Ok(n) => self.buffer.extend_from_slice(&chunk[..n]),
                Err(error) if error.kind() == ErrorKind::Interrupted => continue,
                Err(error) => panic!("reading the stream: {error}"),
            }
        }
    }

    /// The next block, up to the empty line that ends it, without that line.
    fn next_block(&mut self) -> String {
        let end = self.fill_until(b"\n\n");
        let block: Vec<u8> = self.buffer.drain(..end + 2).collect();
        String::from_utf8(block[..end].to_vec()).expect("UTF-8 block")
    }

    /// Reads the rest of the stream to its close.
    fn rest(&mut self) -> Vec<u8> {
        let mut rest = std::mem::take(&mut self.buffer);
        rest.extend(read_to_close(&mut self.stream));
        rest
    }
}

fn entry_block(number: u64) -> String {
    format!("id: {number}\nevent: entry\ndata: {{\"number\":{number}}}")
}

fn reset_block(newest: u64) -> String {
    format!("id: {newest}\nevent: reset\ndata: {{\"newest\":{newest}}}")
}

fn stream_head(answer: &Answer) {
    assert_eq!(answer.status, 200);
    let mut expected = vec!["Content-Type: text/event-stream".to_string()];
    expected.extend(SECURITY.iter().map(|line| line.to_string()));
    let date = format!("Date: {}", answer.header("Date").expect("Date"));
    expected.push(date);
    expected.push("Connection: close".to_string());
    assert_eq!(answer.lines, expected);
}

// ---------------------------------------------------------------------------------------------
// Routes and methods.

#[test]
fn case_01_root_serves_index_and_page_files_by_their_type() {
    let source = TestSource::new();
    let (_server, port) = start(&source, Limits::default());
    let root = get(port, "/", None);
    assert_eq!(root.status, 200);
    assert_eq!(
        root.header("Content-Type"),
        Some("text/html; charset=utf-8")
    );
    assert_eq!(root.body, INDEX);
    let script = get(port, "/view.js", None);
    assert_eq!(script.status, 200);
    assert_eq!(
        script.header("Content-Type"),
        Some("text/javascript; charset=utf-8")
    );
    assert_eq!(script.body, VIEW_JS);
    let style = get(port, "/view.css", None);
    assert_eq!(style.status, 200);
    assert_eq!(
        style.header("Content-Type"),
        Some("text/css; charset=utf-8")
    );
    assert_eq!(style.body, VIEW_CSS);
    get(port, "/index.html", None).refusal(404, "not-found");
}

#[test]
fn case_02_unknown_path_is_404_not_found() {
    let source = TestSource::new();
    let (_server, port) = start(&source, Limits::default());
    get(port, "/nope", None).refusal(404, "not-found");
    get(port, "/api/nope", Some(TOKEN)).refusal(404, "not-found");
}

#[test]
fn case_03_any_method_but_get_is_405_and_the_source_sees_no_call() {
    let source = TestSource::new();
    let (_server, port) = start(&source, Limits::default());
    let session = format!("/api/sessions/{ID_A}");
    for (method, target) in [
        ("POST", "/"),
        ("PUT", "/api/sessions"),
        ("DELETE", session.as_str()),
        ("HEAD", "/"),
    ] {
        let answer = parse(&exchange(port, &request(method, target, port, Some(TOKEN))));
        answer.refusal(405, "method");
        assert_eq!(answer.header("Allow"), Some("GET"), "{method} {target}");
    }
    assert_eq!(source.calls(), 0);
}

#[test]
fn case_04_targets_match_byte_for_byte() {
    let source = TestSource::new();
    let (_server, port) = start(&source, Limits::default());
    let upper = format!("/api/sessions/{}", ID_A.to_ascii_uppercase());
    for target in [
        "/api/sessions/../sessions",
        "/api/%73essions",
        "/api/sessions?x=1",
        upper.as_str(),
    ] {
        get(port, target, Some(TOKEN)).refusal(404, "not-found");
    }
    assert_eq!(
        get(port, &format!("/api/sessions/{ID_A}"), Some(TOKEN)).status,
        200
    );
}

#[test]
fn case_05_absolute_form_and_asterisk_are_400() {
    let source = TestSource::new();
    let (_server, port) = start(&source, Limits::default());
    for line in [
        format!("GET http://127.0.0.1:{port}/ HTTP/1.1"),
        "GET * HTTP/1.1".to_string(),
        "OPTIONS * HTTP/1.1".to_string(),
    ] {
        raw(port, &format!("{line}\r\nHost: 127.0.0.1:{port}\r\n\r\n")).refusal(400, "bad-request");
    }
}

#[test]
fn case_06_other_versions_are_505_and_malformed_versions_400() {
    let source = TestSource::new();
    let (_server, port) = start(&source, Limits::default());
    for version in ["HTTP/1.0", "HTTP/2.0"] {
        raw(
            port,
            &format!("GET / {version}\r\nHost: 127.0.0.1:{port}\r\n\r\n"),
        )
        .refusal(505, "version");
    }
    for version in ["HTTP/1", "HTTPS/1.1"] {
        raw(
            port,
            &format!("GET / {version}\r\nHost: 127.0.0.1:{port}\r\n\r\n"),
        )
        .refusal(400, "bad-request");
    }
}

#[test]
fn case_07_content_length_and_transfer_encoding_are_400() {
    let source = TestSource::new();
    let (_server, port) = start(&source, Limits::default());
    raw(
        port,
        &format!("GET / HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nContent-Length: 5\r\n\r\nhello"),
    )
    .refusal(400, "bad-request");
    raw(
        port,
        &format!(
            "GET / HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nTransfer-Encoding: chunked\r\n\r\n0\r\n\r\n"
        ),
    )
    .refusal(400, "bad-request");
}

#[test]
fn case_08_malformed_header_lines_are_400() {
    let source = TestSource::new();
    let (_server, port) = start(&source, Limits::default());
    for text in [
        format!("GET / HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nNoColon\r\n\r\n"),
        format!("GET / HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n X-Folded: a\r\n\r\n"),
        format!("GET / HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n\tX-Folded: a\r\n\r\n"),
        format!("GET / HTTP/1.1\nHost: 127.0.0.1:{port}\n\n"),
        format!(
            "GET /api/sessions HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nAuthorization: Bearer {TOKEN}\r\nAuthorization: Bearer {TOKEN}\r\n\r\n"
        ),
    ] {
        raw(port, &text).refusal(400, "bad-request");
    }
    assert_eq!(source.calls(), 0);
}

// ---------------------------------------------------------------------------------------------
// Token.

#[test]
fn case_09_no_authorization_is_401_with_the_challenge_and_no_data() {
    let source = TestSource::new();
    let (_server, port) = start(&source, Limits::default());
    let answer = get(port, "/api/sessions", None);
    answer.refusal(401, "unauthorized");
    assert_eq!(answer.header("WWW-Authenticate"), Some("Bearer"));
    assert!(find(&answer.body, ID_A.as_bytes()).is_none());
    assert_eq!(source.calls(), 0);
}

#[test]
fn case_10_a_wrong_or_upper_case_token_is_401() {
    let source = TestSource::new();
    let (_server, port) = start(&source, Limits::default());
    let first = format!("1{}", &TOKEN[1..]);
    let last = format!("{}e", &TOKEN[..63]);
    let upper = TOKEN.to_ascii_uppercase();
    for token in [first, last, upper] {
        assert_eq!(token.len(), 64);
        let answer = get(port, "/api/sessions", Some(&token));
        answer.refusal(401, "unauthorized");
        assert_eq!(answer.header("WWW-Authenticate"), Some("Bearer"));
    }
    assert_eq!(source.calls(), 0);
}

#[test]
fn case_11_the_token_in_the_query_is_404() {
    let source = TestSource::new();
    let (_server, port) = start(&source, Limits::default());
    get(port, &format!("/api/sessions?token={TOKEN}"), None).refusal(404, "not-found");
}

#[test]
fn case_12_session_and_events_without_the_token_are_401() {
    let source = TestSource::new();
    let (_server, port) = start(&source, Limits::default());
    get(port, &format!("/api/sessions/{ID_A}"), None).refusal(401, "unauthorized");
    get(port, "/events", None).refusal(401, "unauthorized");
    assert_eq!(source.calls(), 0);
}

#[test]
fn case_13_the_right_token_answers_the_source_and_the_page_needs_none() {
    let source = TestSource::new();
    let (_server, port) = start(&source, Limits::default());
    let list = get(port, "/api/sessions", Some(TOKEN));
    assert_eq!(list.status, 200);
    assert_eq!(list.header("Content-Type"), Some("application/json"));
    assert_eq!(list.body, LIST);
    let detail = get(port, &format!("/api/sessions/{ID_A}"), Some(TOKEN));
    assert_eq!(detail.status, 200);
    assert_eq!(detail.header("Content-Type"), Some("application/json"));
    assert_eq!(detail.body, DETAIL);
    assert_eq!(get(port, "/", None).status, 200);
}

// ---------------------------------------------------------------------------------------------
// The order of the checks.

#[test]
fn case_14_the_checks_run_in_the_documents_order() {
    let source = TestSource::new();
    let (_server, port) = start(&source, Limits::default());
    get(port, "/api/nope", None).refusal(404, "not-found");
    let unlisted = format!("/api/sessions/{ID_B}");
    get(port, &unlisted, None).refusal(401, "unauthorized");
    get(port, &unlisted, Some(TOKEN)).refusal(404, "not-found");
    raw(port, "POST / HTTP/1.1\r\nHost: evil.example\r\n\r\n").refusal(405, "method");
    raw(port, "GET /nope HTTP/1.1\r\nHost: evil.example\r\n\r\n").refusal(403, "host");
}

// ---------------------------------------------------------------------------------------------
// Host.

#[test]
fn case_15_another_host_name_is_403() {
    let source = TestSource::new();
    let (_server, port) = start(&source, Limits::default());
    raw(
        port,
        &format!("GET / HTTP/1.1\r\nHost: evil.example:{port}\r\n\r\n"),
    )
    .refusal(403, "host");
}

#[test]
fn case_16_localhost_and_another_port_are_403() {
    let source = TestSource::new();
    let (_server, port) = start(&source, Limits::default());
    let other = port.wrapping_add(1);
    for host in [format!("localhost:{port}"), format!("127.0.0.1:{other}")] {
        raw(port, &format!("GET / HTTP/1.1\r\nHost: {host}\r\n\r\n")).refusal(403, "host");
    }
}

#[test]
fn case_17_no_host_and_two_hosts_are_400() {
    let source = TestSource::new();
    let (_server, port) = start(&source, Limits::default());
    raw(port, "GET / HTTP/1.1\r\n\r\n").refusal(400, "bad-request");
    raw(
        port,
        &format!("GET / HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nHost: 127.0.0.1:{port}\r\n\r\n"),
    )
    .refusal(400, "bad-request");
}

// ---------------------------------------------------------------------------------------------
// Limits.

#[test]
fn case_18_a_head_over_8192_bytes_is_431_then_closed() {
    let source = TestSource::new();
    let (_server, port) = start(&source, Limits::default());
    let head = format!("GET / HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nX-Big: ");
    let fill = 8192 - head.len() - 4;
    let at_limit = format!("{head}{}\r\n\r\n", "a".repeat(fill));
    assert_eq!(at_limit.len(), 8192);
    assert_eq!(raw(port, &at_limit).status, 200);
    let over = format!("{head}{}\r\n\r\n", "a".repeat(fill + 1));
    // `raw` reads to the close of the connection.
    raw(port, &over).refusal(431, "too-large");
}

#[test]
fn case_19_51_header_lines_are_431() {
    let source = TestSource::new();
    let (_server, port) = start(&source, Limits::default());
    let lines = |count: usize| {
        let mut text = format!("GET / HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n");
        for n in 1..count {
            text.push_str(&format!("X-Line-{n}: {n}\r\n"));
        }
        text.push_str("\r\n");
        text
    };
    assert_eq!(raw(port, &lines(50)).status, 200);
    raw(port, &lines(51)).refusal(431, "too-large");
}

/// Half a request is closed at the receive limit, not before it, and with no upper bound but
/// [`WAIT`]. `read_to_close` waits at most [`WAIT`] for each read, so a server that never
/// closes and sends nothing fails the test after [`WAIT`]; one that sends a byte now and then
/// keeps it reading until the runner ends it at 60 s.
#[test]
fn case_20_half_a_request_is_closed_without_answer_after_the_receive_limit() {
    let source = TestSource::new();
    let limits = Limits {
        receive: Duration::from_millis(250),
        ..Limits::default()
    };
    let (server, port) = start(&source, limits);
    let started = Instant::now();
    let mut stream = connect(port);
    stream.write_all(b"GET / HT").expect("send");
    let answer = read_to_close(&mut stream);
    let elapsed = started.elapsed();
    assert!(answer.is_empty(), "{:?}", String::from_utf8_lossy(&answer));
    // 90 % of the limit, as the bound of 900 ms was of a limit of 1 s.
    assert!(elapsed >= Duration::from_millis(225), "{elapsed:?}");
    assert!(elapsed <= WAIT, "{elapsed:?}");
    wait_for("the connection count returns to 0", || {
        server.open_connections() == 0
    });
}

#[test]
fn case_21_past_the_connection_limit_503_without_reading() {
    let source = TestSource::new();
    let limits = Limits {
        connections: 4,
        ..Limits::default()
    };
    let (server, port) = start(&source, limits);
    let mut held: Vec<TcpStream> = (0..4)
        .map(|_| {
            let mut stream = connect(port);
            stream.write_all(b"GET / HTTP/1.1\r\n").expect("send");
            stream
        })
        .collect();
    wait_for("four open connections", || server.open_connections() == 4);
    let mut fifth = connect(port);
    let answer = parse(&read_to_close(&mut fifth));
    answer.refusal(503, "busy");
    for stream in &mut held {
        stream
            .write_all(format!("Host: 127.0.0.1:{port}\r\n\r\n").as_bytes())
            .expect("send");
        let answer = parse(&read_to_close(stream));
        assert_eq!(answer.status, 200);
        assert_eq!(answer.body, INDEX);
    }
}

#[test]
fn case_22_serve_refuses_an_address_other_than_127_0_0_1() {
    let listener = TcpListener::bind("0.0.0.0:0").expect("bind 0.0.0.0:0");
    let port = listener.local_addr().expect("local address").port();
    let config = Config {
        token: Token::from_hex(TOKEN).expect("token"),
        host: format!("127.0.0.1:{port}"),
        files: page_files(),
        limits: Limits::default(),
    };
    assert!(serve(listener, config, TestSource::new()).is_err());
}

/// A client that reads nothing is closed at the write limit, the answer and the stream each: not
/// before it, which each half asserts, and with no upper bound but [`WAIT`] of `wait_for`, which
/// fails the test when the close does not come.
#[test]
fn case_23_a_client_that_reads_nothing_is_closed_at_the_write_limit() {
    let source = TestSource::new();
    source.state().sessions = vec![b'a'; 8 << 20];
    let limits = Limits {
        write: Duration::from_millis(250),
        queue_bytes: 64 << 20,
        ..Limits::default()
    };
    let write = limits.write;
    let (server, port) = start(&source, limits);
    let before = server.open_connections();

    let mut silent = connect(port);
    let sent = Instant::now();
    silent
        .write_all(&request("GET", "/api/sessions", port, Some(TOKEN)))
        .expect("send");
    wait_for("the answer is being written", || source.calls() == 1);
    wait_for("the connection count returns", || {
        server.open_connections() == before
    });
    let elapsed = sent.elapsed();
    assert!(
        elapsed >= write,
        "the answer was closed after {elapsed:?}, before the write limit of {write:?}"
    );
    let got = read_to_close(&mut silent);
    assert!(got.len() < 8 << 20, "the whole answer arrived");

    let (mut reader, answer) = Stream::open(port, None);
    stream_head(&answer);
    wait_for("one open stream", || server.open_streams() == 1);
    let published = Instant::now();
    source.publish(Item {
        number: 1,
        kind: EventKind::Entry,
        data: vec![b'a'; 8 << 20],
    });
    wait_for("the stream ends", || server.open_streams() == 0);
    wait_for("the connection count returns", || {
        server.open_connections() == before
    });
    let elapsed = published.elapsed();
    assert!(
        elapsed >= write,
        "the stream was closed after {elapsed:?}, before the write limit of {write:?}"
    );
    assert!(reader.rest().len() < 8 << 20, "the whole entry arrived");
}

// ---------------------------------------------------------------------------------------------
// The head.

/// The request and answer examples of `docs/view-stream.md`, in its order, each answer with the
/// CR LF line ends of the wire and its body cut at its `Content-Length`.
fn document_examples() -> Vec<(String, Vec<u8>)> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/view-stream.md");
    let text = std::fs::read_to_string(path).expect("docs/view-stream.md");
    let mut blocks = Vec::new();
    let mut rest = text.as_str();
    while let Some(start) = rest.find("```http\n") {
        let after = &rest[start + 8..];
        let end = after.find("\n```").expect("closed fence");
        blocks.push(&after[..end + 1]);
        rest = &after[end + 4..];
    }
    let mut pairs = Vec::new();
    for pair in blocks.chunks(2) {
        let [request, answer] = pair else {
            panic!("an odd number of http blocks")
        };
        assert!(answer.starts_with("HTTP/1.1 "), "{answer}");
        let (head, body) = answer.split_once("\n\n").expect("head and body");
        let mut wire = String::new();
        for line in head.split('\n') {
            wire.push_str(line);
            wire.push_str("\r\n");
        }
        wire.push_str("\r\n");
        let mut wire = wire.into_bytes();
        let length = head
            .lines()
            .find_map(|line| line.strip_prefix("Content-Length: "))
            .map(|n| n.parse::<usize>().expect("Content-Length"));
        match length {
            Some(length) => wire.extend_from_slice(&body.as_bytes()[..length]),
            None => wire.extend_from_slice(body.as_bytes()),
        }
        pairs.push((request.to_string(), wire));
    }
    pairs
}

/// The answer with the 29 bytes of its `Date` value replaced by the document's.
fn with_document_date(answer: &[u8]) -> Vec<u8> {
    let at = find(answer, b"\r\nDate: ").expect("Date line") + 8;
    let date = std::str::from_utf8(&answer[at..at + 29]).expect("Date value");
    check_date(date);
    let mut masked = answer.to_vec();
    masked[at..at + 29].copy_from_slice(b"Sat, 21 Nov 2026 02:00:00 GMT");
    masked
}

#[test]
fn case_24_answers_equal_the_documents_examples() {
    let examples = document_examples();
    let body_of = |target: &str| -> Vec<u8> {
        let (_, wire) = examples
            .iter()
            .find(|(request, _)| request.starts_with(&format!("GET {target} HTTP/1.1\n")))
            .expect("example");
        let at = find(wire, b"\r\n\r\n").expect("head") + 4;
        wire[at..].to_vec()
    };
    let files = vec![
        PageFile {
            name: "index.html".to_string(),
            content_type: "text/html; charset=utf-8".to_string(),
            bytes: body_of("/"),
        },
        PageFile {
            name: "view.js".to_string(),
            content_type: "text/javascript; charset=utf-8".to_string(),
            bytes: body_of("/view.js"),
        },
    ];
    let source = TestSource::new();
    {
        let mut state = source.state();
        state.sessions = body_of("/api/sessions");
        state
            .session
            .insert(ID_A.to_string(), body_of(&format!("/api/sessions/{ID_A}")));
    }
    let (_server, port) = start_with(&source, Limits::default(), files);

    let mut compared = 0;
    for (example, expected) in &examples {
        let first = example.lines().next().expect("request line");
        let target = first
            .strip_prefix("GET ")
            .and_then(|rest| rest.strip_suffix(" HTTP/1.1"))
            .expect("a GET example");
        if target == "/events" {
            continue;
        }
        let token = example.contains("Authorization: Bearer ").then_some(TOKEN);
        let answer = exchange(port, &request("GET", target, port, token));
        parse(&answer);
        assert_eq!(
            String::from_utf8_lossy(&with_document_date(&answer)),
            String::from_utf8_lossy(expected),
            "GET {target}, token {}",
            token.is_some()
        );
        compared += 1;
    }
    assert_eq!(
        compared, 5,
        "/, /view.js, /api/sessions twice, /api/sessions/<id>"
    );

    let not_found = get(port, "/nope", None);
    let method = parse(&exchange(port, &request("POST", "/", port, None)));
    let mut expected = vec!["Content-Type", "Content-Length"];
    expected.extend(
        SECURITY
            .iter()
            .map(|line| line.split_once(':').expect("name").0),
    );
    expected.extend(["Date", "Connection"]);
    assert_eq!(not_found.names(), expected);
    expected.insert(2, "Allow");
    assert_eq!(method.names(), expected);
    assert_eq!(method.lines[2], "Allow: GET");
}

// ---------------------------------------------------------------------------------------------
// Stream.

#[test]
fn case_25_events_head_live_entries_in_order_and_close() {
    let source = TestSource::new();
    source.keep(1, 3);
    let (_server, port) = start(&source, Limits::default());
    let (mut reader, answer) = Stream::open(port, None);
    stream_head(&answer);
    assert!(answer.header("Content-Length").is_none());
    assert!(answer.header("Transfer-Encoding").is_none());
    for number in 4..=6 {
        source.publish(entry(number));
    }
    for number in 4..=6 {
        assert_eq!(reader.next_block(), entry_block(number));
    }
    source.end();
    assert!(reader.rest().is_empty());
}

#[test]
fn case_26_last_event_id_replays_above_that_number() {
    let source = TestSource::new();
    source.keep(1, 3);
    let (_server, port) = start(&source, Limits::default());
    let (mut reader, answer) = Stream::open(port, Some("2"));
    stream_head(&answer);
    assert_eq!(reader.next_block(), entry_block(3));
    source.publish(entry(4));
    assert_eq!(reader.next_block(), entry_block(4));
    assert_eq!(source.state().read_from, vec![2]);
}

#[test]
fn case_27_a_number_older_than_kept_gets_reset_then_live() {
    let source = TestSource::new();
    source.keep(5, 7);
    let (_server, port) = start(&source, Limits::default());
    let (mut reader, answer) = Stream::open(port, Some("2"));
    stream_head(&answer);
    assert_eq!(reader.next_block(), reset_block(7));
    source.publish(entry(8));
    assert_eq!(reader.next_block(), entry_block(8));
}

#[test]
fn case_28_a_number_above_newest_or_not_a_number_gets_reset() {
    let source = TestSource::new();
    source.keep(1, 3);
    let (_server, port) = start(&source, Limits::default());
    for (next, last) in (4..).zip(["99", "abc", "123456789012345678901"]) {
        let (mut reader, answer) = Stream::open(port, Some(last));
        stream_head(&answer);
        assert_eq!(reader.next_block(), reset_block(next - 1), "{last}");
        source.publish(entry(next));
        assert_eq!(reader.next_block(), entry_block(next), "{last}");
    }
}

#[test]
fn case_29_session_items_are_live_only() {
    let source = TestSource::new();
    source.keep(1, 2);
    let (_server, port) = start(&source, Limits::default());
    let (mut live, answer) = Stream::open(port, None);
    stream_head(&answer);
    source.publish(entry(3));
    let summary = format!("{{\"id\":\"{ID_A}\",\"state\":\"open\"}}");
    source.publish(Item {
        number: 3,
        kind: EventKind::Session,
        data: summary.clone().into_bytes(),
    });
    assert_eq!(live.next_block(), entry_block(3));
    assert_eq!(
        live.next_block(),
        format!("id: 3\nevent: session\ndata: {summary}")
    );
    drop(live);

    let (mut again, answer) = Stream::open(port, Some("1"));
    stream_head(&answer);
    source.publish(entry(4));
    for number in 2..=4 {
        assert_eq!(again.next_block(), entry_block(number));
    }
}

/// Entries of 16 KiB are published, each read by the reading client, until the silent reader
/// is closed, at most 2,000 (32 MiB): the kernel's buffers of the silent connection and the
/// queue bound of 256 KiB fill after a few hundred. The entries published until the close are
/// counted, and fewer than those reached the silent reader.
#[test]
fn case_30_a_reader_that_never_reads_is_closed_alone_at_the_queue_bound() {
    const ENTRIES: u64 = 2000;
    const BOUND: usize = 256 << 10;
    let source = TestSource::new();
    let limits = Limits {
        queue_entries: 16,
        queue_bytes: BOUND,
        ..Limits::default()
    };
    let (server, port) = start(&source, limits);
    let (mut silent, answer) = Stream::open(port, None);
    stream_head(&answer);
    let (mut reading, answer) = Stream::open(port, None);
    stream_head(&answer);
    let data = |number: u64| format!("{number:>16384}").into_bytes();
    let mut published = 0;
    for number in 1..=ENTRIES {
        source.publish(Item {
            number,
            kind: EventKind::Entry,
            data: data(number),
        });
        published = number;
        let block = reading.next_block();
        let expected = format!(
            "id: {number}\nevent: entry\ndata: {}",
            String::from_utf8(data(number)).expect("UTF-8")
        );
        assert!(block == expected, "entry {number} differs");
        if server.open_streams() == 1 {
            break;
        }
    }
    wait_for("the silent reader is closed", || server.open_streams() == 1);
    let peak = server.queued_bytes_peak();
    assert!(peak > 0 && peak <= BOUND, "peak {peak}");
    let rest = silent.rest();
    let arrived = rest.windows(4).filter(|w| w == b"id: ").count() as u64;
    assert!(
        arrived < published,
        "{arrived} of the {published} entries published reached the silent reader"
    );
}

#[test]
fn case_31_past_the_stream_limit_503() {
    let source = TestSource::new();
    let limits = Limits {
        streams: 2,
        ..Limits::default()
    };
    let (server, port) = start(&source, limits);
    let (_first, answer) = Stream::open(port, None);
    stream_head(&answer);
    let (_second, answer) = Stream::open(port, None);
    stream_head(&answer);
    wait_for("two open streams", || server.open_streams() == 2);
    get(port, "/events", Some(TOKEN)).refusal(503, "busy");
}

/// An idle stream receives the keep-alive comment, with no upper bound on its time but [`WAIT`]:
/// each read of `next_block` waits at most [`WAIT`], so a server that sends nothing fails the
/// test after [`WAIT`].
#[test]
fn case_32_an_idle_stream_receives_the_keep_alive_comment() {
    let source = TestSource::new();
    let limits = Limits {
        heartbeat: Duration::from_secs(1),
        ..Limits::default()
    };
    let (_server, port) = start(&source, limits);
    let (mut reader, answer) = Stream::open(port, None);
    stream_head(&answer);
    let started = Instant::now();
    assert_eq!(reader.next_block(), ": keep-alive");
    let elapsed = started.elapsed();
    assert!(
        elapsed <= WAIT,
        "the keep-alive comment came after {elapsed:?}"
    );
}

#[test]
fn case_33_a_stream_whose_reader_is_gone_ends_its_thread() {
    let source = TestSource::new();
    let (server, port) = start(&source, Limits::default());
    let (reader, answer) = Stream::open(port, None);
    stream_head(&answer);
    wait_for("one open stream", || server.open_streams() == 1);
    drop(reader);
    wait_for("the stream ends", || server.open_streams() == 0);
    wait_for("the connection ends", || server.open_connections() == 0);
}
