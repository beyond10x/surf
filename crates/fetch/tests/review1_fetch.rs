//! Review 1 (adversary): limits exactly at and one past their bound, the media type of
//! `surf.net.MediaType` for edge spellings of `Content-Type`, and a limit at `usize::MAX`.
//! Every server is started here on `127.0.0.1` port 0 and ends on its own.

use std::io::{ErrorKind, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use surf_fetch::{Failure, Limits, Response, fetch};

const BOUND: Duration = Duration::from_secs(5);

struct Server {
    address: String,
    handle: JoinHandle<()>,
}

/// Accept one connection, read the request head, write `reply`, then close (`close`) or wait
/// for the client to close.
fn serve(reply: Vec<u8>, close: bool) -> Server {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("address").port();
    listener.set_nonblocking(true).expect("nonblocking");
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
        stream.set_nonblocking(false).expect("blocking");
        stream.set_read_timeout(Some(BOUND)).expect("timeout");
        stream.set_write_timeout(Some(BOUND)).expect("timeout");
        read_request(&mut stream);
        let _ = stream.write_all(&reply);
        let _ = stream.flush();
        if !close {
            let mut buf = [0u8; 1024];
            while let Ok(n) = stream.read(&mut buf) {
                if n == 0 {
                    break;
                }
            }
        }
    });
    Server {
        address: format!("http://127.0.0.1:{port}/"),
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

fn limits(max_header_bytes: usize, max_body_bytes: usize) -> Limits {
    Limits {
        connect_timeout: Duration::from_secs(2),
        read_timeout: Duration::from_secs(2),
        max_header_bytes,
        max_body_bytes,
    }
}

fn run(reply: &[u8], close: bool, limits: &Limits) -> Result<Response, Failure> {
    let server = serve(reply.to_vec(), close);
    let result = fetch(&server.address, limits);
    server.handle.join().expect("server thread");
    result
}

fn ok(media_type: &str, body: &[u8]) -> Result<Response, Failure> {
    Ok(Response {
        status: 200,
        media_type: media_type.to_owned(),
        body: body.to_vec(),
    })
}

fn is_malformed(result: &Result<Response, Failure>) -> bool {
    matches!(result, Err(Failure::MalformedResponse(_)))
}

#[test]
fn review1_header_section_exactly_at_the_limit_is_a_response_and_one_byte_less_is_malformed() {
    let head = b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n";
    let mut reply = head.to_vec();
    reply.extend_from_slice(b"ok");
    let at = run(&reply, false, &limits(head.len(), 16));
    assert_eq!(at, ok("application/octet-stream", b"ok"));
    let under = run(&reply, false, &limits(head.len() - 1, 16));
    assert!(is_malformed(&under), "{under:?}");
}

#[test]
fn review1_body_exactly_at_the_limit_is_a_response_by_length_by_close_and_chunked() {
    let by_length = run(b"HTTP/1.1 200 OK\r\nContent-Length: 4\r\n\r\nabcd", false, &limits(1024, 4));
    assert_eq!(by_length, ok("application/octet-stream", b"abcd"));
    let by_close = run(b"HTTP/1.1 200 OK\r\n\r\nabcd", true, &limits(1024, 4));
    assert_eq!(by_close, ok("application/octet-stream", b"abcd"));
    let chunked = run(
        b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n2\r\nab\r\n2\r\ncd\r\n0\r\n\r\n",
        false,
        &limits(1024, 4),
    );
    assert_eq!(chunked, ok("application/octet-stream", b"abcd"));
}

#[test]
fn review1_body_one_byte_past_the_limit_is_malformed_by_length_by_close_and_chunked() {
    let by_length = run(b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\n\r\nabcde", false, &limits(1024, 4));
    assert!(is_malformed(&by_length), "{by_length:?}");
    let by_close = run(b"HTTP/1.1 200 OK\r\n\r\nabcde", true, &limits(1024, 4));
    assert!(is_malformed(&by_close), "{by_close:?}");
    let chunked = run(
        b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n2\r\nab\r\n3\r\ncde\r\n0\r\n\r\n",
        false,
        &limits(1024, 4),
    );
    assert!(is_malformed(&chunked), "{chunked:?}");
}

/// `surf.net.MediaType`: "the type and subtype of its Content-Type header in lower case, without
/// parameters". The binding `parse-on-navigation-committed` compares it with `text/html`.
#[test]
fn review1_media_type_is_lower_case_type_and_subtype_for_edge_spellings() {
    let cases: &[(&[u8], &str)] = &[
        (b"text/html ", "text/html"),
        (b"\ttext/html\t", "text/html"),
        (b"TEXT/HTML", "text/html"),
        (b"text/html;charset=utf-8", "text/html"),
        (b"text/html ; charset=\"a;b\"", "text/html"),
        (b"text/html;", "text/html"),
        (b"Text/Html; Charset=ISO-8859-1 ", "text/html"),
        (b"text/html\xff", "application/octet-stream"),
        (b"text /html", "application/octet-stream"),
        (b"", "application/octet-stream"),
    ];
    for (value, expected) in cases {
        let mut reply = b"HTTP/1.1 200 OK\r\nContent-Type:".to_vec();
        reply.extend_from_slice(value);
        reply.extend_from_slice(b"\r\nContent-Length: 2\r\n\r\nok");
        let result = run(&reply, false, &limits(1024, 16));
        assert_eq!(result, ok(expected, b"ok"), "Content-Type {:?}", String::from_utf8_lossy(value));
    }
}

/// The crate documentation: "Every limit below ends the fetch with a Failure; none of them is a
/// hang or an unbounded allocation, and no input makes this crate panic." `Limits` is public and
/// every field is the caller's; `usize::MAX` is the natural "no body limit".
///
/// The body read until close computes `max - body.len() + 1` while the body is still empty,
/// which happens when no body byte arrived with the head: here an empty body.
#[test]
fn review1_body_limit_at_usize_max_reads_a_body_ended_by_close_without_panicking() {
    let result = std::panic::catch_unwind(|| {
        run(b"HTTP/1.1 200 OK\r\n\r\n", true, &limits(1024, usize::MAX))
    });
    match result {
        Ok(result) => assert_eq!(result, ok("application/octet-stream", b"")),
        Err(panic) => {
            let message = panic
                .downcast_ref::<String>()
                .map(String::as_str)
                .or_else(|| panic.downcast_ref::<&str>().copied())
                .unwrap_or("?");
            panic!("fetch panicked with max_body_bytes = usize::MAX: {message}");
        }
    }
}
