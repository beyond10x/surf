//! Case 23 of `story:agent-fetch-redirects`: a session over `surf_session::fetch`, in process,
//! goes to an address that redirects. The session's page is the address the response came
//! from; its path, and the request, keep the address asked for; the redirect adds no entry.
//!
//! Case 24, from the adversarial review of that story: a redirect to an address of
//! [`surf_fetch::MAX_ADDRESS_BYTES`] whose quoted form does not fit the status record. The
//! answer holds it, the entries stay within their limit, and the record goes without it.

#![forbid(unsafe_code)]

use crate::common;

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use common::{opened, request};
use surf_session::Session;
use surfd::link::{self, EntryKind};

const ID: &str = "4c2a9e1d-7b3f-4d5e-8a6b-9c0d1e2f3a4b";

/// The 302 of case 23: `Location: /b`.
const TO_B: &[u8] =
    b"HTTP/1.1 302 Found\r\nLocation: /b\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";

/// An HTTP server on `127.0.0.1` port 0: `/a` answers with the redirect it is given, every
/// other target 200 `text/html`. Stopped when dropped.
struct Redirecting {
    port: u16,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl Redirecting {
    /// `redirect` makes the answer to `/a` from the server's port.
    fn start(redirect: impl FnOnce(u16) -> Vec<u8>) -> Redirecting {
        let listener = TcpListener::bind("127.0.0.1:0").expect("a port on 127.0.0.1");
        let port = listener.local_addr().expect("local address").port();
        let redirect = redirect(port);
        let stop = Arc::new(AtomicBool::new(false));
        let stopping = Arc::clone(&stop);
        let thread = thread::spawn(move || {
            for stream in listener.incoming() {
                if stopping.load(Ordering::SeqCst) {
                    break;
                }
                if let Ok(stream) = stream {
                    answer(stream, &redirect);
                }
            }
        });
        Redirecting {
            port,
            stop,
            thread: Some(thread),
        }
    }

    fn url(&self, path: &str) -> String {
        format!("http://127.0.0.1:{}{path}", self.port)
    }
}

impl Drop for Redirecting {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect(("127.0.0.1", self.port));
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// Answer one request, read to the end of its head (at most 128 KiB, so the request for an
/// address of 64 KiB is read whole before the answer).
fn answer(mut stream: TcpStream, redirect: &[u8]) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));
    let mut head = Vec::new();
    let mut buf = [0u8; 1024];
    while !head.ends_with(b"\r\n\r\n") && head.len() < 128 * 1024 {
        match stream.read(&mut buf) {
            Ok(0) | Err(_) => return,
            Ok(n) => head.extend_from_slice(&buf[..n]),
        }
    }
    let reply: &[u8] = if head.starts_with(b"GET /a ") {
        redirect
    } else {
        b"HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: 15\r\nConnection: \
          close\r\n\r\n<p>a page</p>\r\n"
    };
    let _ = stream.write_all(reply);
}

/// The link answer of `open url=<address>` on a new session over `surf_session::fetch`.
fn open(address: &str) -> link::Answer {
    let mut session = Session::new(ID, surf_session::fetch).expect("a session");
    let reply = session.answer(&request("open", None, None, &[("url", address)]));
    reply.to_link()
}

/// The one event entry named `name`.
fn event<'a>(answer: &'a link::Answer, name: &str) -> &'a str {
    let prefix = format!("event {name} ");
    let found: Vec<&str> = answer
        .entries
        .iter()
        .filter(|entry| entry.kind == EntryKind::Event && entry.text.starts_with(&prefix))
        .map(|entry| entry.text.as_str())
        .collect();
    assert_eq!(found.len(), 1, "{name}: {:?}", answer.entries);
    found[0]
}

#[test]
fn case_23_a_redirect_shows_the_address_it_ended_at_and_adds_no_entry() {
    let server = Redirecting::start(|_| TO_B.to_vec());
    let asked = server.url("/a");
    let ended = server.url("/b");
    let redirected = open(&asked);
    let straight = open(&ended);

    assert_eq!(redirected.status, 0, "{}", redirected.text);
    let lines: Vec<&str> = redirected.text.lines().collect();
    assert!(
        lines.contains(&format!("page 200 \"text/html\" \"{ended}\"").as_str()),
        "{}",
        redirected.text
    );
    assert!(
        lines.contains(&format!("  2 go \"{asked}\"").as_str()),
        "{}",
        redirected.text
    );

    assert_eq!(straight.status, 0, "{}", straight.text);
    assert_eq!(redirected.entries.len(), straight.entries.len());

    let url_field = |address: &str| format!(" url={} ", surf_answer::quote(address));
    let started = event(&redirected, "surf.net.RequestStarted");
    assert!(started.contains(url_field(&asked).trim_end()), "{started}");
    let received = event(&redirected, "surf.net.ResponseReceived");
    assert!(received.contains(&url_field(&ended)), "{received}");
}

#[test]
fn case_24_a_redirect_to_an_address_that_does_not_fit_the_record_is_answered_without_it() {
    // The address the redirect leads to: `/a?` and backslashes, exactly MAX_ADDRESS_BYTES
    // long. surf-url does not percent-encode a backslash in a query, and the quoting writes
    // each as `\x5c`.
    let location = |port: u16| {
        let prefix_length = format!("http://127.0.0.1:{port}/a?").len();
        format!(
            "?{}",
            "\\".repeat(surf_fetch::MAX_ADDRESS_BYTES - prefix_length)
        )
    };
    // The shortest head the fetch takes, so the Location fits the header limit of 64 KiB.
    let redirect = |port: u16| format!("HTTP/1.1 302\nLocation:{}\n\n", location(port));
    let server = Redirecting::start(|port| redirect(port).into_bytes());
    assert!(
        redirect(server.port).len() <= surf_fetch::Limits::default().max_header_bytes,
        "the redirect's head is {} bytes",
        redirect(server.port).len()
    );
    let asked = server.url("/a");
    let prefix_length = server.url("/a?").len();
    let backslashes = location(server.port).len() - 1;
    let ended = format!("{asked}{}", location(server.port));
    assert_eq!(ended.len(), 65_536);
    assert_eq!(ended.len(), surf_fetch::MAX_ADDRESS_BYTES);
    let quoted = surf_answer::quote(&ended);
    assert_eq!(quoted.len(), 2 + prefix_length + 4 * backslashes);
    assert!(quoted.len() <= 262_146, "{}", quoted.len());
    assert!(quoted.len() > link::RECORD_MAX, "{}", quoted.len());

    let mut session = Session::new(ID, surf_session::fetch).expect("a session");
    let open = session.answer(&request("open", None, None, &[])).to_link();
    let token = opened(&open.text).token;
    let link = session
        .answer(&request("go", Some(ID), Some(&token), &[("url", &asked)]))
        .to_link();

    assert_eq!(link.status, 0, "{:?}", link.reason);
    let lines: Vec<&str> = link.text.lines().collect();
    assert!(
        lines.contains(&format!("page 200 \"text/html\" {quoted}").as_str()),
        "no page line with the whole address quoted"
    );
    assert!(lines.contains(&format!("did go \"{asked}\"").as_str()));
    assert_eq!(surf_agent::wire::ANSWER_TEXT_MAX, 4_194_304);
    assert!(
        link.text.len() <= surf_agent::wire::ANSWER_TEXT_MAX,
        "the answer is {} bytes",
        link.text.len()
    );

    assert!(!link.entries.is_empty());
    let mut entries_bytes = 0;
    for entry in &link.entries {
        match entry.kind {
            EntryKind::Action | EntryKind::Event => {
                assert!(entry.text.ends_with('\n'), "{:.80}", entry.text);
                assert_eq!(entry.text.matches('\n').count(), 1, "{:.80}", entry.text);
            }
            EntryKind::Answer => {
                entries_bytes += format!("answer-entry {}\n", entry.text.len()).len();
            }
        }
        entries_bytes += entry.text.len();
    }
    assert_eq!(link::ENTRIES_MAX, 8_388_608);
    assert!(
        entries_bytes <= link::ENTRIES_MAX,
        "the entries are {entries_bytes} bytes"
    );
    let received = event(&link, "surf.net.ResponseReceived");
    assert!(
        received.contains(&format!(" url={quoted} ")),
        "{received:.80}"
    );

    assert_eq!(link.address, None, "the record goes without the address");
    assert!(link.encode().is_ok(), "the link answer encodes");
}
