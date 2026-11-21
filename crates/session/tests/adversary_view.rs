//! Adversary cases against `story:view-surfd-serves`, over the real session process with the
//! shipped limits: an entry whose `data:` line alone is more than the stream's queue of 1 MiB
//! (`Limits::queue_bytes`). The answer entry of a `go` holds the page line's address and the
//! last five actions of the path, each argument an address of up to `REQUEST_MAX` bytes, and
//! the JSON writes each `&` of an address as the six bytes `&`.

#![forbid(unsafe_code)]

mod common;

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::time::Duration;

use common::{TestDir, surfd_in_process, wire_as, wire_open};

/// The bytes of `&` in the query of each address: 60,000, so the request stays under
/// `wire::REQUEST_MAX` (64 KiB) and the JSON of one address is 360,000 bytes.
const AMPERSANDS: usize = 60_000;

/// A port on 127.0.0.1 that nothing listens on: one the system gave and the test let go.
fn closed_port() -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("a port");
    listener.local_addr().unwrap().port()
}

fn view_url(dir: &TestDir) -> (u16, String) {
    let line = std::fs::read_to_string(dir.surf().join("view-url")).expect("view-url");
    let rest = line
        .strip_prefix("http://127.0.0.1:")
        .expect("http://127.0.0.1:");
    let (port, rest) = rest.split_once("/#").expect("/#");
    (
        port.parse().expect("a port"),
        rest.strip_suffix('\n').expect("a line feed").to_string(),
    )
}

/// One connection of `GET /events` from `Last-Event-ID: last`, read to its end or until `want`
/// is received: the `id:` and `event:` of each event.
fn one_stream(port: u16, token: &str, last: u64, want: u64) -> Vec<(u64, String)> {
    let mut socket = TcpStream::connect(("127.0.0.1", port)).expect("the view listens");
    socket
        .set_read_timeout(Some(Duration::from_secs(10)))
        .unwrap();
    let request = format!(
        "GET /events HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nAuthorization: Bearer {token}\r\n\
         Last-Event-ID: {last}\r\n\r\n"
    );
    socket.write_all(request.as_bytes()).unwrap();
    let mut reader = BufReader::new(socket);
    let mut events = Vec::new();
    let (mut id, mut event) = (None, None);
    let mut line = String::new();
    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) | Err(_) => return events,
            Ok(_) => {}
        }
        let text = line.trim_end_matches(['\r', '\n']);
        if text.is_empty() {
            if let (Some(id), Some(event)) = (id.take(), event.take()) {
                let done = id == want && event == "entry";
                events.push((id, event));
                if done {
                    return events;
                }
            }
        } else if let Some(value) = text.strip_prefix("id: ") {
            id = value.parse().ok();
        } else if let Some(value) = text.strip_prefix("event: ") {
            event = Some(value.to_string());
        }
    }
}

/// `docs/view-stream.md`, "Where a stream starts": with `Last-Event-ID: <n>` not more than 256
/// below the newest and no entry above it dropped, every kept entry above `<n>` in number order,
/// then live; "A stream is closed, and its reader resumes with `Last-Event-ID`". A reader that
/// resumes from the last `id:` it read each time must get every entry, or a `reset`.
#[test]
fn adversary_an_entry_over_the_stream_queue_is_never_delivered_and_never_reset() {
    let dir = TestDir::new();
    let daemon = surfd_in_process(&dir, |_| {});
    let session = wire_open(&dir, None);
    let before = daemon.newest_entry();
    let url = format!(
        "http://127.0.0.1:{}/?{}",
        closed_port(),
        "&".repeat(AMPERSANDS)
    );
    for _ in 0..3 {
        wire_as(&dir, &session, "go", &[("url", &url)]);
    }
    let kept: Vec<u64> = daemon
        .session_entries(&session.id)
        .expect("listed")
        .entries
        .iter()
        .map(|entry| entry.number)
        .filter(|&number| number > before)
        .collect();
    let newest = daemon.newest_entry();
    assert_eq!(kept.last().copied(), Some(newest), "the gos kept entries");
    assert!(newest - before <= 256, "a replay within the 256 numbers");

    let (port, token) = view_url(&dir);
    let mut received: Vec<u64> = Vec::new();
    let mut last = before;
    let mut stuck = 0;
    while last < newest && stuck < 3 {
        let events = one_stream(port, &token, last, newest);
        assert!(
            events.iter().all(|(_, event)| event != "reset"),
            "a reset with nothing dropped: {events:?}"
        );
        let entries: Vec<u64> = events
            .iter()
            .filter(|(_, event)| event == "entry")
            .map(|(id, _)| *id)
            .collect();
        match entries.last() {
            Some(&id) => {
                received.extend(&entries);
                last = id;
                stuck = 0;
            }
            None => stuck += 1,
        }
    }
    let next = kept.iter().find(|&&number| number > last).copied();
    assert_eq!(
        received, kept,
        "docs/view-stream.md \"Where a stream starts\": every kept entry above Last-Event-ID \
         {before} is replayed (newest {newest}, 256 numbers allow it, nothing dropped); a reader \
         resuming from its last id: three times in a row got no entry after {last}, so entry \
         {next:?} is never delivered, nothing after it either, and no reset is sent"
    );
}
