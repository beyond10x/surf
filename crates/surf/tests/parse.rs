//! Story html-document-port: the parser step of `Browser`. Each case loads pages from a server the
//! test starts on 127.0.0.1 port 0 and stops before it ends.

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
use surf_types::document::{DocumentId, DocumentState, Documents, ElementKind};

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
    answer(fetch_http(request, &surf_fetch::Limits::default()))
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
        .with_parser(counting(HtmlParser, Rc::clone(&calls)));
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
