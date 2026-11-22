//! Cases 1 to 6 of story `headless-cli`: each runs the `surf` binary, against a server the test
//! starts itself on `127.0.0.1` port 0 where the case needs one. Every server thread ends on its
//! own: it waits at most `BOUND` for a connection and at most `BOUND` for the client to close.
//! The binary is killed if it runs longer than `RUN_BOUND`.

use std::io::{ErrorKind, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process::{Command, Stdio};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

/// The longest any server thread waits for anything.
const BOUND: Duration = Duration::from_secs(5);

/// The longest the binary may run before the test kills it and fails.
const RUN_BOUND: Duration = Duration::from_secs(20);

struct Server {
    address: String,
    handle: JoinHandle<()>,
}

/// Start a server that accepts one connection, reads the request header section, writes
/// `reply`, and reads until the client closes.
fn serve(reply: &'static [u8]) -> Server {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind 127.0.0.1:0");
    let port = listener.local_addr().expect("local address").port();
    listener
        .set_nonblocking(true)
        .expect("nonblocking listener");
    let handle = thread::spawn(move || {
        let deadline = Instant::now() + BOUND;
        let mut stream = loop {
            match listener.accept() {
                Ok((stream, _)) => break stream,
                Err(e) if e.kind() == ErrorKind::WouldBlock && Instant::now() < deadline => {
                    thread::sleep(Duration::from_millis(5));
                }
                Err(_) => return,
            }
        };
        drop(listener);
        stream.set_nonblocking(false).expect("blocking stream");
        stream.set_read_timeout(Some(BOUND)).expect("read timeout");
        stream
            .set_write_timeout(Some(BOUND))
            .expect("write timeout");
        read_request(&mut stream);
        let _ = stream.write_all(reply);
        let _ = stream.flush();
        wait_for_close(&mut stream);
    });
    Server {
        address: format!("http://127.0.0.1:{port}/page"),
        handle,
    }
}

fn read_request(stream: &mut TcpStream) {
    let mut request = Vec::new();
    let mut buf = [0u8; 1024];
    while !request.windows(4).any(|w| w == b"\r\n\r\n") && request.len() < 64 * 1024 {
        match stream.read(&mut buf) {
            Ok(0) | Err(_) => break,
            Ok(n) => request.extend_from_slice(&buf[..n]),
        }
    }
}

fn wait_for_close(stream: &mut TcpStream) {
    let mut buf = [0u8; 1024];
    while let Ok(n) = stream.read(&mut buf) {
        if n == 0 {
            break;
        }
    }
}

struct Run {
    status: i32,
    stdout: String,
    stderr: String,
}

/// Run the binary with `args`, killing it after `RUN_BOUND`.
fn surf(args: &[&str]) -> Run {
    let mut child = Command::new(env!("CARGO_BIN_EXE_surf"))
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn surf");
    let deadline = Instant::now() + RUN_BOUND;
    loop {
        if child.try_wait().expect("wait for surf").is_some() {
            break;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("surf ran longer than {RUN_BOUND:?}");
        }
        thread::sleep(Duration::from_millis(10));
    }
    let output = child.wait_with_output().expect("surf output");
    Run {
        status: output.status.code().expect("surf exited, not signalled"),
        stdout: String::from_utf8(output.stdout).expect("stdout is UTF-8"),
        stderr: String::from_utf8(output.stderr).expect("stderr is UTF-8"),
    }
}

/// One output line, split into its kind and its fields; the last field is the rest of the line.
#[derive(Debug, PartialEq, Eq)]
struct Line {
    kind: String,
    fields: Vec<String>,
}

fn lines(stdout: &str) -> Vec<Line> {
    stdout
        .lines()
        .map(|line| {
            let (kind, rest) = line.split_once(' ').expect("a line has a kind and fields");
            let fixed = match kind {
                "navigation" | "request" | "fetch-failed" => 2,
                "document" | "title" => 1,
                "note" => 0,
                other => panic!("unknown line kind {other:?} in {line:?}"),
            };
            let mut fields: Vec<String> = rest.splitn(fixed + 1, ' ').map(str::to_owned).collect();
            assert_eq!(fields.len(), fixed + 1, "fields of {line:?}");
            let address = fields.pop().expect("address");
            fields.push(address);
            Line {
                kind: kind.to_owned(),
                fields,
            }
        })
        .collect()
}

fn of_kind<'a>(lines: &'a [Line], kind: &str) -> Vec<&'a Line> {
    lines.iter().filter(|line| line.kind == kind).collect()
}

/// Asserts one navigation and one request with the same identity, in the given states, at
/// `address`; returns that identity.
fn assert_load(lines: &[Line], address: &str, navigation: &str, request: &str) -> String {
    let navigations = of_kind(lines, "navigation");
    assert_eq!(navigations.len(), 1, "{lines:?}");
    let requests = of_kind(lines, "request");
    assert_eq!(requests.len(), 1, "{lines:?}");
    let id = navigations[0].fields[0].clone();
    assert_eq!(navigations[0].fields, [id.as_str(), navigation, address]);
    assert_eq!(requests[0].fields, [id.as_str(), request, address]);
    id
}

fn assert_failed(run: &Run, address: &str, reason: &str) {
    assert_eq!(
        run.status, 1,
        "stdout {:?} stderr {:?}",
        run.stdout, run.stderr
    );
    let lines = lines(&run.stdout);
    let id = assert_load(&lines, address, "Failed", "Failed");
    assert!(of_kind(&lines, "document").is_empty(), "{lines:?}");
    let failures = of_kind(&lines, "fetch-failed");
    assert_eq!(failures.len(), 1, "{lines:?}");
    assert_eq!(failures[0].fields, [id.as_str(), reason, address]);
    assert_eq!(lines.len(), 3, "{lines:?}");
}

#[test]
fn case_1_html_response_commits_and_parses_one_document() {
    let server = serve(
        b"HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: 13\r\n\r\n<p>surf</p>\r\n",
    );
    let address = server.address.clone();
    let run = surf(&[&address]);
    server.handle.join().expect("server thread");
    assert_eq!(
        run.status, 0,
        "stdout {:?} stderr {:?}",
        run.stdout, run.stderr
    );
    let lines = lines(&run.stdout);
    assert_load(&lines, &address, "Committed", "Responded");
    let documents = of_kind(&lines, "document");
    assert_eq!(documents.len(), 1, "{lines:?}");
    assert_eq!(documents[0].fields[1], address);
    assert!(!documents[0].fields[0].is_empty());
    assert!(of_kind(&lines, "fetch-failed").is_empty(), "{lines:?}");
    // Since `story:html-document-port` the page is parsed: its empty title, no element, and the
    // parser's note while `NOT_YET_PARSED` is not empty.
    let titles = of_kind(&lines, "title");
    assert_eq!(titles.len(), 1, "{lines:?}");
    assert_eq!(titles[0].fields, [documents[0].fields[0].as_str(), "\"\""]);
    let notes = of_kind(&lines, "note");
    assert_eq!(
        notes.len(),
        usize::from(!surf::NOT_YET_PARSED.is_empty()),
        "{lines:?}"
    );
    assert_eq!(lines.len(), 4 + notes.len(), "{lines:?}");
}

#[test]
fn case_2_plain_text_response_commits_without_a_document() {
    let server =
        serve(b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 4\r\n\r\nsurf");
    let address = server.address.clone();
    let run = surf(&[&address]);
    server.handle.join().expect("server thread");
    assert_eq!(
        run.status, 0,
        "stdout {:?} stderr {:?}",
        run.stdout, run.stderr
    );
    let lines = lines(&run.stdout);
    assert_load(&lines, &address, "Committed", "Responded");
    assert!(of_kind(&lines, "document").is_empty(), "{lines:?}");
    assert!(of_kind(&lines, "fetch-failed").is_empty(), "{lines:?}");
    assert_eq!(lines.len(), 2, "{lines:?}");
}

#[test]
fn case_3_closed_port_fails_unreachable() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind 127.0.0.1:0");
    let port = listener.local_addr().expect("local address").port();
    drop(listener);
    let address = format!("http://127.0.0.1:{port}/");
    let run = surf(&[&address]);
    assert_failed(&run, &address, "Unreachable");
}

#[test]
fn case_4_reply_that_is_not_http_fails_malformed_response() {
    let server = serve(b"SSH-2.0-OpenSSH_9.6\r\n");
    let address = server.address.clone();
    let run = surf(&[&address]);
    server.handle.join().expect("server thread");
    assert_failed(&run, &address, "MalformedResponse");
}

#[test]
fn case_5_ftp_address_fails_unsupported_scheme() {
    let address = "ftp://127.0.0.1/file";
    let run = surf(&[address]);
    assert_failed(&run, address, "UnsupportedScheme");
}

#[test]
fn case_6_no_argument_is_a_usage_error() {
    let run = surf(&[]);
    assert_eq!(
        run.status, 2,
        "stdout {:?} stderr {:?}",
        run.stdout, run.stderr
    );
    assert_eq!(run.stdout, "");
    assert_eq!(run.stderr, "usage: surf <url>\n");
}
