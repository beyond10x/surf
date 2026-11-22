//! Cases 1 to 22 of `story:agent-press-submit`: `press <handle>` on a submit button submits its
//! form with `GET` or `POST` and the session lands on the page the server answers with; the
//! `submits` part of a button line; the session's refusals before any submission starts
//! (`no-default-action`, `disabled`, `password-in-query`, `address-too-long`,
//! `unsupported-scheme`, `forbidden-destination`) and the words of the end reasons a started
//! submission can reach (`no-form-owner`, `invalid-controls`, `invalid-action`,
//! `unsupported-enctype`, `dialog-method`).
//!
//! Most cases drive the session core against a [`FormServer`] on `127.0.0.1` that records each
//! request it reads; cases 10 and 16 run `surfd` in the test process (decision D29) with the
//! built `surf-session`; case 19 runs the session over a transport in memory, so that a page can
//! stand at a public address that is never dialed.

#![forbid(unsafe_code)]

use crate::common;
use crate::core::{Fetch, ID, call, holder, open_at};

use std::io::{ErrorKind, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use common::{TestDir, opened, request, surfd_in_process, wire_as, wire_open};
use surf_answer::{Answer, Did, Page, RefusalReason, State, Target, parse, render};
use surf_session::Session;
use surfd::link::EntryKind;

/// One request the server read: its request line, its `Content-Type` and its body.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Received {
    line: String,
    content_type: Option<String>,
    body: String,
}

/// An HTTP server on `127.0.0.1` port 0 that records every request it reads. A `GET` of a path
/// it was given a page for (the query left out) answers that page; `POST /session` answers 303
/// to `/home`; anything else 404. Stopped when dropped.
struct FormServer {
    port: u16,
    pages: Arc<Mutex<Vec<(String, String)>>>,
    received: Arc<Mutex<Vec<Received>>>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

/// The page `POST /session` leads to.
const HOME: &str = "<!doctype html><title>Home</title><p>Welcome.";

impl FormServer {
    fn start() -> FormServer {
        let listener = TcpListener::bind("127.0.0.1:0").expect("a port on 127.0.0.1");
        let port = listener.local_addr().unwrap().port();
        let pages = Arc::new(Mutex::new(vec![("/home".to_string(), HOME.to_string())]));
        let received = Arc::new(Mutex::new(Vec::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let (serving, recording, stopping) =
            (Arc::clone(&pages), Arc::clone(&received), Arc::clone(&stop));
        let thread = thread::spawn(move || {
            for stream in listener.incoming() {
                if stopping.load(Ordering::SeqCst) {
                    break;
                }
                if let Ok(stream) = stream {
                    serve(stream, &serving, &recording);
                }
            }
        });
        FormServer {
            port,
            pages,
            received,
            stop,
            thread: Some(thread),
        }
    }

    fn url(&self, path: &str) -> String {
        format!("http://127.0.0.1:{}{path}", self.port)
    }

    /// Serve `html` at `path`.
    fn page(&self, path: &str, html: &str) {
        self.pages
            .lock()
            .unwrap()
            .push((path.to_string(), html.to_string()));
    }

    fn received(&self) -> Vec<Received> {
        self.received.lock().unwrap().clone()
    }

    /// The request lines read so far.
    fn lines(&self) -> Vec<String> {
        self.received().into_iter().map(|got| got.line).collect()
    }
}

impl Drop for FormServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect(("127.0.0.1", self.port));
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn serve(
    mut stream: TcpStream,
    pages: &Mutex<Vec<(String, String)>>,
    received: &Mutex<Vec<Received>>,
) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));
    let mut head = Vec::new();
    let mut byte = [0u8; 1];
    while !head.ends_with(b"\r\n\r\n") && head.len() < 256 * 1024 {
        match stream.read(&mut byte) {
            Ok(1) => head.push(byte[0]),
            Err(error) if error.kind() == ErrorKind::Interrupted => {}
            _ => return,
        }
    }
    let head = String::from_utf8_lossy(&head).into_owned();
    let line = head.lines().next().unwrap_or("").to_string();
    let header = |name: &str| {
        head.lines().find_map(|line| {
            let (key, value) = line.split_once(':')?;
            key.eq_ignore_ascii_case(name)
                .then(|| value.trim().to_string())
        })
    };
    let length: usize = header("content-length")
        .and_then(|value| value.parse().ok())
        .unwrap_or(0);
    let mut body = vec![0u8; length];
    if stream.read_exact(&mut body).is_err() {
        return;
    }
    received.lock().unwrap().push(Received {
        line: line.clone(),
        content_type: header("content-type"),
        body: String::from_utf8_lossy(&body).into_owned(),
    });
    let mut words = line.split(' ');
    let (method, target) = (words.next().unwrap_or(""), words.next().unwrap_or(""));
    let path = target.split('?').next().unwrap_or("");
    let reply = if method == "POST" && path == "/session" {
        "HTTP/1.1 303 See Other\r\nLocation: /home\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            .to_string()
    } else {
        let page = pages
            .lock()
            .unwrap()
            .iter()
            .find(|(at, _)| at == path)
            .map(|(_, html)| html.clone());
        match page {
            Some(html) => format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: \
                 close\r\n\r\n{html}",
                html.len()
            ),
            None => "HTTP/1.1 404 Not Found\r\nContent-Type: text/plain\r\nContent-Length: 4\r\n\
                     Connection: close\r\n\r\nnone"
                .to_string(),
        }
    };
    let _ = stream.write_all(reply.as_bytes());
}

/// The sign-in form of cases 1, 3, 10 and 11: `f1` the user name, `f2` the password, `b1`.
const LOGIN: &str = "<!doctype html><title>Sign in</title>\
<form action=\"/session\" method=\"post\">\
<label>User <input name=\"user\"></label>\
<label>Password <input type=\"password\" name=\"password\"></label>\
<button>Sign in</button></form>";

/// A session opened at `path` of `server`, which serves `html` there, and its token.
fn at(server: &FormServer, path: &str, html: &str) -> (Session<Fetch>, String) {
    server.page(path, html);
    let (session, token, _) = open_at(&server.url(path), &[]);
    (session, token)
}

/// The reply to `command` with `args`: its exit status, its lines, its typed answer, and the
/// text of each entry it carries for `surfd`.
fn ask<T: surf::Transport>(
    session: &mut Session<T>,
    token: &str,
    command: &str,
    args: &[(&str, &str)],
) -> (u8, String, Answer, Vec<String>) {
    let reply = session.answer(&holder(command, token, args));
    let link = reply.to_link();
    let entries = link
        .entries
        .iter()
        .map(|entry| entry.text.clone())
        .collect();
    (link.status, link.text, reply.answer, entries)
}

/// The `now` line of `handle` in `text`.
fn line<'a>(text: &'a str, handle: &str) -> &'a str {
    let start = format!("  {handle} ");
    text.lines()
        .find(|line| line.starts_with(&start))
        .unwrap_or_else(|| panic!("no now line {handle}: {text}"))
}

/// The address of a loaded `page` line.
fn page_address(answer: &Answer) -> String {
    match &answer.page {
        Some(Page::Loaded { address, .. }) => address.clone(),
        other => panic!("not a loaded page: {other:?}"),
    }
}

/// `press b1` on the page `html` is refused `reason`, status 1, with no request after the
/// page's own; `started` says whether a submission started (the end reasons) or not (the
/// session's refusals, and then the page and the path are as before).
fn refused(html: &str, reason: RefusalReason, started: bool) {
    let server = FormServer::start();
    let (mut session, token) = at(&server, "/form.html", html);
    let (_, _, before, _) = ask(&mut session, &token, "look", &[]);
    let (status, text, typed, entries) = ask(&mut session, &token, "press", &[("handle", "b1")]);
    assert_eq!(status, 1, "{text}");
    assert_eq!(typed.did, Some(Did::Refused(reason)), "{text}");
    assert!(
        text.contains(&format!("\ndid refused {}\n", reason.word())),
        "{text}"
    );
    assert_eq!(server.lines().len(), 1, "no request: {:?}", server.lines());
    assert_eq!(typed.page, before.page, "the page stays: {text}");
    let submission = entries
        .iter()
        .any(|entry| entry.starts_with("event surf.document.SubmissionStarted "));
    assert_eq!(submission, started, "{entries:#?}");
    if !started {
        assert_eq!(typed.path, before.path, "no action: {text}");
    }
}

#[test]
fn case_01_a_filled_login_form_posts_and_lands_on_home() {
    let server = FormServer::start();
    let (mut session, token) = at(&server, "/login.html", LOGIN);
    for (handle, value) in [("f1", "ada"), ("f2", "s3cret")] {
        let (status, text, _, _) = ask(
            &mut session,
            &token,
            "fill",
            &[("handle", handle), ("value", value)],
        );
        assert_eq!(status, 0, "{text}");
    }
    let (status, text, typed, _) = ask(&mut session, &token, "press", &[("handle", "b1")]);
    assert_eq!(status, 0, "{text}");
    assert!(text.contains("\ndid press \"b1\"\n"), "{text}");
    let got = server.received();
    let post = got
        .iter()
        .find(|got| got.line.starts_with("POST "))
        .expect("a POST");
    assert_eq!(post.line, "POST /session HTTP/1.1");
    assert_eq!(
        post.content_type.as_deref(),
        Some("application/x-www-form-urlencoded")
    );
    assert_eq!(post.body, "user=ada&password=s3cret");
    assert_eq!(page_address(&typed), server.url("/home"));
    assert!(text.contains(" \"Home\"\n"), "{text}");
    assert!(text.contains("\n  5 press b1\n"), "{text}");
    assert!(!text.contains("s3cret"), "{text}");
}

#[test]
fn case_02_the_search_form_gets_its_results() {
    let server = FormServer::start();
    server.page(
        "/results",
        include_str!("../../agent-tasks/sites/search/results-lighthouse.html"),
    );
    let (mut session, token) = at(
        &server,
        "/search.html",
        include_str!("../../agent-tasks/sites/search/search.html"),
    );
    let (status, text, _, _) = ask(
        &mut session,
        &token,
        "fill",
        &[("handle", "f1"), ("value", "lighthouse")],
    );
    assert_eq!(status, 0, "{text}");
    let (status, text, _, _) = ask(&mut session, &token, "press", &[("handle", "b1")]);
    assert_eq!(status, 0, "{text}");
    assert!(
        server
            .lines()
            .contains(&"GET /results?q=lighthouse HTTP/1.1".to_string()),
        "{:?}",
        server.lines()
    );
    let page = text.lines().find(|line| line.starts_with("page ")).unwrap();
    assert!(
        page.contains("/results?q=lighthouse\" \"Results for lighthouse\""),
        "{page}"
    );
}

#[test]
fn case_03_a_button_line_names_what_it_submits() {
    let server = FormServer::start();
    let (mut session, token) = at(&server, "/login.html", LOGIN);
    let (_, text, _, _) = ask(&mut session, &token, "look", &[]);
    assert_eq!(
        line(&text, "b1"),
        "  b1 button ? \"Sign in\" submits F1 post \"/session\""
    );
}

#[test]
fn case_04_a_plain_button_has_no_default_action() {
    refused(
        "<!doctype html><form action=/session method=post><input name=a>\
         <button type=button>Plain</button></form>",
        RefusalReason::NoDefaultAction,
        false,
    );
}

#[test]
fn case_05_a_reset_button_has_no_default_action() {
    refused(
        "<!doctype html><form action=/session method=post><input name=a>\
         <button type=reset>Reset</button></form>",
        RefusalReason::NoDefaultAction,
        false,
    );
}

#[test]
fn case_06_a_disabled_submit_button_is_refused() {
    refused(
        "<!doctype html><form action=/session method=post><input name=a>\
         <button disabled>Send</button></form>",
        RefusalReason::Disabled,
        false,
    );
}

#[test]
fn case_07_a_multipart_form_is_not_encoded() {
    refused(
        "<!doctype html><form action=/session method=post enctype=multipart/form-data>\
         <input name=a value=1><button>Send</button></form>",
        RefusalReason::UnsupportedEnctype,
        true,
    );
}

#[test]
fn case_08_a_javascript_or_mailto_action_is_refused() {
    for action in ["javascript:x", "mailto:a@example.org"] {
        refused(
            &format!(
                "<!doctype html><form action=\"{action}\"><input name=a value=1>\
                 <button>Send</button></form>"
            ),
            RefusalReason::UnsupportedScheme,
            false,
        );
    }
}

/// The page of case 9: an action holding a line feed, `"` and U+202E, once resolved (the URL
/// parser drops the line feed and percent-encodes the rest) and once as written after a host
/// that does not parse.
const FORGING: &str = "<!doctype html><title>Forging</title>\
<form action=\"/a&#10;b&quot;c&#x202e;d\"><button>Resolved</button></form>\
<form action=\"http://[a&#10;b&quot;c&#x202e;d\"><button>Written</button></form>";

#[test]
fn case_09_a_forged_action_stays_in_its_line() {
    let server = FormServer::start();
    let (mut session, token) = at(&server, "/form.html", FORGING);
    let (_, text, _, _) = ask(&mut session, &token, "look", &[]);
    assert_eq!(text.matches("\n  b").count(), 2, "{text}");
    assert_eq!(
        line(&text, "b1"),
        "  b1 button ? \"Resolved\" submits F1 get \"/ab%22c%E2%80%AEd\""
    );
    assert_eq!(
        line(&text, "b2"),
        "  b2 button ? \"Written\" submits F2 get \"http://[a\\x0ab\\x22c\\xe2\\x80\\xaed\" unresolved"
    );
    assert!(!text.contains('\u{202e}'), "{text}");
}

#[test]
fn case_10_press_events_map_with_their_fields() {
    let dir = TestDir::new();
    let server = FormServer::start();
    server.page("/login.html", LOGIN);
    let daemon = surfd_in_process(&dir, |_| {});
    let session = wire_open(&dir, Some(&server.url("/login.html")));
    for (handle, value) in [("f1", "ada"), ("f2", "s3cret")] {
        let answer = wire_as(
            &dir,
            &session,
            "fill",
            &[("handle", handle), ("value", value)],
        );
        assert_eq!(answer.status, 0, "{answer:?}");
    }
    let answer = wire_as(&dir, &session, "press", &[("handle", "b1")]);
    assert_eq!(answer.status, 0, "{answer:?}");
    let entries = daemon
        .session_entries(&session.id)
        .expect("the session is listed")
        .entries;
    for entry in &entries {
        assert!(!entry.text.contains("s3cret"), "{}", entry.text);
    }
    let event = |name: &str| -> String {
        entries
            .iter()
            .filter(|entry| entry.kind == EntryKind::Event)
            .map(|entry| entry.text.clone())
            .find(|text| text.starts_with(&format!("event {name} ")))
            .unwrap_or_else(|| panic!("no {name} entry"))
    };
    for name in [
        "surf.agent.PressRequested",
        "surf.document.SubmissionStarted",
        "surf.document.FormSubmitted",
    ] {
        assert!(!event(name).contains(" unmapped"), "{}", event(name));
    }
    assert!(
        event("surf.agent.PressRequested").contains(" handle=\"b1\""),
        "{}",
        event("surf.agent.PressRequested")
    );
    let submitted = event("surf.document.FormSubmitted");
    assert!(
        submitted.contains(&format!(
            " body=\"{} bytes\"",
            "user=ada&password=s3cret".len()
        )),
        "{submitted}"
    );
    assert!(submitted.contains(" method=\"Post\""), "{submitted}");
}

/// `parse` of `text` gives `typed`, and `render` of it gives `text` byte for byte.
fn round_trip(text: &str, typed: &Answer) {
    let parsed = parse(text).unwrap_or_else(|error| panic!("{error}: {text}"));
    assert_eq!(&parsed, typed, "{text}");
    assert_eq!(render(&parsed).unwrap(), text);
}

#[test]
fn case_11_button_lines_read_back_to_their_typed_value() {
    let server = FormServer::start();
    for (path, html) in [("/login.html", LOGIN), ("/forging.html", FORGING)] {
        let (mut session, token) = at(&server, path, html);
        let (_, text, typed, _) = ask(&mut session, &token, "look", &[]);
        round_trip(&text, &typed);
        let element = &typed.now.as_ref().unwrap().elements;
        let button = element.iter().find(|e| e.handle == "b1").unwrap();
        match &button.target {
            Target::State(State {
                submits: Some(_), ..
            }) => {}
            other => panic!("no submits part: {other:?}"),
        }
    }
}

#[test]
fn case_12_a_submit_button_with_no_form_owner() {
    refused(
        "<!doctype html><form action=/session method=post><input name=a></form>\
         <button>Alone</button>",
        RefusalReason::NoFormOwner,
        true,
    );
}

#[test]
fn case_13_a_required_field_left_empty() {
    refused(
        "<!doctype html><form action=/session method=post><input name=a required>\
         <button>Send</button></form>",
        RefusalReason::InvalidControls,
        true,
    );
}

#[test]
fn case_14_an_action_that_does_not_parse() {
    refused(
        "<!doctype html><form action=\"http://[\" method=post><input name=a value=1>\
         <button>Send</button></form>",
        RefusalReason::InvalidAction,
        true,
    );
}

#[test]
fn case_15_a_dialog_form() {
    refused(
        "<!doctype html><form method=dialog><input name=a value=1>\
         <button>Close</button></form>",
        RefusalReason::DialogMethod,
        true,
    );
}

/// The `GET` form of cases 16 and 17: `f1` a text field, `f2` a password.
const GET_LOGIN: &str = "<!doctype html><title>Get</title>\
<form action=\"/find\"><label>User <input name=\"user\" value=\"ada\"></label>\
<label>Password <input type=\"password\" name=\"pw\"></label>\
<button>Go</button></form>";

#[test]
fn case_16_password_in_query_is_refused() {
    let dir = TestDir::new();
    let server = FormServer::start();
    server.page("/get.html", GET_LOGIN);
    let daemon = surfd_in_process(&dir, |_| {});
    let session = wire_open(&dir, Some(&server.url("/get.html")));
    let answer = wire_as(
        &dir,
        &session,
        "fill",
        &[("handle", "f2"), ("value", "s3cret-get")],
    );
    assert_eq!(answer.status, 0, "{answer:?}");
    let answer = wire_as(&dir, &session, "press", &[("handle", "b1")]);
    assert_eq!(answer.status, 1, "{answer:?}");
    assert!(
        answer.text.contains("\ndid refused password-in-query\n"),
        "{}",
        answer.text
    );
    assert_eq!(server.lines(), vec!["GET /get.html HTTP/1.1".to_string()]);
    let entries = daemon
        .session_entries(&session.id)
        .expect("the session is listed")
        .entries;
    for entry in &entries {
        assert!(!entry.text.contains("s3cret-get"), "{}", entry.text);
    }
    assert!(!answer.text.contains("s3cret-get"), "{}", answer.text);
}

#[test]
fn case_17_a_get_form_with_an_empty_password_is_submitted() {
    let server = FormServer::start();
    server.page("/find", HOME);
    let (mut session, token) = at(&server, "/get.html", GET_LOGIN);
    let (status, text, _, _) = ask(&mut session, &token, "press", &[("handle", "b1")]);
    assert_eq!(status, 0, "{text}");
    assert!(
        server
            .lines()
            .contains(&"GET /find?user=ada&pw= HTTP/1.1".to_string()),
        "{:?}",
        server.lines()
    );
}

#[test]
fn case_18_address_too_long_is_refused() {
    refused(
        &format!(
            "<!doctype html><form action=\"/results\"><input type=hidden name=q value=\"{}\">\
             <button>Find</button></form>",
            "a".repeat(65_536)
        ),
        RefusalReason::AddressTooLong,
        false,
    );
}

/// The transport of case 19: `start` answers `page`, every other address [`HOME`]; each
/// request is recorded as its method and address.
type Memory = Box<dyn FnMut(&surf::RequestStarted) -> surf::Answer>;

fn in_memory(start: &str, page: String) -> (Session<Memory>, String, Arc<Mutex<Vec<String>>>) {
    let asked = Arc::new(Mutex::new(Vec::new()));
    let record = Arc::clone(&asked);
    let start_address = start.to_string();
    let transport: Memory = Box::new(move |request: &surf::RequestStarted| {
        record
            .lock()
            .unwrap()
            .push(format!("{:?} {}", request.method, request.url.0));
        let body = if request.url.0 == start_address {
            page.clone()
        } else {
            HOME.to_string()
        };
        surf::Answer::Response {
            url: request.url.0.clone(),
            status: 200,
            media_type: "text/html".to_string(),
            body: body.into_bytes(),
        }
    });
    let mut session = Session::new(ID, transport).unwrap();
    let (status, text) = call(
        &mut session,
        &request("open", None, None, &[("url", start)]),
    );
    assert_eq!(status, 0, "{text}");
    (session, opened(&text).token, asked)
}

#[test]
fn case_19_forbidden_destination_is_refused() {
    let form = "<!doctype html><form action=\"http://127.0.0.1:8080/session\" method=post>\
                <input name=a value=1><button>Send</button></form>"
        .to_string();
    let public = "http://93.184.216.34/form.html";
    let (mut session, token, asked) = in_memory(public, form.clone());
    let (status, text, typed, _) = ask(&mut session, &token, "press", &[("handle", "b1")]);
    assert_eq!(status, 1, "{text}");
    assert_eq!(
        typed.did,
        Some(Did::Refused(RefusalReason::ForbiddenDestination)),
        "{text}"
    );
    assert_eq!(*asked.lock().unwrap(), vec![format!("Get {public}")]);

    let local = "http://127.0.0.1:8080/form.html";
    let (mut session, token, asked) = in_memory(local, form);
    let (status, text, _, _) = ask(&mut session, &token, "press", &[("handle", "b1")]);
    assert_eq!(status, 0, "{text}");
    assert_eq!(
        *asked.lock().unwrap(),
        vec![
            format!("Get {local}"),
            "Post http://127.0.0.1:8080/session".to_string()
        ]
    );
}

#[test]
fn case_20_submitter_formaction_and_formmethod() {
    let server = FormServer::start();
    server.page("/draft", HOME);
    let (mut session, token) = at(
        &server,
        "/form.html",
        "<!doctype html><form action=\"/session\" method=\"post\">\
         <label>User <input name=\"user\"></label><button>Save</button>\
         <button formaction=\"/draft\" formmethod=\"get\">Draft</button></form>",
    );
    let (_, text, _, _) = ask(&mut session, &token, "look", &[]);
    assert_eq!(
        line(&text, "b1"),
        "  b1 button ? \"Save\" submits F1 post \"/session\""
    );
    assert_eq!(
        line(&text, "b2"),
        "  b2 button ? \"Draft\" submits F1 get \"/draft\""
    );
    let (status, _, _, _) = ask(
        &mut session,
        &token,
        "fill",
        &[("handle", "f1"), ("value", "ada")],
    );
    assert_eq!(status, 0);
    let (status, text, _, _) = ask(&mut session, &token, "press", &[("handle", "b2")]);
    assert_eq!(status, 0, "{text}");
    assert!(
        server
            .lines()
            .contains(&"GET /draft?user=ada HTTP/1.1".to_string()),
        "{:?}",
        server.lines()
    );
}

#[test]
fn case_21_a_mailto_formaction_is_refused() {
    refused(
        "<!doctype html><form action=\"/session\" method=post><input name=a value=1>\
         <button formaction=\"mailto:a@example.org\">Mail</button></form>",
        RefusalReason::UnsupportedScheme,
        false,
    );
}

#[test]
fn case_22_the_shown_action_has_no_userinfo_and_a_double_slash_path_is_whole() {
    let server = FormServer::start();
    let port = server.port;
    let html = format!(
        "<!doctype html>\
         <form action=\"http://user:pw@127.0.0.1:{port}/session\" method=post>\
         <button>Sign in</button></form>\
         <form action=\"http://127.0.0.1:{port}//x\" method=post><button>Slash</button></form>"
    );
    let (mut session, token) = at(&server, "/form.html", &html);
    let (_, text, _, _) = ask(&mut session, &token, "look", &[]);
    assert_eq!(
        line(&text, "b1"),
        "  b1 button ? \"Sign in\" submits F1 post \"/session\""
    );
    assert!(!text.contains("user"), "{text}");
    assert!(!text.contains("pw"), "{text}");
    assert_eq!(
        line(&text, "b2"),
        format!("  b2 button ? \"Slash\" submits F2 post \"http://127.0.0.1:{port}//x\"")
    );
}

/// `press` takes one handle and nothing else.
#[test]
fn press_without_its_handle_is_usage() {
    let server = FormServer::start();
    let (mut session, token) = at(&server, "/login.html", LOGIN);
    for args in [vec![], vec![("handle", "b1"), ("value", "x")]] {
        let (status, text, _, _) = ask(&mut session, &token, "press", &args);
        assert_eq!(status, 2, "{text}");
    }
    let (status, text, typed, _) = ask(&mut session, &token, "press", &[("handle", "f1")]);
    assert_eq!(status, 1, "{text}");
    assert_eq!(typed.did, Some(Did::Refused(RefusalReason::WrongKind)));
}

/// Decision 1 of the fix round of `story:agent-fill-pick`: a text field holds its value without
/// line breaks, so a `press` submits the stripped value; a textarea's keeps them.
#[test]
fn a_pressed_form_submits_the_stripped_value() {
    let server = FormServer::start();
    server.page("/find", HOME);
    let (mut session, token) = at(
        &server,
        "/form.html",
        "<!doctype html><form action=\"/find\"><label>Q <input name=\"q\"></label>\
         <label>Note <textarea name=\"t\"></textarea></label><button>Find</button></form>",
    );
    for (handle, value) in [("f1", "light\r\nhouse\n"), ("f2", "a\nb")] {
        let (status, text, _, _) = ask(
            &mut session,
            &token,
            "fill",
            &[("handle", handle), ("value", value)],
        );
        assert_eq!(status, 0, "{text}");
    }
    let (status, text, _, _) = ask(&mut session, &token, "press", &[("handle", "b1")]);
    assert_eq!(status, 0, "{text}");
    assert!(
        server
            .lines()
            .contains(&"GET /find?q=lighthouse&t=a%0D%0Ab HTTP/1.1".to_string()),
        "{:?}",
        server.lines()
    );
}
