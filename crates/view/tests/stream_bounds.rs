//! Where a stream starts while entries are kept (F1), and an entry whose frame alone is over the
//! stream's bound of bytes (F2), `docs/view-stream.md` "The stream, `GET /events`". Every case
//! starts its own server on 127.0.0.1 port 0 with a source of its own and stops it.

#![forbid(unsafe_code)]

use std::io::{BufRead, BufReader, ErrorKind, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
use std::time::{Duration, Instant};

use surf_secret::Token;
use surf_view::{Config, EventKind, Item, Limits, Next, Reader, Server, Source, TooOld, serve};

const TOKEN: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const WAIT: Duration = Duration::from_secs(30);
const MIB: usize = 1 << 20;

// ---------------------------------------------------------------------------------------------
// The source.

#[derive(Default)]
struct State {
    newest: u64,
    /// Entries kept right after each call of `newest()`: the newest number moves by this much.
    kept_after_newest: u64,
    /// `read_from` answers `TooOld` for a number more than this below the newest, as `surfd`'s
    /// source does.
    queue_entries: u64,
    live: Vec<Sender<Item>>,
    /// The numbers the readers handed to the server, in order.
    handed: Vec<u64>,
}

/// A source fed by the test. With `one_hold` it gives `read_newest` in one hold of its lock, as
/// `surfd`'s source does; without, it keeps the trait's default.
#[derive(Clone)]
struct Fed {
    state: Arc<Mutex<State>>,
    one_hold: bool,
}

impl Fed {
    fn new(newest: u64, kept_after_newest: u64, one_hold: bool) -> Self {
        let state = State {
            newest,
            kept_after_newest,
            queue_entries: Limits::default().queue_entries as u64,
            ..State::default()
        };
        Self {
            state: Arc::new(Mutex::new(state)),
            one_hold,
        }
    }

    fn state(&self) -> MutexGuard<'_, State> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Yields `item` live to every reader; the last one gets it without a copy.
    fn publish(&self, item: Item) {
        let mut state = self.state();
        state.newest = state.newest.max(item.number);
        let mut item = Some(item);
        let live = std::mem::take(&mut state.live);
        let last = live.len().saturating_sub(1);
        for (at, sender) in live.into_iter().enumerate() {
            let sent = if at == last { item.take() } else { item.clone() };
            if sent.is_some_and(|sent| sender.send(sent).is_ok()) {
                state.live.push(sender);
            }
        }
    }

    fn reader(&self, state: &mut State) -> Box<dyn Reader> {
        let (sender, live) = mpsc::channel();
        state.live.push(sender);
        Box::new(FedReader {
            live,
            state: Arc::clone(&self.state),
        })
    }

    fn handed(&self, number: u64) -> bool {
        self.state().handed.contains(&number)
    }
}

impl Source for Fed {
    fn sessions(&self) -> Vec<u8> {
        b"{\"newest\":0,\"sessions\":[]}".to_vec()
    }

    fn session(&self, _id: &str) -> Option<Vec<u8>> {
        None
    }

    fn newest(&self) -> u64 {
        let mut state = self.state();
        let newest = state.newest;
        state.newest += state.kept_after_newest;
        newest
    }

    fn read_from(&self, number: u64) -> Result<Box<dyn Reader>, TooOld> {
        let mut state = self.state();
        if state.newest.saturating_sub(number) > state.queue_entries {
            return Err(TooOld);
        }
        Ok(self.reader(&mut state))
    }

    fn read_newest(&self) -> Result<(u64, Box<dyn Reader>), TooOld> {
        if !self.one_hold {
            let newest = self.newest();
            return self.read_from(newest).map(|reader| (newest, reader));
        }
        let mut state = self.state();
        let newest = state.newest;
        Ok((newest, self.reader(&mut state)))
    }
}

struct FedReader {
    live: Receiver<Item>,
    state: Arc<Mutex<State>>,
}

impl Reader for FedReader {
    fn next(&mut self, wait: Duration) -> Next {
        match self.live.recv_timeout(wait) {
            Ok(item) => {
                self.state
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .handed
                    .push(item.number);
                Next::Item(item)
            }
            Err(RecvTimeoutError::Timeout) => Next::Idle,
            Err(RecvTimeoutError::Disconnected) => Next::End,
        }
    }
}

/// An entry whose data is `size` bytes of one letter chosen by its number.
fn entry(number: u64, size: usize) -> Item {
    Item {
        number,
        kind: EventKind::Entry,
        data: vec![b'a' + (number % 26) as u8; size],
    }
}

/// The bytes of `entry(number, size)`'s frame on the stream.
fn frame_len(number: u64, size: usize) -> usize {
    format!("id: {number}\nevent: entry\ndata: \n\n").len() + size
}

// ---------------------------------------------------------------------------------------------
// The server and a client.

fn start(source: &Fed, limits: Limits) -> (Server, u16) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind 127.0.0.1:0");
    let port = listener.local_addr().expect("local address").port();
    let config = Config {
        token: Token::from_hex(TOKEN).expect("token"),
        host: format!("127.0.0.1:{port}"),
        files: Vec::new(),
        limits,
    };
    (serve(listener, config, source.clone()).expect("serve"), port)
}

fn wait_for(what: &str, within: Duration, mut condition: impl FnMut() -> bool) {
    let started = Instant::now();
    while !condition() {
        assert!(started.elapsed() < within, "not within {within:?}: {what}");
        thread::sleep(Duration::from_millis(10));
    }
}

/// One event of the stream: `(id, event name, data)`.
type Event = (u64, String, Vec<u8>);

struct Client(BufReader<TcpStream>);

impl Client {
    /// Opens `/events`, with `Last-Event-ID: <last>` when given, and reads the head, which must
    /// be a 200.
    fn open(port: u16, last: Option<&str>) -> Self {
        let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
        stream.set_read_timeout(Some(WAIT)).expect("read timeout");
        let mut text = format!(
            "GET /events HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nAuthorization: Bearer {TOKEN}\r\n"
        );
        if let Some(last) = last {
            text.push_str(&format!("Last-Event-ID: {last}\r\n"));
        }
        text.push_str("\r\n");
        stream.write_all(text.as_bytes()).expect("send");
        let mut client = Self(BufReader::new(stream));
        let status = client.line().expect("a status line");
        assert_eq!(status, b"HTTP/1.1 200 OK\r\n");
        while client.line().expect("a header line") != b"\r\n" {}
        client
    }

    /// The next line with its end, `None` at the close.
    fn line(&mut self) -> Option<Vec<u8>> {
        let mut line = Vec::new();
        match self.0.read_until(b'\n', &mut line) {
            Ok(0) => None,
            Ok(_) => Some(line),
            Err(error) if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {
                panic!("nothing within {WAIT:?}")
            }
            Err(_) => None,
        }
    }

    /// The next event, `: keep-alive` skipped; `None` when the stream closes.
    fn event(&mut self) -> Option<Event> {
        let (mut id, mut name, mut data) = (None, None, None);
        loop {
            let mut line = self.line()?;
            assert_eq!(line.pop(), Some(b'\n'), "a line ends in LF");
            if line.is_empty() {
                if let (Some(id), Some(name), Some(data)) = (id.take(), name.take(), data.take()) {
                    return Some((id, name, data));
                }
            } else if let Some(value) = line.strip_prefix(b"id: ") {
                id = std::str::from_utf8(value).ok().and_then(|v| v.parse().ok());
            } else if let Some(value) = line.strip_prefix(b"event: ") {
                name = Some(String::from_utf8_lossy(value).into_owned());
            } else if line.starts_with(b"data: ") {
                // The data stays in the line's own buffer: an entry here can be 100 MiB.
                line.drain(..6);
                data = Some(line);
            }
        }
    }

    /// Reads to the close; true when nothing more came.
    fn closes_with_nothing(&mut self) -> bool {
        let mut rest = Vec::new();
        matches!(self.0.read_to_end(&mut rest), Ok(_)) && rest.is_empty()
    }
}

/// Checks that `event` is `entry(number, size)`, without printing megabytes on a failure.
fn assert_entry(event: Option<Event>, number: u64, size: usize) {
    let Some((id, name, data)) = event else {
        panic!("the stream closed before entry {number}");
    };
    assert_eq!((id, name.as_str()), (number, "entry"), "the event after entry {}", number - 1);
    let letter = b'a' + (number % 26) as u8;
    assert!(
        data.len() == size && data.iter().all(|&b| b == letter),
        "entry {number}: {} bytes of data, not the {size} it was published with",
        data.len()
    );
}

// ---------------------------------------------------------------------------------------------
// F1: where a stream starts while entries are kept.

/// `Last-Event-ID` within the kept range when the server reads `newest()`, then more than 256
/// entries kept before its `read_from()`: `reset` with the number the reader starts at, then
/// live from the next number.
#[test]
fn f1_a_number_within_range_that_falls_behind_gets_reset_at_the_readers_number() {
    let source = Fed::new(1_000, 300, true);
    let (server, port) = start(&source, Limits::default());
    let mut client = Client::open(port, Some("1000"));
    let reset = client.event().expect("reset");
    assert_eq!(
        reset,
        (1_300, "reset".to_string(), b"{\"newest\":1300}".to_vec()),
        "reset names the number the reader starts at"
    );
    source.publish(entry(1_301, 8));
    assert_entry(client.event(), 1_301, 8);
    drop(client);
    server.stop();
}

/// Without `Last-Event-ID`, more than 256 entries kept after each `newest()`: the stream opens
/// live and delivers the next entry.
#[test]
fn f1_a_live_stream_opens_while_entries_are_kept() {
    let source = Fed::new(1_000, 300, true);
    let (server, port) = start(&source, Limits::default());
    let mut client = Client::open(port, None);
    let newest = source.state().newest;
    source.publish(entry(newest + 1, 8));
    assert_entry(client.event(), newest + 1, 8);
    drop(client);
    server.stop();
}

/// A source that keeps the default `read_newest` and never gives a reader: the head, `reset`
/// when one is due, then the close (the last resort).
#[test]
fn f1_without_any_reader_the_head_and_reset_come_then_the_close() {
    let source = Fed::new(1_000, 300, false);
    let (server, port) = start(&source, Limits::default());
    let mut client = Client::open(port, Some("1"));
    let (id, name, _) = client.event().expect("reset");
    assert_eq!(name, "reset");
    assert!(id >= 1_000, "reset at {id}");
    assert!(client.closes_with_nothing(), "the stream closes after reset");
    let mut client = Client::open(port, None);
    assert!(client.closes_with_nothing(), "the live stream closes after its head");
    server.stop();
}

// ---------------------------------------------------------------------------------------------
// F2: an entry whose frame alone is over the bound of bytes.

/// One frame of 2 MiB, over the default bound of 1 MiB, into an empty queue: delivered, the
/// stream goes on, and the peak counts it.
#[test]
fn f2_a_frame_over_the_bound_alone_is_delivered_and_counted() {
    let source = Fed::new(0, 0, true);
    let (server, port) = start(&source, Limits::default());
    let mut client = Client::open(port, None);
    source.publish(entry(1, 2 * MIB));
    source.publish(entry(2, 8));
    assert_entry(client.event(), 1, 2 * MIB);
    assert_entry(client.event(), 2, 8);
    assert_eq!(server.open_streams(), 1, "the stream goes on");
    assert_eq!(server.queued_bytes_peak(), frame_len(1, 2 * MIB));
    drop(client);
    server.stop();
}

/// The data of an entry no client that does not read can take: three times the largest send
/// and receive buffers of TCP together (`/proc/sys/net/ipv4/tcp_wmem`, `tcp_rmem`), so the
/// writer stays in its write of it until the client reads or goes away.
fn blocking_size() -> usize {
    let most = |path: &str| {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|text| text.split_whitespace().nth(2)?.parse::<usize>().ok())
    };
    match (
        most("/proc/sys/net/ipv4/tcp_wmem"),
        most("/proc/sys/net/ipv4/tcp_rmem"),
    ) {
        (Some(send), Some(receive)) => 3 * (send + receive),
        _ => 128 * MIB,
    }
}

/// A stream whose client does not read, its bound of bytes [`blocking_size`] and 1 MiB, its
/// write limit 120 s: entries 1 and 2 of 8 bytes; entry 3 of [`blocking_size`], within the bound,
/// which the writer cannot finish; entry 4 of 8 bytes, queued behind it; then entry 5, one byte
/// over the bound, taken by the pump while entries 3 and 4 are queued. Returns the server, the
/// client, the source and the sizes of entries 3 and 5.
fn blocked_then_large() -> (Server, Client, Fed, usize, usize) {
    let blocking = blocking_size();
    let queue_bytes = blocking + MIB;
    let large = queue_bytes + 1;
    let limits = Limits {
        queue_bytes,
        write: Duration::from_secs(120),
        ..Limits::default()
    };
    let source = Fed::new(0, 0, true);
    let (server, port) = start(&source, limits);
    let client = Client::open(port, None);
    source.publish(entry(1, 8));
    source.publish(entry(2, 8));
    source.publish(entry(3, blocking));
    source.publish(entry(4, 8));
    source.publish(entry(5, large));
    wait_for("the pump took entry 5", WAIT, || source.handed(5));
    (server, client, source, blocking, large)
}

/// The large frame arriving while the queue is not empty: delivered after the frames before
/// it, in order, nothing lost; the entries after it follow on the same stream; it was queued
/// alone, so the peak is its frame.
#[test]
fn f2_a_large_frame_behind_a_full_queue_waits_and_comes_in_order() {
    let (server, mut client, source, blocking, large) = blocked_then_large();
    for number in 6..=8 {
        source.publish(entry(number, 8));
    }
    assert_entry(client.event(), 1, 8);
    assert_entry(client.event(), 2, 8);
    assert_entry(client.event(), 3, blocking);
    assert_entry(client.event(), 4, 8);
    assert_entry(client.event(), 5, large);
    for number in 6..=8 {
        assert_entry(client.event(), number, 8);
    }
    assert_eq!(server.open_streams(), 1, "the stream goes on");
    assert_eq!(
        server.queued_bytes_peak(),
        frame_len(5, large),
        "the large frame was queued alone, and counted"
    );
    drop(client);
    server.stop();
}

/// The pump waiting with a large frame ends when the stream closes: the client goes away while
/// entries 3 and 4 are queued, and the stream's threads end within 5 s, long before the write
/// limit of 120 s.
#[test]
fn f2_the_wait_for_an_empty_queue_ends_when_the_stream_closes() {
    let (server, client, _source, _, _) = blocked_then_large();
    assert_eq!(server.open_streams(), 1);
    drop(client);
    wait_for(
        "the stream ends after its client went away",
        Duration::from_secs(5),
        || server.open_streams() == 0,
    );
    server.stop();
}
