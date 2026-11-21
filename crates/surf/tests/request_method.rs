//! Story agent-request-body-spec: a navigation's method, body and media type reach the transport
//! unchanged, `Browser::held` counts the request bodies, and `fetch_http` fetches only a `Get`
//! without a body and without a media type: any other request ends as `UnsupportedScheme` before a
//! connection is made, until `agent-fetch-post`.

#![forbid(unsafe_code)]

use std::cell::RefCell;
use std::io::ErrorKind;
use std::net::TcpListener;
use std::rc::Rc;
use std::time::Duration;

use surf::{Answer, Browser, Held, Method, RequestStarted, fetch_http};
use surf_ports::ComponentPorts;
use surf_types::navigation::{Navigate, NavigateOutcome};
use surf_types::net::{MediaType, RequestId};
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
    Navigate {
        tab_id: TabId(Uuid("6f1c2d3e-4b5a-4c6d-8e7f-001122334466".to_owned())),
        url: Url("http://127.0.0.1/form".to_owned()),
        method: Method::Post,
        body: Some(b"a=1".to_vec()),
        body_type: Some(MediaType(FORM.to_owned())),
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
    match fetch_http(request, &short()) {
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
        }]
    );
}

#[test]
fn fetch_http_refuses_post_without_connecting() {
    let (listener, address) = silent_listener();
    let started = request(&address, Method::Post, Some(b"a=1"), Some(FORM));
    assert_refused_without_connecting(&listener, &started, "POST");
    let empty = request(&address, Method::Post, None, None);
    assert_refused_without_connecting(&listener, &empty, "POST");
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
