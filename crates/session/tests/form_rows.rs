//! `story:agent-form-port`: the parser step now adds a form row per form and an option row per
//! option, so the bound on the element entries of one request counts `surf.document.FormAdded`
//! and `surf.document.OptionAdded` with `surf.document.ElementAdded`: a page with thousands of
//! options keeps the request within `surfd`'s bounds and the session open. A real
//! `surf-session` under an in-process `surfd` against a page server on 127.0.0.1.

use crate::common;

use common::{PageServer, TestDir, surfd_in_process, wire_as, wire_open};
use surf_session::ELEMENT_ENTRIES_MAX;
use surfd::State;

#[test]
fn a_page_with_more_options_than_a_request_carries_keeps_the_session_open() {
    const OPTIONS: usize = 2_000;
    let mut body = b"<!doctype html><title>Options</title><form><select name=s>".to_vec();
    for _ in 0..OPTIONS {
        body.extend_from_slice(b"<option>x");
    }
    body.extend_from_slice(b"</select></form>");
    let dir = TestDir::new();
    let pages = PageServer::with_body(body);
    let daemon = surfd_in_process(&dir, |_| {});
    let session = wire_open(&dir, None);
    let answer = wire_as(&dir, &session, "go", &[("url", &pages.url("/t"))]);
    assert_eq!(answer.status, 0);
    let open = daemon
        .sessions()
        .get(&session.id)
        .map(|listed| listed.state)
        == Some(State::Open);
    assert!(open, "the session is open");
    let log = daemon.session_entries(&session.id).expect("listed");
    assert_eq!(log.dropped, 0, "no entry dropped by the log's bounds");
    let count = |name: &str| {
        log.entries
            .iter()
            .filter(|entry| entry.text.starts_with(&format!("event {name} ")))
            .count()
    };
    let rows = count("surf.document.FormAdded")
        + count("surf.document.ElementAdded")
        + count("surf.document.OptionAdded");
    assert_eq!(rows, ELEMENT_ENTRIES_MAX);
    assert_eq!(count("surf.document.FormAdded"), 1);
    assert_eq!(count("surf.document.ElementAdded"), 1);
    let left_out: Vec<&str> = log
        .entries
        .iter()
        .filter(|entry| entry.text.starts_with("event surf.agent.EventsLeftOut "))
        .map(|entry| entry.text.as_str())
        .collect();
    // One form, one select and its options, less the 256 kept.
    assert_eq!(
        left_out,
        [format!(
            "event surf.agent.EventsLeftOut count=\"{}\"\n",
            1 + 1 + OPTIONS - ELEMENT_ENTRIES_MAX
        )]
    );
}
