//! Adversary cases against `story:agent-session-events`, through `Daemon::start` with the built
//! `surf-session`.

#![forbid(unsafe_code)]

mod common;

use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;
use std::time::Duration;

use common::{TestDir, surfd_in_process, wire_as, wire_open};
use surfd::link::EntryKind;
use surfd::log::Entry;

fn kept(daemon: &surfd::Daemon, id: &str) -> Vec<Entry> {
    daemon
        .session_entries(id)
        .expect("the session is listed")
        .entries
}

/// A server that accepts, waits `delay`, then answers a short HTML page.
fn slow_server(delay: Duration) -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    thread::spawn(move || {
        for stream in listener.incoming().take(4) {
            let Ok(mut stream) = stream else { continue };
            thread::spawn(move || {
                let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));
                let mut buffer = [0u8; 4096];
                let _ = stream.read(&mut buffer);
                thread::sleep(delay);
                let body = b"<!doctype html><p>slow</p>";
                let head = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\n\
                     Connection: close\r\n\r\n",
                    body.len()
                );
                let _ = stream.write_all(head.as_bytes());
                let _ = stream.write_all(body);
            });
        }
    });
    port
}

/// D19 and "Change": after each command that records an action, its action entry, its events
/// and its answer entry are kept. A `go` whose answer arrives after `surfd`'s answer timeout is
/// recorded by the session (its path shows it), and the log never holds its action entry.
#[test]
fn adversary_an_action_recorded_after_a_session_timeout_has_its_entries_kept() {
    let dir = TestDir::new();
    let daemon = surfd_in_process(&dir, |config| {
        config.session_answer_timeout = Duration::from_secs(2);
    });
    let session = wire_open(&dir, None);
    let port = slow_server(Duration::from_secs(4));
    let url = format!("http://127.0.0.1:{port}/slow");
    let answer = wire_as(&dir, &session, "go", &[("url", &url)]);
    assert_eq!(answer.status, 3, "the go timed out in surfd: {answer:?}");
    thread::sleep(Duration::from_secs(5));
    let look = wire_as(&dir, &session, "look", &[]);
    assert_eq!(look.status, 0, "{look:?}");
    let path_line = format!("  2 go {}\n", surf_answer::quote(&url));
    assert!(
        look.text.contains(&path_line),
        "the session recorded the go: {}",
        look.text
    );
    let entries = kept(&daemon, &session.id);
    let action = format!("action 2 go {}\n", surf_answer::quote(&url));
    assert!(
        entries
            .iter()
            .any(|entry| entry.kind == EntryKind::Action && entry.text == action),
        "the log holds no action entry for the recorded go: {:#?}",
        entries
            .iter()
            .map(|entry| entry.text.lines().next().unwrap_or("").to_string())
            .collect::<Vec<_>>()
    );
}
