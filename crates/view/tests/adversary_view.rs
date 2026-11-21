//! Adversary cases against `story:view-surfd-serves`: the stream's start in `sse.rs` over a
//! source that answers `read_from` as `surfd`'s source does (`crates/surfd/src/view.rs`):
//! `TooOld` when the newest number is more than `Limits::queue_entries` above the number asked
//! for. Between the server's call of `newest()` and its call of `read_from()`, `surfd` keeps
//! entries (one answer's entries are kept in one hold of its lock, `daemon.rs` `settle`); the
//! source below keeps 300 of them right after each `newest()`.

#![forbid(unsafe_code)]

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use surf_secret::Token;
use surf_view::{Config, Limits, Next, Reader, Source, TooOld, serve};

const TOKEN: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

/// Entries kept between the server's `newest()` and its `read_from()`: more than the 256 of
/// `Limits::queue_entries`.
const KEPT_BETWEEN: u64 = 300;

struct Idle;

impl Reader for Idle {
    fn next(&mut self, wait: Duration) -> Next {
        std::thread::sleep(wait.min(Duration::from_millis(10)));
        Next::Idle
    }
}

/// The newest number moves by [`KEPT_BETWEEN`] after each read of it, and `read_from` answers
/// as `surfd`'s `View::read_from` does.
struct Busy {
    newest: Arc<AtomicU64>,
    queue_entries: u64,
}

impl Source for Busy {
    fn sessions(&self) -> Vec<u8> {
        b"{\"newest\":0,\"sessions\":[]}".to_vec()
    }

    fn session(&self, _id: &str) -> Option<Vec<u8>> {
        None
    }

    fn newest(&self) -> u64 {
        self.newest.fetch_add(KEPT_BETWEEN, Ordering::SeqCst)
    }

    fn read_from(&self, number: u64) -> Result<Box<dyn Reader>, TooOld> {
        let newest = self.newest.load(Ordering::SeqCst);
        if newest.saturating_sub(number) > self.queue_entries {
            return Err(TooOld);
        }
        Ok(Box::new(Idle))
    }
}

/// What the server writes to `GET /events` with `last` as `Last-Event-ID`, read until it closes
/// or for 2 s, whichever comes first.
fn events_head(last: Option<&str>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("a port on 127.0.0.1");
    let port = listener.local_addr().unwrap().port();
    let limits = Limits::default();
    let source = Busy {
        newest: Arc::new(AtomicU64::new(1_000)),
        queue_entries: limits.queue_entries as u64,
    };
    let config = Config {
        token: Token::from_hex(TOKEN).unwrap(),
        host: format!("127.0.0.1:{port}"),
        files: Vec::new(),
        limits,
    };
    let server = serve(listener, config, source).expect("the server starts");
    let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    let mut request =
        format!("GET /events HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nAuthorization: Bearer {TOKEN}\r\n");
    if let Some(last) = last {
        request.push_str(&format!("Last-Event-ID: {last}\r\n"));
    }
    request.push_str("\r\n");
    stream.write_all(request.as_bytes()).unwrap();
    let mut read = Vec::new();
    let mut chunk = [0u8; 4096];
    // The head, and with a Last-Event-ID the first event after it.
    let complete = |read: &[u8]| {
        let text = String::from_utf8_lossy(read);
        match text.split_once("\r\n\r\n") {
            Some((_, after)) => last.is_none() || after.contains("\n\n"),
            None => false,
        }
    };
    while !complete(&read) {
        match stream.read(&mut chunk) {
            Ok(0) | Err(_) => break,
            Ok(n) => read.extend_from_slice(&chunk[..n]),
        }
    }
    drop(stream);
    server.stop();
    String::from_utf8_lossy(&read).into_owned()
}

/// `docs/view-stream.md`, "Where a stream starts": without `Last-Event-ID` the stream starts
/// after the newest number at the time it opens, live only. It is answered 200 whatever was
/// kept meanwhile.
#[test]
fn adversary_a_stream_without_last_event_id_is_answered_while_entries_are_kept() {
    let head = events_head(None);
    assert!(
        head.starts_with("HTTP/1.1 200 OK\r\n"),
        "docs/view-stream.md \"Where a stream starts\": a stream without Last-Event-ID opens \
         live; with more than 256 entries kept between sse.rs's newest() and read_from() the \
         server wrote {head:?} and closed the connection"
    );
}

/// `docs/view-stream.md`, "Where a stream starts": a `Last-Event-ID` older than what is kept is
/// answered with `reset` first, then live.
#[test]
fn adversary_a_stream_from_an_old_number_is_reset_while_entries_are_kept() {
    let head = events_head(Some("1"));
    assert!(
        head.starts_with("HTTP/1.1 200 OK\r\n") && head.contains("event: reset\n"),
        "docs/view-stream.md \"Where a stream starts\": an old Last-Event-ID is answered with \
         reset; with more than 256 entries kept between sse.rs's newest() and read_from() the \
         server wrote {head:?} and closed the connection"
    );
}
