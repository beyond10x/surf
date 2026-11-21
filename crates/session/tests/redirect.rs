//! Case 23 of `story:agent-fetch-redirects`: a session over `surf_session::fetch`, in process,
//! goes to an address that redirects. The session's page is the address the response came
//! from; its path, and the request, keep the address asked for; the redirect adds no entry.

#![forbid(unsafe_code)]

mod common;

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use common::request;
use surf_session::Session;
use surfd::link::{self, EntryKind};

const ID: &str = "4c2a9e1d-7b3f-4d5e-8a6b-9c0d1e2f3a4b";

/// An HTTP server on `127.0.0.1` port 0: `/a` answers 302 with `Location: /b`, every other
/// target 200 `text/html`. Stopped when dropped.
struct Redirecting {
    port: u16,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl Redirecting {
    fn start() -> Redirecting {
        let listener = TcpListener::bind("127.0.0.1:0").expect("a port on 127.0.0.1");
        let port = listener.local_addr().expect("local address").port();
        let stop = Arc::new(AtomicBool::new(false));
        let stopping = Arc::clone(&stop);
        let thread = thread::spawn(move || {
            for stream in listener.incoming() {
                if stopping.load(Ordering::SeqCst) {
                    break;
                }
                if let Ok(stream) = stream {
                    answer(stream);
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

fn answer(mut stream: TcpStream) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));
    let mut head = Vec::new();
    let mut buf = [0u8; 1024];
    while !head.windows(4).any(|w| w == b"\r\n\r\n") && head.len() < 16 * 1024 {
        match stream.read(&mut buf) {
            Ok(0) | Err(_) => return,
            Ok(n) => head.extend_from_slice(&buf[..n]),
        }
    }
    let reply: &[u8] = if head.starts_with(b"GET /a ") {
        b"HTTP/1.1 302 Found\r\nLocation: /b\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
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
    let server = Redirecting::start();
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
