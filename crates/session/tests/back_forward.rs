//! Cases 1 to 13 of `story:agent-back-forward`: `back` and `forward` through the session's
//! history. Cases 1 to 8, 10, 11 and 13 drive the session core against a [`Pages`] server on
//! `127.0.0.1` the test starts, through the binary's transport [`surf_session::fetch`]; case 9
//! gives the core a transport in memory; case 12 runs `surfd` in the test process (decision D29)
//! with the built `surf-session`.

#![forbid(unsafe_code)]

use crate::common;
use crate::core::{Fetch, ID, call, holder, open_at};

use std::io::{ErrorKind, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use common::{PageServer, TestDir, opened, request, surfd_in_process, wire_as, wire_open};
use surf_answer::{Answer, Did, RefusalReason};
use surf_session::Session;
use surfd::link::EntryKind;

const PAGE_A: &str = "<!doctype html><title>A</title><p><a href=\"/b\">to B</a>";
const PAGE_B: &str = "<!doctype html><title>B</title><p><a href=\"/a\">to A</a> \
                      <a href=\"/c\">to C</a>";
const PAGE_C: &str = "<!doctype html><title>C</title><p>The end.";
const FORM: &str = "<!doctype html><title>Form</title><form action=\"/sent\" method=post>\
                    <input name=q value=1><button>Send</button></form>";
const SENT: &str = "<!doctype html><title>Sent</title><p><a href=\"/b\">to B</a>";
const SEE_FORM: &str = "<!doctype html><title>Search</title><form action=\"/see\" method=post>\
                        <input name=q value=1><button>Send</button></form>";
const PAGE_R: &str = "<!doctype html><title>R</title><p><a href=\"/b\">to B</a>";

fn html(body: &str) -> String {
    format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: \
         close\r\n\r\n{body}",
        body.len()
    )
}

/// An HTTP server on `127.0.0.1` with the pages above, `/see` a 303 to `/r`; it records the
/// request line of each request it reads. While it is down it closes every connection before
/// reading it, so a fetch ends `Unreachable`.
struct Pages {
    port: u16,
    lines: Arc<Mutex<Vec<String>>>,
    down: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl Pages {
    fn start() -> Pages {
        let listener = TcpListener::bind("127.0.0.1:0").expect("a port on 127.0.0.1");
        let port = listener.local_addr().unwrap().port();
        let lines = Arc::new(Mutex::new(Vec::new()));
        let down = Arc::new(AtomicBool::new(false));
        let stop = Arc::new(AtomicBool::new(false));
        let (recording, closing, stopping) =
            (Arc::clone(&lines), Arc::clone(&down), Arc::clone(&stop));
        let thread = thread::spawn(move || {
            for stream in listener.incoming() {
                if stopping.load(Ordering::SeqCst) {
                    break;
                }
                let Ok(stream) = stream else { continue };
                if closing.load(Ordering::SeqCst) {
                    drop(stream);
                    continue;
                }
                serve(stream, &recording);
            }
        });
        Pages {
            port,
            lines,
            down,
            stop,
            thread: Some(thread),
        }
    }

    fn url(&self, path: &str) -> String {
        format!("http://127.0.0.1:{}{path}", self.port)
    }

    fn lines(&self) -> Vec<String> {
        self.lines.lock().unwrap().clone()
    }

    fn set_down(&self, down: bool) {
        self.down.store(down, Ordering::SeqCst);
    }
}

impl Drop for Pages {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        self.down.store(false, Ordering::SeqCst);
        let _ = TcpStream::connect(("127.0.0.1", self.port));
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn serve(mut stream: TcpStream, lines: &Mutex<Vec<String>>) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));
    let mut head = Vec::new();
    let mut byte = [0u8; 1];
    while !head.ends_with(b"\r\n\r\n") && head.len() < 64 * 1024 {
        match stream.read(&mut byte) {
            Ok(1) => head.push(byte[0]),
            Err(error) if error.kind() == ErrorKind::Interrupted => {}
            _ => return,
        }
    }
    let head = String::from_utf8_lossy(&head).into_owned();
    let line = head.lines().next().unwrap_or("").to_string();
    let length: usize = head
        .lines()
        .find_map(|line| {
            let (key, value) = line.split_once(':')?;
            key.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse().ok())?
        })
        .unwrap_or(0);
    let mut body = vec![0u8; length];
    if stream.read_exact(&mut body).is_err() {
        return;
    }
    lines.lock().unwrap().push(line.clone());
    let path = line.split(' ').nth(1).unwrap_or("");
    let reply = match path {
        "/a" => html(PAGE_A),
        "/b" => html(PAGE_B),
        "/c" => html(PAGE_C),
        "/form" => html(FORM),
        "/sent" => html(SENT),
        "/search" => html(SEE_FORM),
        "/see" => "HTTP/1.1 303 See Other\r\nLocation: /r\r\nContent-Length: 0\r\nConnection: \
                   close\r\n\r\n"
            .to_string(),
        "/r" => html(PAGE_R),
        _ => "HTTP/1.1 404 Not Found\r\nContent-Type: text/plain\r\nContent-Length: 4\r\n\
              Connection: close\r\n\r\nnone"
            .to_string(),
    };
    let _ = stream.write_all(reply.as_bytes());
}

/// The reply to `command` with `args` as the holder: its status, its lines, its typed answer.
fn ask<T: surf::Transport>(
    session: &mut Session<T>,
    token: &str,
    command: &str,
    args: &[(&str, &str)],
) -> (u8, String, Answer) {
    let reply = session.answer(&holder(command, token, args));
    let link = reply.to_link();
    (link.status, link.text, reply.answer)
}

/// The `page` line of `text`.
fn page_line(text: &str) -> &str {
    text.lines()
        .find(|line| line.starts_with("page "))
        .unwrap_or_else(|| panic!("no page line: {text}"))
}

/// The page line of a loaded `path` of `pages` with `title`.
fn loaded(pages: &Pages, path: &str, title: &str) -> String {
    format!("page 200 \"text/html\" \"{}\" \"{title}\"", pages.url(path))
}

/// A session at `/a` of `pages` that followed `l1` to `/b`, and its token.
fn at_b(pages: &Pages) -> (Session<Fetch>, String) {
    let (mut session, token, _) = open_at(&pages.url("/a"), &[]);
    let (status, text, _) = ask(&mut session, &token, "follow", &[("handle", "l1")]);
    assert_eq!(status, 0, "{text}");
    assert_eq!(page_line(&text), loaded(pages, "/b", "B"));
    (session, token)
}

fn refused(typed: &Answer) -> Option<RefusalReason> {
    match &typed.did {
        Some(Did::Refused(reason)) => Some(*reason),
        _ => None,
    }
}

#[test]
fn case_01_back_answers_the_page_line_of_a() {
    let pages = Pages::start();
    let (mut session, token) = at_b(&pages);
    let (status, text, _) = ask(&mut session, &token, "back", &[]);
    assert_eq!(status, 0, "{text}");
    assert!(text.lines().any(|line| line == "did back"), "{text}");
    assert_eq!(page_line(&text), loaded(&pages, "/a", "A"), "{text}");
}

#[test]
fn case_02_forward_answers_the_page_line_of_b() {
    let pages = Pages::start();
    let (mut session, token) = at_b(&pages);
    let (status, text, _) = ask(&mut session, &token, "back", &[]);
    assert_eq!(status, 0, "{text}");
    let (status, text, _) = ask(&mut session, &token, "forward", &[]);
    assert_eq!(status, 0, "{text}");
    assert!(text.lines().any(|line| line == "did forward"), "{text}");
    assert_eq!(page_line(&text), loaded(&pages, "/b", "B"), "{text}");
}

#[test]
fn case_03_back_at_the_first_page_is_refused_at_start() {
    let pages = Pages::start();
    let (mut session, token, _) = open_at(&pages.url("/a"), &[]);
    let before = pages.lines().len();
    let (status, text, typed) = ask(&mut session, &token, "back", &[]);
    assert_eq!(refused(&typed), Some(RefusalReason::AtStart), "{text}");
    assert_eq!(status, 1, "{text}");
    assert_eq!(page_line(&text), loaded(&pages, "/a", "A"), "{text}");
    assert_eq!(pages.lines().len(), before, "no request");
}

#[test]
fn case_04_forward_at_the_last_page_is_refused_at_end() {
    let pages = Pages::start();
    let (mut session, token, _) = open_at(&pages.url("/a"), &[]);
    let (status, text, typed) = ask(&mut session, &token, "forward", &[]);
    assert_eq!(refused(&typed), Some(RefusalReason::AtEnd), "{text}");
    assert_eq!(status, 1, "{text}");
    assert_eq!(page_line(&text), loaded(&pages, "/a", "A"), "{text}");
}

#[test]
fn case_05_a_go_after_back_drops_the_entry_forward() {
    let pages = Pages::start();
    let (mut session, token) = at_b(&pages);
    let (status, text, _) = ask(&mut session, &token, "back", &[]);
    assert_eq!(status, 0, "{text}");
    let (status, text, _) = ask(&mut session, &token, "go", &[("url", &pages.url("/c"))]);
    assert_eq!(status, 0, "{text}");
    let (status, text, typed) = ask(&mut session, &token, "forward", &[]);
    assert_eq!(refused(&typed), Some(RefusalReason::AtEnd), "{text}");
    assert_eq!(status, 1, "{text}");
    assert_eq!(page_line(&text), loaded(&pages, "/c", "C"), "{text}");
}

/// A form `POST` answered 200 makes an entry of `POST`: `back` leaves it, and `forward` to it is
/// refused `post-entry` before anything is sent.
#[test]
fn case_06_forward_to_a_post_entry_is_refused_post_entry() {
    let pages = Pages::start();
    let (mut session, token, _) = open_at(&pages.url("/form"), &[]);
    let (status, text, _) = ask(&mut session, &token, "press", &[("handle", "b1")]);
    assert_eq!(status, 0, "{text}");
    assert_eq!(page_line(&text), loaded(&pages, "/sent", "Sent"), "{text}");
    let (status, back, _) = ask(&mut session, &token, "back", &[]);
    assert_eq!(status, 0, "{back}");
    assert_eq!(page_line(&back), loaded(&pages, "/form", "Form"), "{back}");
    let before = pages.lines();
    let (status, text, typed) = ask(&mut session, &token, "forward", &[]);
    assert_eq!(refused(&typed), Some(RefusalReason::PostEntry), "{text}");
    assert_eq!(status, 1, "{text}");
    assert_eq!(page_line(&text), loaded(&pages, "/form", "Form"), "{text}");
    assert_eq!(pages.lines(), before, "no request");
    assert_eq!(
        common::path_block(&text),
        common::path_block(&back),
        "no action"
    );
}

/// The server closes every connection while the agent goes back: `page failed`. Up again, `back`
/// lands on A, as the failed traversal left the current entry at B.
#[test]
fn case_07_a_failed_back_leaves_the_current_entry() {
    let pages = Pages::start();
    let (mut session, token) = at_b(&pages);
    pages.set_down(true);
    let (_, text, _) = ask(&mut session, &token, "back", &[]);
    assert!(page_line(&text).starts_with("page failed "), "{text}");
    pages.set_down(false);
    let (status, text, _) = ask(&mut session, &token, "back", &[]);
    assert_eq!(status, 0, "{text}");
    assert_eq!(page_line(&text), loaded(&pages, "/a", "A"), "{text}");
}

#[test]
fn case_08_handles_start_at_1_on_the_page_back_lands_on() {
    let pages = Pages::start();
    let (mut session, token) = at_b(&pages);
    let handles = |typed: &Answer| -> Vec<String> {
        typed
            .now
            .as_ref()
            .map(|now| now.elements.iter().map(|e| e.handle.clone()).collect())
            .unwrap_or_default()
    };
    let (_, text, typed) = ask(&mut session, &token, "look", &[]);
    assert_eq!(handles(&typed), ["l1", "l2"], "B has two links: {text}");
    let (status, text, _) = ask(&mut session, &token, "back", &[]);
    assert_eq!(status, 0, "{text}");
    let (_, text, typed) = ask(&mut session, &token, "look", &[]);
    assert_eq!(handles(&typed), ["l1"], "{text}");
}

/// The answers of `back` and `forward` of cases 1 and 2, over the server and over a transport in
/// memory that answers the same addresses with the same pages.
fn back_and_forward<T: surf::Transport>(transport: T, start: &str) -> Vec<String> {
    let mut session = Session::new(ID, transport).unwrap();
    let (status, text) = call(
        &mut session,
        &request("open", None, None, &[("url", start)]),
    );
    assert_eq!(status, 0, "{text}");
    let token = opened(&text).token;
    ["follow", "back", "forward"]
        .into_iter()
        .map(|command| {
            let args: &[(&str, &str)] = if command == "follow" {
                &[("handle", "l1")]
            } else {
                &[]
            };
            let (status, text, _) = ask(&mut session, &token, command, args);
            format!("{status}\n{text}")
        })
        .collect()
}

#[test]
fn case_09_the_core_with_a_transport_in_memory_answers_the_same() {
    let pages = Pages::start();
    let over_the_server = back_and_forward(surf_session::fetch as Fetch, &pages.url("/a"));
    for (answer, command, page) in [
        (&over_the_server[1], "back", loaded(&pages, "/a", "A")),
        (&over_the_server[2], "forward", loaded(&pages, "/b", "B")),
    ] {
        assert!(answer.starts_with("0\n"), "{answer}");
        assert!(
            answer.contains(&format!("\ndid {command}\n{page}\n")),
            "{answer}"
        );
    }
    let (a, b) = (pages.url("/a"), pages.url("/b"));
    let in_memory = move |asked: &surf::RequestStarted| {
        let body = if asked.url.0 == a {
            PAGE_A
        } else if asked.url.0 == b {
            PAGE_B
        } else {
            PAGE_C
        };
        surf::Answer::Response {
            url: asked.url.0.clone(),
            method: asked.method,
            status: 200,
            media_type: "text/html".to_string(),
            body: body.as_bytes().to_vec(),
        }
    };
    assert_eq!(
        back_and_forward(in_memory, &pages.url("/a")),
        over_the_server
    );
}

/// The `path` of `go` A (by `open`), `follow` B, `back`, `forward`.
fn path_after_back_and_forward(pages: &Pages) -> (String, Answer) {
    let (mut session, token) = at_b(pages);
    ask(&mut session, &token, "back", &[]);
    let (status, text, typed) = ask(&mut session, &token, "forward", &[]);
    assert_eq!(status, 0, "{text}");
    (text, typed)
}

#[test]
fn case_10_path_lists_back_and_forward() {
    let pages = Pages::start();
    let (text, _) = path_after_back_and_forward(&pages);
    assert_eq!(
        common::path_block(&text),
        format!(
            "path 5 of 5\n  1 open\n  2 go \"{}\"\n  3 follow l1 \"to B\"\n  4 back\n  5 forward\n",
            pages.url("/a")
        ),
        "{text}"
    );
}

#[test]
fn case_11_answers_read_back_and_render_the_same() {
    let pages = Pages::start();
    let mut answers = Vec::new();
    let (mut session, token) = at_b(&pages);
    let (_, text, typed) = ask(&mut session, &token, "back", &[]);
    answers.push((text, typed));
    let (mut first, first_token, _) = open_at(&pages.url("/a"), &[]);
    let (_, text, typed) = ask(&mut first, &first_token, "back", &[]);
    answers.push((text, typed));
    answers.push(path_after_back_and_forward(&pages));
    for (text, typed) in answers {
        let parsed = surf_answer::parse(&text).unwrap_or_else(|e| panic!("{e:?}: {text}"));
        assert_eq!(parsed, typed, "{text}");
        assert_eq!(surf_answer::render(&parsed).unwrap(), text);
    }
}

/// Cases 1 and 2 through `surfd` in the test process: the `GoBack` and `GoForward` events,
/// `surf.agent.TraversalRequested` with `step` `back` and `forward`, and the navigator's
/// `surf.navigation.TraversalStarted`, are entries with every field and no `unmapped` mark,
/// between the action entry and the answer entry of their command.
#[test]
fn case_12_history_events_map_with_their_fields() {
    let page = "<!doctype html><title>P</title><a href=\"/next\">next</a>";
    let pages = PageServer::with_body(page.as_bytes().to_vec());
    let dir = TestDir::new();
    let daemon = surfd_in_process(&dir, |_| {});
    let session = wire_open(&dir, Some(&pages.url("/")));
    for (command, args) in [
        ("follow", &[("handle", "l1")][..]),
        ("back", &[]),
        ("forward", &[]),
    ] {
        let answer = wire_as(&dir, &session, command, args);
        assert_eq!(answer.status, 0, "{command}: {answer:?}");
    }
    let entries = daemon
        .session_entries(&session.id)
        .expect("the session is listed")
        .entries;
    for (sequence, command, step) in [(4, "back", "back"), (5, "forward", "forward")] {
        let at = entries
            .iter()
            .position(|entry| {
                entry.kind == EntryKind::Action
                    && entry.text == format!("action {sequence} {command}\n")
            })
            .unwrap_or_else(|| panic!("no action entry of {command}: {entries:#?}"));
        let end = at
            + entries[at..]
                .iter()
                .position(|entry| entry.kind == EntryKind::Answer)
                .expect("an answer entry");
        let between = &entries[at + 1..end];
        assert!(
            between.iter().all(|entry| entry.kind == EntryKind::Event
                && entries[at].number < entry.number
                && entry.number < entries[end].number),
            "{entries:#?}"
        );
        let event = |name: &str| {
            between
                .iter()
                .find(|entry| entry.text.starts_with(&format!("event {name} ")))
                .unwrap_or_else(|| panic!("no {name} for {command}: {between:#?}"))
                .text
                .clone()
        };
        let requested = event("surf.agent.TraversalRequested");
        for field in [
            format!(" session_id=\"{}\"", session.id),
            " tab_id=\"".to_string(),
            format!(" step=\"{step}\""),
        ] {
            assert!(requested.contains(&field), "{field} in {requested}");
        }
        let started = event("surf.navigation.TraversalStarted");
        for field in [" tab_id=\"", " url=\"", " entry_id=\"", " entry_next="] {
            assert!(started.contains(field), "{field} in {started}");
        }
        assert!(
            !between.iter().any(|entry| entry.text.contains(" unmapped")),
            "{between:#?}"
        );
    }
}

/// After a form `POST` answered by a 303 to R and a `follow` from R to B, `back` lands on R,
/// fetched with `GET`: R's entry was committed by the `GET` after the redirect (decision D267).
#[test]
fn case_13_back_to_a_page_a_post_redirected_to_is_a_get() {
    let pages = Pages::start();
    let (mut session, token, _) = open_at(&pages.url("/search"), &[]);
    let (status, text, _) = ask(&mut session, &token, "press", &[("handle", "b1")]);
    assert_eq!(status, 0, "{text}");
    assert_eq!(page_line(&text), loaded(&pages, "/r", "R"), "{text}");
    let (status, text, _) = ask(&mut session, &token, "follow", &[("handle", "l1")]);
    assert_eq!(status, 0, "{text}");
    let (status, text, typed) = ask(&mut session, &token, "back", &[]);
    assert_eq!(refused(&typed), None, "{text}");
    assert_eq!(status, 0, "{text}");
    assert_eq!(page_line(&text), loaded(&pages, "/r", "R"), "{text}");
    assert_eq!(
        pages.lines().last().map(String::as_str),
        Some("GET /r HTTP/1.1")
    );
}
