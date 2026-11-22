//! Cases 4 to 18 of `story:agent-follow-links`: the `now` block of the current document, `follow
//! <handle>`, its refusals, the title on the `page` line and in the status record. Cases 4 to 16
//! drive the session core against a page server on `127.0.0.1` the test starts ([`Site`]);
//! cases 17 and 18 run `surfd` in the test process (decision D29) with the built `surf-session`.

#![forbid(unsafe_code)]

use crate::common;
use crate::core::{Fetch, HTML, ID, Site, holder, open_at};

use common::{PageServer, TestDir, golden, placeholders, surfd_in_process, wire_as, wire_open};
use surf_agent::wire::Request;
use surf_answer::{Answer, Note, Target, parse, render};
use surf_session::Session;
use surfd::link::{EntryKind, Quoted};

/// A page with each kind of link case 4 names: a relative one, one to another origin, one inside
/// a `hidden` element, one inside `aria-hidden="true"`, an `a` without `href`, and one whose
/// target does not resolve.
const LINKS: &str = "<!doctype html><html><head><title>Links</title></head><body>\
<p><a href=\"next.html\">Next page</a></p>\
<p><a href=\"http://other.example/x?y#z\">Elsewhere</a></p>\
<div hidden><p><a href=\"/secret\">Members</a></p></div>\
<div aria-hidden=\"true\"><a href=\"/aria\">Behind aria</a></div>\
<p><a name=\"anchor\">No target</a></p>\
<p><a href=\"http://[\">Broken</a></p>\
</body></html>";

/// The page `next.html` of [`LINKS`]: a new document, whose handles start at 1 again.
const NEXT: &str = "<!doctype html><title>Next</title><a href=\"index.html\">Back</a>";

/// The value cases 11 to 13 put into a page: a line feed, a double quote and U+202E.
const FORGING: &str = "a&#10;b\"c&#x202e;d";

/// [`FORGING`] as an attribute value of the page, the double quote as a character reference.
const FORGING_ATTRIBUTE: &str = "a&#10;b&quot;c&#x202e;d";

/// The same value as the page's text gives it to the title and the label: the accessible name
/// and `document.title` strip and collapse ASCII whitespace, so the line feed is one space.
const FORGING_COLLAPSED: &str = "a b\"c\u{202e}d";

/// The same value in an attribute, kept as written.
const FORGING_RAW: &str = "a\nb\"c\u{202e}d";

fn links_site() -> Site {
    Site::start(vec![
        ("/links/index.html", HTML, LINKS.into()),
        ("/links/next.html", HTML, NEXT.into()),
    ])
}

/// The reply to `request`: its exit status, its lines and its typed answer.
fn ask(session: &mut Session<Fetch>, request: &Request) -> (u8, String, Answer) {
    let reply = session.answer(request);
    let link = reply.to_link();
    (link.status, link.text, reply.answer)
}

/// The lines of the `now` block of `text`, its header first.
fn now_lines(text: &str) -> Vec<&str> {
    let mut lines = text.lines().skip_while(|line| !line.starts_with("now "));
    let Some(header) = lines.next() else {
        return Vec::new();
    };
    let mut block = vec![header];
    block.extend(lines.take_while(|line| line.starts_with("  ")));
    block
}

/// `parse` of `text` gives `typed`, and `render` of it gives `text` byte for byte (case 16).
fn round_trip(text: &str, typed: &Answer) {
    let parsed = parse(text).unwrap_or_else(|error| panic!("{error}: {text}"));
    assert_eq!(&parsed, typed, "{text}");
    assert_eq!(render(&parsed).unwrap(), text);
}

#[test]
fn case_04_now_lists_the_links_as_its_golden_text() {
    let site = links_site();
    let (mut session, token, _) = open_at(&site.url("/links/index.html"), &[]);
    let (status, text, typed) = ask(&mut session, &holder("look", &token, &[]));
    assert_eq!(status, 0, "{text}");
    assert_eq!(
        placeholders(&text, ID, &token, site_port(&site)),
        golden("look-links.txt")
    );
    round_trip(&text, &typed);
}

fn site_port(site: &Site) -> u16 {
    site.url("")
        .rsplit(':')
        .next()
        .and_then(|port| port.parse().ok())
        .expect("a port")
}

#[test]
fn case_05_follow_answers_the_target_page() {
    let site = links_site();
    let (mut session, token, _) = open_at(&site.url("/links/index.html"), &[]);
    let (status, text, typed) = ask(&mut session, &holder("follow", &token, &[("handle", "l1")]));
    assert_eq!(status, 0, "{text}");
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines[1], "did follow \"l1\"");
    assert_eq!(
        lines[2],
        format!(
            "page 200 \"text/html\" {} \"Next\"",
            surf_answer::quote(&site.url("/links/next.html"))
        )
    );
    assert!(lines.contains(&"  3 follow l1 \"Next page\""), "{text}");
    assert_eq!(
        now_lines(&text),
        vec![
            "now 1 of 1",
            "  l1 link ? \"Back\" -> \"/links/index.html\""
        ]
    );
    round_trip(&text, &typed);
}

#[test]
fn case_06_an_unknown_handle_is_refused() {
    let site = links_site();
    let (mut session, token, _) = open_at(&site.url("/links/index.html"), &[]);
    let (_, before, _) = ask(&mut session, &holder("look", &token, &[]));
    for handle in ["l9", "b1", "f1", "l0", "x"] {
        let (status, text, _) = ask(
            &mut session,
            &holder("follow", &token, &[("handle", handle)]),
        );
        assert_eq!(status, 1, "{handle}: {text}");
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines[1], "did refused unknown-handle", "{handle}");
        assert_eq!(
            &lines[2..],
            &before.lines().collect::<Vec<_>>()[1..],
            "{handle}"
        );
    }
    // With no current document there is no handle at all.
    let mut fresh = Session::new(ID, surf_session::fetch as Fetch).unwrap();
    let (_, text, _) = ask(&mut fresh, &common::request("open", None, None, &[]));
    let token = common::opened(&text).token;
    let (status, text, _) = ask(&mut fresh, &holder("follow", &token, &[("handle", "l1")]));
    assert_eq!(status, 1);
    assert_eq!(text.lines().nth(1), Some("did refused unknown-handle"));
}

#[test]
fn case_07_a_hidden_link_is_refused() {
    let site = links_site();
    let (mut session, token, _) = open_at(&site.url("/links/index.html"), &[]);
    let (status, text, _) = ask(&mut session, &holder("follow", &token, &[("handle", "l3")]));
    assert_eq!(status, 1, "{text}");
    assert_eq!(text.lines().nth(1), Some("did refused hidden"));
    // `aria-hidden` does not make a link hidden: l4 is followed (the server answers 404).
    let (status, text, _) = ask(&mut session, &holder("follow", &token, &[("handle", "l4")]));
    assert_eq!(status, 0, "{text}");
    assert_eq!(text.lines().nth(1), Some("did follow \"l4\""));
}

#[test]
fn case_08_an_unresolved_target_is_refused() {
    let site = links_site();
    let (mut session, token, _) = open_at(&site.url("/links/index.html"), &[]);
    let (status, text, _) = ask(&mut session, &holder("follow", &token, &[("handle", "l5")]));
    assert_eq!(status, 1, "{text}");
    assert_eq!(text.lines().nth(1), Some("did refused unresolved"));
    assert!(text.contains("\n  2 go "), "nothing recorded: {text}");
    assert!(!text.contains("\n  3 "), "nothing recorded: {text}");
}

#[test]
fn case_09_two_runs_give_the_same_handles() {
    let site = links_site();
    let run = || {
        let (mut session, token, _) = open_at(&site.url("/links/index.html"), &[]);
        let (_, first, _) = ask(&mut session, &holder("look", &token, &[]));
        let (_, second, _) = ask(&mut session, &holder("follow", &token, &[("handle", "l1")]));
        let (_, third, _) = ask(&mut session, &holder("follow", &token, &[("handle", "l1")]));
        [first, second, third].map(|text| {
            now_lines(&text)
                .into_iter()
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
    };
    let (one, two) = (run(), run());
    assert_eq!(one, two);
    assert_eq!(one[0].len(), 6, "{one:?}");
    // Back on the first page, a new document: its handles start at 1 again.
    assert_eq!(one[2], one[0]);
}

#[test]
fn case_10_a_long_label_is_cut_at_60() {
    let label = "abcdefghij".repeat(20);
    let page = format!("<!doctype html><a href=\"/t\">{label}</a>");
    let site = Site::start(vec![("/", HTML, page.into_bytes())]);
    let (mut session, token, _) = open_at(&site.url("/"), &[]);
    let (_, text, typed) = ask(&mut session, &holder("look", &token, &[]));
    let cut = format!("\"{}\"...", &label[..60]);
    assert_eq!(now_lines(&text)[1], format!("  l1 link ? {cut} -> \"/t\""));
    round_trip(&text, &typed);
    let (_, text, typed) = ask(&mut session, &holder("follow", &token, &[("handle", "l1")]));
    assert!(text.contains(&format!("\n  3 follow l1 {cut}\n")), "{text}");
    round_trip(&text, &typed);
}

/// A session at a page whose `body` is `body` and whose title is `title`, after `look`: its
/// lines and typed answer, and the same of a page with plain values in the same places.
fn forged_and_plain(title: &str, body: &str, plain_body: &str) -> [(String, Answer); 2] {
    let forged = format!("<!doctype html><title>{title}</title>{body}");
    let plain = format!("<!doctype html><title>plain</title>{plain_body}");
    let site = Site::start(vec![
        ("/forged", HTML, forged.into_bytes()),
        ("/plain", HTML, plain.into_bytes()),
    ]);
    [site.url("/forged"), site.url("/plain")].map(|url| {
        let (mut session, token, _) = open_at(&url, &[]);
        let (status, text, typed) = ask(&mut session, &holder("look", &token, &[]));
        assert_eq!(status, 0, "{text}");
        (text, typed)
    })
}

/// No line was added by the forged value, every byte is printable ASCII or a line feed, and the
/// answer reads back (case 16).
fn stays_in_its_field(forged: &(String, Answer), plain: &(String, Answer)) {
    assert_eq!(
        forged.0.lines().count(),
        plain.0.lines().count(),
        "{}",
        forged.0
    );
    assert!(
        forged
            .0
            .bytes()
            .all(|b| b == b'\n' || (0x20..=0x7e).contains(&b))
    );
    round_trip(&forged.0, &forged.1);
}

#[test]
fn case_11_a_forged_title_stays_in_its_field() {
    let link = "<a href=\"/x\">x</a>";
    let [forged, plain] = forged_and_plain(FORGING, link, link);
    stays_in_its_field(&forged, &plain);
    let page = forged.0.lines().nth(1).unwrap();
    assert!(
        page.ends_with(&format!(" {}", surf_answer::quote(FORGING_COLLAPSED))),
        "{page}"
    );
    assert!(page.ends_with(" \"a b\\x22c\\xe2\\x80\\xaed\""), "{page}");
}

#[test]
fn case_12_a_forged_label_stays_in_its_field_on_the_link_and_path_lines() {
    let site = Site::start(vec![(
        "/",
        HTML,
        format!("<!doctype html><a href=\"/next\">{FORGING}</a>").into_bytes(),
    )]);
    let (mut session, token, _) = open_at(&site.url("/"), &[]);
    let (_, text, typed) = ask(&mut session, &holder("look", &token, &[]));
    let quoted = surf_answer::quote(FORGING_COLLAPSED);
    assert_eq!(
        now_lines(&text),
        vec![
            "now 1 of 1".to_string(),
            format!("  l1 link ? {quoted} -> \"/next\"")
        ]
    );
    round_trip(&text, &typed);
    let (status, text, typed) = ask(&mut session, &holder("follow", &token, &[("handle", "l1")]));
    assert_eq!(status, 0, "{text}");
    assert!(
        text.contains(&format!("\n  3 follow l1 {quoted}\n")),
        "{text}"
    );
    assert_eq!(
        text.lines().count(),
        7,
        "head, did, page, path and 3 lines: {text}"
    );
    round_trip(&text, &typed);
}

#[test]
fn case_13_a_forged_unresolved_target_is_printed_as_written() {
    let forged = format!("<a href=\"http://[{FORGING_ATTRIBUTE}\">x</a>");
    let [forged, plain] = forged_and_plain("t", &forged, "<a href=\"http://[\">x</a>");
    stays_in_its_field(&forged, &plain);
    let line = now_lines(&forged.0)[1].to_string();
    assert_eq!(
        line,
        format!(
            "  l1 link ? \"x\" -> {} unresolved",
            surf_answer::quote(&format!("http://[{FORGING_RAW}"))
        )
    );
    assert!(
        matches!(&forged.1.now.as_ref().unwrap().elements[0].target, Target::Unresolved(target) if target.kept == format!("http://[{FORGING_RAW}"))
    );
}

#[test]
fn case_14_a_forged_href_resolves_to_a_percent_encoded_path() {
    let forged = format!("<a href=\"/p{FORGING_ATTRIBUTE}q\">x</a>");
    let [forged, plain] = forged_and_plain("t", &forged, "<a href=\"/pq\">x</a>");
    stays_in_its_field(&forged, &plain);
    assert_eq!(
        now_lines(&forged.0)[1],
        "  l1 link ? \"x\" -> \"/pab%22c%E2%80%AEdq\""
    );
}

#[test]
fn case_15_a_note_parser_line_only_when_parts_are_not_yet_parsed() {
    let site = links_site();
    let (mut session, token, text) = open_at(&site.url("/links/index.html"), &[]);
    let (_, look, _) = ask(&mut session, &holder("look", &token, &[]));
    let parts = surf::NOT_YET_PARSED.join(",");
    for text in [&text, &look] {
        let notes: Vec<&str> = text
            .lines()
            .filter(|line| line.starts_with("note parser: "))
            .collect();
        if surf::NOT_YET_PARSED.is_empty() {
            assert!(notes.is_empty(), "{text}");
        } else {
            assert_eq!(
                notes,
                vec![format!("note parser: {parts}").as_str()],
                "{text}"
            );
        }
    }
}

#[test]
fn case_16_answers_read_back_to_their_typed_value() {
    // Cases 4, 5, 10, 11, 12 and 13 hold the round trip of each of their answers; here the one
    // of an answer with every block this story writes: a title, a `follow` path line and a `now`
    // block with each kind of target.
    let site = links_site();
    let (mut session, token, _) = open_at(&site.url("/links/index.html"), &[]);
    let (_, text, typed) = ask(&mut session, &holder("follow", &token, &[("handle", "l1")]));
    round_trip(&text, &typed);
    let (_, text, typed) = ask(&mut session, &holder("follow", &token, &[("handle", "l1")]));
    round_trip(&text, &typed);
    assert!(typed.notes.contains(&Note::Visibility));
    assert_eq!(typed.now.as_ref().map(|now| now.total), Some(5));
}

#[test]
fn case_17_follow_entries_hold_handle_and_label() {
    let dir = TestDir::new();
    let pages = PageServer::with_body(
        b"<!doctype html><title>T</title><a href=\"next\">Next page</a>".to_vec(),
    );
    let daemon = surfd_in_process(&dir, |_| {});
    let session = wire_open(&dir, Some(&pages.url("/a")));
    let answer = wire_as(&dir, &session, "follow", &[("handle", "l1")]);
    assert_eq!(answer.status, 0, "{answer:?}");
    let entries = daemon
        .session_entries(&session.id)
        .expect("the session is listed")
        .entries;
    let start = entries
        .iter()
        .position(|entry| {
            entry.kind == EntryKind::Action && entry.text.starts_with("action 3 follow ")
        })
        .expect("the action entry of follow");
    let end = start
        + entries[start..]
            .iter()
            .position(|entry| entry.kind == EntryKind::Answer)
            .expect("the answer entry of follow");
    let between: Vec<&str> = entries[start + 1..end]
        .iter()
        .map(|entry| entry.text.as_str())
        .collect();
    let event = |name: &str| {
        between
            .iter()
            .find(|text| text.starts_with(&format!("event {name} ")))
            .unwrap_or_else(|| panic!("no {name} entry among {between:#?}"))
            .to_string()
    };
    for name in [
        "surf.agent.ActionRecorded",
        "surf.agent.NavigationRequested",
    ] {
        let line = event(name);
        assert!(!line.contains(" unmapped"), "{line}");
        assert!(line.contains(" handle=\"l1\""), "{line}");
        assert!(line.contains(" label=\"Next page\""), "{line}");
        assert!(!line.contains("label_bytes="), "{line}");
    }
}

#[test]
fn case_18_session_list_title() {
    let body = b"<!doctype html><title>Opening hours</title><p>10 to 14</p>".to_vec();
    let pages = PageServer::with_body(body);
    let mut session = Session::new(ID, surf_session::fetch as Fetch).unwrap();
    let (_, text, _) = ask(
        &mut session,
        &common::request("open", None, None, &[("url", &pages.url("/hours"))]),
    );
    let token = common::opened(&text).token;
    let reply = session.answer(&holder("go", &token, &[("url", &pages.url("/hours"))]));
    assert_eq!(
        reply.to_link().title,
        Some(Quoted::new("Opening hours")),
        "the reply's title"
    );

    let dir = TestDir::new();
    let daemon = surfd_in_process(&dir, |_| {});
    let opened = wire_open(&dir, None);
    let answer = wire_as(&dir, &opened, "go", &[("url", &pages.url("/hours"))]);
    assert_eq!(answer.status, 0, "{answer:?}");
    let listed = daemon.sessions();
    let title = listed
        .get(&opened.id)
        .and_then(|listed| listed.title.as_ref())
        .map(Quoted::as_str)
        .map(str::to_string);
    assert_eq!(title.as_deref(), Some("\"Opening hours\""));
}

/// The bound of `surf_session::TITLE_CHARS_MAX`: a title of 1,024 characters is on the `page`
/// line and in the record, one of 1,025 in neither.
#[test]
fn a_title_over_its_bound_is_left_out() {
    for (length, shown) in [
        (surf_session::TITLE_CHARS_MAX, true),
        (surf_session::TITLE_CHARS_MAX + 1, false),
    ] {
        let title = "\u{20ac}".repeat(length);
        let page = format!("<!doctype html><meta charset=\"utf-8\"><title>{title}</title><p>x</p>");
        let site = Site::start(vec![("/", HTML, page.into_bytes())]);
        let (mut session, token, _) = open_at(&site.url("/"), &[]);
        let reply = session.answer(&holder("look", &token, &[]));
        let link = reply.to_link();
        let page_line = link.text.lines().nth(1).unwrap().to_string();
        assert_eq!(
            page_line.ends_with(&format!(" {}", surf_answer::quote(&title))),
            shown,
            "{length}"
        );
        assert_eq!(link.title.is_some(), shown, "{length}");
    }
}

/// The bound of `surf_session::NOW_BYTES_MAX`: a page of 100 links whose targets quote to about
/// 12 KiB each shows the first lines that fit, `now <shown> of 100`, and the answer reads back.
#[test]
fn a_now_block_over_its_byte_bound_shows_its_first_lines() {
    let href = format!("http://[{}", "\u{20ac}".repeat(1_100));
    let page: String = std::iter::once("<!doctype html><meta charset=\"utf-8\">".to_string())
        .chain((0..100).map(|n| format!("<a href=\"{href}\">link {n}</a>")))
        .collect();
    let site = Site::start(vec![("/", HTML, page.into_bytes())]);
    let (mut session, token, _) = open_at(&site.url("/"), &[]);
    let (status, text, typed) = ask(&mut session, &holder("look", &token, &[]));
    assert_eq!(status, 0);
    let now = typed.now.as_ref().expect("a now block");
    assert_eq!(now.total, 100);
    assert!(
        now.elements.len() < 100 && now.elements.len() > 50,
        "{}",
        now.elements.len()
    );
    let block: usize = now_lines(&text).iter().map(|line| line.len() + 1).sum();
    assert!(block <= surf_session::NOW_BYTES_MAX, "{block}");
    assert_eq!(
        now.elements.last().unwrap().handle,
        format!("l{}", now.elements.len())
    );
    round_trip(&text, &typed);
}
