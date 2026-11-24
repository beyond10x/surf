//! The body of a request, kept beside its request line (`story:session-follow-press-https`,
//! Change: the server keeps the body of a request so that a test reads what a form sent). The
//! body is the `Content-Length` bytes after the head, in as many records as the client sends
//! them, within the server's bound of a request; a request without `Content-Length` has none.

use std::time::{Duration, Instant};

use surf_tls_test_server::{Config, Seen, TestServer};

use crate::common::{self, HOST};

/// How long a test waits for the server to finish its side of a connection.
const SEEN_WITHIN: Duration = Duration::from_secs(10);

/// Sends `parts` as application data, one record each, on a fresh connection to a server of
/// the default config, reads the response to its end and returns its status line and what the
/// server saw.
fn send(parts: &[&[u8]]) -> (String, Seen) {
    let started = Instant::now();
    let server = TestServer::start(Config::default()).expect("the server starts");
    let mut connection = common::connect(&server, "ok-ecdsa-p256").expect("the handshake");
    for part in parts {
        connection.write_data(part).expect("the client writes");
    }
    let mut response = Vec::new();
    let mut buf = [0u8; 4096];
    while let Ok(n) = connection.read_data(&mut buf) {
        if n == 0 {
            break;
        }
        response.extend_from_slice(&buf[..n]);
    }
    drop(connection);
    let seen = server.wait_seen(1, SEEN_WITHIN);
    assert_eq!(seen.len(), 1, "the server ended one connection: {seen:?}");
    eprintln!("took {:?}", started.elapsed());
    (
        common::status_line(&response),
        seen.into_iter().next().expect("one"),
    )
}

/// The head of a `POST /` with `length` as its `Content-Length`.
fn post_head(length: usize) -> String {
    format!(
        "POST / HTTP/1.1\r\nHost: {HOST}\r\nContent-Type: application/x-www-form-urlencoded\r\n\
         content-length: {length}\r\nConnection: close\r\n\r\n"
    )
}

#[test]
fn request_body_in_the_record_of_the_head_and_in_a_later_one_is_kept_whole() {
    let head = post_head(12);
    let first = format!("{head}q=bl");
    let (status, seen) = send(&[first.as_bytes(), b"ue&r=red"]);
    assert_eq!(status, "HTTP/1.1 200 OK");
    assert_eq!(seen.request.as_deref(), Some("POST / HTTP/1.1"));
    assert_eq!(seen.body, b"q=blue&r=red".to_vec(), "{seen:?}");
}

#[test]
fn request_body_is_empty_for_a_request_without_content_length() {
    let head = format!("GET / HTTP/1.1\r\nHost: {HOST}\r\nConnection: close\r\n\r\n");
    let (status, seen) = send(&[head.as_bytes()]);
    assert_eq!(status, "HTTP/1.1 200 OK");
    assert_eq!(seen.request.as_deref(), Some("GET / HTTP/1.1"));
    assert!(seen.body.is_empty(), "{seen:?}");
}

/// A head and a declared body over the bound of a request (64 KiB) are refused before the
/// body is read: `internal_error`, no response, nothing of the body kept.
#[test]
fn request_body_over_the_bound_of_a_request_is_refused() {
    let head = post_head(64 * 1024);
    let (status, seen) = send(&[head.as_bytes(), b"q=blue"]);
    assert_eq!(status, "", "no response");
    assert!(seen.body.is_empty(), "{seen:?}");
    assert!(seen.end.contains("internal_error"), "{seen:?}");
}
