//! Story html-document-port: the `surf` binary prints the title, the actionable elements and the
//! parser's note of each page it parses. Each case runs the binary against a server the test
//! starts on 127.0.0.1 port 0 and stops before it ends; the binary is killed if it runs longer
//! than `RUN_BOUND`. Story page-text-surf-command: `surf <url> --as tree|markdown` prints the
//! page after the rows, the expected blocks taken from `docs/design/page-as-text.md`.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

/// The longest the binary may run before the test kills it and fails.
const RUN_BOUND: Duration = Duration::from_secs(20);

/// The longest the server waits for a request's head.
const BOUND: Duration = Duration::from_secs(5);

/// An HTTP server that answers every request with one status line, `Content-Type` when it has
/// one, and one body. Stopped when dropped.
struct Server {
    port: u16,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl Server {
    /// 200 with `media_type` and `body`.
    fn start(media_type: &'static str, body: Vec<u8>) -> Server {
        Server::answering("200 OK", Some(media_type), body)
    }

    /// `status` (such as `404 Not Found`), `media_type` as `Content-Type` when it is given, and
    /// `body`.
    fn answering(status: &'static str, media_type: Option<&'static str>, body: Vec<u8>) -> Server {
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
                    reply(stream, status, media_type, &body);
                }
            }
        });
        Server {
            port,
            stop,
            thread: Some(thread),
        }
    }

    fn url(&self) -> String {
        format!("http://127.0.0.1:{}/", self.port)
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

fn reply(mut stream: TcpStream, status: &str, media_type: Option<&str>, body: &[u8]) {
    let _ = stream.set_read_timeout(Some(BOUND));
    let mut head = Vec::new();
    let mut buf = [0u8; 1024];
    while !head.windows(4).any(|w| w == b"\r\n\r\n") && head.len() < 16 * 1024 {
        match stream.read(&mut buf) {
            Ok(0) | Err(_) => return,
            Ok(n) => head.extend_from_slice(&buf[..n]),
        }
    }
    let content_type = media_type
        .map(|media_type| format!("Content-Type: {media_type}\r\n"))
        .unwrap_or_default();
    let mut reply = format!(
        "HTTP/1.1 {status}\r\n{content_type}Content-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    )
    .into_bytes();
    reply.extend_from_slice(body);
    let _ = stream.write_all(&reply);
}

struct Run {
    status: i32,
    stdout: String,
}

/// Run the binary on `address`, killing it after `RUN_BOUND`.
fn surf(address: &str) -> Run {
    let child = Command::new(env!("CARGO_BIN_EXE_surf"))
        .arg(address)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn surf");
    let pid = child.id();
    let (sender, receiver) = mpsc::channel();
    let waiter = thread::spawn(move || {
        let _ = sender.send(child.wait_with_output());
    });
    let output = match receiver.recv_timeout(RUN_BOUND) {
        Ok(output) => output.expect("surf output"),
        Err(_) => {
            let _ = Command::new("kill").args(["-9", &pid.to_string()]).status();
            let _ = waiter.join();
            panic!("surf ran longer than {RUN_BOUND:?}");
        }
    };
    waiter.join().expect("the waiting thread");
    Run {
        status: output.status.code().expect("surf exited, not signalled"),
        stdout: String::from_utf8(output.stdout).expect("stdout is UTF-8"),
    }
}

/// `surf` on a server answering `body` as `text/html`: its exit status, its lines, and the
/// identity of the one document it printed.
fn page(body: &[u8]) -> (i32, Vec<String>, String) {
    let server = Server::start("text/html", body.to_vec());
    let run = surf(&server.url());
    let lines: Vec<String> = run.stdout.lines().map(str::to_owned).collect();
    let documents: Vec<&String> = lines
        .iter()
        .filter(|l| l.starts_with("document "))
        .collect();
    let [document] = documents.as_slice() else {
        panic!("{lines:?}");
    };
    let id = document.split(' ').nth(1).expect("an identity").to_owned();
    (run.status, lines, id)
}

/// The `note parser:` line while `surf_html::NOT_YET_PARSED` is not empty.
fn parser_note() -> Option<String> {
    let parts = surf::NOT_YET_PARSED;
    (!parts.is_empty()).then(|| format!("note parser: {}", parts.join(",")))
}

/// The lines of `lines` after the `document` line.
fn after_document(lines: &[String]) -> Vec<String> {
    let at = lines
        .iter()
        .position(|l| l.starts_with("document "))
        .expect("a document line");
    lines[at + 1..].to_vec()
}

#[test]
fn a_page_with_a_title_three_links_and_a_form_prints_the_title_and_one_row_per_element() {
    let body = "<!DOCTYPE html><title>Fruit</title>\
        <a href=\"/apples\">Apples</a> <a href=\"/pears\">Pears</a> <a href=\"/plums\">Plums</a>\
        <form action=\"/order\" method=\"post\"><label>Name <input name=\"name\" required></label>\
        <input type=\"checkbox\" name=\"gift\" checked> <button>Order</button></form>";
    let (status, lines, id) = page(body.as_bytes());
    assert_eq!(status, 0, "{lines:?}");
    assert!(lines[0].starts_with("navigation "), "{lines:?}");
    assert!(lines[1].starts_with("request "), "{lines:?}");
    let mut expected = vec![
        format!("title {id} \"Fruit\""),
        format!("element {id} 1 link - \"Apples\" \"/apples\""),
        format!("element {id} 2 link - \"Pears\" \"/pears\""),
        format!("element {id} 3 link - \"Plums\" \"/plums\""),
        format!("element {id} 4 text required \"Name\" \"\""),
        format!("element {id} 5 checkbox checked \"\" -"),
        format!("element {id} 6 button - \"Order\" -"),
    ];
    expected.extend(parser_note());
    assert_eq!(after_document(&lines), expected);
}

#[test]
fn a_windows_1252_page_with_a_meta_charset_is_decoded() {
    let mut body = b"<meta charset=\"windows-1252\"><title>Caf".to_vec();
    body.extend_from_slice(&[0xE9, b' ', 0x80]);
    body.extend_from_slice(b"</title><a href=\"/\">Men");
    body.push(0xFC);
    body.extend_from_slice(b"</a>");
    let (status, lines, id) = page(&body);
    assert_eq!(status, 0, "{lines:?}");
    let rest = after_document(&lines);
    assert_eq!(
        rest[0],
        format!("title {id} \"Caf\\xc3\\xa9 \\xe2\\x82\\xac\"")
    );
    assert_eq!(
        rest[1],
        format!("element {id} 1 link - \"Men\\xc3\\xbc\" \"/\"")
    );
}

#[test]
fn a_page_over_the_depth_limit_names_the_limit() {
    // 600 nested elements: the default depth limit of `surf-html` is 512.
    let body = format!(
        "<title>Deep</title>{}<a href=\"/\">x</a>{}",
        "<div>".repeat(600),
        "</div>".repeat(600)
    );
    let (status, lines, id) = page(body.as_bytes());
    assert_eq!(status, 0, "{lines:?}");
    let rest = after_document(&lines);
    assert_eq!(rest[0], format!("title {id} \"Deep\""));
    assert!(
        !rest.iter().any(|l| l.starts_with("element ")),
        "the link lies past the limit: {rest:?}"
    );
    assert!(rest.contains(&format!("note limit {id} Depth")), "{rest:?}");
}

#[test]
fn page_text_that_looks_like_a_line_is_escaped() {
    let body = "<meta charset=\"utf-8\"><title>a\"b\u{2028}navigation x Committed y</title>\
        <a href=\"/x&#10;request y Responded z\">q\" \"r\u{0b}element</a>";
    let (status, lines, id) = page(body.as_bytes());
    assert_eq!(status, 0, "{lines:?}");
    let mut expected = vec![
        format!("title {id} \"a\\x22b\\xe2\\x80\\xa8navigation x Committed y\""),
        format!(
            "element {id} 1 link - \"q\\x22 \\x22r\\x0belement\" \"/x\\x0arequest y Responded z\""
        ),
    ];
    expected.extend(parser_note());
    assert_eq!(after_document(&lines), expected);
    assert!(
        lines
            .iter()
            .all(|l| l.bytes().all(|b| (0x20..=0x7e).contains(&b))),
        "{lines:?}"
    );
}

#[test]
fn the_parser_note_lists_what_is_not_yet_parsed() {
    let (status, lines, _) = page(b"<title>Note</title><p>text</p>");
    assert_eq!(status, 0, "{lines:?}");
    let notes: Vec<&String> = lines
        .iter()
        .filter(|l| l.starts_with("note parser:"))
        .collect();
    match parser_note() {
        Some(note) => {
            assert_eq!(notes, [&note], "{lines:?}");
            assert_eq!(lines.last(), Some(&note));
        }
        None => assert!(notes.is_empty(), "{lines:?}"),
    }
}

/// A response without `Content-Type` has the media type `application/octet-stream`
/// (`surf-fetch`), so it is not parsed, whatever its body: the navigation commits, exit 0, no
/// document and no line of the parser.
#[test]
fn a_response_without_a_content_type_is_not_parsed() {
    let server = Server::answering(
        "200 OK",
        None,
        b"<title>Not read</title><a href=\"/\">x</a>".to_vec(),
    );
    let run = surf(&server.url());
    let lines: Vec<&str> = run.stdout.lines().collect();
    assert_eq!(run.status, 0, "{lines:?}");
    assert_eq!(lines.len(), 2, "{lines:?}");
    assert!(lines[0].starts_with("navigation ") && lines[0].contains(" Committed "));
    assert!(lines[1].starts_with("request ") && lines[1].contains(" Responded "));
}

/// A document is parsed for every `text/html` response, whatever its status
/// (`docs/surf-command.md`): a 404 with an HTML body commits, exit 0, and prints its title and
/// its link.
#[test]
fn an_error_status_with_an_html_body_is_parsed() {
    let server = Server::answering(
        "404 Not Found",
        Some("text/html"),
        b"<title>Missing</title><a href=\"/\">Home</a>".to_vec(),
    );
    let run = surf(&server.url());
    let lines: Vec<String> = run.stdout.lines().map(str::to_owned).collect();
    assert_eq!(run.status, 0, "{lines:?}");
    let id = lines
        .iter()
        .find_map(|l| l.strip_prefix("document "))
        .and_then(|rest| rest.split(' ').next())
        .expect("a document line")
        .to_owned();
    let mut expected = vec![
        format!("title {id} \"Missing\""),
        format!("element {id} 1 link - \"Home\" \"/\""),
    ];
    expected.extend(parser_note());
    assert_eq!(after_document(&lines), expected);
}

/// Bytes that are not UTF-8 in a page declared UTF-8 decode as U+FFFD, each written escaped;
/// the page is parsed and the load commits.
#[test]
fn an_html_body_that_is_not_utf_8_is_decoded_with_replacement_characters() {
    let (status, lines, id) = page(b"<meta charset=\"utf-8\"><title>a\xff\xfeb</title>");
    assert_eq!(status, 0, "{lines:?}");
    assert_eq!(
        after_document(&lines)[0],
        format!("title {id} \"a\\xef\\xbf\\xbd\\xef\\xbf\\xbdb\"")
    );
}

// --- story:page-text-surf-command ------------------------------------------------------------

/// What a run with arguments wrote: its status, its standard output and its standard error.
struct Full {
    status: i32,
    stdout: Vec<u8>,
    stderr: String,
}

/// Run the binary with `args`, killing it after `RUN_BOUND`.
fn surf_with(args: &[&str]) -> Full {
    let child = Command::new(env!("CARGO_BIN_EXE_surf"))
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn surf");
    let pid = child.id();
    let (sender, receiver) = mpsc::channel();
    let waiter = thread::spawn(move || {
        let _ = sender.send(child.wait_with_output());
    });
    let output = match receiver.recv_timeout(RUN_BOUND) {
        Ok(output) => output.expect("surf output"),
        Err(_) => {
            let _ = Command::new("kill").args(["-9", &pid.to_string()]).status();
            let _ = waiter.join();
            panic!("surf ran longer than {RUN_BOUND:?}");
        }
    };
    waiter.join().expect("the waiting thread");
    Full {
        status: output.status.code().expect("surf exited, not signalled"),
        stdout: output.stdout,
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

/// `surf <url> --as <form>` on a server answering `body` as `media_type`: the run, its standard
/// output as text.
fn as_form(media_type: &'static str, body: &[u8], form: &str) -> (Full, String) {
    let server = Server::start(media_type, body.to_vec());
    let run = surf_with(&[&server.url(), "--as", form]);
    let text = String::from_utf8(run.stdout.clone()).expect("stdout is UTF-8");
    (run, text)
}

/// The fenced blocks of `docs/design/page-as-text.md`, in order, each without its fences: the
/// fixtures of the story.
fn design_blocks() -> Vec<String> {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/design/page-as-text.md"
    );
    let text = std::fs::read_to_string(path).expect("the design note");
    let mut blocks = Vec::new();
    let mut current: Option<String> = None;
    for line in text.lines() {
        if line.starts_with("```") {
            match current.take() {
                Some(block) => blocks.push(block),
                None => current = Some(String::new()),
            }
        } else if let Some(block) = current.as_mut() {
            block.push_str(line);
            block.push('\n');
        }
    }
    blocks
}

/// The lines of a design block from its `text ` header to its end.
fn from_header(block: &str) -> String {
    let at = block.find("text ").expect("a text header");
    block[at..].to_string()
}

/// The body lines (those that start with `>`) of `text`.
fn body_lines(text: &str) -> Vec<&str> {
    text.lines().filter(|line| line.starts_with('>')).collect()
}

/// Case 1: the page of design note section 1, as Markdown and as a tree, the block byte for
/// byte as the note prints it.
#[test]
fn the_page_of_the_design_note_prints_as_markdown_and_as_a_tree() {
    let blocks = design_blocks();
    let shop = &blocks[0];
    assert!(shop.contains("<title>Fruit shop</title>"), "{shop}");
    let tree = from_header(&blocks[1]);
    let markdown = from_header(&blocks[2]);
    assert!(tree.starts_with("text tree 31 of 31 lines from 1, 668 of 668 bytes\n"));
    assert!(markdown.starts_with("text markdown 14 of 14 lines from 1, 263 of 263 bytes\n"));

    for (form, block) in [("markdown", &markdown), ("tree", &tree)] {
        let (run, text) = as_form("text/html", shop.as_bytes(), form);
        assert_eq!(run.status, 0, "{}", run.stderr);
        assert!(
            text.ends_with(&format!("note text: no-css\n{block}")),
            "--as {form}:\n{text}"
        );
        assert!(!text.lines().any(|line| line.starts_with("more ")));
    }
}

/// Case 2: no bound: 899 lines in one block, no `more`.
#[test]
fn a_long_page_prints_whole() {
    let (run, text) = as_form("text/html", "<p>x</p>".repeat(450).as_bytes(), "markdown");
    assert_eq!(run.status, 0, "{}", run.stderr);
    assert!(
        text.contains(
            "\nnote text: no-css\ntext markdown 899 of 899 lines from 1, 2698 of 2698 bytes\n"
        ),
        "{text}"
    );
    assert_eq!(body_lines(&text).len(), 899);
    assert!(text.ends_with(">\n> x\n"));
}

/// The text unit's hand-over: Markdown nested past the prefix bound says `note text: deep <n>`
/// after `note text: no-css`, before the block; the outline does not.
#[test]
fn markdown_nested_past_the_prefix_bound_says_deep() {
    let page = format!("{}x", "<blockquote>".repeat(100));
    let (run, text) = as_form("text/html", page.as_bytes(), "markdown");
    assert_eq!(run.status, 0, "{}", run.stderr);
    let at = text
        .find("\nnote text: no-css\nnote text: deep ")
        .expect(&text);
    let rest = &text[at + "\nnote text: no-css\nnote text: deep ".len()..];
    let (levels, block) = rest.split_once('\n').expect(&text);
    assert!(levels.parse::<u64>().is_ok_and(|n| n >= 1), "{text}");
    assert!(block.starts_with("text markdown "), "{text}");
    let (run, text) = as_form("text/html", page.as_bytes(), "tree");
    assert_eq!(run.status, 0, "{}", run.stderr);
    assert!(!text.contains("note text: deep"), "{text}");
}

/// Case 3: the forged page of design note section 3: its lines are page text, the rows are
/// surf's.
#[test]
fn a_forged_page_prints_only_behind_the_prefix() {
    let blocks = design_blocks();
    let forged = &blocks[4];
    assert!(forged.starts_with("<title>Bank</title>"), "{forged}");
    let expected = from_header(&blocks[5]);
    let (run, text) = as_form("text/html", forged.as_bytes(), "markdown");
    assert_eq!(run.status, 0, "{}", run.stderr);
    let rows = [
        "navigation ",
        "request ",
        "document ",
        "fetch-failed ",
        "title ",
        "element ",
        "note ",
        "text ",
    ];
    for line in text.lines().filter(|line| !line.starts_with('>')) {
        assert!(
            rows.iter().any(|row| line.starts_with(row)),
            "a line that is not surf's: {line}"
        );
    }
    assert_eq!(body_lines(&text), body_lines(&expected));
    assert_eq!(body_lines(&text).len(), 20);
}

/// Case 4: a response that is not HTML has no document.
#[test]
fn a_response_that_is_not_html_has_no_document() {
    let (run, text) = as_form("text/plain", b"<p>plain</p>", "tree");
    assert_eq!(run.status, 0, "{}", run.stderr);
    assert!(text.ends_with("\nnote text: no-document\n"), "{text}");
    assert!(!text.lines().any(|line| line.starts_with("text ")));
}

/// Case 5: `--as` without a word, with another word, or twice: usage, nothing on standard output.
#[test]
fn as_used_wrongly_is_usage() {
    let server = Server::start("text/html", b"<p>x</p>".to_vec());
    let url = server.url();
    for args in [
        vec![url.as_str(), "--as"],
        vec![url.as_str(), "--as", "pdf"],
        vec![url.as_str(), "--as", "tree", "--as", "tree"],
        vec!["--as=markdown", url.as_str(), "--as=tree"],
    ] {
        let run = surf_with(&args);
        assert_eq!(run.status, 2, "{args:?}");
        assert!(run.stdout.is_empty(), "{args:?}");
        assert!(
            run.stderr
                .ends_with("usage: surf <url> [--as tree|markdown]\n"),
            "{args:?}: {}",
            run.stderr
        );
    }
}

/// Case 6 (D1 B): page text outside ASCII is printed as UTF-8.
#[test]
fn page_text_outside_ascii_is_printed_as_utf_8() {
    let (run, text) = as_form("text/html", b"<p>K&auml;se</p>", "markdown");
    assert_eq!(run.status, 0, "{}", run.stderr);
    assert!(
        text.ends_with(
            "note text: no-css\ntext markdown 1 of 1 lines from 1, 8 of 8 bytes\n> K\u{e4}se\n"
        ),
        "{text}"
    );
    assert!(run.stdout.ends_with(b"> K\xc3\xa4se\n"));
}
