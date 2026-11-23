//! The refusals of `follow` before any navigation, from the fix round of the adversary's cases
//! on `story:agent-follow-links`: a target over `surf_fetch::MAX_ADDRESS_BYTES`
//! (`address-too-long`), a scheme surf does not fetch (`unsupported-scheme`), and from a page on
//! a public address a target that is a local IP address (`forbidden-destination`). Each leaves
//! the page as it was and sends no request. Beside them, how the `now` line shows a target: no
//! userinfo, and a same-origin path that starts with `//` written whole.
//!
//! The session runs over an in-memory transport that records every address it is asked for, so
//! "no request" is read from the record, and a page can stand at any address: the public
//! address `93.184.216.34` and the name `shop.example` are never dialed.

#![forbid(unsafe_code)]

use crate::common::{opened, request};
use crate::core::{ID, call, holder};

use std::sync::{Arc, Mutex};

use surf_answer::{Answer, Did, Page, RefusalReason, Target};
use surf_session::Session;

/// The page every address but the start page answers.
const REACHED: &str = "<!doctype html><title>Reached</title><p>x</p>";

/// The transport of these cases: `start` answers `page`, every other address [`REACHED`], each
/// with status 200 and `text/html`; every address asked for is recorded in order.
type Memory = Box<dyn FnMut(&surf::RequestStarted) -> surf::Answer>;

struct Opened {
    session: Session<Memory>,
    token: String,
    asked: Arc<Mutex<Vec<String>>>,
}

impl Opened {
    /// A session opened at `start`, which answers `page`.
    fn at(start: &str, page: String) -> Opened {
        let asked = Arc::new(Mutex::new(Vec::new()));
        let record = Arc::clone(&asked);
        let start_address = start.to_string();
        let transport: Memory = Box::new(move |request: &surf::RequestStarted| {
            record.lock().unwrap().push(request.url.0.clone());
            let body = if request.url.0 == start_address {
                page.clone().into_bytes()
            } else {
                REACHED.as_bytes().to_vec()
            };
            surf::Answer::Response {
                url: request.url.0.clone(),
                method: request.method,
                status: 200,
                media_type: "text/html".to_string(),
                body,
            }
        });
        let mut session = Session::new(ID, transport).unwrap();
        let (status, text) = call(
            &mut session,
            &request("open", None, None, &[("url", start)]),
        );
        assert_eq!(status, 0, "{text}");
        let token = opened(&text).token;
        Opened {
            session,
            token,
            asked,
        }
    }

    /// The reply to `command`: exit status, lines, typed answer, and whether the link answer
    /// encodes.
    fn ask(&mut self, command: &str, args: &[(&str, &str)]) -> (u8, String, Answer, bool) {
        let reply = self.session.answer(&holder(command, &self.token, args));
        let link = reply.to_link();
        let encodes = link.encode().is_ok();
        (link.status, link.text, reply.answer, encodes)
    }

    fn asked(&self) -> Vec<String> {
        self.asked.lock().unwrap().clone()
    }
}

/// A page whose only link is `href`, written as an attribute value.
fn one_link(href: &str) -> String {
    format!("<!doctype html><title>Start</title><a href=\"{href}\">Go</a>")
}

/// The address of a loaded `page` line.
fn page_address(answer: &Answer) -> String {
    match &answer.page {
        Some(Page::Loaded { address, .. }) => address.clone(),
        other => panic!("not a loaded page: {other:?}"),
    }
}

/// The target the `now` line of `l1` shows, and whether it is cut.
fn shown(answer: &Answer) -> (String, bool) {
    let element = &answer.now.as_ref().expect("a now block").elements[0];
    match &element.target {
        Target::Resolved(cut) => (cut.kept.clone(), cut.cut),
        Target::Unresolved(cut) => panic!("unresolved: {:?}", cut.kept),
        Target::State(_) => panic!("a control, not a link"),
    }
}

/// `follow l1` from a page at `start` with one link to `href` is refused `reason` with status
/// 1: no request after the page's own, the page line and the path as before, the `now` block
/// still lists the link.
fn refused(start: &str, href: &str, reason: RefusalReason) {
    let mut opened = Opened::at(start, one_link(href));
    let (_, before, before_typed, _) = opened.ask("look", &[]);
    let (status, text, typed, encodes) = opened.ask("follow", &[("handle", "l1")]);
    assert_eq!(status, 1, "{href}: {text}");
    assert_eq!(typed.did, Some(Did::Refused(reason)), "{href}: {text}");
    assert!(
        text.contains(&format!("\ndid refused {}\n", reason.word())),
        "{text}"
    );
    assert!(encodes, "{href}");
    assert_eq!(opened.asked(), vec![start.to_string()], "{href}");
    assert_eq!(typed.page, before_typed.page, "{href}");
    assert_eq!(typed.path, before_typed.path, "{href}");
    assert_eq!(typed.now, before_typed.now, "{href}: {before}");
    assert!(typed.now.is_some(), "{href}: the link is still listed");
}

/// `follow l1` from a page at `start` with one link to `href` reaches `href` resolved as
/// `reached`.
fn followed(start: &str, href: &str, reached: &str) {
    let mut opened = Opened::at(start, one_link(href));
    let (status, text, typed, _) = opened.ask("follow", &[("handle", "l1")]);
    assert_eq!(status, 0, "{href}: {text}");
    assert_eq!(page_address(&typed), reached, "{text}");
    assert_eq!(
        opened.asked(),
        vec![start.to_string(), reached.to_string()],
        "{href}"
    );
}

const LOCAL_PAGE: &str = "http://127.0.0.1:8000/start.html";
const PUBLIC_PAGE: &str = "http://93.184.216.34/start.html";

#[test]
fn a_target_of_1_1_mb_is_refused_address_too_long_and_the_session_answers_on() {
    let href = format!("#{}", "\\".repeat(1_100_000));
    let mut opened = Opened::at(LOCAL_PAGE, one_link(&href));
    let (status, text, typed, encodes) = opened.ask("follow", &[("handle", "l1")]);
    assert_eq!(status, 1, "{}", &text[..200]);
    assert_eq!(typed.did, Some(Did::Refused(RefusalReason::AddressTooLong)));
    assert!(encodes, "the refusal encodes");
    assert_eq!(page_address(&typed), LOCAL_PAGE);
    assert_eq!(opened.asked(), vec![LOCAL_PAGE.to_string()]);
    let (status, text, typed, encodes) = opened.ask("look", &[]);
    assert_eq!(status, 0, "{}", &text[..200]);
    assert!(encodes, "the next answer encodes");
    assert_eq!(typed.did, None, "a look has no did line");
    assert_eq!(page_address(&typed), LOCAL_PAGE);
}

#[test]
fn a_target_of_max_address_bytes_is_followed_and_one_byte_more_is_refused() {
    let room = surf_fetch::MAX_ADDRESS_BYTES - LOCAL_PAGE.len() - 1;
    let at_bound = format!("#{}", "f".repeat(room));
    followed(LOCAL_PAGE, &at_bound, &format!("{LOCAL_PAGE}{at_bound}"));
    assert_eq!(
        LOCAL_PAGE.len() + at_bound.len(),
        surf_fetch::MAX_ADDRESS_BYTES
    );
    refused(
        LOCAL_PAGE,
        &format!("{at_bound}f"),
        RefusalReason::AddressTooLong,
    );
}

#[test]
fn a_target_with_a_scheme_surf_does_not_fetch_is_refused_unsupported_scheme() {
    for href in [
        "javascript:alert(1)",
        "mailto:someone@shop.example",
        "data:text/html,<p>x</p>",
        "file:///etc/passwd",
        "ftp://shop.example/x",
        "https://shop.example/x",
        "about:blank",
    ] {
        refused(LOCAL_PAGE, href, RefusalReason::UnsupportedScheme);
    }
    let mut opened = Opened::at(LOCAL_PAGE, one_link("javascript:alert(1)"));
    let (_, text, typed, _) = opened.ask("look", &[]);
    assert_eq!(
        shown(&typed),
        ("javascript:alert(1)".to_string(), false),
        "{text}"
    );
}

#[test]
fn from_a_public_page_a_local_address_is_refused_forbidden_destination() {
    for href in [
        "http://127.0.0.1:8000/",
        "http://[::1]/",
        "http://10.0.0.1/",
        "http://169.254.169.254/latest/meta-data/",
        "http://192.168.1.1/",
        "http://[::ffff:127.0.0.1]/",
        "http://0.0.0.0/",
        "http://2130706433/",
        "http://localhost/x",
        "http://shop.localhost/x",
    ] {
        refused(PUBLIC_PAGE, href, RefusalReason::ForbiddenDestination);
        refused(
            "http://shop.example/start.html",
            href,
            RefusalReason::ForbiddenDestination,
        );
    }
}

#[test]
fn from_a_public_page_a_public_address_and_a_name_are_followed() {
    followed(
        PUBLIC_PAGE,
        "http://93.184.216.35/x",
        "http://93.184.216.35/x",
    );
    followed(PUBLIC_PAGE, "/next", "http://93.184.216.34/next");
    // A name other than `localhost` is resolved by the fetch, which applies the rule of
    // redirects only to a redirect; `follow` cannot read where it leads from its text.
    followed(
        PUBLIC_PAGE,
        "http://intranet.example/x",
        "http://intranet.example/x",
    );
}

#[test]
fn from_a_local_page_a_local_address_is_followed() {
    followed(
        LOCAL_PAGE,
        "http://127.0.0.1:8000/next",
        "http://127.0.0.1:8000/next",
    );
    followed(LOCAL_PAGE, "http://10.0.0.1/", "http://10.0.0.1/");
    followed(
        "http://10.0.0.5/start.html",
        "http://127.0.0.1/",
        "http://127.0.0.1/",
    );
    followed(
        "http://[::1]/start.html",
        "http://169.254.169.254/",
        "http://169.254.169.254/",
    );
    followed(
        "http://localhost:8000/start.html",
        "http://127.0.0.1/",
        "http://127.0.0.1/",
    );
}

#[test]
fn a_shown_target_holds_no_userinfo_and_follow_goes_to_the_whole_address() {
    let href = "http://user:secret@other.example/login";
    let mut opened = Opened::at(LOCAL_PAGE, one_link(href));
    let (_, text, typed, _) = opened.ask("look", &[]);
    assert_eq!(
        shown(&typed),
        ("http://other.example/login".to_string(), false),
        "{text}"
    );
    let (status, text, typed, _) = opened.ask("follow", &[("handle", "l1")]);
    assert_eq!(status, 0, "{text}");
    assert_eq!(page_address(&typed), href);
}

#[test]
fn a_cut_target_keeps_its_scheme_host_and_port_whole() {
    let href = format!(
        "http://{}@other.example:8080/{}",
        "u".repeat(1_100),
        "p".repeat(1_100)
    );
    let mut opened = Opened::at(LOCAL_PAGE, one_link(&href));
    let (_, _, typed, _) = opened.ask("look", &[]);
    let (kept, cut) = shown(&typed);
    assert!(cut);
    assert!(
        kept.starts_with("http://other.example:8080/ppp"),
        "{kept:.60}"
    );
    assert_eq!(kept.chars().count(), surf_answer::TARGET_CHARS_MAX);
}

#[test]
fn a_same_origin_path_that_starts_with_two_slashes_is_shown_whole() {
    let mut opened = Opened::at(LOCAL_PAGE, one_link("/.//bank.example/login"));
    let (_, text, typed, _) = opened.ask("look", &[]);
    assert_eq!(
        shown(&typed),
        (
            "http://127.0.0.1:8000//bank.example/login".to_string(),
            false
        ),
        "{text}"
    );
    let mut opened = Opened::at(LOCAL_PAGE, one_link("/a//b"));
    let (_, text, typed, _) = opened.ask("look", &[]);
    assert_eq!(shown(&typed), ("/a//b".to_string(), false), "{text}");
}
