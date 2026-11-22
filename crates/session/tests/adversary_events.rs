//! Adversary cases against `story:agent-session-events`, through `Daemon::start` with the built
//! `surf-session`.

#![forbid(unsafe_code)]

use crate::common;

use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;
use std::time::{Duration, Instant};

use common::{TestDir, surfd_in_process, wire_as, wire_open};
use surfd::link::EntryKind;
use surfd::log::Entry;

/// How long a case waits for an event of the session: under the runner's 60 s, so a failure
/// says what was awaited.
const WAIT: Duration = Duration::from_secs(20);

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
///
/// The answer timeout is 0.5 s and the server answers 1.5 s after it read the request, which
/// it reads after `surfd` sent the `go`: each step 1 s apart. Then `look` until the path shows
/// the `go` (a `look` while the late answer holds the session's turn is answered status 3),
/// at most [`WAIT`].
#[test]
fn adversary_an_action_recorded_after_a_session_timeout_has_its_entries_kept() {
    let dir = TestDir::new();
    let daemon = surfd_in_process(&dir, |config| {
        config.session_answer_timeout = Duration::from_millis(500);
    });
    let session = wire_open(&dir, None);
    let port = slow_server(Duration::from_millis(1500));
    let url = format!("http://127.0.0.1:{port}/slow");
    let answer = wire_as(&dir, &session, "go", &[("url", &url)]);
    assert_eq!(answer.status, 3, "the go timed out in surfd: {answer:?}");
    let path_line = format!("  2 go {}\n", surf_answer::quote(&url));
    let started = Instant::now();
    let look = loop {
        let look = wire_as(&dir, &session, "look", &[]);
        if (look.status == 0 && look.text.contains(&path_line)) || started.elapsed() > WAIT {
            break look;
        }
        thread::sleep(Duration::from_millis(50));
    };
    assert_eq!(look.status, 0, "{look:?}");
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
