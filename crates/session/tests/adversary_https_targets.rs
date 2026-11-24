//! The adversary's cases on `story:session-follow-press-https` (`d819b4a`): `follow` and
//! `press` take `https:` targets.
//!
//! In memory (a transport that records every request as its method and address, so a page can
//! stand at a public address that is never dialed): from a page that is not local by its text,
//! an `https:` target that is local by its text is `forbidden-destination` for `follow` and for
//! `press` (`POST` and `GET`), in every spelling of the host, with nothing requested; a scheme
//! that only starts with `http` stays `unsupported-scheme`; a `GET` form to an `https:` action is
//! requested with its query.
//!
//! Over TLS (`surf-tls-test-server` with the case `ok-ecdsa-p256`, the session over
//! [`surf_session::fetch`] opened with that case's root as `arg anchors`): a `GET` form on an
//! `https:` page is one request over TLS with the query; `back` and `forward` cross between an
//! `http:` and an `https:` page; `back` to the answer of an `https:` `POST` is `post-entry`; a
//! 303 after an `https:` `POST` lands with `GET`; the answer of an `https:` `follow` and the
//! `log` after it read as those of the same `http:` pages; and from a public page an `https:`
//! link whose server redirects to a local address is not followed there.

#![forbid(unsafe_code)]

use crate::common::{opened, request};
use crate::core::{Fetch, HTML, ID, Site, call, holder, open_at};

use std::fs;
use std::net::{IpAddr, SocketAddr, TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use surf_answer::{Answer, Did, Page, RefusalReason};
use surf_session::Session;
use surf_tls_test_server::{Config, Response, TestServer, certs_dir};

// --- In memory ------------------------------------------------------------------------------

/// The page every address but the start page answers.
const REACHED: &str = "<!doctype html><title>Reached</title><p>x</p>";

type Memory = Box<dyn FnMut(&surf::RequestStarted) -> surf::Answer>;

/// A session over a transport in memory opened at `start`, which answers `page`; every other
/// address answers [`REACHED`]. Each request is recorded as `<method> <address>`.
fn in_memory(start: &str, page: &str) -> (Session<Memory>, String, Arc<Mutex<Vec<String>>>) {
    let asked = Arc::new(Mutex::new(Vec::new()));
    let record = Arc::clone(&asked);
    let start_address = start.to_string();
    let page = page.to_string();
    let transport: Memory = Box::new(move |request: &surf::RequestStarted| {
        record
            .lock()
            .unwrap()
            .push(format!("{:?} {}", request.method, request.url.0));
        let body = if request.url.0 == start_address {
            page.clone()
        } else {
            REACHED.to_string()
        };
        surf::Answer::Response {
            url: request.url.0.clone(),
            method: request.method,
            status: 200,
            media_type: HTML.to_string(),
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

/// The reply to `command`: its status, its lines and its typed answer.
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

fn page_address(answer: &Answer) -> String {
    match &answer.page {
        Some(Page::Loaded { address, .. }) => address.clone(),
        other => panic!("not a loaded page: {other:?}"),
    }
}

/// A page whose only link is `href`.
fn link_page(href: &str) -> String {
    format!("<!doctype html><title>Start</title><a href=\"{href}\">Go</a>")
}

/// A page whose only form sends `a=1` to `action` with `method`, by the button `b1`.
fn form_page(action: &str, method: &str) -> String {
    format!(
        "<!doctype html><title>Start</title><form action=\"{action}\" method={method}>\
         <input name=a value=1><button>Send</button></form>"
    )
}

/// `command handle` on the page `body` at `start` is refused `reason`, status 1, with no
/// request after the page's own and the page and the path as before. A failure is returned as
/// its text, so that a case lists every spelling that fails.
fn refused(
    start: &str,
    body: &str,
    command: &str,
    handle: &str,
    reason: RefusalReason,
) -> Result<(), String> {
    let (mut session, token, asked) = in_memory(start, body);
    let (_, _, before) = ask(&mut session, &token, "look", &[]);
    let (status, text, typed) = ask(&mut session, &token, command, &[("handle", handle)]);
    let asked = asked.lock().unwrap().clone();
    if status == 1
        && typed.did == Some(Did::Refused(reason))
        && asked == vec![format!("Get {start}")]
        && typed.page == before.page
        && typed.path == before.path
    {
        Ok(())
    } else {
        Err(format!(
            "{command} on {body}\nfrom {start}: status {status}, asked {asked:?}\n{text}"
        ))
    }
}

/// Item 1 of the adversary's brief: every spelling of a local host in an `https:` target, from a
/// page that is not local by its text (an `http:` and an `https:` one), is refused
/// `forbidden-destination` by `follow` and by `press` with `POST` and with `GET`, before any
/// request.
#[test]
fn adversary_https_a_local_https_target_from_a_public_page_is_forbidden_for_follow_and_press() {
    let targets = [
        "https://127.0.0.1/x",
        "https://127.0.0.1:8443/x",
        "https://localhost/x",
        "https://localhost:8443/x",
        "https://[::1]/x",
        "https://[::1]:8443/x",
        "https://shop.localhost/x",
        "https://user:secret@127.0.0.1/x",
        "https://user@localhost:8443/x",
        "HTTPS://LOCALHOST/x",
        "HtTpS://Shop.LocalHost:8443/x",
        "https://localhost./x",
        "https://shop.localhost.:8443/x",
        "https://[::ffff:127.0.0.1]/x",
        "https://0x7f.1/x",
        "https://2130706433/x",
        "https://169.254.169.254/latest/meta-data/",
        "https://10.0.0.1/",
        "https://0.0.0.0/",
    ];
    let mut wrong = Vec::new();
    for start in [
        "http://shop.example/start.html",
        "https://shop.example/start.html",
        "https://93.184.216.34/start.html",
    ] {
        for target in targets {
            let cases = [
                (link_page(target), "follow", "l1"),
                (form_page(target, "post"), "press", "b1"),
                (form_page(target, "get"), "press", "b1"),
            ];
            for (body, command, handle) in cases {
                if let Err(text) = refused(
                    start,
                    &body,
                    command,
                    handle,
                    RefusalReason::ForbiddenDestination,
                ) {
                    wrong.push(text);
                }
            }
        }
    }
    assert!(
        wrong.is_empty(),
        "{} not refused:\n{}",
        wrong.len(),
        wrong.join("\n\n")
    );
}

/// Item 2 of the adversary's brief: a scheme that only starts with `http` is neither `http:` nor
/// `https:`, and stays `unsupported-scheme` for `follow` and for `press`, before any request or
/// submission.
#[test]
fn adversary_https_a_scheme_that_only_starts_with_http_is_unsupported() {
    let start = "http://127.0.0.1:8000/start.html";
    let mut wrong = Vec::new();
    for target in [
        "httpx://shop.example/x",
        "https2://shop.example/x",
        "httpss://shop.example/x",
        "http+unix://shop.example/x",
    ] {
        let cases = [
            (link_page(target), "follow", "l1"),
            (form_page(target, "post"), "press", "b1"),
            (form_page(target, "get"), "press", "b1"),
        ];
        for (body, command, handle) in cases {
            if let Err(text) = refused(
                start,
                &body,
                command,
                handle,
                RefusalReason::UnsupportedScheme,
            ) {
                wrong.push(text);
            }
        }
    }
    assert!(
        wrong.is_empty(),
        "{} not refused:\n{}",
        wrong.len(),
        wrong.join("\n\n")
    );
}

/// Item 3 of the adversary's brief, in memory: a `GET` form with an `https:` action, relative on
/// an `https:` page and absolute on an `http:` page, is pressed: one `GET` of the action with the
/// form's entries as its query, and the press lands on it.
#[test]
fn adversary_https_press_sends_a_get_form_to_an_https_action_with_its_query() {
    for (start, action, sent) in [
        (
            "https://shop.example/form.html",
            "/find",
            "https://shop.example/find?a=1",
        ),
        (
            "http://127.0.0.1:8000/form.html",
            "https://127.0.0.1:8000/find",
            "https://127.0.0.1:8000/find?a=1",
        ),
    ] {
        let (mut session, token, asked) = in_memory(start, &form_page(action, "get"));
        let (status, text, typed) = ask(&mut session, &token, "press", &[("handle", "b1")]);
        assert_eq!(status, 0, "{start}\n{text}");
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
            vec![format!("Get {start}"), format!("Get {sent}")],
            "{start}"
        );
    }
}

// --- Over TLS -------------------------------------------------------------------------------

/// How long a case waits for a TLS server to end the connections it expects.
const SEEN_WITHIN: Duration = Duration::from_secs(10);

/// 200 `text/html` with `body`.
fn html(body: &str) -> Response {
    Response {
        status: 200,
        reason: "OK".to_string(),
        fields: vec![("Content-Type".to_string(), HTML.to_string())],
        body: body.as_bytes().to_vec(),
    }
}

/// `status` with `Location: location` and no body.
fn redirect(status: u16, location: &str) -> Response {
    Response {
        status,
        reason: "Moved".to_string(),
        fields: vec![("Location".to_string(), location.to_string())],
        body: Vec::new(),
    }
}

/// The TLS test server of the certificate case `case`, answering each path of `pages`; any
/// other path 404.
fn tls(case: &str, pages: Vec<(String, Response)>) -> TestServer {
    let mut config = Config {
        case: case.to_string(),
        ..Config::default()
    };
    config.responses.clear();
    config.responses.extend(pages);
    TestServer::start(config).expect("the TLS test server starts")
}

/// The PEM text of the root of the certificate case `case`.
fn root_pem(case: &str) -> String {
    fs::read_to_string(certs_dir().join(case).join("root.pem")).expect("the root of the case")
}

/// A session over [`surf_session::fetch`] opened at `url` with the root of `ok-ecdsa-p256` as
/// its extra anchors; its token.
fn trusting(url: &str) -> (Session<Fetch>, String) {
    let pem = root_pem("ok-ecdsa-p256");
    let (session, token, text) = open_at(url, &[("anchors", &pem)]);
    assert!(
        text.contains(&format!("page 200 \"text/html\" \"{url}\"")),
        "{text}"
    );
    (session, token)
}

fn origin(server: &TestServer) -> String {
    format!("https://localhost:{}", server.port())
}

/// The request lines and bodies of the connections `server` ended, waiting for `count`.
fn requests(server: &TestServer, count: usize) -> Vec<(String, String)> {
    server
        .wait_seen(count, SEEN_WITHIN)
        .into_iter()
        .map(|seen| {
            (
                seen.request.unwrap_or_default(),
                String::from_utf8_lossy(&seen.body).into_owned(),
            )
        })
        .collect()
}

fn fill<T: surf::Transport>(session: &mut Session<T>, token: &str, handle: &str, value: &str) {
    let (status, text, _) = ask(
        session,
        token,
        "fill",
        &[("handle", handle), ("value", value)],
    );
    assert_eq!(status, 0, "{text}");
}

/// A `GET` form on an `https:` page with a relative action: one request over TLS, its target
/// the action with the query, no body; the press lands on it.
#[test]
fn adversary_https_a_get_form_on_an_https_page_is_one_request_over_tls_with_the_query() {
    let server = tls(
        "ok-ecdsa-p256",
        vec![
            (
                "/".to_string(),
                html("<!doctype html><form action=/find><input name=q><button>Go</button></form>"),
            ),
            (
                "/find?q=blue".to_string(),
                html("<!doctype html><p>found</p>"),
            ),
        ],
    );
    let start = format!("{}/", origin(&server));
    let (mut session, token) = trusting(&start);
    fill(&mut session, &token, "f1", "blue");
    let (status, text, typed) = ask(&mut session, &token, "press", &[("handle", "b1")]);
    assert_eq!(status, 0, "{text}");
    assert_eq!(
        page_address(&typed),
        format!("{start}find?q=blue"),
        "{text}"
    );
    let mut seen = requests(&server, 2);
    seen.sort();
    assert_eq!(
        seen,
        vec![
            ("GET / HTTP/1.1".to_string(), String::new()),
            ("GET /find?q=blue HTTP/1.1".to_string(), String::new()),
        ]
    );
    assert_eq!(server.accepted(), 2);
}

/// `back` and `forward` between an `http:` page and the `https:` page its link led to: each
/// lands on the page it names, the `https:` one fetched again over TLS.
#[test]
fn adversary_https_back_and_forward_cross_between_http_and_https() {
    let server = tls(
        "ok-ecdsa-p256",
        vec![(
            "/".to_string(),
            html("<!doctype html><title>Secure</title><p>s</p>"),
        )],
    );
    let secure = format!("{}/", origin(&server));
    let link = format!("<!doctype html><title>Plain</title><a href=\"{secure}\">Secure</a>");
    let site = Site::start(vec![("/", HTML, link.into_bytes())]);
    let plain = site.url("/");
    let (mut session, token) = trusting(&plain);
    let (status, text, typed) = ask(&mut session, &token, "follow", &[("handle", "l1")]);
    assert_eq!(status, 0, "{text}");
    assert_eq!(page_address(&typed), secure, "{text}");
    for (command, lands) in [
        ("back", &plain),
        ("forward", &secure),
        ("back", &plain),
        ("forward", &secure),
    ] {
        let (status, text, typed) = ask(&mut session, &token, command, &[]);
        assert_eq!(status, 0, "{command}: {text}");
        assert_eq!(&page_address(&typed), lands, "{command}: {text}");
        assert!(
            text.contains(&format!("\ndid {command}\n")),
            "{command}: {text}"
        );
    }
    assert_eq!(server.accepted(), 3, "the follow and two forwards");
}

/// `back` to the page an `https:` `POST` answered is refused `post-entry` with nothing sent; a
/// page a 303 led to after an `https:` `POST` was fetched with `GET` and is gone forward to.
#[test]
fn adversary_https_back_to_an_https_post_answer_is_post_entry_and_a_303_lands_with_get() {
    let form = "<!doctype html><form method=post action=/send><input name=q>\
                <button>Send</button></form>";
    let server = tls(
        "ok-ecdsa-p256",
        vec![
            ("/".to_string(), html(form)),
            (
                "/send".to_string(),
                html("<!doctype html><a href=\"/after\">After</a>"),
            ),
            ("/after".to_string(), html("<!doctype html><p>after</p>")),
        ],
    );
    let start = format!("{}/", origin(&server));
    let (mut session, token) = trusting(&start);
    fill(&mut session, &token, "f1", "blue");
    let (status, text, _) = ask(&mut session, &token, "press", &[("handle", "b1")]);
    assert_eq!(status, 0, "{text}");
    let (status, text, typed) = ask(&mut session, &token, "follow", &[("handle", "l1")]);
    assert_eq!(status, 0, "{text}");
    assert_eq!(page_address(&typed), format!("{start}after"), "{text}");
    let (status, text, typed) = ask(&mut session, &token, "back", &[]);
    assert_eq!(status, 1, "{text}");
    assert_eq!(
        typed.did,
        Some(Did::Refused(RefusalReason::PostEntry)),
        "{text}"
    );
    assert_eq!(page_address(&typed), format!("{start}after"), "{text}");
    assert_eq!(server.accepted(), 3, "nothing sent again");

    let server = tls(
        "ok-ecdsa-p256",
        vec![
            ("/".to_string(), html(form)),
            ("/send".to_string(), redirect(303, "/done")),
            ("/done".to_string(), html("<!doctype html><p>done</p>")),
        ],
    );
    let start = format!("{}/", origin(&server));
    let (mut session, token) = trusting(&start);
    fill(&mut session, &token, "f1", "blue");
    let (status, text, typed) = ask(&mut session, &token, "press", &[("handle", "b1")]);
    assert_eq!(status, 0, "{text}");
    assert_eq!(page_address(&typed), format!("{start}done"), "{text}");
    let mut seen = requests(&server, 3);
    seen.sort();
    assert_eq!(
        seen,
        vec![
            ("GET / HTTP/1.1".to_string(), String::new()),
            ("GET /done HTTP/1.1".to_string(), String::new()),
            ("POST /send HTTP/1.1".to_string(), "q=blue".to_string()),
        ]
    );
    let (status, text, typed) = ask(&mut session, &token, "back", &[]);
    assert_eq!(status, 0, "{text}");
    assert_eq!(page_address(&typed), start, "{text}");
    let (status, text, typed) = ask(&mut session, &token, "forward", &[]);
    assert_eq!(status, 0, "{text}");
    assert_eq!(page_address(&typed), format!("{start}done"), "{text}");
    let seen = requests(&server, 5);
    assert_eq!(
        seen.iter()
            .filter(|(line, _)| line == "POST /send HTTP/1.1")
            .count(),
        1,
        "the POST is not sent again: {seen:?}"
    );
}

/// The answer of `follow` to an `https:` page, its `changed` block included, and the `log`
/// after it, are those of the same pages over `http:`, the origin aside.
#[test]
fn adversary_https_a_follow_and_the_log_read_as_over_http() {
    const FIRST: &str =
        "<!doctype html><title>One</title><p>first text</p><a href=\"/next\">Next</a>";
    const SECOND: &str =
        "<!doctype html><title>Two</title><p>second text</p><a href=\"/\">Home</a>";
    let server = tls(
        "ok-ecdsa-p256",
        vec![
            ("/".to_string(), html(FIRST)),
            ("/next".to_string(), html(SECOND)),
        ],
    );
    let site = Site::start(vec![
        ("/", HTML, FIRST.as_bytes().to_vec()),
        ("/next", HTML, SECOND.as_bytes().to_vec()),
    ]);
    let mut seen = Vec::new();
    for at in [site.url(""), origin(&server)] {
        let (mut session, token) = trusting(&format!("{at}/"));
        let (status, follow, _) = ask(&mut session, &token, "follow", &[("handle", "l1")]);
        assert_eq!(status, 0, "{follow}");
        let (status, log, _) = ask(&mut session, &token, "log", &[]);
        assert_eq!(status, 0, "{log}");
        seen.push((
            follow.replace(&at, "<origin>"),
            log.replace(&at, "<origin>"),
        ));
    }
    assert!(
        seen[1]
            .0
            .contains("changed page \"<origin>/\" -> \"<origin>/next\""),
        "{}",
        seen[1].0
    );
    assert!(
        seen[1].0.contains("  + text \"second text\""),
        "{}",
        seen[1].0
    );
    assert_eq!(seen[1].0, seen[0].0, "follow: https, then http");
    assert_eq!(seen[1].1, seen[0].1, "log: https, then http");
}

/// A network whose name `a.example.test` is the public address 203.0.113.5 and `localhost` is
/// 127.0.0.1; a connection to 203.0.113.5 goes to the TLS server on 127.0.0.1, every other one
/// to its address. Every address dialed is noted.
struct Routed {
    tls_port: u16,
    dialed: Mutex<Vec<SocketAddr>>,
}

impl surf_fetch::Network for Routed {
    fn resolve(
        &self,
        host: &str,
        port: u16,
        _: Duration,
    ) -> Result<Vec<SocketAddr>, surf_fetch::Failure> {
        let ip: IpAddr = match host {
            "a.example.test" => "203.0.113.5".parse().unwrap(),
            "localhost" => "127.0.0.1".parse().unwrap(),
            _ => {
                return Err(surf_fetch::Failure::Unreachable(format!(
                    "the test does not resolve {host}"
                )));
            }
        };
        Ok(vec![SocketAddr::new(ip, port)])
    }

    fn connect(&self, address: &SocketAddr, timeout: Duration) -> std::io::Result<TcpStream> {
        self.dialed.lock().unwrap().push(*address);
        if address.ip() == "203.0.113.5".parse::<IpAddr>().unwrap() {
            return TcpStream::connect_timeout(
                &SocketAddr::from(([127, 0, 0, 1], self.tls_port)),
                timeout,
            );
        }
        TcpStream::connect_timeout(address, timeout)
    }
}

/// Item 1 of the adversary's brief, the redirect path: from a public page, an `https:` link to a
/// public server that answers 302 to a local `http:` or `https:` address is not followed there:
/// the load fails `ForbiddenDestination`, and nothing connects to the local listener.
#[test]
fn adversary_https_a_public_https_link_redirected_to_a_local_address_is_not_followed_there() {
    let held = TcpListener::bind("127.0.0.1:0").expect("a port on 127.0.0.1");
    held.set_nonblocking(true).unwrap();
    let p = held.local_addr().unwrap().port();
    let locations = [
        format!("http://127.0.0.1:{p}/"),
        format!("https://127.0.0.1:{p}/"),
        format!("https://localhost:{p}/"),
        format!("http://[::ffff:127.0.0.1]:{p}/"),
    ];
    let server = tls(
        "ok-wildcard",
        locations
            .iter()
            .enumerate()
            .map(|(n, location)| (format!("/{n}"), redirect(302, location)))
            .collect(),
    );
    let pem = root_pem("ok-wildcard");
    let public = "http://93.184.216.34/start.html";
    for (n, location) in locations.iter().enumerate() {
        let link = format!("https://a.example.test/{n}");
        let page = link_page(&link);
        let network = std::rc::Rc::new(Routed {
            tls_port: server.port(),
            dialed: Mutex::new(Vec::new()),
        });
        let through = std::rc::Rc::clone(&network);
        let start = public.to_string();
        let transport = move |request: &surf::RequestStarted| {
            if request.url.0 == start {
                surf::Answer::Response {
                    url: request.url.0.clone(),
                    method: request.method,
                    status: 200,
                    media_type: HTML.to_string(),
                    body: page.clone().into_bytes(),
                }
            } else {
                surf_session::fetch_through(request, through.as_ref())
            }
        };
        let mut session = Session::new(ID, transport).unwrap();
        let (status, text) = call(
            &mut session,
            &request("open", None, None, &[("url", public), ("anchors", &pem)]),
        );
        assert_eq!(status, 0, "{text}");
        let token = opened(&text).token;
        let (_, text, typed) = ask(&mut session, &token, "follow", &[("handle", "l1")]);
        match &typed.page {
            Some(Page::Failed { reason, .. }) => {
                assert_eq!(reason, "ForbiddenDestination", "{location}: {text}");
            }
            other => panic!("{location}: not a failed load: {other:?}\n{text}"),
        }
        assert_eq!(
            *network.dialed.lock().unwrap(),
            vec![SocketAddr::from(([203, 0, 113, 5], 443))],
            "{location}"
        );
        assert_eq!(
            held.accept().map(|_| ()).map_err(|error| error.kind()),
            Err(std::io::ErrorKind::WouldBlock),
            "{location}: the local listener was connected to"
        );
    }
    assert_eq!(server.accepted(), locations.len());
}
