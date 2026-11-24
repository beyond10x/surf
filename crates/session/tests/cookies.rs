//! Cases S1 to S7 of `story:agent-cookie-jar`, one test: a session keeps the cookies its pages
//! set, sends them by the rules of `surf-cookie`, counts them in its answers, and shows no
//! cookie anywhere. `surfd` runs in the test process (`surfd_in_process`) with the built
//! `surf-session`, driven by the built `surf-agent`; one page server on 127.0.0.1 port `<p>` and
//! one on 127.0.0.2 port `<q>` (Linux routes 127.0.0.0/8 to the loopback) record the head of
//! every request. The session cookie's value is 32 hexadecimal digits drawn for this run, so no
//! failure text of this test prints it.

#![forbid(unsafe_code)]

use crate::common;
use crate::view::{Events, body, view_url};

use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::process::Output;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use common::{
    Opened, TestDir, agent, golden, opened, placeholders, regular_files_under, run,
    surfd_in_process,
};

/// How long a server waits for a client to send.
const BOUND: Duration = Duration::from_secs(5);

/// One request a server read: its method, its target and its lines whose name is `Cookie` in
/// any case.
#[derive(Debug, PartialEq, Eq)]
struct Hop {
    method: String,
    target: String,
    cookies: Vec<String>,
}

type Route = Box<dyn Fn(&str) -> Vec<u8> + Send>;

/// A server on `ip` port 0 that answers each request with the bytes `route` gives for its target
/// and records it.
struct Recorder {
    ip: &'static str,
    port: u16,
    hops: Arc<Mutex<Vec<Hop>>>,
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl Recorder {
    fn start(ip: &'static str, route: impl Fn(&str) -> Vec<u8> + Send + 'static) -> Recorder {
        let listener = TcpListener::bind((ip, 0)).expect("bind port 0");
        let port = listener.local_addr().expect("local address").port();
        let hops = Arc::new(Mutex::new(Vec::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let route: Route = Box::new(route);
        let handle = {
            let hops = Arc::clone(&hops);
            let stop = Arc::clone(&stop);
            thread::spawn(move || {
                for stream in listener.incoming() {
                    if stop.load(Ordering::SeqCst) {
                        break;
                    }
                    let Ok(stream) = stream else { continue };
                    serve(stream, &route, &hops);
                }
            })
        };
        Recorder {
            ip,
            port,
            hops,
            stop,
            handle: Some(handle),
        }
    }

    fn url(&self, path: &str) -> String {
        format!("http://{}:{}{path}", self.ip, self.port)
    }

    /// The requests read since the last call.
    fn take(&self) -> Vec<Hop> {
        std::mem::take(&mut *self.hops.lock().expect("hops"))
    }
}

impl Drop for Recorder {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            self.stop.store(true, Ordering::SeqCst);
            let _ = TcpStream::connect((self.ip, self.port));
            let _ = handle.join();
        }
    }
}

fn serve(mut stream: TcpStream, route: &Route, hops: &Mutex<Vec<Hop>>) {
    let _ = stream.set_read_timeout(Some(BOUND));
    let _ = stream.set_write_timeout(Some(BOUND));
    let mut bytes = Vec::new();
    let mut buf = [0u8; 4096];
    let end = loop {
        if let Some(at) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
            break at + 4;
        }
        match stream.read(&mut buf) {
            Ok(0) | Err(_) => return,
            Ok(n) => bytes.extend_from_slice(&buf[..n]),
        }
    };
    let head = String::from_utf8_lossy(&bytes[..end]).into_owned();
    let length: usize = head
        .split("\r\n")
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse().ok())?
        })
        .unwrap_or(0);
    while bytes.len() < end + length {
        match stream.read(&mut buf) {
            Ok(0) | Err(_) => break,
            Ok(n) => bytes.extend_from_slice(&buf[..n]),
        }
    }
    let mut words = head.split(' ');
    let method = words.next().unwrap_or_default().to_string();
    let target = words.next().unwrap_or_default().to_string();
    let cookies = head
        .split("\r\n")
        .filter(|line| {
            line.split_once(':')
                .is_some_and(|(name, _)| name.eq_ignore_ascii_case("cookie"))
        })
        .map(str::to_string)
        .collect();
    let reply = route(&target);
    hops.lock().expect("hops").push(Hop {
        method,
        target,
        cookies,
    });
    let _ = stream.write_all(&reply);
    let _ = stream.flush();
    let _ = stream.shutdown(Shutdown::Write);
    while let Ok(n) = stream.read(&mut buf) {
        if n == 0 {
            break;
        }
    }
}

/// A response of `status` with `fields`, an HTML `body` and its length.
fn reply(status: u16, fields: &[String], body: &str) -> Vec<u8> {
    let mut text = format!("HTTP/1.1 {status} Status\r\n");
    for field in fields {
        text.push_str(field);
        text.push_str("\r\n");
    }
    text.push_str(&format!(
        "Content-Type: text/html\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    ));
    text.into_bytes()
}

/// `method target` received `Cookie: <cookie>`, or no `Cookie` for `None`.
fn hop(method: &str, target: &str, cookie: Option<&str>) -> Hop {
    Hop {
        method: method.to_string(),
        target: target.to_string(),
        cookies: cookie.map(|c| format!("Cookie: {c}")).into_iter().collect(),
    }
}

/// 32 hexadecimal digits from the random source.
fn digits() -> String {
    let mut bytes = [0u8; 16];
    surf_secret::fill_random(&mut bytes).expect("the random source");
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Every `surf-agent` call of the case, its standard output and error kept for S6.
struct Calls<'a> {
    dir: &'a TestDir,
    outputs: Vec<Output>,
}

impl Calls<'_> {
    /// `surf-agent open`, which must exit 0.
    fn open(&mut self) -> Opened {
        let output = run(agent(self.dir).arg("open"));
        assert_eq!(output.status.code(), Some(0), "open");
        let session = opened(&String::from_utf8_lossy(&output.stdout));
        self.outputs.push(output);
        session
    }

    /// `surf-agent <words>` as the holder of `session`, which must exit 0; its standard output.
    fn call(&mut self, session: &Opened, words: &[&str]) -> String {
        let output = run(agent(self.dir)
            .args(words)
            .env("SURF_SESSION", &session.id)
            .env("SURF_TOKEN", &session.token));
        assert_eq!(output.status.code(), Some(0), "{words:?}");
        let text = String::from_utf8_lossy(&output.stdout).into_owned();
        self.outputs.push(output);
        text
    }
}

/// The handle of the line of the `now` block of `text` whose label is `label`.
fn handle_of(text: &str, label: &str) -> String {
    let quoted = format!("\"{label}\"");
    text.lines()
        .filter(|line| line.starts_with("  ") && line.contains(&quoted))
        .find_map(|line| line.split_whitespace().next())
        .unwrap_or_else(|| panic!("no now line labelled {label}: {text}"))
        .to_string()
}

#[test]
fn a_session_keeps_its_cookies_to_itself() {
    let digits = digits();
    let session_cookie = format!("zqsid={digits}");
    let set_session = format!("Set-Cookie: {session_cookie}; Path=/");
    let site = Recorder::start("127.0.0.1", move |target| match target {
        "/login" => reply(
            302,
            &["Location: /home".to_string(), set_session.clone()],
            "",
        ),
        "/home" => reply(200, &[], "<title>Home</title><p>Signed in.</p>"),
        "/set-site" => reply(
            200,
            &[
                "Set-Cookie: lax=1; SameSite=Lax; Path=/".to_string(),
                "Set-Cookie: strict=1; SameSite=Strict; Path=/".to_string(),
                "Set-Cookie: dflt=1; Path=/".to_string(),
            ],
            "<p>set</p>",
        ),
        _ => reply(200, &[], "<p>echo</p>"),
    });
    let echo = site.url("/echo");
    let from_page = format!(
        "<title>From</title><a href=\"{echo}\">Echo</a>\
         <form method=\"post\" action=\"{echo}\"><button>Send</button></form>"
    );
    let other = Recorder::start("127.0.0.2", move |_| reply(200, &[], &from_page));
    let dir = TestDir::new();
    let daemon = surfd_in_process(&dir, |_| {});
    let view = view_url(&dir);
    let events = Events::open(&view);
    let mut calls = Calls {
        dir: &dir,
        outputs: Vec::new(),
    };

    // S1: the redirect's cookie reaches the next hop.
    let a = calls.open();
    calls.call(&a, &["go", &site.url("/login")]);
    assert!(
        site.take()
            == [
                hop("GET", "/login", None),
                hop("GET", "/home", Some(&session_cookie))
            ],
        "S1: /home did not receive exactly the cookie /login set"
    );

    // S2: the answer counts the cookie and names none.
    calls.call(&a, &["look"]);
    let look = calls.call(&a, &["look"]);
    let parts = surf::NOT_YET_PARSED.join(",");
    let expected = golden("look-cookies.txt").replace(
        "note parser: {NOT_YET_PARSED}\n",
        &if parts.is_empty() {
            String::new()
        } else {
            format!("note parser: {parts}\n")
        },
    );
    assert_eq!(
        placeholders(&look, &a.id, &a.token, site.port),
        expected,
        "S2"
    );

    // S3: another session has none of them.
    let b = calls.open();
    calls.call(&b, &["go", &site.url("/home")]);
    assert!(
        site.take() == [hop("GET", "/home", None)],
        "S3: B sent a cookie"
    );
    let b_look = calls.call(&b, &["look"]);
    assert!(
        !b_look.lines().any(|line| line.starts_with("cookies")),
        "S3: {b_look}"
    );

    // S4: a cross-site request sends by SameSite and method.
    calls.call(&a, &["go", &site.url("/set-site")]);
    calls.call(&a, &["go", &other.url("/from")]);
    site.take();
    let from = calls.call(&a, &["look"]);
    calls.call(&a, &["follow", &handle_of(&from, "Echo")]);
    let cross_get = format!("{session_cookie}; lax=1; dflt=1");
    assert!(
        site.take() == [hop("GET", "/echo", Some(&cross_get))],
        "S4: the follow from 127.0.0.2 did not send the session, lax and dflt cookies"
    );
    // Decision 9: `back` is the browser's control and has no initiator, so going back over the
    // page of 127.0.0.2 to the page of 127.0.0.1 is same-site and sends the Strict cookie.
    let all = format!("{session_cookie}; lax=1; strict=1; dflt=1");
    calls.call(&a, &["back"]);
    calls.call(&a, &["back"]);
    assert!(
        site.take() == [hop("GET", "/set-site", Some(&all))],
        "decision 9: back to /set-site did not send all four cookies"
    );
    calls.call(&a, &["go", &other.url("/from")]);
    let from = calls.call(&a, &["look"]);
    calls.call(&a, &["press", &handle_of(&from, "Send")]);
    assert!(
        site.take() == [hop("POST", "/echo", None)],
        "S4: the cross-site POST sent a cookie"
    );
    calls.call(&a, &["go", &echo]);
    let same = format!("{session_cookie}; lax=1; strict=1; dflt=1");
    assert!(
        site.take() == [hop("GET", "/echo", Some(&same))],
        "S4: go did not send all four cookies"
    );
    let a_look = calls.call(&a, &["look"]);
    assert!(a_look.contains("\ncookies 4\n"), "S4: {a_look}");

    // S5: a closed session's cookies are gone with it.
    let closed = calls.call(&a, &["close"]);
    assert!(
        !closed.lines().any(|line| line.starts_with("cookies")),
        "S5: {closed}"
    );
    let c = calls.open();
    calls.call(&c, &["go", &echo]);
    assert!(
        site.take() == [hop("GET", "/echo", None)],
        "S5: C sent a cookie"
    );
    events.until(|event| {
        event.event == "session" && event.data.contains(&c.id) && event.data.contains("/echo")
    });

    // S6: no cookie in any output, event, view answer, entry or file.
    let mut seen: Vec<(String, Vec<u8>)> = Vec::new();
    for (at, output) in calls.outputs.iter().enumerate() {
        seen.push((format!("call {at} stdout"), output.stdout.clone()));
        seen.push((format!("call {at} stderr"), output.stderr.clone()));
    }
    seen.push(("the event stream".to_string(), events.bytes()));
    seen.push(("/api/sessions".to_string(), body(&view, "/api/sessions")));
    for session in [&a, &b, &c] {
        let detail = format!("/api/sessions/{}", session.id);
        seen.push((detail.clone(), body(&view, &detail)));
        seen.push((
            format!("the entries of {}", session.id),
            format!("{:?}", daemon.session_entries(&session.id)).into_bytes(),
        ));
    }
    for file in regular_files_under(&dir.runtime()) {
        let bytes = std::fs::read(&file).unwrap_or_default();
        seen.push((file.display().to_string(), bytes));
    }
    for (what, bytes) in &seen {
        for needle in ["zqsid", digits.as_str()] {
            assert!(
                !bytes.windows(needle.len()).any(|w| w == needle.as_bytes()),
                "S6: {what} holds the session cookie's name or value"
            );
        }
    }
    drop(events);
    drop(daemon);
}
