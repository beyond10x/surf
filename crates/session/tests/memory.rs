//! What the session core keeps across commands: after each answer, no response body, no event
//! and no binding invocation of the system's record, however many pages it loaded. Counted
//! through the core's read-only `Session::held`, not the process's resident size, so the case
//! does not depend on the allocator or on the machine's load.

#![forbid(unsafe_code)]

use crate::common;

use common::{PageServer, opened, request};
use surf_session::Session;

const ID: &str = "0b0e4f5a-6c7d-4e8f-9a0b-1c2d3e4f5a6b";

#[test]
fn bodies_of_answered_commands_are_not_kept() {
    // A body of 4 KiB, loaded by `open <url>` and 5 `go`s: kept, it would be 5 copies per load
    // (three events and two binding invocations each carry it). The counts of `held` are not
    // zero at the first command that keeps one, whatever the body's size.
    let mut body = b"<!doctype html><p>".to_vec();
    body.resize(4 * 1024, b'x');
    let pages = PageServer::with_body(body);
    let mut session = Session::new(ID, surf_session::fetch).unwrap();

    let reply = session.answer(&request("open", None, None, &[("url", &pages.url("/0"))]));
    assert_eq!(reply.status, 0);
    let token = opened(&reply.to_link().text).token;
    let held = session.held();
    assert_eq!(
        (held.events, held.invocations, held.body_bytes),
        (0, 0, 0),
        "after open"
    );

    for n in 1..=5 {
        let url = pages.url(&format!("/{n}"));
        let reply = session.answer(&request("go", Some(ID), Some(&token), &[("url", &url)]));
        let text = reply.to_link().text;
        assert_eq!(reply.status, 0, "{text}");
        assert!(
            text.lines()
                .any(|line| line == format!("page 200 \"text/html\" \"{url}\"")),
            "go {n}: {text}"
        );
        let held = session.held();
        assert_eq!(
            (held.events, held.invocations, held.body_bytes),
            (0, 0, 0),
            "after go {n}"
        );
    }

    let text = session
        .answer(&request("look", Some(ID), Some(&token), &[]))
        .to_link()
        .text;
    let url = pages.url("/5");
    assert!(
        text.lines()
            .any(|line| line == format!("page 200 \"text/html\" \"{url}\"")),
        "{text}"
    );
    // `open <url>` is two actions (`open`, then `go`), and 5 more `go`: 7, cut to the last five.
    assert!(text.contains("path 5 of 7\n"), "{text}");
}

/// Story values-and-labels-before-fill, acceptance 5: the text a `look` kept is dropped by the
/// next request that is not `look`, here a `go` that asks for no text.
#[test]
fn kept_text_is_dropped_on_a_request_other_than_look() {
    let pages = PageServer::start();
    let mut session = Session::new(ID, surf_session::fetch).unwrap();
    let reply = session.answer(&request("open", None, None, &[("url", &pages.url("/0"))]));
    assert_eq!(reply.status, 0);
    let token = opened(&reply.to_link().text).token;
    assert_eq!(
        session.kept_text_lines(),
        0,
        "before any text was asked for"
    );

    let reply = session.answer(&request(
        "look",
        Some(ID),
        Some(&token),
        &[("form", "tree")],
    ));
    assert_eq!(reply.status, 0, "{}", reply.to_link().text);
    assert!(session.kept_text_lines() > 0, "after look");

    let reply = session.answer(&request(
        "go",
        Some(ID),
        Some(&token),
        &[("url", &pages.url("/1"))],
    ));
    assert_eq!(reply.status, 0, "{}", reply.to_link().text);
    assert_eq!(session.kept_text_lines(), 0, "after go");
}
