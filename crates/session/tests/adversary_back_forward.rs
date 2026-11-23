//! The adversary's cases on `story:agent-back-forward` (`d4499d1`): the public-page rule of a
//! `follow` against an entry loaded again by `back` or `forward`, the refusal of an entry made
//! by `POST` in the direction case 6 does not take, the entries of a traversal that fails and
//! of one that is refused, more than one step through the history, what drops the entries after
//! the current one and what does not, and the state kept after a refused first request
//! (`Kept`) across a traversal.
//!
//! The cases drive the session core against a [`Site`] on `127.0.0.1` the test starts, or over
//! a transport in memory where a page must stand at a public address that is never dialed. A
//! name is resolved only by a resolver of the test's own, never the host's.

#![forbid(unsafe_code)]

use crate::common;
use crate::core::{ID, call, holder, open_at};

use std::io::{ErrorKind, Read, Write};
use std::net::{IpAddr, SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use common::{opened, path_block, request};
use surf_answer::{Answer, Did, RefusalReason};
use surf_session::Session;
use surfd::link::{Entry, EntryKind};

type Memory = Box<dyn FnMut(&surf::RequestStarted) -> surf::Answer>;

/// The public address the pages in memory stand at; it is never dialed.
const PUBLIC: &str = "http://93.184.216.34";

const PAGE_A: &str = "<!doctype html><title>A</title><p><a href=\"/b\">to B</a> \
                      <a href=\"javascript:void 0\">script</a>\
                      <form action=\"/c\"><button>Go</button></form>";
const PAGE_B: &str = "<!doctype html><title>B</title><p><a href=\"/c\">to C</a>";
const PAGE_C: &str = "<!doctype html><title>C</title><p>The end.";
const FORM: &str = "<!doctype html><title>Form</title><form action=\"/sent\" method=post>\
                    <input name=q value=1><button>Send</button></form>";
const SENT: &str = "<!doctype html><title>Sent</title><p><a href=\"/b\">to B</a>";

fn html(body: &str) -> String {
    format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: \
         close\r\n\r\n{body}",
        body.len()
    )
}

/// An HTTP server on `127.0.0.1` with the pages above at `/a`, `/b`, `/c`, `/form` and `/sent`,
/// 404 otherwise; it records the request line of each request it reads. While it is down it
/// closes every connection before reading it.
struct Site {
    port: u16,
    lines: Arc<Mutex<Vec<String>>>,
    down: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl Site {
    fn start() -> Site {
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
        Site {
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

impl Drop for Site {
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
    let target = line.split(' ').nth(1).unwrap_or("");
    let path = target.split('?').next().unwrap_or("");
    let reply = match path {
        "/a" => html(PAGE_A),
        "/b" => html(PAGE_B),
        "/c" => html(PAGE_C),
        "/form" => html(FORM),
        "/sent" => html(SENT),
        _ => "HTTP/1.1 404 Not Found\r\nContent-Type: text/plain\r\nContent-Length: 4\r\n\
              Connection: close\r\n\r\nnone"
            .to_string(),
    };
    let _ = stream.write_all(reply.as_bytes());
}

/// A [`surf_fetch::Network`] whose names resolve to `first` the first `times` resolutions and to
/// `then` every one after: a name whose owner changes what it points to between two fetches.
/// Every connection goes to the site on `127.0.0.1` at the port asked for, whatever address was
/// checked, so no address but the test's own server is ever dialed; the address the fetch chose
/// is noted.
struct Rebind {
    first: Vec<IpAddr>,
    times: usize,
    then: Vec<IpAddr>,
    resolved: Mutex<Vec<String>>,
    dialed: Mutex<Vec<SocketAddr>>,
}

impl Rebind {
    fn new(first: &str, times: usize, then: &str) -> Rebind {
        Rebind {
            first: vec![first.parse().unwrap()],
            times,
            then: vec![then.parse().unwrap()],
            resolved: Mutex::new(Vec::new()),
            dialed: Mutex::new(Vec::new()),
        }
    }

    fn dialed(&self) -> Vec<SocketAddr> {
        self.dialed.lock().unwrap().clone()
    }
}

impl surf_fetch::Network for Rebind {
    fn resolve(
        &self,
        host: &str,
        port: u16,
        _: Duration,
    ) -> Result<Vec<SocketAddr>, surf_fetch::Failure> {
        let mut resolved = self.resolved.lock().unwrap();
        resolved.push(host.to_string());
        let addresses = if resolved.len() > self.times {
            &self.then
        } else {
            &self.first
        };
        Ok(addresses
            .iter()
            .map(|ip| SocketAddr::new(*ip, port))
            .collect())
    }

    fn connect(&self, address: &SocketAddr, timeout: Duration) -> std::io::Result<TcpStream> {
        self.dialed.lock().unwrap().push(*address);
        TcpStream::connect_timeout(
            &SocketAddr::new([127, 0, 0, 1].into(), address.port()),
            timeout,
        )
    }
}

/// The reply to `command` with `args` as the holder: its status, its lines, its typed answer
/// and its entries, the answer entry last when there are any.
fn ask<T: surf::Transport>(
    session: &mut Session<T>,
    token: &str,
    command: &str,
    args: &[(&str, &str)],
) -> (u8, String, Answer, Vec<Entry>) {
    let reply = session.answer(&holder(command, token, args));
    let link = reply.to_link();
    (link.status, link.text, reply.answer, link.entries)
}

/// `command`, without arguments, must answer status 0 with the page line `page`.
fn lands<T: surf::Transport>(session: &mut Session<T>, token: &str, command: &str, page: &str) {
    lands_with(session, token, command, &[], page);
}

fn page_line(text: &str) -> &str {
    text.lines()
        .find(|line| line.starts_with("page "))
        .unwrap_or_else(|| panic!("no page line: {text}"))
}

fn loaded(address: &str, title: &str) -> String {
    format!("page 200 \"text/html\" \"{address}\" \"{title}\"")
}

fn refused(typed: &Answer) -> Option<RefusalReason> {
    match &typed.did {
        Some(Did::Refused(reason)) => Some(*reason),
        _ => None,
    }
}

/// A session over `transport` opened at `url`, and its token.
fn open_over(transport: Memory, url: &str) -> (Session<Memory>, String) {
    let mut session = Session::new(ID, transport).unwrap();
    let (status, text) = call(&mut session, &request("open", None, None, &[("url", url)]));
    assert_eq!(status, 0, "{text}");
    let token = opened(&text).token;
    (session, token)
}

/// A page in memory at `address` with `body`, every other address through
/// [`surf_session::fetch_through`] over `network`.
fn memory_then(pages: Vec<(String, String)>, network: Arc<dyn surf_fetch::Network>) -> Memory {
    Box::new(move |asked: &surf::RequestStarted| {
        match pages.iter().find(|(address, _)| *address == asked.url.0) {
            Some((_, body)) => surf::Answer::Response {
                url: asked.url.0.clone(),
                method: asked.method,
                status: 200,
                media_type: "text/html".to_string(),
                body: body.clone().into_bytes(),
            },
            None => surf_session::fetch_through(asked, network.as_ref()),
        }
    })
}

/// A public page in memory whose link names `rebind.test` at the site's port; the name is
/// public at its first resolution and `127.0.0.1` after. The session follows the link (allowed:
/// the name is public then), and goes back to the public page.
fn followed_from_public_and_back(site: &Site) -> (Session<Memory>, String, Arc<Rebind>, String) {
    let rebind = Arc::new(Rebind::new("93.184.216.35", 1, "127.0.0.1"));
    let public = format!("{PUBLIC}/p");
    let target = format!("http://rebind.test:{}/b", site.port);
    let page = format!("<!doctype html><title>P</title><p><a href=\"{target}\">next</a>");
    let network: Arc<dyn surf_fetch::Network> = rebind.clone();
    let (mut session, token) =
        open_over(memory_then(vec![(public.clone(), page)], network), &public);
    let (status, text, _, _) = ask(&mut session, &token, "follow", &[("handle", "l1")]);
    assert_eq!(status, 0, "{text}");
    assert_eq!(page_line(&text), loaded(&target, "B"), "{text}");
    assert_eq!(site.lines(), ["GET /b HTTP/1.1"]);
    let (status, text, _, _) = ask(&mut session, &token, "back", &[]);
    assert_eq!(status, 0, "{text}");
    assert_eq!(page_line(&text), loaded(&public, "P"), "{text}");
    (session, token, rebind, target)
}

/// Point 1 of the brief, the control: from the public page, a second `follow` of the same link,
/// whose name now resolves to `127.0.0.1`, is refused `forbidden-destination` by the fetch
/// before any connection. This is the rule the next case runs against.
#[test]
fn adversary_bf_follow_again_after_the_name_turned_local_is_refused() {
    let site = Site::start();
    let (mut session, token, rebind, _) = followed_from_public_and_back(&site);
    let (status, text, typed, _) = ask(&mut session, &token, "follow", &[("handle", "l1")]);
    assert_eq!(
        refused(&typed),
        Some(RefusalReason::ForbiddenDestination),
        "{text}"
    );
    assert_eq!(status, 1, "{text}");
    assert_eq!(site.lines(), ["GET /b HTTP/1.1"], "no second request");
    assert!(
        rebind
            .dialed()
            .iter()
            .all(|address| !surf_fetch::is_local(address.ip())),
        "dialed {:?}",
        rebind.dialed()
    );
}

/// Point 1 of the brief: the entry of `rebind.test` was made by a `follow` from a public page,
/// when the name was public. `forward` from that same public page loads the entry again; the
/// name now resolves to `127.0.0.1`. A request of the page's own link from there is refused
/// (the case above), so the step forward must not reach a local address either: the session
/// would otherwise let a public page, by its link and the name it controls, make the agent's
/// next `forward` (or `back`) request a private address.
#[test]
fn adversary_bf_forward_to_an_entry_followed_from_a_public_page_dials_no_local_address() {
    let site = Site::start();
    let (mut session, token, rebind, _) = followed_from_public_and_back(&site);
    let (_, text, _, _) = ask(&mut session, &token, "forward", &[]);
    let local: Vec<SocketAddr> = rebind
        .dialed()
        .into_iter()
        .filter(|address| surf_fetch::is_local(address.ip()))
        .collect();
    assert_eq!(
        local,
        Vec::<SocketAddr>::new(),
        "forward dialed a local address; the server read {:?}\n{text}",
        site.lines()
    );
    assert_eq!(site.lines(), ["GET /b HTTP/1.1"], "{text}");
}

/// The same with `back`: public page P, `follow` to B at `rebind.test`, `follow` from B to C on
/// the same name (both while the name is public: two resolutions), then `back` loads the entry
/// of B again, the name local now.
#[test]
fn adversary_bf_back_to_an_entry_followed_from_a_public_page_dials_no_local_address() {
    let site = Site::start();
    let rebind = Arc::new(Rebind::new("93.184.216.35", 2, "127.0.0.1"));
    let public = format!("{PUBLIC}/p");
    let target = format!("http://rebind.test:{}/b", site.port);
    let page = format!("<!doctype html><title>P</title><p><a href=\"{target}\">next</a>");
    let network: Arc<dyn surf_fetch::Network> = rebind.clone();
    let (mut session, token) =
        open_over(memory_then(vec![(public.clone(), page)], network), &public);
    lands_with(
        &mut session,
        &token,
        "follow",
        &[("handle", "l1")],
        &loaded(&target, "B"),
    );
    lands_with(
        &mut session,
        &token,
        "follow",
        &[("handle", "l1")],
        &loaded(&format!("http://rebind.test:{}/c", site.port), "C"),
    );
    assert_eq!(site.lines(), ["GET /b HTTP/1.1", "GET /c HTTP/1.1"]);
    let (_, text, _, _) = ask(&mut session, &token, "back", &[]);
    let local: Vec<SocketAddr> = rebind
        .dialed()
        .into_iter()
        .filter(|address| surf_fetch::is_local(address.ip()))
        .collect();
    assert_eq!(
        local,
        Vec::<SocketAddr>::new(),
        "the traversal dialed a local address; the server read {:?}\n{text}",
        site.lines()
    );
}

/// Point 5, the direction case 6 does not take: form, `press` (a `POST` answered 200: an entry
/// of `POST`), `follow` to B, `back`: refused `post-entry`, status 1, no request, no entry, the
/// page and the path as they were; `forward` then is `at-end`, and the session is still at B.
#[test]
fn adversary_bf_back_to_a_post_entry_is_refused_with_no_request_and_no_entry() {
    let site = Site::start();
    let (mut session, token, _) = open_at(&site.url("/form"), &[]);
    let (status, text, _, _) = ask(&mut session, &token, "press", &[("handle", "b1")]);
    assert_eq!(status, 0, "{text}");
    assert_eq!(
        page_line(&text),
        loaded(&site.url("/sent"), "Sent"),
        "{text}"
    );
    let (status, at_b, _, _) = ask(&mut session, &token, "follow", &[("handle", "l1")]);
    assert_eq!(status, 0, "{at_b}");
    let before = site.lines();
    let (status, text, typed, entries) = ask(&mut session, &token, "back", &[]);
    assert_eq!(refused(&typed), Some(RefusalReason::PostEntry), "{text}");
    assert_eq!(status, 1, "{text}");
    assert_eq!(page_line(&text), loaded(&site.url("/b"), "B"), "{text}");
    assert_eq!(site.lines(), before, "no request");
    assert_eq!(entries, Vec::<Entry>::new(), "no entry");
    assert_eq!(path_block(&text), path_block(&at_b), "no action");
    let (status, text, typed, entries) = ask(&mut session, &token, "forward", &[]);
    assert_eq!(refused(&typed), Some(RefusalReason::AtEnd), "{text}");
    assert_eq!(status, 1, "{text}");
    assert_eq!(entries, Vec::<Entry>::new(), "no entry");
    assert_eq!(page_line(&text), loaded(&site.url("/b"), "B"), "{text}");
    assert_eq!(site.lines(), before, "no request");
}

/// Point 5, case 6's direction, the entries: `forward` to the `POST` entry is refused with no
/// entry at all (docs/view-stream.md, "a refused command has no entry").
#[test]
fn adversary_bf_forward_to_a_post_entry_has_no_entry() {
    let site = Site::start();
    let (mut session, token, _) = open_at(&site.url("/form"), &[]);
    let (status, text, _, _) = ask(&mut session, &token, "press", &[("handle", "b1")]);
    assert_eq!(status, 0, "{text}");
    let (status, text, _, _) = ask(&mut session, &token, "back", &[]);
    assert_eq!(status, 0, "{text}");
    let before = site.lines();
    let (status, text, typed, entries) = ask(&mut session, &token, "forward", &[]);
    assert_eq!(refused(&typed), Some(RefusalReason::PostEntry), "{text}");
    assert_eq!(status, 1, "{text}");
    assert_eq!(entries, Vec::<Entry>::new(), "{text}");
    assert_eq!(site.lines(), before, "no request");
}

/// `back` at the first page and `forward` at the last are refused with no entry.
#[test]
fn adversary_bf_refused_at_either_end_has_no_entry() {
    let site = Site::start();
    let (mut session, token, _) = open_at(&site.url("/a"), &[]);
    for (command, reason) in [
        ("back", RefusalReason::AtStart),
        ("forward", RefusalReason::AtEnd),
    ] {
        let (status, text, typed, entries) = ask(&mut session, &token, command, &[]);
        assert_eq!(refused(&typed), Some(reason), "{text}");
        assert_eq!(status, 1, "{text}");
        assert_eq!(entries, Vec::<Entry>::new(), "{command}: {text}");
    }
    assert_eq!(site.lines(), ["GET /a HTTP/1.1"]);
}

/// Point 4: a `back` whose fetch fails has the order of docs/view-stream.md, "The entries":
/// the action entry `back` first, then the events (`TraversalRequested` among them, and the
/// navigation's failure), then the answer entry, whose page line is `page failed`.
#[test]
fn adversary_bf_a_failed_back_has_its_entries_in_order() {
    let site = Site::start();
    let (mut session, token, _) = open_at(&site.url("/a"), &[]);
    let (status, text, _, _) = ask(&mut session, &token, "follow", &[("handle", "l1")]);
    assert_eq!(status, 0, "{text}");
    site.set_down(true);
    let (_, text, _, entries) = ask(&mut session, &token, "back", &[]);
    site.set_down(false);
    assert!(page_line(&text).starts_with("page failed "), "{text}");
    let kinds: Vec<EntryKind> = entries.iter().map(|entry| entry.kind).collect();
    assert!(entries.len() >= 3, "{entries:#?}");
    assert_eq!(kinds[0], EntryKind::Action, "{entries:#?}");
    assert_eq!(entries[0].text, "action 4 back\n", "{entries:#?}");
    assert_eq!(kinds[kinds.len() - 1], EntryKind::Answer, "{entries:#?}");
    assert!(
        kinds[1..kinds.len() - 1]
            .iter()
            .all(|kind| *kind == EntryKind::Event),
        "{entries:#?}"
    );
    for name in [
        "surf.agent.TraversalRequested",
        "surf.navigation.TraversalStarted",
        "surf.navigation.NavigationFailed",
    ] {
        assert!(
            entries
                .iter()
                .any(|entry| entry.text.starts_with(&format!("event {name} "))),
            "no {name}: {entries:#?}"
        );
    }
    assert!(
        entries[entries.len() - 1].text.contains("\ndid back\n"),
        "{entries:#?}"
    );
}

/// `back` and `forward` on a closed session are refused `closed`, not `at-start` or `at-end`,
/// with no request; with an argument they are `usage`, status 2.
#[test]
fn adversary_bf_closed_and_usage_come_before_the_history() {
    let site = Site::start();
    let (mut session, token, _) = open_at(&site.url("/a"), &[]);
    for command in ["back", "forward"] {
        let (status, text, typed, _) = ask(&mut session, &token, command, &[("handle", "l1")]);
        assert_eq!(refused(&typed), Some(RefusalReason::Usage), "{text}");
        assert_eq!(status, 2, "{text}");
    }
    let (status, text, _, _) = ask(&mut session, &token, "follow", &[("handle", "l1")]);
    assert_eq!(status, 0, "{text}");
    let (status, text, _, _) = ask(&mut session, &token, "close", &[]);
    assert_eq!(status, 0, "{text}");
    let before = site.lines();
    for command in ["back", "forward"] {
        let (status, text, typed, entries) = ask(&mut session, &token, command, &[]);
        assert_eq!(refused(&typed), Some(RefusalReason::Closed), "{text}");
        assert_eq!(status, 1, "{text}");
        assert_eq!(entries, Vec::<Entry>::new(), "{text}");
    }
    assert_eq!(site.lines(), before, "no request");
}

/// More than one step: A, B, C; back twice to A, `at-start`; forward twice to C, `at-end`.
/// Then back twice to A and a `follow` to B again: the old B and C are dropped, forward is
/// `at-end`, back is A and forward is the new B.
#[test]
fn adversary_bf_two_steps_each_way_and_a_push_from_the_first() {
    let site = Site::start();
    let (a, b, c) = (
        loaded(&site.url("/a"), "A"),
        loaded(&site.url("/b"), "B"),
        loaded(&site.url("/c"), "C"),
    );
    let (mut session, token, _) = open_at(&site.url("/a"), &[]);
    for _ in 0..2 {
        let (status, text, _, _) = ask(&mut session, &token, "follow", &[("handle", "l1")]);
        assert_eq!(status, 0, "{text}");
    }
    lands(&mut session, &token, "back", &b);
    lands(&mut session, &token, "back", &a);
    let (_, text, typed, _) = ask(&mut session, &token, "back", &[]);
    assert_eq!(refused(&typed), Some(RefusalReason::AtStart), "{text}");
    lands(&mut session, &token, "forward", &b);
    lands(&mut session, &token, "forward", &c);
    let (_, text, typed, _) = ask(&mut session, &token, "forward", &[]);
    assert_eq!(refused(&typed), Some(RefusalReason::AtEnd), "{text}");
    lands(&mut session, &token, "back", &b);
    lands(&mut session, &token, "back", &a);
    let (status, text, _, _) = ask(&mut session, &token, "follow", &[("handle", "l1")]);
    assert_eq!(status, 0, "{text}");
    assert_eq!(page_line(&text), b, "{text}");
    let (_, text, typed, _) = ask(&mut session, &token, "forward", &[]);
    assert_eq!(refused(&typed), Some(RefusalReason::AtEnd), "{text}");
    lands(&mut session, &token, "back", &a);
    lands(&mut session, &token, "forward", &b);
    let (_, text, typed, _) = ask(&mut session, &token, "forward", &[]);
    assert_eq!(refused(&typed), Some(RefusalReason::AtEnd), "{text}");
    assert_eq!(site.lines().len(), 12, "{:?}", site.lines());
}

/// docs/agent-answer.md: "A `go`, `follow` or `press` after `back` drops the pages after the
/// current one": a `press` (a `GET` form) after `back` drops B, so `forward` is `at-end` and
/// `back` is A.
#[test]
fn adversary_bf_a_press_after_back_drops_the_entry_forward() {
    let site = Site::start();
    let (mut session, token, _) = open_at(&site.url("/a"), &[]);
    let (status, text, _, _) = ask(&mut session, &token, "follow", &[("handle", "l1")]);
    assert_eq!(status, 0, "{text}");
    lands(&mut session, &token, "back", &loaded(&site.url("/a"), "A"));
    let (status, text, _, _) = ask(&mut session, &token, "press", &[("handle", "b1")]);
    assert_eq!(status, 0, "{text}");
    assert_eq!(page_line(&text), loaded(&site.url("/c?"), "C"), "{text}");
    let (_, text, typed, _) = ask(&mut session, &token, "forward", &[]);
    assert_eq!(refused(&typed), Some(RefusalReason::AtEnd), "{text}");
    lands(&mut session, &token, "back", &loaded(&site.url("/a"), "A"));
}

/// A `follow` refused by the session's checks after `back` starts no navigation and drops
/// nothing: `forward` still lands on B.
#[test]
fn adversary_bf_a_refused_follow_after_back_drops_nothing() {
    let site = Site::start();
    let (mut session, token, _) = open_at(&site.url("/a"), &[]);
    let (status, text, _, _) = ask(&mut session, &token, "follow", &[("handle", "l1")]);
    assert_eq!(status, 0, "{text}");
    lands(&mut session, &token, "back", &loaded(&site.url("/a"), "A"));
    let (_, text, typed, _) = ask(&mut session, &token, "follow", &[("handle", "l2")]);
    assert_eq!(
        refused(&typed),
        Some(RefusalReason::UnsupportedScheme),
        "{text}"
    );
    lands(
        &mut session,
        &token,
        "forward",
        &loaded(&site.url("/b"), "B"),
    );
}

/// A `go` that fails after `back` commits no entry, so it drops nothing: `forward` lands on B.
/// (docs/agent-answer.md says a `go` after `back` drops the pages after the current one; this
/// pins the navigator's rule that only a committed push drops them.)
#[test]
fn adversary_bf_a_failed_go_after_back_drops_nothing() {
    let site = Site::start();
    let (mut session, token, _) = open_at(&site.url("/a"), &[]);
    let (status, text, _, _) = ask(&mut session, &token, "follow", &[("handle", "l1")]);
    assert_eq!(status, 0, "{text}");
    lands(&mut session, &token, "back", &loaded(&site.url("/a"), "A"));
    site.set_down(true);
    let (_, text, _, _) = ask(&mut session, &token, "go", &[("url", &site.url("/c"))]);
    site.set_down(false);
    assert!(page_line(&text).starts_with("page failed "), "{text}");
    lands(
        &mut session,
        &token,
        "forward",
        &loaded(&site.url("/b"), "B"),
    );
}

/// Point 3 (D243): public P, `follow` to public Q, a `follow` from Q to a name of a local
/// address refused by the fetch (the state of Q kept), then `back` to P, `forward` to Q and the
/// same `follow` again: each answer shows the page it is at, and the last is refused again
/// with Q's page; the server reads nothing.
#[test]
fn adversary_bf_the_kept_state_after_a_refused_first_request_ends_at_a_traversal() {
    let site = Site::start();
    let names: Arc<dyn surf_fetch::Network> =
        Arc::new(common::Names::default().name("intranet.test", &["127.0.0.1"]));
    let (p, q) = (format!("{PUBLIC}/p"), format!("{PUBLIC}/q"));
    let page_p = format!("<!doctype html><title>P</title><a href=\"{q}\">to Q</a>");
    let page_q = format!(
        "<!doctype html><title>Q</title><a href=\"http://intranet.test:{}/a\">in</a>",
        site.port
    );
    let (mut session, token) = open_over(
        memory_then(vec![(p.clone(), page_p), (q.clone(), page_q)], names),
        &p,
    );
    lands_with(
        &mut session,
        &token,
        "follow",
        &[("handle", "l1")],
        &loaded(&q, "Q"),
    );
    let (_, text, typed, _) = ask(&mut session, &token, "follow", &[("handle", "l1")]);
    assert_eq!(
        refused(&typed),
        Some(RefusalReason::ForbiddenDestination),
        "{text}"
    );
    assert_eq!(page_line(&text), loaded(&q, "Q"), "{text}");
    lands(&mut session, &token, "back", &loaded(&p, "P"));
    let (_, text, _, _) = ask(&mut session, &token, "look", &[]);
    assert_eq!(page_line(&text), loaded(&p, "P"), "look after back: {text}");
    lands(&mut session, &token, "forward", &loaded(&q, "Q"));
    let (_, text, typed, _) = ask(&mut session, &token, "follow", &[("handle", "l1")]);
    assert_eq!(
        refused(&typed),
        Some(RefusalReason::ForbiddenDestination),
        "{text}"
    );
    assert_eq!(page_line(&text), loaded(&q, "Q"), "{text}");
    lands(&mut session, &token, "back", &loaded(&p, "P"));
    assert_eq!(site.lines(), Vec::<String>::new());
}

fn lands_with<T: surf::Transport>(
    session: &mut Session<T>,
    token: &str,
    command: &str,
    args: &[(&str, &str)],
    page: &str,
) {
    let (status, text, _, _) = ask(session, token, command, args);
    assert_eq!(status, 0, "{command}: {text}");
    assert_eq!(page_line(&text), page, "{command}: {text}");
}

// --- Decision D279: a traversal follows the rule of the request that made its entry ----------

/// The addresses of `rebind` that are local.
fn local_dialed(rebind: &Rebind) -> Vec<SocketAddr> {
    rebind
        .dialed()
        .into_iter()
        .filter(|address| surf_fetch::is_local(address.ip()))
        .collect()
}

/// Public O links to public P, P links to `rebind.test` (public at its first resolution, local
/// after). The session goes O, P, B by `follow`, then `back` to P: the session at P, with O
/// before it and the entry of B, made from a public page, after it.
fn o_p_b_and_back(site: &Site) -> (Session<Memory>, String, Arc<Rebind>, String, String) {
    let rebind = Arc::new(Rebind::new("93.184.216.35", 1, "127.0.0.1"));
    let (o, p) = (format!("{PUBLIC}/o"), format!("{PUBLIC}/p"));
    let target = format!("http://rebind.test:{}/b", site.port);
    let page_o = format!("<!doctype html><title>O</title><a href=\"{p}\">to P</a>");
    let page_p = format!("<!doctype html><title>P</title><a href=\"{target}\">next</a>");
    let network: Arc<dyn surf_fetch::Network> = rebind.clone();
    let (mut session, token) = open_over(
        memory_then(vec![(o.clone(), page_o), (p.clone(), page_p)], network),
        &o,
    );
    lands_with(
        &mut session,
        &token,
        "follow",
        &[("handle", "l1")],
        &loaded(&p, "P"),
    );
    lands_with(
        &mut session,
        &token,
        "follow",
        &[("handle", "l1")],
        &loaded(&target, "B"),
    );
    lands(&mut session, &token, "back", &loaded(&p, "P"));
    assert_eq!(site.lines(), ["GET /b HTTP/1.1"]);
    (session, token, rebind, o, p)
}

/// D279, test 1: `forward` to the entry of B, its name local now, is refused
/// `forbidden-destination`, status 1, before any connection to a local address: the path is the
/// one before, the answer and a `look` show P, and a traversal the other way (`back` to O, then
/// `forward` to P) still works.
#[test]
fn d279_a_refused_traversal_leaves_the_session_where_it_was() {
    let site = Site::start();
    let (mut session, token, rebind, o, p) = o_p_b_and_back(&site);
    let (_, before, _, _) = ask(&mut session, &token, "look", &[]);
    let (status, text, typed, _) = ask(&mut session, &token, "forward", &[]);
    assert_eq!(
        refused(&typed),
        Some(RefusalReason::ForbiddenDestination),
        "{text}"
    );
    assert_eq!(status, 1, "{text}");
    assert!(
        text.lines()
            .any(|line| line == "did refused forbidden-destination"),
        "{text}"
    );
    assert_eq!(page_line(&text), loaded(&p, "P"), "{text}");
    assert_eq!(path_block(&text), path_block(&before), "{text}");
    assert_eq!(local_dialed(&rebind), Vec::<SocketAddr>::new());
    assert_eq!(site.lines(), ["GET /b HTTP/1.1"], "no second request");
    let (_, text, _, _) = ask(&mut session, &token, "look", &[]);
    assert_eq!(page_line(&text), loaded(&p, "P"), "look: {text}");
    assert_eq!(path_block(&text), path_block(&before), "look: {text}");
    lands(&mut session, &token, "back", &loaded(&o, "O"));
    lands(&mut session, &token, "forward", &loaded(&p, "P"));
}

/// D279, test 2, the control: the rule is the entry's, not the current page's. The agent typed
/// a local address with `go` (by `open`), then went to a public page with `go`; `back` from the
/// public page to the local entry is answered with that page.
#[test]
fn d279_back_from_a_public_page_to_a_local_entry_made_by_go_is_allowed() {
    let site = Site::start();
    let public = format!("{PUBLIC}/p");
    let page = "<!doctype html><title>P</title><p>Public.".to_string();
    let network: Arc<dyn surf_fetch::Network> = Arc::new(surf_fetch::OsNetwork);
    let (mut session, token) = open_over(
        memory_then(vec![(public.clone(), page)], network),
        &site.url("/a"),
    );
    lands_with(
        &mut session,
        &token,
        "go",
        &[("url", &public)],
        &loaded(&public, "P"),
    );
    lands(&mut session, &token, "back", &loaded(&site.url("/a"), "A"));
    assert_eq!(site.lines(), ["GET /a HTTP/1.1", "GET /a HTTP/1.1"]);
}

/// D279, test 3: an entry made by a `GET` `press` from a public page follows the rule of a
/// `follow`: `forward` to it, its name local now, is refused `forbidden-destination` and dials
/// no local address.
#[test]
fn d279_an_entry_of_a_get_press_from_a_public_page_follows_the_same_rule() {
    let site = Site::start();
    let rebind = Arc::new(Rebind::new("93.184.216.35", 1, "127.0.0.1"));
    let public = format!("{PUBLIC}/p");
    let action = format!("http://rebind.test:{}/c", site.port);
    let page = format!(
        "<!doctype html><title>P</title><form action=\"{action}\"><button>Go</button></form>"
    );
    let network: Arc<dyn surf_fetch::Network> = rebind.clone();
    let (mut session, token) =
        open_over(memory_then(vec![(public.clone(), page)], network), &public);
    lands_with(
        &mut session,
        &token,
        "press",
        &[("handle", "b1")],
        &loaded(&format!("{action}?"), "C"),
    );
    lands(&mut session, &token, "back", &loaded(&public, "P"));
    let (status, text, typed, _) = ask(&mut session, &token, "forward", &[]);
    assert_eq!(
        refused(&typed),
        Some(RefusalReason::ForbiddenDestination),
        "{text}"
    );
    assert_eq!(status, 1, "{text}");
    assert_eq!(page_line(&text), loaded(&public, "P"), "{text}");
    assert_eq!(local_dialed(&rebind), Vec::<SocketAddr>::new());
    assert_eq!(site.lines(), ["GET /c? HTTP/1.1"]);
}

/// D279, test 4, the entries (docs/view-stream.md, "The entries"): the refused traversal
/// records no action, so it has no entry, as a `follow` the fetch refused; the traversal after
/// it has its action entry first, then its events, then its answer entry.
#[test]
fn d279_the_entries_of_a_refused_traversal_and_of_the_next() {
    let site = Site::start();
    let (mut session, token, _, o, _) = o_p_b_and_back(&site);
    let (status, text, _, entries) = ask(&mut session, &token, "forward", &[]);
    assert_eq!(status, 1, "{text}");
    assert_eq!(entries, Vec::<Entry>::new(), "{text}");
    let (status, text, _, entries) = ask(&mut session, &token, "back", &[]);
    assert_eq!(status, 0, "{text}");
    assert_eq!(page_line(&text), loaded(&o, "O"), "{text}");
    let kinds: Vec<EntryKind> = entries.iter().map(|entry| entry.kind).collect();
    assert!(kinds.len() >= 3, "{entries:#?}");
    assert_eq!(kinds[0], EntryKind::Action, "{entries:#?}");
    assert_eq!(entries[0].text, "action 6 back\n", "{entries:#?}");
    assert_eq!(kinds[kinds.len() - 1], EntryKind::Answer, "{entries:#?}");
    assert!(
        kinds[1..kinds.len() - 1]
            .iter()
            .all(|kind| *kind == EntryKind::Event),
        "{entries:#?}"
    );
    assert!(
        entries.iter().any(|entry| entry
            .text
            .starts_with("event surf.agent.TraversalRequested ")),
        "{entries:#?}"
    );
}
