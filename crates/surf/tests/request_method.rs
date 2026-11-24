//! Story agent-request-body-spec: a navigation's method, body and media type reach the transport
//! unchanged, `Browser::held` counts the request bodies, and `fetch_http` refuses a `Get` with a
//! body or a media type as `UnsupportedScheme` before a connection is made. Since the story that
//! sends a `POST` (wave 9), `fetch_http` sends a `Post` with its body and media type.

#![forbid(unsafe_code)]

use std::cell::RefCell;
use std::io::{ErrorKind, Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::rc::Rc;
use std::thread;
use std::time::Duration;

use surf::{Answer, Browser, Held, Method, RequestStarted, answer, fetch_http};
use surf_ports::ComponentPorts;
use surf_types::navigation::{Navigate, NavigateOutcome};
use surf_types::net::{MediaType, RequestId, RequestState};
use surf_types::primitives::Uuid;
use surf_types::shell::TabId;
use surf_types::url::Url;

/// The media type of a form's body.
const FORM: &str = "application/x-www-form-urlencoded";

type Recorded = Rc<RefCell<Vec<RequestStarted>>>;

/// A transport that records every request it is handed and leaves it unanswered.
fn recording() -> (impl FnMut(&RequestStarted) -> Answer, Recorded) {
    let recorded: Recorded = Rc::default();
    let seen = Rc::clone(&recorded);
    let transport = move |request: &RequestStarted| {
        seen.borrow_mut().push(request.clone());
        Answer::Unanswered
    };
    (transport, recorded)
}

/// `Navigate` of a tab no shell holds to `http://127.0.0.1/form` with `Post`, the body `a=1` and
/// the media type of a form.
fn post() -> Navigate {
    post_to("http://127.0.0.1/form")
}

/// `Navigate` of a tab no shell holds to `address` with `Post`, the body `a=1` and the media type
/// of a form.
fn post_to(address: &str) -> Navigate {
    Navigate {
        tab_id: TabId(Uuid("6f1c2d3e-4b5a-4c6d-8e7f-001122334466".to_owned())),
        url: Url(address.to_owned()),
        method: Method::Post,
        body: Some(b"a=1".to_vec()),
        body_type: Some(MediaType(FORM.to_owned())),
        initiator: None,
    }
}

/// A started request for `address`.
fn request(
    address: &str,
    method: Method,
    body: Option<&[u8]>,
    body_type: Option<&str>,
) -> RequestStarted {
    RequestStarted {
        request_id: RequestId(Uuid("00000000-0000-4000-8000-000000000071".to_owned())),
        url: Url(address.to_owned()),
        method,
        body: body.map(<[u8]>::to_vec),
        body_type: body_type.map(|media_type| MediaType(media_type.to_owned())),
        initiator: None,
    }
}

/// Limits that end a fetch within seconds, so that a fetch that does connect to the silent
/// listener below fails the test instead of hanging it.
fn short() -> surf_fetch::Limits {
    surf_fetch::Limits {
        connect_timeout: Duration::from_secs(2),
        read_timeout: Duration::from_secs(2),
        ..surf_fetch::Limits::default()
    }
}

/// A listener on 127.0.0.1 port 0 that never blocks: `accept` answers `WouldBlock` as long as
/// nobody connected.
fn silent_listener() -> (TcpListener, String) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    listener.set_nonblocking(true).expect("non-blocking");
    let address = format!("http://{}/form", listener.local_addr().expect("address"));
    (listener, address)
}

/// `fetch_http` refuses `request` with `UnsupportedScheme` naming `method`, and nothing connected
/// to the listener.
fn assert_refused_without_connecting(
    listener: &TcpListener,
    request: &RequestStarted,
    method: &str,
) {
    match fetch_http(request, &short(), None) {
        Err(surf_fetch::Failure::UnsupportedScheme(detail)) => {
            assert!(detail.contains(method), "{detail:?}");
        }
        other => panic!("{request:?} answered {other:?}"),
    }
    match listener.accept() {
        Err(error) => assert_eq!(error.kind(), ErrorKind::WouldBlock, "{error}"),
        Ok((_, peer)) => panic!("{request:?}: {peer} connected"),
    }
}

#[test]
fn load_hands_get_without_body_to_the_transport() {
    let (transport, recorded) = recording();
    let mut browser = Browser::from_ports(ComponentPorts::new(), transport);
    let report = browser.load("http://127.0.0.1/").expect("load");
    let recorded = recorded.borrow();
    assert_eq!(recorded.len(), 1, "{recorded:?}");
    assert_eq!(recorded[0].request_id, report.navigation_id);
    assert_eq!(recorded[0].url, Url("http://127.0.0.1/".to_owned()));
    assert_eq!(recorded[0].method, Method::Get);
    assert_eq!(recorded[0].body, None);
    assert_eq!(recorded[0].body_type, None);
}

#[test]
fn post_navigation_reaches_the_transport() {
    let (transport, recorded) = recording();
    let mut browser = Browser::from_ports(ComponentPorts::new(), transport);
    let NavigateOutcome::Started { navigation_started } =
        browser.navigator_mut().navigate(post()).expect("navigate");
    browser.run().expect("run");
    let recorded = recorded.borrow();
    assert_eq!(
        *recorded,
        vec![RequestStarted {
            request_id: navigation_started.navigation_id,
            url: Url("http://127.0.0.1/form".to_owned()),
            method: Method::Post,
            body: Some(b"a=1".to_vec()),
            body_type: Some(MediaType(FORM.to_owned())),
            initiator: None,
        }]
    );
}

/// The head and the `Content-Length` bytes of the body of the one request `listener` accepts,
/// read before the server answers 200 with `ok`.
fn serve_one(listener: TcpListener) -> thread::JoinHandle<(String, Vec<u8>)> {
    thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .expect("read timeout");
        let mut bytes = Vec::new();
        let mut buf = [0u8; 4096];
        let end = loop {
            if let Some(end) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                break end + 4;
            }
            match stream.read(&mut buf) {
                Ok(0) | Err(_) => panic!("the request ended inside its head"),
                Ok(n) => bytes.extend_from_slice(&buf[..n]),
            }
        };
        let body = bytes.split_off(end);
        let head = String::from_utf8(bytes).expect("the head is UTF-8");
        let length: usize = head
            .lines()
            .find_map(|line| line.strip_prefix("Content-Length: "))
            .map_or(0, |value| value.parse().expect("a length"));
        let mut body = body;
        while body.len() < length {
            match stream.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => body.extend_from_slice(&buf[..n]),
            }
        }
        let _ = stream.write_all(
            b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 2\r\n\r\nok",
        );
        let _ = stream.shutdown(Shutdown::Write);
        (head, body)
    })
}

#[test]
fn post_navigation_is_sent_by_fetch_http() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let local = listener.local_addr().expect("address");
    let address = format!("http://{local}/form");
    let server = serve_one(listener);
    let transport = |request: &RequestStarted| answer(fetch_http(request, &short(), None));
    let mut browser = Browser::from_ports(ComponentPorts::new(), transport);
    let NavigateOutcome::Started { navigation_started } = browser
        .navigator_mut()
        .navigate(post_to(&address))
        .expect("navigate");
    browser.run().expect("run");
    // A transport that sent nothing would leave the server in `accept`: this connection, queued
    // after any the fetch made and closed at once, ends it with an empty head instead.
    let _ = TcpStream::connect(local);
    let (head, body) = server.join().expect("the server read one request");
    assert!(head.starts_with("POST /form HTTP/1.1\r\n"), "{head:?}");
    assert!(
        head.contains(&format!("\r\nContent-Type: {FORM}\r\n")),
        "{head:?}"
    );
    assert!(head.contains("\r\nContent-Length: 3\r\n"), "{head:?}");
    assert_eq!(body, b"a=1");
    let requests = browser.system().net.requests().expect("requests");
    assert_eq!(requests.len(), 1, "{requests:?}");
    assert_eq!(requests[0].request_id, navigation_started.navigation_id);
    assert_eq!(requests[0].state, RequestState::Responded);
}

#[test]
fn fetch_http_refuses_get_with_body() {
    let (listener, address) = silent_listener();
    let with_body = request(&address, Method::Get, Some(b"a=1"), Some(FORM));
    assert_refused_without_connecting(&listener, &with_body, "GET");
    let body_only = request(&address, Method::Get, Some(b""), None);
    assert_refused_without_connecting(&listener, &body_only, "GET");
    let media_type_only = request(&address, Method::Get, None, Some(FORM));
    assert_refused_without_connecting(&listener, &media_type_only, "GET");
}

#[test]
fn held_counts_a_request_body() {
    let (transport, _recorded) = recording();
    let mut browser = Browser::from_ports(ComponentPorts::new(), transport);
    browser.navigator_mut().navigate(post()).expect("navigate");
    browser.run().expect("run");
    // `a=1` in `NavigationStarted`, in `RequestStarted` and in the invocation of `Fetch`.
    let held = browser.held();
    assert_eq!(held.body_bytes, 9, "{held:?}");
    browser.forget_delivered();
    assert_eq!(
        browser.held(),
        Held {
            events: 0,
            invocations: 0,
            body_bytes: 0
        }
    );
}
