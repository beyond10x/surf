//! Review 2: each limit of `surf-fetch` at its exact value and one past it, for the header and the
//! three body framings, after the rewrite of review 1; the body buffer of a chunked body; and the
//! IPv6 forms the bracket check admits or refuses. Servers run on `127.0.0.1` port 0 and end on
//! their own.

use std::io::{ErrorKind, Read, Write};
use std::net::{Shutdown, TcpListener};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use surf_fetch::{fetch, Failure, Limits, Response};

const BOUND: Duration = Duration::from_secs(5);

/// Serves `reply` to one connection after reading its request head, then closes its side.
fn serve_on(listener: TcpListener, reply: Vec<u8>) -> JoinHandle<()> {
    listener.set_nonblocking(true).expect("nonblocking");
    thread::spawn(move || {
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
        let mut request = Vec::new();
        let mut buf = [0u8; 1024];
        while !request.windows(4).any(|w| w == b"\r\n\r\n") {
            match stream.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => request.extend_from_slice(&buf[..n]),
            }
        }
        let _ = stream.write_all(&reply);
        let _ = stream.shutdown(Shutdown::Write);
        while let Ok(n) = stream.read(&mut buf) {
            if n == 0 {
                break;
            }
        }
    })
}

fn serve(reply: Vec<u8>) -> (String, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("address").port();
    (format!("http://127.0.0.1:{port}/"), serve_on(listener, reply))
}

fn limits(max_header_bytes: usize, max_body_bytes: usize) -> Limits {
    Limits {
        connect_timeout: Duration::from_secs(2),
        read_timeout: Duration::from_secs(5),
        max_header_bytes,
        max_body_bytes,
        max_request_body_bytes: 16 * 1024 * 1024,
    }
}

fn fetch_reply(reply: Vec<u8>, limits: &Limits) -> Result<Response, Failure> {
    let (address, server) = serve(reply);
    let result = fetch(&address, limits);
    server.join().expect("server");
    result
}

/// A head of `len` bytes, the empty line included, declaring an empty body.
fn head_of(len: usize) -> Vec<u8> {
    let start = b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nX: ".to_vec();
    let end = b"\r\n\r\n";
    let pad = len - start.len() - end.len();
    let mut head = start;
    head.extend(std::iter::repeat_n(b'a', pad));
    head.extend_from_slice(end);
    assert_eq!(head.len(), len);
    head
}

#[test]
fn review2_a_head_of_exactly_the_header_limit_is_read_and_one_byte_more_is_not() {
    let len = 300;
    let at = fetch_reply(head_of(len), &limits(len, 1024));
    assert_eq!(at.map(|r| r.status), Ok(200), "head of {len} bytes, limit {len}");
    let over = fetch_reply(head_of(len + 1), &limits(len, 1024));
    assert!(
        matches!(over, Err(Failure::MalformedResponse(_))),
        "head of {} bytes, limit {len}: {over:?}",
        len + 1
    );
}

#[test]
fn review2_a_declared_length_of_exactly_the_body_limit_is_read_and_one_more_is_not() {
    let max = 1000;
    let mut reply = format!("HTTP/1.1 200 OK\r\nContent-Length: {max}\r\n\r\n").into_bytes();
    reply.extend(std::iter::repeat_n(b'b', max));
    let at = fetch_reply(reply, &limits(1024, max)).map(|r| r.body.len());
    assert_eq!(at, Ok(max));
    let mut reply = format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n", max + 1).into_bytes();
    reply.extend(std::iter::repeat_n(b'b', max + 1));
    let over = fetch_reply(reply, &limits(1024, max));
    assert!(matches!(over, Err(Failure::MalformedResponse(_))), "{over:?}");
}

fn chunked(sizes: &[usize]) -> Vec<u8> {
    let mut reply = b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n".to_vec();
    for &size in sizes {
        reply.extend(format!("{size:x}\r\n").into_bytes());
        reply.extend(std::iter::repeat_n(b'c', size));
        reply.extend_from_slice(b"\r\n");
    }
    reply.extend_from_slice(b"0\r\n\r\n");
    reply
}

#[test]
fn review2_a_chunked_body_of_exactly_the_body_limit_is_read_within_it_and_one_more_is_not() {
    let max = 40_000;
    let at = fetch_reply(chunked(&[4_000; 10]), &limits(1024, max))
        .unwrap_or_else(|f| panic!("chunked body of {max} bytes, limit {max}: {f:?}"));
    assert_eq!(at.body.len(), max);
    assert!(
        at.body.capacity() <= max,
        "chunked body limit {max} bytes, body buffer allocated {} bytes",
        at.body.capacity()
    );
    let mut sizes = vec![4_000; 10];
    sizes.push(1);
    let over = fetch_reply(chunked(&sizes), &limits(1024, max));
    assert!(matches!(over, Err(Failure::MalformedResponse(_))), "{over:?}");
}

#[test]
fn review2_a_close_delimited_body_one_byte_over_the_limit_is_refused() {
    let max = 1000;
    let mut reply = b"HTTP/1.1 200 OK\r\n\r\n".to_vec();
    reply.extend(std::iter::repeat_n(b'd', max));
    let at = fetch_reply(reply.clone(), &limits(1024, max)).map(|r| r.body.len());
    assert_eq!(at, Ok(max));
    reply.push(b'd');
    let over = fetch_reply(reply, &limits(1024, max));
    assert!(matches!(over, Err(Failure::MalformedResponse(_))), "{over:?}");
}

/// A chunk-size line of exactly 1024 bytes, its CRLF included, is read; 1025 is not.
#[test]
fn review2_a_chunk_size_line_of_exactly_1024_bytes_is_read_and_1025_is_not() {
    let line = |len: usize| {
        let mut line = b"1;x=".to_vec();
        line.extend(std::iter::repeat_n(b'e', len - line.len() - 2));
        line.extend_from_slice(b"\r\n");
        assert_eq!(line.len(), len);
        line
    };
    let reply = |len: usize| {
        let mut reply = b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n".to_vec();
        reply.extend(line(len));
        reply.extend_from_slice(b"z\r\n0\r\n\r\n");
        reply
    };
    let at = fetch_reply(reply(1024), &limits(1024, 1024)).map(|r| r.body);
    assert_eq!(at, Ok(b"z".to_vec()));
    let over = fetch_reply(reply(1025), &limits(1024, 1024));
    assert!(matches!(over, Err(Failure::MalformedResponse(_))), "{over:?}");
}

/// IPv4-mapped IPv6 in brackets, in lower and upper case, reaches the IPv4 listener it names.
#[test]
fn review2_ipv4_mapped_ipv6_in_brackets_is_admitted_in_any_case() {
    for host in ["[::ffff:127.0.0.1]", "[::FFFF:7F00:1]"] {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let port = listener.local_addr().expect("address").port();
        let server = serve_on(
            listener,
            b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok".to_vec(),
        );
        let result = fetch(&format!("http://{host}:{port}/"), &limits(1024, 1024));
        server.join().expect("server");
        assert_eq!(result.map(|r| r.body), Ok(b"ok".to_vec()), "{host}");
    }
}

/// An empty pair of brackets and a zone identifier are refused before any connection.
#[test]
fn review2_empty_brackets_and_a_zone_identifier_are_refused_without_connecting() {
    for address in ["http://[]/", "http://[]:80/", "http://[fe80::1%25lo]/", "http://[::1%lo]/"] {
        let start = Instant::now();
        let result = fetch(address, &limits(1024, 1024));
        assert!(
            matches!(&result, Err(Failure::Unreachable(detail)) if detail.contains("brackets")),
            "{address}: {result:?}"
        );
        assert!(start.elapsed() < Duration::from_secs(1), "{address}");
    }
}
