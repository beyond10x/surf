//! The adversary's cases on the body of a request the server keeps (`Seen::body`,
//! `story:session-follow-press-https`): the body is the `Content-Length` bytes after the head and
//! not what the client sent after them; a head and a body of exactly the bound of a request
//! (65,536 bytes) are read and kept, one byte more is refused; a body shorter than its
//! `Content-Length` ends the connection at the idle bound of the server, with no response.

use std::net::TcpStream;
use std::time::{Duration, Instant};

use surf_tls::connection::Connection;
use surf_tls_test_server::{Config, IDLE, Seen, TestServer};

use crate::common::{self, HOST};

/// The bound of a request the server reads, its head and its body together.
const BOUND: usize = 64 * 1024;

/// How long a test waits for the server to finish its side of a connection.
const SEEN_WITHIN: Duration = Duration::from_secs(10);

/// The read timeout of the client in these cases: longer than [`IDLE`], so that the server ends
/// first.
const CLIENT_TIMEOUT: Duration = Duration::from_secs(6);

/// The head of a `POST /` with `length` as its `Content-Length`.
fn post_head(length: usize) -> String {
    format!(
        "POST / HTTP/1.1\r\nHost: {HOST}\r\nContent-Type: application/x-www-form-urlencoded\r\n\
         Content-Length: {length}\r\nConnection: close\r\n\r\n"
    )
}

/// Sends `bytes` in records of at most 4,096 bytes on a fresh connection to a server of the
/// default config, reads until the connection ends, and returns the status line of the
/// response (empty without one), what the server saw and how long the exchange took. A write
/// the server no longer reads is not an error of the case.
fn exchange(bytes: &[u8]) -> (String, Seen, Duration) {
    let server = TestServer::start(Config::default()).expect("the server starts");
    let mut connection: Connection<TcpStream> = Connection::new(
        common::socket(&server, CLIENT_TIMEOUT),
        HOST,
        &common::accepting("ok-ecdsa-p256"),
        common::now(),
    )
    .expect("the handshake completes");
    let started = Instant::now();
    for chunk in bytes.chunks(4096) {
        if connection.write_data(chunk).is_err() {
            break;
        }
    }
    let mut response = Vec::new();
    let mut buf = [0u8; 4096];
    while let Ok(n) = connection.read_data(&mut buf) {
        if n == 0 {
            break;
        }
        response.extend_from_slice(&buf[..n]);
    }
    let took = started.elapsed();
    drop(connection);
    let seen = server.wait_seen(1, SEEN_WITHIN);
    assert_eq!(seen.len(), 1, "the server ended one connection: {seen:?}");
    eprintln!("took {took:?}");
    (
        common::status_line(&response),
        seen.into_iter().next().expect("one"),
        took,
    )
}

/// What the client sends after the `Content-Length` bytes is not part of the body.
#[test]
fn adversary_request_body_ends_at_its_content_length() {
    let request = format!("{}q=blue&r=red", post_head(6));
    let (status, seen, _) = exchange(request.as_bytes());
    assert_eq!(status, "HTTP/1.1 200 OK");
    assert_eq!(seen.request.as_deref(), Some("POST / HTTP/1.1"));
    assert_eq!(
        String::from_utf8_lossy(&seen.body),
        "q=blue",
        "{:?}",
        seen.end
    );
}

/// A head and a body of 65,536 bytes together are read whole and answered; with a
/// `Content-Length` one higher the request is refused `internal_error` before its body is read.
#[test]
fn adversary_request_body_of_exactly_the_bound_is_kept_and_one_byte_more_is_refused() {
    let head_len = post_head(10_000).len();
    let length = BOUND - head_len;
    let head = post_head(length);
    assert_eq!(head.len(), head_len, "the length has five digits");
    let mut request = head.into_bytes();
    request.extend(std::iter::repeat_n(b'a', length));
    assert_eq!(request.len(), BOUND);
    let (status, seen, _) = exchange(&request);
    assert_eq!(status, "HTTP/1.1 200 OK", "{}", seen.end);
    assert_eq!(seen.body.len(), length, "{}", seen.end);

    let over = post_head(length + 1);
    let (status, seen, _) = exchange(over.as_bytes());
    assert_eq!(status, "", "no response");
    assert!(seen.body.is_empty(), "{:?}", seen.end);
    assert!(seen.end.contains("internal_error"), "{}", seen.end);
}

/// A body shorter than its `Content-Length`, the connection left open: the server waits for the
/// rest until its idle bound, then ends the connection without a response and keeps no body.
#[test]
fn adversary_request_body_shorter_than_its_content_length_ends_at_the_idle_bound() {
    let request = format!("{}q=blue", post_head(100));
    let (status, seen, took) = exchange(request.as_bytes());
    assert_eq!(status, "", "no response");
    assert_eq!(seen.request.as_deref(), Some("POST / HTTP/1.1"));
    assert!(seen.body.is_empty(), "{:?}", seen.end);
    assert_eq!(seen.end, "timed out", "the end the idle bound gives");
    assert!(
        took >= IDLE && took < IDLE + Duration::from_secs(2),
        "the connection ended after {took:?}, the idle bound is {IDLE:?}"
    );
}
