//! The whole-fetch time bound and the body buffer bound of `surf-fetch`, through the public API.
//! `read_timeout` bounds sending the request and receiving the response to its last byte;
//! running out of it is `Unreachable`. The body buffer never allocates more than the body limit
//! plus a few KiB, for every framing. Every server is started here on `127.0.0.1` port 0 and ends
//! on its own within `BOUND`.

use std::io::{ErrorKind, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use surf_fetch::{fetch, Failure, Limits, Response};

const BOUND: Duration = Duration::from_secs(10);

/// The read timeout of the drip cases.
const READ_TIMEOUT: Duration = Duration::from_millis(200);

/// Accept one connection on `listener`, hand the stream to `serve`, then read until the client
/// closes.
fn accept_then<F>(listener: TcpListener, serve: F) -> JoinHandle<()>
where
    F: FnOnce(&mut TcpStream) + Send + 'static,
{
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
        stream.set_write_timeout(Some(BOUND)).expect("timeout");
        serve(&mut stream);
        let mut buf = [0u8; 1024];
        while let Ok(n) = stream.read(&mut buf) {
            if n == 0 {
                break;
            }
        }
    })
}

/// A server that reads the request header section, then hands the stream to `respond`.
fn serve<F>(respond: F) -> (String, JoinHandle<()>)
where
    F: FnOnce(&mut TcpStream) + Send + 'static,
{
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let address = format!(
        "http://127.0.0.1:{}/",
        listener.local_addr().expect("address").port()
    );
    let handle = accept_then(listener, move |s| {
        let mut request = Vec::new();
        let mut buf = [0u8; 1024];
        while !request.windows(4).any(|w| w == b"\r\n\r\n") && request.len() < 64 * 1024 {
            match s.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => request.extend_from_slice(&buf[..n]),
            }
        }
        respond(s);
    });
    (address, handle)
}

fn limits() -> Limits {
    Limits {
        connect_timeout: Duration::from_secs(2),
        read_timeout: READ_TIMEOUT,
        max_header_bytes: 1024,
        max_body_bytes: 1024,
        max_request_body_bytes: 16 * 1024 * 1024,
    }
}

/// Write `head` at once, then `drip` one byte per half read timeout.
fn drip(head: &'static [u8], drip: &'static [u8]) -> (String, JoinHandle<()>) {
    serve(move |s| {
        if s.write_all(head).is_err() {
            return;
        }
        for byte in drip {
            thread::sleep(READ_TIMEOUT / 2);
            if s.write_all(&[*byte]).is_err() {
                return;
            }
        }
    })
}

/// Fetch `address` with `limits` and assert it ends `Unreachable` within five read timeouts.
fn assert_ends_unreachable_in_time(address: &str, limits: &Limits, case: &str) {
    let start = Instant::now();
    let result = fetch(address, limits);
    let elapsed = start.elapsed();
    assert!(
        elapsed < limits.read_timeout * 5,
        "{case}: one fetch took {elapsed:?} with a read timeout of {:?} and ended {:?}",
        limits.read_timeout,
        result.map(|r| r.body.len())
    );
    assert!(
        matches!(result, Err(Failure::Unreachable(_))),
        "{case}: expected Unreachable, got {result:?}"
    );
}

/// A header section dripped one byte per half read timeout: every read waits less than the
/// read timeout, and the fetch still ends at it.
#[test]
fn a_header_section_dripped_within_each_read_timeout_ends_at_the_read_timeout() {
    let (address, server) = drip(
        b"HTTP/1.1 200 OK\r\n",
        b"X-Slow: aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\r\n\r\n",
    );
    assert_ends_unreachable_in_time(&address, &limits(), "header drip");
    server.join().expect("server");
}

/// A chunked body dripped one byte per half read timeout.
#[test]
fn a_chunked_body_dripped_within_each_read_timeout_ends_at_the_read_timeout() {
    let (address, server) = drip(
        b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n28\r\n",
        b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\r\n0\r\n\r\n",
    );
    assert_ends_unreachable_in_time(&address, &limits(), "chunked drip");
    server.join().expect("server");
}

/// A body read until the connection closes, dripped one byte per half read timeout.
#[test]
fn a_close_delimited_body_dripped_within_each_read_timeout_ends_at_the_read_timeout() {
    let (address, server) = drip(
        b"HTTP/1.1 200 OK\r\n\r\n",
        b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    );
    assert_ends_unreachable_in_time(&address, &limits(), "close-delimited drip");
    server.join().expect("server");
}

/// A server that accepts and never reads. The request is larger than the socket buffers hold,
/// so sending it blocks; the read timeout bounds that too.
#[test]
fn a_server_that_never_reads_the_request_ends_at_the_read_timeout() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("address").port();
    let (done, wait) = mpsc::channel::<()>();
    let server = accept_then(listener, move |_| {
        let _ = wait.recv_timeout(BOUND);
    });
    let address = format!("http://127.0.0.1:{port}/{}", "a".repeat(16 * 1024 * 1024));
    let start = Instant::now();
    let result = fetch(&address, &limits());
    let elapsed = start.elapsed();
    let _ = done.send(());
    server.join().expect("server");
    assert!(
        elapsed < READ_TIMEOUT * 5,
        "one fetch took {elapsed:?} with a read timeout of {READ_TIMEOUT:?} and ended {result:?}"
    );
    assert!(
        matches!(result, Err(Failure::Unreachable(_))),
        "expected Unreachable, got {result:?}"
    );
}

/// A response that arrives in pieces, each well inside the read timeout and all of them inside
/// it together, is a response: each read is given the time that is left, not less.
#[test]
fn a_response_in_pieces_within_the_read_timeout_is_a_response() {
    let (address, server) = serve(|s| {
        for piece in [
            &b"HTTP/1.1 200 OK\r\n"[..],
            b"Content-Length: 5\r\n\r\n",
            b"he",
            b"llo",
        ] {
            thread::sleep(Duration::from_millis(50));
            if s.write_all(piece).is_err() {
                return;
            }
        }
    });
    let limits = Limits {
        read_timeout: Duration::from_secs(2),
        ..limits()
    };
    let result = fetch(&address, &limits);
    server.join().expect("server");
    assert_eq!(result.map(|r| r.body), Ok(b"hello".to_vec()));
}

/// Serve `head` and then `body` in pieces of 4095 bytes, 20 ms apart, and close.
fn pieces(head: String, body_len: usize) -> (String, JoinHandle<()>) {
    serve(move |s| {
        if s.write_all(head.as_bytes()).is_err() {
            return;
        }
        let mut left = body_len;
        while left > 0 {
            thread::sleep(Duration::from_millis(20));
            let n = left.min(4095);
            if s.write_all(&vec![b'a'; n]).is_err() {
                return;
            }
            left -= n;
        }
        let _ = s.shutdown(std::net::Shutdown::Write);
    })
}

fn body_limits(max_body_bytes: usize) -> Limits {
    Limits {
        read_timeout: Duration::from_secs(5),
        max_body_bytes,
        ..limits()
    }
}

fn assert_capacity(response: Result<Response, Failure>, len: usize, max: usize, case: &str) {
    let response = response.unwrap_or_else(|f| panic!("{case}: {f:?}"));
    assert_eq!(response.body.len(), len, "{case}");
    assert!(
        response.body.capacity() <= max + 9 * 1024,
        "{case}: body limit {max} bytes, body buffer allocated {} bytes",
        response.body.capacity()
    );
}

/// A body read until the connection closes, exactly at the limit.
#[test]
fn a_close_delimited_body_allocates_within_the_body_limit_plus_a_few_kib() {
    let max = 64 * 1024;
    let (address, server) = pieces("HTTP/1.1 200 OK\r\n\r\n".to_string(), max);
    let result = fetch(&address, &body_limits(max));
    server.join().expect("server");
    assert_capacity(result, max, max, "close-delimited");
}

/// A body of a declared length at the limit, where the limit is not a power of two.
#[test]
fn a_body_of_declared_length_allocates_within_the_body_limit_plus_a_few_kib() {
    let max = 40_000;
    let (address, server) = pieces(
        format!("HTTP/1.1 200 OK\r\nContent-Length: {max}\r\n\r\n"),
        max,
    );
    let result = fetch(&address, &body_limits(max));
    server.join().expect("server");
    assert_capacity(result, max, max, "content-length");
}
