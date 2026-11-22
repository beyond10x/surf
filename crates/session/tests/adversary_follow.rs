//! Adversarial cases on `story:agent-follow-links`: what a page can put into a link's `href`
//! and what `follow` then does with it, against a page server on `127.0.0.1` the test starts.

#![forbid(unsafe_code)]

use crate::core::{Fetch, HTML, Site, holder, open_at};

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use surf_answer::{Answer, Target};
use surf_session::Session;

/// The reply to the call: its exit status, its lines, its typed answer and whether the link
/// answer encodes (what the session process writes; nothing when it does not).
fn ask(
    session: &mut Session<Fetch>,
    command: &str,
    token: &str,
    args: &[(&str, &str)],
) -> (u8, String, Answer, Result<usize, String>) {
    let reply = session.answer(&holder(command, token, args));
    let link = reply.to_link();
    let encoded = link
        .encode()
        .map(|bytes| bytes.len())
        .map_err(|error| format!("{error}"));
    (link.status, link.text, reply.answer, encoded)
}

/// The `now` line of `handle` in `answer`: its target as the line shows it.
fn shown_target(answer: &Answer, handle: &str) -> (String, bool) {
    let element = answer
        .now
        .as_ref()
        .expect("a now block")
        .elements
        .iter()
        .find(|element| element.handle == handle)
        .unwrap_or_else(|| panic!("no now line {handle}"));
    match &element.target {
        Target::Resolved(cut) => (cut.kept.clone(), cut.cut),
        Target::Unresolved(cut) => panic!("{handle} unresolved: {:?}", cut.kept),
        Target::State(_) => panic!("{handle} is a control, not a link"),
    }
}

/// The address on the `page` line of `text`, unquoted.
fn page_address(text: &str) -> String {
    let line = text.lines().nth(2).expect("a page line");
    let field = line
        .split(' ')
        .nth(3)
        .unwrap_or_else(|| panic!("no address on {line:?}"));
    surf_answer::unquote(field).unwrap_or_else(|error| panic!("{error:?}: {line}"))
}

/// The bound of "What it keeps" (`crates/session/src/lib.rs`) that keeps an answer under
/// `ANSWER_TEXT_MAX` counts two sources of an address: a request of at most 64 KiB, and the
/// address a response came from after redirects, at most `surf_fetch::MAX_ADDRESS_BYTES`
/// (64 KiB). `follow` adds a third, the page's `href`, which nothing bounds: the `page` line
/// then holds the address the page wrote. A fragment is never sent, so the server answers 200.
/// At 1,100,000 backslashes (4 bytes each quoted) the answer passes 4 MiB and does not encode;
/// that case runs over 60 s, so this one asserts the premise with 70,000.
#[test]
fn adversary_follow_a_page_written_address_is_bounded_like_a_redirect() {
    let fragment = "\\".repeat(70_000);
    let page = format!("<!doctype html><title>t</title><a href=\"#{fragment}\">same</a>");
    let site = Site::start(vec![("/", HTML, page.into_bytes())]);
    let (mut session, token, _) = open_at(&site.url("/"), &[]);
    let (status, text, _, _) = ask(&mut session, "follow", &token, &[("handle", "l1")]);
    if status == 0 {
        let address = page_address(&text);
        assert!(
            address.len() <= surf_fetch::MAX_ADDRESS_BYTES,
            "follow put an address of {} bytes on the page line ({} quoted), over the {} \
             the answer's bound counts",
            address.len(),
            surf_answer::quote(&address).len(),
            surf_fetch::MAX_ADDRESS_BYTES
        );
    }
}

/// A target over `TARGET_CHARS_MAX` is shown cut, and `follow` goes to the whole target.
#[test]
fn adversary_follow_a_cut_target_is_followed_whole() {
    let path: &'static str = format!("/{}", "p".repeat(1_500)).leak();
    let page = format!("<!doctype html><a href=\"{path}\">long</a>");
    let site = Site::start(vec![
        ("/", HTML, page.into_bytes()),
        (
            path,
            HTML,
            b"<!doctype html><title>Whole</title><p>x</p>".to_vec(),
        ),
    ]);
    let (mut session, token, _) = open_at(&site.url("/"), &[]);
    let (_, _, typed, _) = ask(&mut session, "look", &token, &[]);
    let (shown, cut) = shown_target(&typed, "l1");
    assert!(cut, "{shown}");
    assert_eq!(shown.chars().count(), surf_answer::TARGET_CHARS_MAX);
    let (status, text, typed, _) = ask(&mut session, "follow", &token, &[("handle", "l1")]);
    assert_eq!(status, 0, "{text}");
    assert_eq!(page_address(&text), site.url(path));
    assert!(
        matches!(&typed.page, Some(surf_answer::Page::Loaded { title: Some(title), .. }) if title == "Whole"),
        "{text}"
    );
}

/// A same-origin target is shown as its path; the path must read back as that target. A path
/// that starts with `//` reads as a network-path reference to another host: the line says
/// `-> "//bank.example/login"` for a link that stays on the page's own origin.
#[test]
fn adversary_follow_a_shortened_target_does_not_read_as_another_host() {
    let page = "<!doctype html><a href=\"/.//bank.example/login\">Bank</a>";
    let site = Site::start(vec![("/", HTML, page.as_bytes().to_vec())]);
    let (mut session, token, _) = open_at(&site.url("/"), &[]);
    let (_, text, typed, _) = ask(&mut session, "look", &token, &[]);
    let (shown, _) = shown_target(&typed, "l1");
    let (_, followed, _, _) = ask(&mut session, "follow", &token, &[("handle", "l1")]);
    let reached = page_address(&followed);
    assert!(
        !shown.starts_with("//"),
        "the now line shows {shown:?}, which reads as http:{shown}, and follow reaches \
         {reached}: {text}"
    );
}

/// A target to another origin is shown whole and cut at `TARGET_CHARS_MAX`; the cut must not
/// leave out the host. A userinfo of 1,100 characters pushes the host past the cut, and the
/// line shows a name the link does not go to.
#[test]
fn adversary_follow_a_cut_target_keeps_its_host() {
    let userinfo = format!("bank.example{}", "a".repeat(1_100));
    let page = format!("<!doctype html><a href=\"http://{userinfo}@other.example/login\">Bank</a>");
    let site = Site::start(vec![("/", HTML, page.into_bytes())]);
    let (mut session, token, _) = open_at(&site.url("/"), &[]);
    let (_, _, typed, _) = ask(&mut session, "look", &token, &[]);
    let (shown, cut) = shown_target(&typed, "l1");
    assert!(
        shown.contains("other.example"),
        "the target goes to other.example; the line shows {:?}...{} (cut {cut})",
        &shown[..40],
        &shown[shown.len() - 20..]
    );
}

/// An HTTP server on `127.0.0.1` port 0: `/r` redirects to `/dir/page.html`, which links to
/// `next.html`; every other target 404. Stopped when dropped.
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
                    redirecting_answer(stream);
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

fn redirecting_answer(mut stream: TcpStream) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));
    let mut head = Vec::new();
    let mut buf = [0u8; 1024];
    while !head.ends_with(b"\r\n\r\n") && head.len() < 16 * 1024 {
        match stream.read(&mut buf) {
            Ok(0) | Err(_) => return,
            Ok(n) => head.extend_from_slice(&buf[..n]),
        }
    }
    const PAGE: &str = "<!doctype html><a href=\"next.html\">Next</a>";
    let reply = if head.starts_with(b"GET /r ") {
        "HTTP/1.1 302 Found\r\nLocation: /dir/page.html\r\nContent-Length: 0\r\nConnection: \
         close\r\n\r\n"
            .to_string()
    } else if head.starts_with(b"GET /dir/page.html ") {
        format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: \
             close\r\n\r\n{PAGE}",
            PAGE.len()
        )
    } else {
        "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_string()
    };
    let _ = stream.write_all(reply.as_bytes());
}

/// After a redirect, a relative link resolves against the address the response came from,
/// not the one asked for.
#[test]
fn adversary_follow_a_relative_link_after_a_redirect_resolves_against_where_it_ended() {
    let server = Redirecting::start();
    let (mut session, token, _) = open_at(&server.url("/r"), &[]);
    let (_, text, typed, _) = ask(&mut session, "look", &token, &[]);
    assert_eq!(shown_target(&typed, "l1").0, "/dir/next.html", "{text}");
}

/// The session resolves a link against the document base URL, not the document's address: a
/// `base` in the page moves both the `now` line and where `follow` goes. No case of the story
/// puts a `base` into a page the session reads.
#[test]
fn adversary_follow_the_session_resolves_against_the_base_element() {
    let page = "<!doctype html><head><base href=\"/b/\"></head><a href=\"x?q=1\">x</a>";
    let site = Site::start(vec![("/", HTML, page.as_bytes().to_vec())]);
    let (mut session, token, _) = open_at(&site.url("/"), &[]);
    let (_, text, typed, _) = ask(&mut session, "look", &token, &[]);
    assert_eq!(shown_target(&typed, "l1").0, "/b/x?q=1", "{text}");
    let (_, text, _, _) = ask(&mut session, "follow", &token, &[("handle", "l1")]);
    assert_eq!(page_address(&text), site.url("/b/x?q=1"));
}

/// A target on the page's origin is shown with its query and its fragment.
#[test]
fn adversary_follow_a_same_origin_target_keeps_its_query_and_fragment() {
    let page = "<!doctype html><a href=\"/q?a=1#part\">q</a>";
    let site = Site::start(vec![("/", HTML, page.as_bytes().to_vec())]);
    let (mut session, token, _) = open_at(&site.url("/"), &[]);
    let (_, text, typed, _) = ask(&mut session, "look", &token, &[]);
    assert_eq!(shown_target(&typed, "l1").0, "/q?a=1#part", "{text}");
}

/// An answer with page text has no `now` block, and so no `note visibility:` line, which goes
/// with the block only (`docs/agent-answer.md`, "Note").
#[test]
fn adversary_follow_an_answer_with_text_has_no_visibility_note() {
    let page = "<!doctype html><a href=\"/x\">x</a>";
    let site = Site::start(vec![("/", HTML, page.as_bytes().to_vec())]);
    let (mut session, token, _) = open_at(&site.url("/"), &[]);
    let (_, plain, _, _) = ask(&mut session, "look", &token, &[]);
    assert!(plain.contains("\nnote visibility: "), "{plain}");
    for form in ["markdown", "tree"] {
        let (_, text, _, _) = ask(&mut session, "look", &token, &[("form", form)]);
        assert!(!text.contains("\nnote visibility: "), "{text}");
    }
}
