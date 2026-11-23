//! Review 1 (adversary): the claim that `Browser::run` hands each started request to the
//! transport once, on the two paths the public API opens where the `Pending` check in
//! `Browser::answer` is what keeps it; and `escape` against every control character README.md
//! names.

use std::cell::Cell;
use std::rc::Rc;

use surf::{Answer, Browser, Method, RequestStarted, System, escape};
use surf_ports::ComponentPorts;
use surf_types::navigation::{Navigate, NavigateOutcome};
use surf_types::net::{DeliverResponse, Fetch, MediaType, RequestId, RequestState, Status};
use surf_types::primitives::Uuid;
use surf_types::shell::TabId;
use surf_types::url::Url;

fn counting() -> (impl FnMut(&RequestStarted) -> Answer, Rc<Cell<usize>>) {
    let calls = Rc::new(Cell::new(0));
    let seen = Rc::clone(&calls);
    let transport = move |request: &RequestStarted| {
        seen.set(seen.get() + 1);
        Answer::Response {
            url: request.url.0.clone(),
            method: request.method,
            status: 200,
            media_type: "text/plain".to_owned(),
            body: Vec::new(),
        }
    };
    (transport, calls)
}

/// `Browser::new` documents that the first run reads the system's log from its start. A system
/// whose request was already answered must not be fetched again.
#[test]
fn review1_a_system_whose_request_was_answered_is_not_fetched_again_by_a_new_browser() {
    let ports = ComponentPorts::new();
    let mut system = System::new(
        ports.agent,
        ports.document,
        ports.navigator,
        ports.net,
        ports.renderer,
        ports.shell,
    );
    let NavigateOutcome::Started { navigation_started } = system
        .navigator
        .navigate(Navigate {
            tab_id: TabId(Uuid("6f1c2d3e-4b5a-4c6d-8e7f-001122334455".to_owned())),
            url: Url("http://127.0.0.1/".to_owned()),
            method: Method::Get,
            body: None,
            body_type: None,
        })
        .expect("navigate");
    system.pump().expect("pump");
    system
        .net
        .deliver_response(DeliverResponse {
            request_id: navigation_started.navigation_id.clone(),
            url: Url("http://127.0.0.1/".to_owned()),
            method: Method::Get,
            status: Status(200),
            media_type: MediaType("text/plain".to_owned()),
            body: Vec::new(),
        })
        .expect("deliver");
    system.pump().expect("pump");

    let (transport, calls) = counting();
    let mut browser = Browser::new(system, transport);
    browser.run().expect("run");
    assert_eq!(
        calls.get(),
        0,
        "an answered request was handed to the transport"
    );
}

/// Two `Fetch` commands for one request, issued on the `net` port before a run: one request,
/// so one fetch.
#[test]
fn review1_two_fetches_of_one_request_reach_the_transport_once() {
    let (transport, calls) = counting();
    let mut browser = Browser::from_ports(ComponentPorts::new(), transport);
    let id = RequestId(Uuid("00000000-0000-4000-8000-000000000001".to_owned()));
    for _ in 0..2 {
        browser
            .net_mut()
            .fetch(Fetch {
                request_id: id.clone(),
                url: Url("http://127.0.0.1/".to_owned()),
                method: Method::Get,
                body: None,
                body_type: None,
            })
            .expect("fetch");
    }
    browser.run().expect("run");
    assert_eq!(calls.get(), 1);
    let requests = browser.system().net.requests().expect("requests");
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].state, RequestState::Responded);
}

/// README.md: "A backslash and every control character in it, line feeds included, are written
/// as `\x` and two lower-case hexadecimal digits per byte of their UTF-8 encoding."
#[test]
fn review1_escape_writes_every_control_character_as_hex_bytes() {
    assert_eq!(escape("\0\t\r\n\x1b\x7f"), "\\x00\\x09\\x0d\\x0a\\x1b\\x7f");
    assert_eq!(escape("\u{85}\u{9f}"), "\\xc2\\x85\\xc2\\x9f");
    assert_eq!(escape("a\\b"), "a\\x5cb");
    assert_eq!(escape("http://h/\u{e9} x"), "http://h/\\xc3\\xa9 x");
}
