//! What the session core keeps across commands: after each answer, no response body, no event
//! and no binding invocation of the system's record, however many pages it loaded. Counted
//! through the core's read-only `Session::held`, not the process's resident size, so the case
//! does not depend on the allocator or on the machine's load.

#![forbid(unsafe_code)]

mod common;

use common::{PageServer, opened, request};
use surf_session::Session;

const ID: &str = "0b0e4f5a-6c7d-4e8f-9a0b-1c2d3e4f5a6b";

#[test]
fn bodies_of_answered_commands_are_not_kept() {
    // A body of 256 KiB, loaded 21 times: kept, it would be 5 copies per load (three events and
    // two binding invocations each carry it), 26.25 MiB in all.
    let mut body = b"<!doctype html><p>".to_vec();
    body.resize(256 * 1024, b'x');
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

    for n in 1..=20 {
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
    let url = pages.url("/20");
    assert!(
        text.lines()
            .any(|line| line == format!("page 200 \"text/html\" \"{url}\"")),
        "{text}"
    );
    // `open <url>` is two actions (`open`, then `go`), and 20 more `go`: 22.
    assert!(text.contains("path 5 of 22\n"), "{text}");
}
