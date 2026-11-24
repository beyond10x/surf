//! The adversary's cases on `story:agent-press-submit` (`b11ed6c`): the session's refusals
//! before `Press` at their bounds and through every route that builds the submitted address,
//! a password and the method of a submit button, the `submits` part of a button line, and the
//! state after a refusal.
//!
//! The cases drive the session core against a [`Server`] on `127.0.0.1` that records each
//! request it reads, or over a transport in memory when the page must stand at a public address
//! that is never dialed (as case 19 of `press.rs` does).

#![forbid(unsafe_code)]

use crate::common;
use crate::core::{Fetch, ID, call, holder, open_at};

use std::io::{ErrorKind, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use common::{opened, request};
use surf_answer::{Answer, Did, Page, RefusalReason};
use surf_session::Session;

/// One request the server read.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Received {
    line: String,
    content_type: Option<String>,
    body: String,
}

/// An HTTP server on `127.0.0.1` that answers each path (the query left out) with the raw
/// response it was given, 404 otherwise, and records every request it reads.
struct Server {
    port: u16,
    routes: Arc<Mutex<Vec<(String, String)>>>,
    received: Arc<Mutex<Vec<Received>>>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

const HOME: &str = "<!doctype html><title>Home</title><p>Welcome.";

fn html(body: &str) -> String {
    format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: \
         close\r\n\r\n{body}",
        body.len()
    )
}

fn redirect(status: u16, location: &str) -> String {
    format!(
        "HTTP/1.1 {status} Moved\r\nLocation: {location}\r\nContent-Length: 0\r\nConnection: \
         close\r\n\r\n"
    )
}

impl Server {
    fn start() -> Server {
        let listener = TcpListener::bind("127.0.0.1:0").expect("a port on 127.0.0.1");
        let port = listener.local_addr().unwrap().port();
        let routes = Arc::new(Mutex::new(vec![("/home".to_string(), html(HOME))]));
        let received = Arc::new(Mutex::new(Vec::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let (serving, recording, stopping) = (
            Arc::clone(&routes),
            Arc::clone(&received),
            Arc::clone(&stop),
        );
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
        Server {
            port,
            routes,
            received,
            stop,
            thread: Some(thread),
        }
    }

    fn url(&self, path: &str) -> String {
        format!("http://127.0.0.1:{}{path}", self.port)
    }

    fn route(&self, path: &str, response: String) {
        self.routes
            .lock()
            .unwrap()
            .insert(0, (path.to_string(), response));
    }

    fn page(&self, path: &str, body: &str) {
        self.route(path, html(body));
    }

    fn received(&self) -> Vec<Received> {
        self.received.lock().unwrap().clone()
    }

    fn lines(&self) -> Vec<String> {
        self.received().into_iter().map(|got| got.line).collect()
    }
}

impl Drop for Server {
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
    routes: &Mutex<Vec<(String, String)>>,
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
    let target = line.split(' ').nth(1).unwrap_or("");
    let path = target.split('?').next().unwrap_or("");
    let reply = routes
        .lock()
        .unwrap()
        .iter()
        .find(|(at, _)| at == path)
        .map(|(_, reply)| reply.clone())
        .unwrap_or_else(|| {
            "HTTP/1.1 404 Not Found\r\nContent-Type: text/plain\r\nContent-Length: 4\r\n\
             Connection: close\r\n\r\nnone"
                .to_string()
        });
    let _ = stream.write_all(reply.as_bytes());
}

/// A session opened at `path` of `server`, which serves `body` there, and its token.
fn at(server: &Server, path: &str, body: &str) -> (Session<Fetch>, String) {
    server.page(path, body);
    let (session, token, _) = open_at(&server.url(path), &[]);
    (session, token)
}

/// The reply to `command`: its status, its lines, its typed answer and its entries' text.
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

fn fill<T: surf::Transport>(session: &mut Session<T>, token: &str, handle: &str, value: &str) {
    let (status, text, _, _) = ask(
        session,
        token,
        "fill",
        &[("handle", handle), ("value", value)],
    );
    assert_eq!(status, 0, "fill {handle}: {text}");
}

fn line<'a>(text: &'a str, handle: &str) -> &'a str {
    let start = format!("  {handle} ");
    text.lines()
        .find(|line| line.starts_with(&start))
        .unwrap_or_else(|| panic!("no now line {handle}: {text}"))
}

fn page_address(answer: &Answer) -> String {
    match &answer.page {
        Some(Page::Loaded { address, .. }) => address.clone(),
        other => panic!("not a loaded page: {other:?}"),
    }
}

fn started(entries: &[String]) -> bool {
    entries
        .iter()
        .any(|entry| entry.starts_with("event surf.document.SubmissionStarted "))
}

/// `press b1` on `body` served by a fresh server is refused `reason` before any submission
/// starts: status 1, no request after the page's own, no submission entry, page and path as
/// before.
fn refused_before(body: &str, reason: RefusalReason) {
    let server = Server::start();
    let (mut session, token) = at(&server, "/form.html", body);
    let (_, _, before, _) = ask(&mut session, &token, "look", &[]);
    let (status, text, typed, entries) = ask(&mut session, &token, "press", &[("handle", "b1")]);
    assert_eq!(typed.did, Some(Did::Refused(reason)), "{body}\n{text}");
    assert_eq!(status, 1, "{text}");
    assert_eq!(server.lines().len(), 1, "no request: {:?}", server.lines());
    assert!(!started(&entries), "{entries:#?}");
    assert_eq!(typed.page, before.page, "{text}");
    assert_eq!(typed.path, before.path, "{text}");
}

/// A transport in memory: `start` answers `page`, every other address [`HOME`]; each request
/// is recorded as its method and address.
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
            method: request.method,
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

/// Statement 6 of `story:session-follow-press-https`, in process: an action of a scheme
/// neither `http:` nor `https:` (in the form's `action` or the button's `formaction`, in any
/// case of its letters) is refused `unsupported-scheme` by the session before `Press`.
#[test]
fn adversary_press_follow_press_case_6_another_scheme_is_refused_before_a_submission_starts() {
    let server = Server::start();
    let port = server.port;
    drop(server);
    refused_before(
        &format!(
            "<!doctype html><form action=\"ftp://127.0.0.1:{port}/session\" method=post>\
             <input name=a value=1><button>Send</button></form>"
        ),
        RefusalReason::UnsupportedScheme,
    );
    refused_before(
        "<!doctype html><form action=/session method=post><input name=a value=1>\
         <button formaction=\"FTP://example.org/x\">Send</button></form>",
        RefusalReason::UnsupportedScheme,
    );
}

/// Statement 2 of `story:session-follow-press-https`, in process: an `https:` action, in the
/// form's `action` or a button's `formaction` in capitals, is submitted: the transport is asked
/// for a `POST` of the resolved address after the page's own `GET`, and the press lands on it.
#[test]
fn adversary_press_follow_press_case_2_an_https_action_is_submitted() {
    let start = "http://127.0.0.1:8000/form.html";
    for (page, sent) in [
        (
            "<!doctype html><form action=\"https://127.0.0.1:8000/session\" method=post>\
             <input name=a value=1><button>Send</button></form>",
            "https://127.0.0.1:8000/session",
        ),
        (
            "<!doctype html><form action=/session method=post><input name=a value=1>\
             <button formaction=\"HTTPS://shop.example/x\">Send</button></form>",
            "https://shop.example/x",
        ),
    ] {
        let (mut session, token, asked) = in_memory(start, page.to_string());
        let (status, text, typed, _) = ask(&mut session, &token, "press", &[("handle", "b1")]);
        assert_eq!(status, 0, "{page}\n{text}");
        assert_eq!(
            typed.did,
            Some(Did::Action {
                command: "press".to_string(),
                argument: Some("b1".to_string()),
            }),
            "{text}"
        );
        assert_eq!(page_address(&typed), sent, "{text}");
        assert_eq!(
            *asked.lock().unwrap(),
            vec![format!("Get {start}"), format!("Post {sent}")],
            "{page}"
        );
    }
}

/// The bound is "over 65,536 bytes": a `GET` address of exactly 65,536 bytes is requested, one
/// of 65,537 is refused `address-too-long` before `Press`.
#[test]
fn adversary_press_an_address_of_exactly_64_kib_is_submitted_and_one_byte_more_is_refused() {
    for extra in [0usize, 1] {
        let server = Server::start();
        server.page("/r", HOME);
        let prefix = format!("{}?q=", server.url("/r"));
        let value = "a".repeat(65_536 - prefix.len() + extra);
        let body = format!(
            "<!doctype html><form action=\"/r\"><input type=hidden name=q value=\"{value}\">\
             <button>Find</button></form>"
        );
        let (mut session, token) = at(&server, "/form.html", &body);
        let (status, text, typed, entries) =
            ask(&mut session, &token, "press", &[("handle", "b1")]);
        if extra == 0 {
            assert_eq!(status, 0, "{}", &text[..text.len().min(400)]);
            let lines = server.lines();
            assert_eq!(lines.len(), 2, "the GET is requested");
            assert_eq!(
                lines[1].len(),
                "GET ".len() + 65_536 - server.url("").len() + " HTTP/1.1".len()
            );
        } else {
            assert_eq!(
                typed.did,
                Some(Did::Refused(RefusalReason::AddressTooLong)),
                "{}",
                &text[..text.len().min(400)]
            );
            assert!(!started(&entries));
            assert_eq!(server.lines().len(), 1);
        }
    }
}

/// The method is the submitter's: a `formmethod="get"` button of a `POST` form with a filled
/// password is refused `password-in-query`; a `formmethod="post"` button of a `GET` form sends
/// the password in the body and not in the address.
#[test]
fn adversary_press_the_password_rule_follows_the_buttons_formmethod() {
    let server = Server::start();
    server.page("/find", HOME);
    let (mut session, token) = at(
        &server,
        "/form.html",
        "<!doctype html><form action=/find method=post>\
         <input type=password name=pw><button formmethod=get>Peek</button></form>",
    );
    fill(&mut session, &token, "f1", "s3cret-fm");
    let (status, text, typed, entries) = ask(&mut session, &token, "press", &[("handle", "b1")]);
    assert_eq!(
        typed.did,
        Some(Did::Refused(RefusalReason::PasswordInQuery)),
        "{text}"
    );
    assert_eq!(status, 1);
    assert!(!started(&entries));
    assert_eq!(server.lines().len(), 1, "{:?}", server.lines());

    let server = Server::start();
    server.route("/find", html(HOME));
    let (mut session, token) = at(
        &server,
        "/form.html",
        "<!doctype html><form action=/find>\
         <input type=password name=pw><button formmethod=post>Send</button></form>",
    );
    fill(&mut session, &token, "f1", "s3cret-fm");
    let (status, text, typed, _) = ask(&mut session, &token, "press", &[("handle", "b1")]);
    assert_eq!(status, 0, "{text}");
    let got = server.received();
    assert_eq!(got[1].line, "POST /find HTTP/1.1");
    assert_eq!(got[1].body, "pw=s3cret-fm");
    assert!(!page_address(&typed).contains("s3cret"));
    assert!(!text.contains("s3cret"), "{text}");
}

/// A password the page wrote into the `value` attribute, and one whose control is outside the
/// form and owned through the `form` attribute, are in a `GET`'s entry list all the same: both
/// refused `password-in-query`, no request.
#[test]
fn adversary_press_a_password_from_its_value_attribute_or_owned_by_form_id_is_refused_in_a_get() {
    refused_before(
        "<!doctype html><form action=/find><input type=password name=pw value=pre-set>\
         <button>Go</button></form>",
        RefusalReason::PasswordInQuery,
    );
    refused_before(
        "<!doctype html><form id=g action=/find><button>Go</button></form>\
         <input type=password name=pw value=outside form=g>",
        RefusalReason::PasswordInQuery,
    );
    refused_before(
        "<!doctype html><form action=/find><input type=PassWord name=pw value=x>\
         <button>Go</button></form>",
        RefusalReason::PasswordInQuery,
    );
}

/// A password filled and then emptied is no password: the `GET` is submitted with `pw=`.
#[test]
fn adversary_press_a_password_filled_then_emptied_is_submitted_in_a_get() {
    let server = Server::start();
    server.page("/find", HOME);
    let (mut session, token) = at(
        &server,
        "/form.html",
        "<!doctype html><form action=/find><input type=password name=pw>\
         <button>Go</button></form>",
    );
    fill(&mut session, &token, "f1", "x");
    fill(&mut session, &token, "f1", "");
    let (status, text, _, _) = ask(&mut session, &token, "press", &[("handle", "b1")]);
    assert_eq!(status, 0, "{text}");
    assert_eq!(server.lines()[1], "GET /find?pw= HTTP/1.1");
}

/// From a page at a public address, every spelling of a local destination that the URL parser
/// reads as a local IP address or `localhost`, by the form's `action`, a `formaction`, a `base`
/// element or a scheme-relative action, is refused `forbidden-destination`, with no request
/// but the page's own.
#[test]
fn adversary_press_every_spelling_of_a_local_destination_from_a_public_page_is_refused() {
    let public = "http://93.184.216.34/form.html";
    let forms = [
        "<form action=\"http://127.1:8080/s\" method=post>",
        "<form action=\"http://0x7f000001/s\" method=post>",
        "<form action=\"http://2130706433/s\" method=post>",
        "<form action=\"http://0177.0.0.1/s\" method=post>",
        "<form action=\"http://127.0.0.1./s\" method=post>",
        "<form action=\"http://[::ffff:127.0.0.1]/s\" method=post>",
        "<form action=\"http://[::1]/s\" method=post>",
        "<form action=\"http://0/s\" method=post>",
        "<form action=\"http://10.1.2.3/s\" method=post>",
        "<form action=\"http://169.254.169.254/latest\" method=post>",
        "<form action=\"http://100.64.0.1/s\" method=post>",
        "<form action=\"http://LOCALHOST:8080/s\" method=post>",
        "<form action=\"http://localhost.:8080/s\" method=post>",
        "<form action=\"http://api.localhost/s\" method=post>",
        "<form action=\"http:\\\\127.0.0.1\\s\" method=post>",
        "<form action=\"//127.0.0.1:8080/s\" method=post>",
        "<form action=\"http://user@127.0.0.1/s\">",
        "<form action=\"http://127.0.0.1%2e/s\">",
    ];
    for form in forms {
        let page =
            format!("<!doctype html>{form}<input name=a value=1><button>Send</button></form>");
        let (mut session, token, asked) = in_memory(public, page.clone());
        let (status, text, typed, entries) =
            ask(&mut session, &token, "press", &[("handle", "b1")]);
        assert_eq!(
            typed.did,
            Some(Did::Refused(RefusalReason::ForbiddenDestination)),
            "{form}\n{text}"
        );
        assert_eq!(status, 1);
        assert!(!started(&entries), "{form}");
        assert_eq!(
            *asked.lock().unwrap(),
            vec![format!("Get {public}")],
            "{form}"
        );
    }
    for page in [
        "<!doctype html><base href=\"http://127.0.0.1:8080/\">\
         <form action=session method=post><input name=a value=1><button>Send</button></form>",
        "<!doctype html><form action=/session method=post><input name=a value=1>\
         <button>Save</button>\
         <button formaction=\"http://127.0.0.1:8080/admin\">Admin</button></form>",
    ] {
        let handle = if page.contains("Admin") { "b2" } else { "b1" };
        let (mut session, token, asked) = in_memory(public, page.to_string());
        let (_, text, typed, _) = ask(&mut session, &token, "press", &[("handle", handle)]);
        assert_eq!(
            typed.did,
            Some(Did::Refused(RefusalReason::ForbiddenDestination)),
            "{page}\n{text}"
        );
        assert_eq!(
            *asked.lock().unwrap(),
            vec![format!("Get {public}")],
            "{page}"
        );
    }
}

/// A refused `press` leaves the filled values, the page and the path as they were.
#[test]
fn adversary_press_a_refused_press_keeps_the_filled_values_and_the_path() {
    let public = "http://93.184.216.34/form.html";
    let page = "<!doctype html><form action=\"http://127.0.0.1:8080/s\" method=post>\
                <input name=user><button>Send</button></form>"
        .to_string();
    let (mut session, token, _) = in_memory(public, page);
    fill(&mut session, &token, "f1", "ada");
    let (_, before_text, before, _) = ask(&mut session, &token, "look", &[]);
    let (status, _, _, _) = ask(&mut session, &token, "press", &[("handle", "b1")]);
    assert_eq!(status, 1);
    let (_, after_text, after, _) = ask(&mut session, &token, "look", &[]);
    assert_eq!(after.page, before.page);
    assert_eq!(after.path, before.path, "{after_text}");
    assert_eq!(line(&after_text, "f1"), line(&before_text, "f1"));
    assert!(
        line(&after_text, "f1").contains("= \"ada\""),
        "{after_text}"
    );
}

/// An empty `formaction` submits to the document's address, not to the form's `action`; an
/// unknown `formmethod` submits with `GET`, not with the form's `POST`; the button line says so.
#[test]
fn adversary_press_an_empty_formaction_and_an_unknown_formmethod() {
    let server = Server::start();
    let (mut session, token) = at(
        &server,
        "/form.html",
        "<!doctype html><form action=/session method=post><input name=a value=1>\
         <button formaction=\"\">Here</button><button formmethod=put>Put</button></form>",
    );
    let (_, text, _, _) = ask(&mut session, &token, "look", &[]);
    assert_eq!(
        line(&text, "b1"),
        "  b1 button ? \"Here\" submits F1 post \"/form.html\""
    );
    assert_eq!(
        line(&text, "b2"),
        "  b2 button ? \"Put\" submits F1 get \"/session\""
    );
    let (status, text, _, _) = ask(&mut session, &token, "press", &[("handle", "b1")]);
    assert_eq!(status, 0, "{text}");
    assert_eq!(server.lines()[1], "POST /form.html HTTP/1.1");

    let server = Server::start();
    let (mut session, token) = at(
        &server,
        "/form.html",
        "<!doctype html><form action=/session method=post><input name=a value=1>\
         <button formmethod=put>Put</button></form>",
    );
    let (status, text, _, _) = ask(&mut session, &token, "press", &[("handle", "b1")]);
    assert_eq!(status, 0, "{text}");
    assert_eq!(server.lines()[1], "GET /session?a=1 HTTP/1.1");
}

/// `formaction` and `formmethod` belong to submit buttons: a `type=button` and a
/// `type=reset` that carry them show the form's method and action.
#[test]
fn adversary_press_formaction_of_a_plain_or_reset_button_is_not_shown() {
    let server = Server::start();
    let (mut session, token) = at(
        &server,
        "/form.html",
        "<!doctype html><form action=/session method=post><input name=a value=1>\
         <button type=button formaction=/elsewhere formmethod=get>Plain</button>\
         <button type=reset formaction=/elsewhere formmethod=get>Reset</button></form>",
    );
    let (_, text, _, _) = ask(&mut session, &token, "look", &[]);
    assert_eq!(
        line(&text, "b1"),
        "  b1 button ? \"Plain\" submits F1 post \"/session\""
    );
    assert_eq!(
        line(&text, "b2"),
        "  b2 button ? \"Reset\" submits F1 post \"/session\""
    );
}

/// `formmethod="dialog"` on a submit button of a `POST` form ends the submission `dialog-method`,
/// status 1, the `press` in the path and no request.
#[test]
fn adversary_press_a_dialog_formmethod_ends_dialog_method_with_the_press_in_the_path() {
    let server = Server::start();
    let (mut session, token) = at(
        &server,
        "/form.html",
        "<!doctype html><form action=/session method=post><input name=a value=1>\
         <button formmethod=DIALOG>Close</button></form>",
    );
    let (status, text, typed, entries) = ask(&mut session, &token, "press", &[("handle", "b1")]);
    assert_eq!(
        typed.did,
        Some(Did::Refused(RefusalReason::DialogMethod)),
        "{text}"
    );
    assert_eq!(status, 1);
    assert!(started(&entries));
    assert!(text.contains(" press b1\n"), "{text}");
    assert_eq!(server.lines().len(), 1);
}

/// After a `press` landed on another page, `b1` of the old page is gone: `unknown-handle`, no
/// request, the path as it was.
#[test]
fn adversary_press_a_stale_handle_after_the_page_changed_is_unknown() {
    let server = Server::start();
    server.route("/session", redirect(303, "/home"));
    let (mut session, token) = at(
        &server,
        "/form.html",
        "<!doctype html><form action=/session method=post><input name=a value=1>\
         <button>Send</button></form>",
    );
    let (status, text, _, _) = ask(&mut session, &token, "press", &[("handle", "b1")]);
    assert_eq!(status, 0, "{text}");
    let requests = server.lines().len();
    let (_, _, before, _) = ask(&mut session, &token, "look", &[]);
    let (status, text, typed, _) = ask(&mut session, &token, "press", &[("handle", "b1")]);
    assert_eq!(status, 1, "{text}");
    assert_eq!(typed.did, Some(Did::Refused(RefusalReason::UnknownHandle)));
    assert_eq!(server.lines().len(), requests);
    assert_eq!(typed.path, before.path);
}

/// A `POST` answered 307 is repeated with its body to the new address, one answered 302 or
/// 303 becomes a `GET` without a body; the answer's page line names the last address, and no
/// line of the answer and no entry holds the password.
#[test]
fn adversary_press_a_redirected_post_keeps_or_drops_its_body_by_status() {
    for (status_code, expect_line, expect_body) in [
        (307u16, "POST /landing HTTP/1.1", "pw=s3cret-r"),
        (308, "POST /landing HTTP/1.1", "pw=s3cret-r"),
        (302, "GET /landing HTTP/1.1", ""),
        (303, "GET /landing HTTP/1.1", ""),
        (301, "GET /landing HTTP/1.1", ""),
    ] {
        let server = Server::start();
        server.route("/session", redirect(status_code, "/landing"));
        server.page("/landing", HOME);
        let (mut session, token) = at(
            &server,
            "/form.html",
            "<!doctype html><form action=/session method=post>\
             <input type=password name=pw><button>Send</button></form>",
        );
        fill(&mut session, &token, "f1", "s3cret-r");
        let (status, text, typed, entries) =
            ask(&mut session, &token, "press", &[("handle", "b1")]);
        assert_eq!(status, 0, "{status_code}: {text}");
        let got = server.received();
        assert_eq!(got.len(), 3, "{status_code}: {got:?}");
        assert_eq!(got[2].line, expect_line, "{status_code}");
        assert_eq!(got[2].body, expect_body, "{status_code}");
        assert_eq!(page_address(&typed), server.url("/landing"));
        assert!(!text.contains("s3cret-r"), "{text}");
        for entry in &entries {
            assert!(!entry.contains("s3cret-r"), "{entry}");
        }
    }
}

/// A `POST` answered 404 lands on that answer: `did press`, the page line with 404 at the
/// action's address.
#[test]
fn adversary_press_a_post_answered_404_lands_on_the_answer() {
    let server = Server::start();
    let (mut session, token) = at(
        &server,
        "/form.html",
        "<!doctype html><form action=/missing method=post><input name=a value=1>\
         <button>Send</button></form>",
    );
    let (status, text, typed, _) = ask(&mut session, &token, "press", &[("handle", "b1")]);
    assert_eq!(status, 0, "{text}");
    assert!(text.contains("\ndid press \"b1\"\n"), "{text}");
    match &typed.page {
        Some(Page::Loaded {
            status, address, ..
        }) => {
            assert_eq!(*status, 404, "{text}");
            assert_eq!(*address, server.url("/missing"));
        }
        other => panic!("{other:?}"),
    }
}

/// An action that does not resolve shows none of its text ("As decided after the adversary's
/// pass", decision 2): the button line holds neither the user nor the password of
/// `http://admin:hunter2@[…`, and reads `submits F1 post unresolved`.
#[test]
fn adversary_press_an_unresolved_action_shows_no_userinfo() {
    let server = Server::start();
    let (mut session, token) = at(
        &server,
        "/form.html",
        "<!doctype html><form action=\"http://admin:hunter2@[::1/x\" method=post>\
         <button>Send</button></form>",
    );
    let (_, text, _, _) = ask(&mut session, &token, "look", &[]);
    let b1 = line(&text, "b1");
    assert_eq!(b1, "  b1 button ? \"Send\" submits F1 post unresolved");
    assert!(!b1.contains("hunter2"), "{b1}");
    assert!(!b1.contains("admin"), "{b1}");
}

/// The refusals before `Press` come in the order of the story: `password-in-query` before
/// `address-too-long`, `address-too-long` before `unsupported-scheme`, `unsupported-scheme`
/// before `forbidden-destination`.
#[test]
fn adversary_press_the_refusals_before_press_come_in_the_storys_order() {
    let long = "a".repeat(65_536);
    refused_before(
        &format!(
            "<!doctype html><form action=/r><input type=password name=pw value=x>\
             <input type=hidden name=q value=\"{long}\"><button>Go</button></form>"
        ),
        RefusalReason::PasswordInQuery,
    );
    refused_before(
        &format!(
            "<!doctype html><form action=\"mailto:a@example.org\">\
             <input type=hidden name=q value=\"{long}\"><button>Go</button></form>"
        ),
        RefusalReason::AddressTooLong,
    );
    let public = "http://93.184.216.34/form.html";
    let page = "<!doctype html><form action=\"ftp://127.0.0.1/s\" method=post>\
                <input name=a value=1><button>Send</button></form>";
    let (mut session, token, asked) = in_memory(public, page.to_string());
    let (_, text, typed, _) = ask(&mut session, &token, "press", &[("handle", "b1")]);
    assert_eq!(
        typed.did,
        Some(Did::Refused(RefusalReason::UnsupportedScheme)),
        "{text}"
    );
    assert_eq!(asked.lock().unwrap().len(), 1);
    // An `https:` action passes the scheme check and meets the next one.
    let page = page.replace("ftp:", "https:");
    let (mut session, token, asked) = in_memory(public, page);
    let (_, text, typed, _) = ask(&mut session, &token, "press", &[("handle", "b1")]);
    assert_eq!(
        typed.did,
        Some(Did::Refused(RefusalReason::ForbiddenDestination)),
        "{text}"
    );
    assert_eq!(asked.lock().unwrap().len(), 1);
}

/// An Image Button is a submit button: its `formaction` and `formmethod` are taken, its line
/// says so, and its press requests the address with `<name>.x=0&<name>.y=0`; a `GET` address
/// that the two entries take past 65,536 bytes is refused `address-too-long`.
#[test]
fn adversary_press_an_image_button_takes_its_formaction_and_counts_its_coordinates() {
    let server = Server::start();
    server.page("/x", HOME);
    let (mut session, token) = at(
        &server,
        "/form.html",
        "<!doctype html><form action=/session method=post><input name=a value=1>\
         <input type=image name=go alt=Go formaction=/x formmethod=get></form>",
    );
    let (_, text, _, _) = ask(&mut session, &token, "look", &[]);
    assert_eq!(
        line(&text, "b1"),
        "  b1 button ? \"Go\" submits F1 get \"/x\""
    );
    let (status, text, _, _) = ask(&mut session, &token, "press", &[("handle", "b1")]);
    assert_eq!(status, 0, "{text}");
    assert_eq!(server.lines()[1], "GET /x?a=1&go.x=0&go.y=0 HTTP/1.1");

    let server = Server::start();
    let prefix = format!("{}?q=", server.url("/r"));
    let value = "a".repeat(65_536 - prefix.len() - "&go.x=0&go.y=0".len() + 1);
    let (mut session, token) = at(
        &server,
        "/form.html",
        &format!(
            "<!doctype html><form action=/r><input type=hidden name=q value=\"{value}\">\
             <input type=image name=go alt=Go></form>"
        ),
    );
    let (_, text, typed, entries) = ask(&mut session, &token, "press", &[("handle", "b1")]);
    assert_eq!(
        typed.did,
        Some(Did::Refused(RefusalReason::AddressTooLong)),
        "{}",
        &text[..text.len().min(300)]
    );
    assert!(!started(&entries));
    assert_eq!(server.lines().len(), 1);
}

/// A page of 120 buttons, each with a `formaction` of 1,100 bidirectional controls that does not
/// resolve (about 9,500 bytes a line once quoted, 1.1 MiB for all): the `now`
/// block stays within `NOW_BYTES_MAX` and its header counts every line.
#[test]
fn adversary_press_a_now_block_of_long_unresolved_actions_stays_within_its_bound() {
    let server = Server::start();
    let action = format!("http://[{}", "\u{202e}".repeat(1_100));
    let mut body = String::from("<!doctype html><form method=post>");
    for _ in 0..120 {
        body.push_str(&format!("<button formaction=\"{action}\">B</button>"));
    }
    body.push_str("</form>");
    let (mut session, token) = at(&server, "/form.html", &body);
    let (_, text, _, _) = ask(&mut session, &token, "look", &[]);
    let start = text.find("\nnow ").expect("a now block") + 1;
    let block: String = text[start..]
        .lines()
        .take_while(|line| line.starts_with("now ") || line.starts_with("  "))
        .map(|line| format!("{line}\n"))
        .collect();
    assert!(
        block.len() <= surf_session::NOW_BYTES_MAX,
        "{} bytes",
        block.len()
    );
    assert!(block.starts_with("now "), "{}", &block[..80]);
    assert!(block.lines().next().unwrap().ends_with(" of 120"));
    assert!(!text.contains('\u{202e}'));
}

/// A `POST` answered 307 to an address of a scheme surf does not fetch (neither `http:` nor
/// `https:`) or to one with a userinfo is not followed: no request after the `POST`, the page
/// line `failed`, no password on any line or entry.
#[test]
fn adversary_press_a_redirect_to_another_scheme_or_a_userinfo_is_not_followed() {
    let server = Server::start();
    let port = server.port;
    for location in [
        format!("ftp://127.0.0.1:{port}/landing"),
        format!("http://u:p@127.0.0.1:{port}/landing"),
        "mailto:a@example.org".to_string(),
    ] {
        server.route("/session", redirect(307, &location));
        let before = server.lines().len();
        let (mut session, token) = at(
            &server,
            "/form.html",
            "<!doctype html><form action=/session method=post>\
             <input type=password name=pw><button>Send</button></form>",
        );
        fill(&mut session, &token, "f1", "s3cret-x");
        let (_, text, typed, entries) = ask(&mut session, &token, "press", &[("handle", "b1")]);
        assert_eq!(
            server.lines().len(),
            before + 2,
            "{location}: {:?}",
            server.lines()
        );
        assert!(
            matches!(typed.page, Some(Page::Failed { .. })),
            "{location}: {text}"
        );
        assert!(!text.contains("s3cret-x"), "{text}");
        assert!(entries.iter().all(|entry| !entry.contains("s3cret-x")));
    }
}

/// A `POST` whose answer is not HTML lands on it: `did press`, the page line with its media
/// type, no `now` block.
#[test]
fn adversary_press_a_post_answered_with_json_lands_on_the_answer() {
    let server = Server::start();
    server.route(
        "/session",
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2\r\n\
         Connection: close\r\n\r\n{}"
            .to_string(),
    );
    let (mut session, token) = at(
        &server,
        "/form.html",
        "<!doctype html><form action=/session method=post><input name=a value=1>\
         <button>Send</button></form>",
    );
    let (status, text, typed, _) = ask(&mut session, &token, "press", &[("handle", "b1")]);
    assert_eq!(status, 0, "{text}");
    match &typed.page {
        Some(Page::Loaded {
            media_type,
            address,
            ..
        }) => {
            assert_eq!(media_type, "application/json");
            assert_eq!(*address, server.url("/session"));
        }
        other => panic!("{other:?}"),
    }
    assert!(typed.now.is_none(), "{text}");
}

/// A button past the 256 row entries of the session log still has its handle and is pressed:
/// the 300th button of a form submits with its own `name=value`.
#[test]
fn adversary_press_the_three_hundredth_button_is_pressed() {
    let server = Server::start();
    server.page("/r", HOME);
    let mut body = String::from("<!doctype html><form action=/r>");
    for n in 1..=300 {
        body.push_str(&format!("<button name=n value={n}>B{n}</button>"));
    }
    body.push_str("</form>");
    let (mut session, token) = at(&server, "/form.html", &body);
    let (status, text, _, _) = ask(&mut session, &token, "press", &[("handle", "b300")]);
    assert_eq!(status, 0, "{}", &text[..text.len().min(300)]);
    assert_eq!(server.lines()[1], "GET /r?n=300 HTTP/1.1");
}

/// From a page at a public address, a form whose action names a local address by a host name
/// is refused `forbidden-destination` by the fetch at its first request, before a connection is
/// made ("As decided after the adversary's pass", decision 1): status 1, the server reads no
/// byte, nothing is recorded, and the page, the path and the filled value are as before, also on
/// the `look` after. The name `intranet.test` resolves to `127.0.0.1` through a resolver of the
/// test's own ([`common::Names`], given to [`surf_session::fetch_through`]), not the host's:
/// the adversary's `ip6-localhost` is `::1` only where `/etc/hosts` says so. The page is
/// served in memory.
#[test]
fn adversary_press_a_host_name_of_a_local_address_from_a_public_page_is_refused() {
    let server = Server::start();
    let names = Arc::new(common::Names::default().name("intranet.test", &["127.0.0.1"]));
    let public = "http://93.184.216.34/form.html";
    let page = format!(
        "<!doctype html><title>Form</title><form action=\"http://intranet.test:{}/session\" \
         method=post><input name=a value=1><button>Send</button></form>",
        server.port
    );
    let start_address = public.to_string();
    let network = Arc::clone(&names);
    let transport: Memory = Box::new(move |request: &surf::RequestStarted| {
        if request.url.0 == start_address {
            surf::Answer::Response {
                url: request.url.0.clone(),
                method: request.method,
                status: 200,
                media_type: "text/html".to_string(),
                body: page.clone().into_bytes(),
            }
        } else {
            surf_session::fetch_through(request, network.as_ref())
        }
    });
    let mut session = Session::new(ID, transport).unwrap();
    let (status, text) = call(
        &mut session,
        &request("open", None, None, &[("url", public)]),
    );
    assert_eq!(status, 0, "{text}");
    let token = opened(&text).token;
    fill(&mut session, &token, "f1", "2");
    let (_, _, before, _) = ask(&mut session, &token, "look", &[]);
    let (status, text, typed, entries) = ask(&mut session, &token, "press", &[("handle", "b1")]);
    assert_eq!(
        typed.did,
        Some(Did::Refused(RefusalReason::ForbiddenDestination)),
        "the server read: {:?}\n{text}",
        server.lines()
    );
    assert_eq!(status, 1, "{text}");
    assert_eq!(server.lines(), Vec::<String>::new());
    assert_eq!(names.dialed(), Vec::new());
    assert_eq!(entries, Vec::<String>::new());
    assert_eq!(typed.page, before.page, "{text}");
    assert_eq!(typed.path, before.path, "{text}");
    assert_eq!(typed.now, before.now, "{text}");
    let (status, text, after, _) = ask(&mut session, &token, "look", &[]);
    assert_eq!(status, 0, "{text}");
    assert_eq!(after.page, before.page, "{text}");
    assert_eq!(after.path, before.path, "{text}");
    assert_eq!(after.now, before.now, "{text}");
}

/// A submit button inside a disabled `fieldset` is disabled: refused `disabled`, no request;
/// one in that fieldset's first `legend` is not, and submits.
#[test]
fn adversary_press_a_button_in_a_disabled_fieldset_is_refused() {
    refused_before(
        "<!doctype html><form action=/r><fieldset disabled><input name=a value=1>\
         <button>Send</button></fieldset></form>",
        RefusalReason::Disabled,
    );
    let server = Server::start();
    server.page("/r", HOME);
    let (mut session, token) = at(
        &server,
        "/form.html",
        "<!doctype html><form action=/r><input name=a value=1><fieldset disabled>\
         <legend><button>Send</button></legend></fieldset></form>",
    );
    let (status, text, _, _) = ask(&mut session, &token, "press", &[("handle", "b1")]);
    assert_eq!(status, 0, "{text}");
    assert_eq!(server.lines()[1], "GET /r?a=1 HTTP/1.1");
}
