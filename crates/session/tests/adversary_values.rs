//! Adversary cases of `story:values-and-labels-before-fill`: what a `go` writes for the events
//! the story changed, held to the example of `docs/view-stream.md`, and the drop of the kept text
//! on a request that is neither `look` nor `go`.

#![forbid(unsafe_code)]

use crate::common;

use common::{PageServer, opened, request};
use surf_session::Session;

const ID: &str = "0b0e4f5a-6c7d-4e8f-9a0b-1c2d3e4f5a6b";

/// The field names of the event `name` in the `GET /api/sessions/<id>` example of
/// `docs/view-stream.md`, in the order written: the keys of its `fields` object.
fn example_fields(name: &str) -> Vec<String> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/view-stream.md");
    let text = std::fs::read_to_string(path).expect("docs/view-stream.md");
    let marker = format!("\"name\":\"{name}\",\"unmapped\":false,\"fields\":{{");
    let start = text
        .find(&marker)
        .unwrap_or_else(|| panic!("no {name} in the example"))
        + marker.len();
    let body = &text[start..];
    let body = &body[..body.find('}').expect("the end of the fields")];
    body.split(',')
        .map(|pair| {
            pair.split_once(':')
                .expect("a key and a value")
                .0
                .trim_matches('"')
                .to_string()
        })
        .collect()
}

/// The field names of an event line, in the order written.
fn line_fields(line: &str) -> Vec<String> {
    let rest = line
        .trim_end()
        .strip_prefix("event ")
        .expect("an event line");
    let (_, mut rest) = rest.split_once(' ').unwrap_or((rest, ""));
    let mut names = Vec::new();
    while !rest.is_empty() {
        let (name, after) = rest.split_once('=').expect("a field");
        names.push(name.to_string());
        let next = match after.strip_prefix('"') {
            Some(inside) => &after[inside.find('"').expect("a closing quote") + 2..],
            None => after,
        };
        rest = next.strip_prefix(' ').unwrap_or(next);
    }
    names
}

/// The event entry `name` of the entries of a `go` the session answered.
fn go_event(name: &str) -> String {
    let pages = PageServer::start();
    let mut session = Session::new(ID, surf_session::fetch).unwrap();
    let reply = session.answer(&request("open", None, None, &[]));
    assert_eq!(reply.status, 0);
    let token = opened(&reply.to_link().text).token;
    let reply = session.answer(&request(
        "go",
        Some(ID),
        Some(&token),
        &[("url", &pages.url("/a.html"))],
    ));
    assert_eq!(reply.status, 0, "{}", reply.to_link().text);
    let prefix = format!("event {name} ");
    reply
        .entries
        .iter()
        .find(|entry| entry.text.starts_with(&prefix))
        .unwrap_or_else(|| panic!("no {name} entry"))
        .text
        .clone()
}

/// `docs/view-stream.md` shows the stream of an `open` and a `go`, "An absent optional field
/// (...) is `null`". The story added `handle` and `label` to `NavigationRequested`, and a `go`
/// now writes both, absent: the example must list them as the stream does.
#[test]
fn a_gos_navigation_requested_has_the_fields_of_the_view_stream_example() {
    let line = go_event("surf.agent.NavigationRequested");
    assert_eq!(
        line_fields(&line),
        example_fields("surf.agent.NavigationRequested"),
        "docs/view-stream.md and the entry a go writes: {line}"
    );
}

/// The same drift for `ActionRecorded`, whose `handle` and `label` came before this story.
#[test]
fn a_gos_action_recorded_has_the_fields_of_the_view_stream_example() {
    let line = go_event("surf.agent.ActionRecorded");
    let example = example_fields("surf.agent.ActionRecorded");
    assert_eq!(
        line_fields(&line),
        example,
        "docs/view-stream.md and the entry a go writes: {line}"
    );
}

/// "drops it with every request but `look` (a `go`, a `close`, ...)": the story's test drops
/// it with a `go` only. A `close` after a `look` keeps no text.
#[test]
fn kept_text_is_dropped_by_close() {
    let pages = PageServer::start();
    let mut session = Session::new(ID, surf_session::fetch).unwrap();
    let reply = session.answer(&request("open", None, None, &[("url", &pages.url("/0"))]));
    assert_eq!(reply.status, 0);
    let token = opened(&reply.to_link().text).token;
    let reply = session.answer(&request(
        "look",
        Some(ID),
        Some(&token),
        &[("form", "markdown")],
    ));
    assert_eq!(reply.status, 0, "{}", reply.to_link().text);
    assert!(session.kept_text_lines() > 0, "after look");
    let reply = session.answer(&request("close", Some(ID), Some(&token), &[]));
    assert_eq!(reply.status, 0, "{}", reply.to_link().text);
    assert_eq!(session.kept_text_lines(), 0, "after close");
}
