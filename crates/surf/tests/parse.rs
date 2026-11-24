//! Story html-document-port: the parser step of `Browser`. The first cases load pages from a
//! server the test starts on 127.0.0.1 port 0 and stops before it ends; the cases of the parser's
//! limits answer from a transport in memory, each with one limit set small.

use std::cell::Cell;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use surf::{Answer, Browser, HtmlParser, Parse, Parser, ParsingStarted, RequestStarted, answer};
use surf::{Report, fetch_http};
use surf_html::NodeData;
use surf_ports::ComponentPorts;
use surf_types::document::{DocumentId, DocumentState, Documents, ElementKind, ParseLimit};

/// The longest the server waits for a request's head.
const BOUND: Duration = Duration::from_secs(5);

/// A page with a title, two links and a form with one text field and one button.
const PAGE: &str = "<!DOCTYPE html><title>Shop</title><a href=\"/a\">First</a>\
<p><a href=\"/b\">Second</a></p><form><input name=q><button>Go</button></form>";

/// An HTTP server that answers every request with 200, `media_type` and `body`. Stopped when
/// dropped.
struct Server {
    port: u16,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl Server {
    fn start(media_type: &'static str, body: &'static str) -> Server {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind 127.0.0.1:0");
        let port = listener.local_addr().expect("local address").port();
        let stop = Arc::new(AtomicBool::new(false));
        let stopping = Arc::clone(&stop);
        let thread = thread::spawn(move || {
            for stream in listener.incoming() {
                if stopping.load(Ordering::SeqCst) {
                    break;
                }
                if let Ok(stream) = stream {
                    reply(stream, media_type, body);
                }
            }
        });
        Server {
            port,
            stop,
            thread: Some(thread),
        }
    }

    fn url(&self, path: &str) -> String {
        format!("http://127.0.0.1:{}{path}", self.port)
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect(("127.0.0.1", self.port));
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn reply(mut stream: TcpStream, media_type: &str, body: &str) {
    let _ = stream.set_read_timeout(Some(BOUND));
    let mut head = Vec::new();
    let mut buf = [0u8; 1024];
    while !head.windows(4).any(|w| w == b"\r\n\r\n") && head.len() < 16 * 1024 {
        match stream.read(&mut buf) {
            Ok(0) | Err(_) => return,
            Ok(n) => head.extend_from_slice(&buf[..n]),
        }
    }
    let reply = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: {media_type}\r\nContent-Length: {}\r\nConnection: \
         close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(reply.as_bytes());
}

/// The transport of the binaries over `surf-fetch`.
fn http(request: &RequestStarted) -> Answer {
    answer(fetch_http(
        request,
        &surf_fetch::Limits::default(),
        surf::RequestContext::default(),
    ))
}

/// A parser step that counts its calls in `calls` and answers what `inner` answers.
fn counting<P: Parser>(
    mut inner: P,
    calls: Rc<Cell<usize>>,
) -> impl FnMut(&ParsingStarted) -> Parse {
    move |started: &ParsingStarted| {
        calls.set(calls.get() + 1);
        inner.parse(started)
    }
}

/// The one document of `report`.
fn only_document(report: &Report) -> &Documents {
    let [document] = report.documents.as_slice() else {
        panic!("{:?}", report.documents);
    };
    document
}

#[test]
fn element_identities_and_nodes_are_the_same_on_every_read() {
    let server = Server::start("text/html", PAGE);
    let mut browser = Browser::from_ports(ComponentPorts::new(), http);
    let report = browser.load(&server.url("/")).expect("load");
    let document = only_document(&report).clone();
    assert_eq!(document.state, DocumentState::Parsed);
    assert_eq!(document.title.as_deref(), Some("Shop"));
    let id: DocumentId = document.document_id.clone();

    let rows = browser
        .system()
        .document
        .elements(id.clone())
        .expect("elements");
    let kinds: Vec<ElementKind> = rows.iter().map(|row| row.kind).collect();
    assert_eq!(
        kinds,
        [
            ElementKind::Link,
            ElementKind::Link,
            ElementKind::Text,
            ElementKind::Button
        ]
    );
    let positions: Vec<i64> = rows.iter().map(|row| row.position).collect();
    assert_eq!(positions, [1, 2, 3, 4]);

    let nodes = browser
        .element_nodes(&id)
        .expect("the document's nodes")
        .to_vec();
    let tree = browser
        .document_tree(&id)
        .expect("the document's tree")
        .clone();
    assert_eq!(
        nodes
            .iter()
            .map(|(element, _)| element.clone())
            .collect::<Vec<_>>(),
        rows.iter()
            .map(|row| row.element_id.clone())
            .collect::<Vec<_>>(),
        "one node per element row, in the order of the rows"
    );
    let names: Vec<&str> = nodes
        .iter()
        .map(|(_, node)| match tree.data(*node) {
            NodeData::Element { local_name, .. } => local_name.as_str(),
            other => panic!("{other:?}"),
        })
        .collect();
    assert_eq!(names, ["a", "a", "input", "button"]);

    // A second read, after the system dropped what it delivered and loaded another page.
    browser.forget_delivered();
    let other = Server::start("text/html", "<title>Other</title><a href=\"/\">Back</a>");
    let second = browser.load(&other.url("/")).expect("second load");
    assert_eq!(second.documents.len(), 2, "{:?}", second.documents);

    let again = browser
        .system()
        .document
        .elements(id.clone())
        .expect("elements");
    assert_eq!(again, rows, "the rows of the first document");
    assert_eq!(browser.element_nodes(&id).expect("nodes"), nodes.as_slice());
    assert_eq!(browser.document_tree(&id).expect("tree"), &tree);
    assert_eq!(browser.parser_not_yet_parsed(), surf_html::NOT_YET_PARSED);
}

#[test]
fn a_parser_step_that_never_answers_leaves_the_parse_open() {
    let server = Server::start("text/html", PAGE);
    let calls = Rc::new(Cell::new(0));
    let seen = Rc::clone(&calls);
    let never = move |_: &ParsingStarted| {
        seen.set(seen.get() + 1);
        Parse::Unanswered
    };
    let mut browser = Browser::from_ports(ComponentPorts::new(), http).with_parser(never);
    let report = browser.load(&server.url("/")).expect("load");
    let document = only_document(&report).clone();
    assert_eq!(calls.get(), 1, "the parser step is asked once");
    assert_eq!(document.state, DocumentState::Parsing);
    assert_eq!(document.title, None);
    assert!(
        browser
            .system()
            .document
            .elements(document.document_id.clone())
            .expect("elements")
            .is_empty()
    );
    assert!(browser.document_tree(&document.document_id).is_none());
    assert!(browser.element_nodes(&document.document_id).is_none());
    let lines = report.lines();
    assert!(
        !lines.iter().any(|line| line.starts_with("title ")
            || line.starts_with("element ")
            || line.starts_with("note ")),
        "{lines:?}"
    );

    // A later run asks nothing again.
    browser.run().expect("run");
    assert_eq!(calls.get(), 1);
}

#[test]
fn a_non_html_response_is_not_parsed() {
    let server = Server::start("text/plain", "<title>Not a page</title><a href=\"/\">x</a>");
    let calls = Rc::new(Cell::new(0));
    let mut browser = Browser::from_ports(ComponentPorts::new(), http)
        .with_parser(counting(HtmlParser::default(), Rc::clone(&calls)));
    let report = browser.load(&server.url("/")).expect("load");
    assert_eq!(calls.get(), 0);
    assert!(report.documents.is_empty(), "{:?}", report.documents);
    let lines = report.lines();
    assert!(
        lines
            .iter()
            .all(|line| line.starts_with("navigation ") || line.starts_with("request ")),
        "{lines:?}"
    );
}

/// Story values-and-labels-before-fill, acceptance 1: the `element` line of a `password` control
/// ends in `(set)` or `(empty)`, never its value; a `text` control's line ends in its value.
#[test]
fn a_password_value_is_printed_as_set_or_empty() {
    let transport = |request: &RequestStarted| Answer::Response {
        url: request.url.0.clone(),
        method: request.method,
        status: 200,
        media_type: "text/html".to_owned(),
        body: b"<input type=password value=\"pw-marker\"><input type=password>\
<input type=text value=\"tx-marker\">"
            .to_vec(),
    };
    let mut browser = Browser::from_ports(ComponentPorts::new(), transport);
    let report = browser.load("http://127.0.0.1/form").expect("load");
    let lines = report.lines();
    let elements: Vec<&String> = lines
        .iter()
        .filter(|line| line.starts_with("element "))
        .collect();
    assert_eq!(elements.len(), 3, "{lines:?}");
    assert!(
        elements[0].contains(" 1 password ") && elements[0].ends_with(" (set)"),
        "{lines:?}"
    );
    assert!(
        elements[1].contains(" 2 password ") && elements[1].ends_with(" (empty)"),
        "{lines:?}"
    );
    assert!(
        elements[2].contains(" 3 text ") && elements[2].ends_with(" \"tx-marker\""),
        "{lines:?}"
    );
    assert!(
        !lines.iter().any(|line| line.contains("pw-marker")),
        "{lines:?}"
    );
}

// --- The parser's limits, each set small ---------------------------------------------------------

/// The page every limit case starts with: a title and the link `/a`, both before the limit.
const BEFORE: &str = "<title>T</title><a href=\"/a\">A</a>";

/// The link every limit case ends with, past the limit.
const AFTER: &str = "<a href=\"/z\">Z</a>";

/// `body` served as `text/html` by a transport in memory and parsed with `limits`: the report,
/// its lines and the identity of its document. The load must end with its navigation
/// committed and its document `Parsed`, stopped by `limit`.
fn parsed_within(
    limits: surf_html::limits::Limits,
    body: String,
    limit: ParseLimit,
) -> (Vec<String>, String) {
    let transport = move |request: &RequestStarted| Answer::Response {
        url: request.url.0.clone(),
        method: request.method,
        status: 200,
        media_type: "text/html".to_owned(),
        body: body.clone().into_bytes(),
    };
    let mut browser = Browser::from_ports(ComponentPorts::new(), transport)
        .with_parser(HtmlParser::with_limits(limits));
    let result = browser.load("http://127.0.0.1/page");
    assert_eq!(
        surf::exit_status(&result),
        surf::EXIT_COMMITTED,
        "{result:?}"
    );
    let report = result.expect("load");
    let document = only_document(&report).clone();
    assert_eq!(document.state, DocumentState::Parsed);
    assert_eq!(document.stopped_by, Some(limit));
    let id = document.document_id.0.0.clone();
    let tree = browser
        .document_tree(&document.document_id)
        .expect("the tree is kept");
    assert_eq!(
        tree.stopped_by().map(|l| format!("{l:?}")),
        Some(format!("{limit:?}"))
    );
    (report.lines(), id)
}

/// The lines of a page stopped by `limit`: the title and the link `/a` before it are kept, the
/// link `/z` after it is not, and `note limit <id> <limit>` names it.
fn assert_kept_before(lines: &[String], id: &str, limit: &str) {
    assert!(lines.contains(&format!("title {id} \"T\"")), "{lines:?}");
    assert!(
        lines.contains(&format!("element {id} 1 link - \"A\" \"/a\"")),
        "{lines:?}"
    );
    assert!(!lines.iter().any(|l| l.ends_with("\"/z\"")), "{lines:?}");
    assert!(
        lines.contains(&format!("note limit {id} {limit}")),
        "{lines:?}"
    );
}

#[test]
fn a_page_over_the_input_limit_is_parsed_up_to_it() {
    let limits = surf_html::limits::Limits {
        max_input_bytes: 64,
        ..surf_html::limits::Limits::default()
    };
    let body = format!("{BEFORE}<p>{}</p>{AFTER}", "x".repeat(100));
    let (lines, id) = parsed_within(limits, body, ParseLimit::InputBytes);
    assert_kept_before(&lines, &id, "InputBytes");
}

#[test]
fn a_page_over_the_depth_limit_is_parsed_up_to_it() {
    let limits = surf_html::limits::Limits {
        max_depth: 8,
        ..surf_html::limits::Limits::default()
    };
    let body = format!("{BEFORE}{}{AFTER}", "<div>".repeat(20));
    let (lines, id) = parsed_within(limits, body, ParseLimit::Depth);
    assert_kept_before(&lines, &id, "Depth");
}

#[test]
fn a_page_over_the_node_limit_is_parsed_up_to_it() {
    let limits = surf_html::limits::Limits {
        max_nodes: 12,
        ..surf_html::limits::Limits::default()
    };
    let body = format!("{BEFORE}{}{AFTER}", "<p>x</p>".repeat(20));
    let (lines, id) = parsed_within(limits, body, ParseLimit::Nodes);
    assert_kept_before(&lines, &id, "Nodes");
}

#[test]
fn a_page_over_the_attribute_limit_is_parsed_up_to_it() {
    let limits = surf_html::limits::Limits {
        max_attributes: 2,
        ..surf_html::limits::Limits::default()
    };
    let body = format!("{BEFORE}<p a=1 b=2 c=3>x</p>{AFTER}");
    let (lines, id) = parsed_within(limits, body, ParseLimit::Attributes);
    assert_kept_before(&lines, &id, "Attributes");
}

#[test]
fn a_page_over_the_stored_byte_limit_is_parsed_up_to_it() {
    let limits = surf_html::limits::Limits {
        max_stored_bytes: 400,
        ..surf_html::limits::Limits::default()
    };
    let body = format!("{BEFORE}<p>{}</p>{AFTER}", "x".repeat(1000));
    let (lines, id) = parsed_within(limits, body, ParseLimit::StoredBytes);
    assert_kept_before(&lines, &id, "StoredBytes");
}

#[test]
fn a_page_over_the_step_limit_is_parsed_up_to_it() {
    let limits = surf_html::limits::Limits {
        max_steps: 64,
        ..surf_html::limits::Limits::default()
    };
    let body = format!("{BEFORE}{}{AFTER}", "<p>x</p>".repeat(200));
    let (lines, id) = parsed_within(limits, body, ParseLimit::Steps);
    assert_kept_before(&lines, &id, "Steps");
}
