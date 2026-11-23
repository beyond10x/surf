//! The cases of story `http-fetch-library`, each against a server the test starts itself on
//! `127.0.0.1` port 0. Every server thread ends on its own: it waits at most `BOUND` for a
//! connection and at most `BOUND` for the client to close.

use std::io::{ErrorKind, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use surf_fetch::{fetch, Failure, Limits, Method, Response};

/// The longest any server thread waits for anything.
const BOUND: Duration = Duration::from_secs(5);

/// The read timeout of the stall cases.
const SHORT: Duration = Duration::from_millis(300);

struct Server {
    address: String,
    port: u16,
    handle: JoinHandle<Vec<u8>>,
}

/// Start a server that accepts one connection, reads the request header section, hands the
/// stream to `respond`, and returns the request bytes it read.
fn serve<F>(respond: F) -> Server
where
    F: FnOnce(&mut TcpStream) + Send + 'static,
{
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
                Err(_) => return Vec::new(),
            }
        };
        drop(listener);
        stream.set_nonblocking(false).expect("blocking stream");
        stream.set_read_timeout(Some(BOUND)).expect("read timeout");
        stream
            .set_write_timeout(Some(BOUND))
            .expect("write timeout");
        let request = read_request(&mut stream);
        respond(&mut stream);
        request
    });
    Server {
        address: format!("http://127.0.0.1:{port}"),
        port,
        handle,
    }
}

/// Join the server thread and return the request it read.
fn finish(server: Server) -> Vec<u8> {
    server.handle.join().expect("server thread")
}

fn read_request(stream: &mut TcpStream) -> Vec<u8> {
    let mut request = Vec::new();
    let mut buf = [0u8; 1024];
    while !request.windows(4).any(|w| w == b"\r\n\r\n") && request.len() < 64 * 1024 {
        match stream.read(&mut buf) {
            Ok(0) | Err(_) => break,
            Ok(n) => request.extend_from_slice(&buf[..n]),
        }
    }
    request
}

/// Write bytes, ignoring a client that has already gone.
fn send(stream: &mut TcpStream, bytes: &[u8]) {
    let _ = stream.write_all(bytes);
    let _ = stream.flush();
}

/// Read and discard until the client closes, or `BOUND` passes.
fn wait_for_close(stream: &mut TcpStream) {
    let mut buf = [0u8; 1024];
    while let Ok(n) = stream.read(&mut buf) {
        if n == 0 {
            break;
        }
    }
}

fn quick() -> Limits {
    Limits {
        connect_timeout: Duration::from_secs(2),
        read_timeout: SHORT,
        ..Limits::default()
    }
}

/// The response to `asked`, which nothing redirected: its `url` is `asked`.
fn response(asked: &str, status: u16, media_type: &str, body: &[u8]) -> Result<Response, Failure> {
    Ok(Response {
        status,
        media_type: media_type.to_string(),
        body: body.to_vec(),
        url: asked.to_string(),
        method: Method::Get,
    })
}

fn assert_unreachable(result: &Result<Response, Failure>) {
    assert!(
        matches!(result, Err(Failure::Unreachable(_))),
        "expected Unreachable, got {result:?}"
    );
}

fn assert_malformed(result: &Result<Response, Failure>) {
    assert!(
        matches!(result, Err(Failure::MalformedResponse(_))),
        "expected MalformedResponse, got {result:?}"
    );
}

#[test]
fn case_01_content_length_response() {
    let server = serve(|s| {
        send(
            s,
            b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\nContent-Type: text/plain\r\n\r\nhello",
        );
        wait_for_close(s);
    });
    let port = server.port;
    let asked = format!("{}/path/x?q=1#frag", server.address);
    let result = fetch(&asked, &quick());
    let request = String::from_utf8(finish(server)).expect("request is text");
    assert_eq!(result, response(&asked, 200, "text/plain", b"hello"));
    assert!(
        request.starts_with("GET /path/x?q=1 HTTP/1.1\r\n"),
        "request line: {request:?}"
    );
    assert!(
        request.contains(&format!("\r\nHost: 127.0.0.1:{port}\r\n")),
        "host: {request:?}"
    );
    assert!(
        request.contains("\r\nConnection: close\r\n"),
        "connection: {request:?}"
    );
}

#[test]
fn case_02_chunked_response() {
    let server = serve(|s| {
        send(
            s,
            b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nContent-Type: text/plain\r\n\r\n",
        );
        send(s, b"5\r\nhello\r\n");
        send(s, b"7;name=value\r\n, world\r\n");
        send(s, b"A\r\n0123456789\r\n0\r\n\r\n");
        wait_for_close(s);
    });
    let asked = server.address.clone();
    let result = fetch(&asked, &quick());
    let request = String::from_utf8(finish(server)).expect("request is text");
    assert_eq!(
        result,
        response(&asked, 200, "text/plain", b"hello, world0123456789")
    );
    assert!(request.starts_with("GET / HTTP/1.1\r\n"), "{request:?}");
}

#[test]
fn case_03_response_ended_by_close() {
    let server = serve(|s| {
        send(
            s,
            b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n\r\nuntil the connection closes",
        );
    });
    let asked = server.address.clone();
    let result = fetch(&asked, &quick());
    finish(server);
    assert_eq!(
        result,
        response(&asked, 200, "text/plain", b"until the connection closes")
    );
}

#[test]
fn case_04_content_type_with_parameters_and_mixed_case() {
    let server = serve(|s| {
        send(
            s,
            b"HTTP/1.1 200 OK\r\nContent-Type:  Text/HTML ; Charset=UTF-8\r\nContent-Length: 2\r\n\r\nhi",
        );
        wait_for_close(s);
    });
    let asked = server.address.clone();
    let result = fetch(&asked, &quick());
    finish(server);
    assert_eq!(result, response(&asked, 200, "text/html", b"hi"));
}

#[test]
fn case_05_no_content_type() {
    let server = serve(|s| {
        send(
            s,
            b"HTTP/1.1 200 OK\r\nContent-Length: 3\r\n\r\n\x00\x01\x02",
        );
        wait_for_close(s);
    });
    let asked = server.address.clone();
    let result = fetch(&asked, &quick());
    finish(server);
    assert_eq!(
        result,
        response(&asked, 200, "application/octet-stream", b"\x00\x01\x02")
    );
}

#[test]
fn case_06_not_found_is_a_response() {
    let server = serve(|s| {
        send(
            s,
            b"HTTP/1.1 404 Not Found\r\nContent-Type: text/plain\r\nContent-Length: 9\r\n\r\nnot found",
        );
        wait_for_close(s);
    });
    let asked = format!("{}/missing", server.address);
    let result = fetch(&asked, &quick());
    finish(server);
    assert_eq!(result, response(&asked, 404, "text/plain", b"not found"));
}

#[test]
fn case_07_refused_connection_is_unreachable() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("local address").port();
    drop(listener);
    let result = fetch(&format!("http://127.0.0.1:{port}/"), &quick());
    assert_unreachable(&result);
}

#[test]
fn case_08_closed_mid_body_is_unreachable() {
    let server = serve(|s| {
        send(
            s,
            b"HTTP/1.1 200 OK\r\nContent-Length: 100\r\n\r\nonly ten b",
        );
    });
    let result = fetch(&server.address, &quick());
    finish(server);
    assert_unreachable(&result);
}

#[test]
fn case_09_silent_server_times_out() {
    let server = serve(wait_for_close);
    let started = Instant::now();
    let result = fetch(&server.address, &quick());
    let elapsed = started.elapsed();
    finish(server);
    assert_unreachable(&result);
    assert!(elapsed < Duration::from_secs(5), "took {elapsed:?}");
}

#[test]
fn case_10_stall_mid_body_times_out() {
    let server = serve(|s| {
        send(
            s,
            b"HTTP/1.1 200 OK\r\nContent-Length: 100\r\n\r\nonly ten b",
        );
        wait_for_close(s);
    });
    let started = Instant::now();
    let result = fetch(&server.address, &quick());
    let elapsed = started.elapsed();
    finish(server);
    assert_unreachable(&result);
    assert!(elapsed < Duration::from_secs(5), "took {elapsed:?}");
}

#[test]
fn case_11_non_http_reply_is_malformed() {
    let server = serve(|s| {
        send(s, b"SSH-2.0-OpenSSH_9.6\r\n");
        wait_for_close(s);
    });
    let result = fetch(&server.address, &quick());
    finish(server);
    assert_malformed(&result);
}

#[test]
fn case_12_header_block_over_limit_is_malformed() {
    let server = serve(|s| {
        let mut reply = b"HTTP/1.1 200 OK\r\nX-Big: ".to_vec();
        reply.extend(std::iter::repeat_n(b'a', 4096));
        reply.extend_from_slice(b"\r\nContent-Length: 0\r\n\r\n");
        send(s, &reply);
        wait_for_close(s);
    });
    let limits = Limits {
        max_header_bytes: 1024,
        ..quick()
    };
    let result = fetch(&server.address, &limits);
    finish(server);
    assert_malformed(&result);
}

#[test]
fn case_13_body_over_limit_by_content_length() {
    let server = serve(|s| {
        send(s, b"HTTP/1.1 200 OK\r\nContent-Length: 1000000\r\n\r\n");
        wait_for_close(s);
    });
    let limits = Limits {
        max_body_bytes: 1000,
        read_timeout: Duration::from_secs(10),
        ..quick()
    };
    let started = Instant::now();
    let result = fetch(&server.address, &limits);
    let elapsed = started.elapsed();
    finish(server);
    assert_malformed(&result);
    assert!(
        elapsed < Duration::from_secs(5),
        "the body was waited for: {elapsed:?}"
    );
}

#[test]
fn case_13_body_over_limit_by_close() {
    let server = serve(|s| {
        send(s, b"HTTP/1.1 200 OK\r\n\r\n");
        send(s, &[b'x'; 5000]);
    });
    let limits = Limits {
        max_body_bytes: 1000,
        ..quick()
    };
    let result = fetch(&server.address, &limits);
    finish(server);
    assert_malformed(&result);
}

#[test]
fn case_13_body_over_limit_by_chunk_size() {
    let server = serve(|s| {
        send(
            s,
            b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n200\r\n",
        );
        wait_for_close(s);
    });
    let limits = Limits {
        max_body_bytes: 500,
        read_timeout: Duration::from_secs(10),
        ..quick()
    };
    let started = Instant::now();
    let result = fetch(&server.address, &limits);
    let elapsed = started.elapsed();
    finish(server);
    assert_malformed(&result);
    assert!(
        elapsed < Duration::from_secs(5),
        "the chunk was waited for: {elapsed:?}"
    );
}

#[test]
fn case_14_other_scheme_is_unsupported_without_connecting() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("local address").port();
    listener
        .set_nonblocking(true)
        .expect("nonblocking listener");
    for address in [
        format!("https://127.0.0.1:{port}/"),
        format!("ftp://127.0.0.1:{port}/"),
        format!("httpx://127.0.0.1:{port}/"),
        "data:text/plain,hello".to_string(),
        String::new(),
    ] {
        let result = fetch(&address, &quick());
        assert!(
            matches!(result, Err(Failure::UnsupportedScheme(_))),
            "{address:?}: expected UnsupportedScheme, got {result:?}"
        );
    }
    match listener.accept() {
        Err(e) if e.kind() == ErrorKind::WouldBlock => {}
        other => panic!("a connection was attempted: {other:?}"),
    }
}

// Beyond the fourteen cases: framing and address rules the implementation relies on.

#[test]
fn scheme_is_case_insensitive() {
    let server = serve(|s| {
        send(s, b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok");
        wait_for_close(s);
    });
    let address = server.address.replacen("http:", "HTTP:", 1);
    let result = fetch(&address, &quick());
    finish(server);
    assert_eq!(result, response(&address, 200, "application/octet-stream", b"ok"));
}

#[test]
fn interim_response_is_skipped() {
    let server = serve(|s| {
        send(
            s,
            b"HTTP/1.1 103 Early Hints\r\nLink: </a>\r\n\r\nHTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nok",
        );
        wait_for_close(s);
    });
    let asked = server.address.clone();
    let result = fetch(&asked, &quick());
    finish(server);
    assert_eq!(result, response(&asked, 200, "application/octet-stream", b"ok"));
}

#[test]
fn no_content_has_no_body_and_does_not_wait() {
    let server = serve(|s| {
        send(s, b"HTTP/1.1 204 No Content\r\n\r\n");
        wait_for_close(s);
    });
    let limits = Limits {
        read_timeout: Duration::from_secs(10),
        ..quick()
    };
    let started = Instant::now();
    let asked = server.address.clone();
    let result = fetch(&asked, &limits);
    let elapsed = started.elapsed();
    finish(server);
    assert_eq!(result, response(&asked, 204, "application/octet-stream", b""));
    assert!(elapsed < Duration::from_secs(5), "took {elapsed:?}");
}

#[test]
fn conflicting_content_lengths_are_malformed() {
    let server = serve(|s| {
        send(
            s,
            b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nContent-Length: 3\r\n\r\nabc",
        );
        wait_for_close(s);
    });
    let result = fetch(&server.address, &quick());
    finish(server);
    assert_malformed(&result);
}

#[test]
fn chunk_size_overflow_is_malformed() {
    let server = serve(|s| {
        send(
            s,
            b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\nFFFFFFFFFFFFFFFFFFFFFFFF\r\n",
        );
        wait_for_close(s);
    });
    let result = fetch(&server.address, &quick());
    finish(server);
    assert_malformed(&result);
}

#[test]
fn status_outside_range_is_malformed() {
    let server = serve(|s| {
        send(s, b"HTTP/1.1 700 Custom\r\nContent-Length: 0\r\n\r\n");
        wait_for_close(s);
    });
    let result = fetch(&server.address, &quick());
    finish(server);
    assert_malformed(&result);
}

#[test]
fn unsupported_transfer_coding_is_malformed() {
    let server = serve(|s| {
        send(
            s,
            b"HTTP/1.1 200 OK\r\nTransfer-Encoding: gzip, chunked\r\n\r\n0\r\n\r\n",
        );
        wait_for_close(s);
    });
    let result = fetch(&server.address, &quick());
    finish(server);
    assert_malformed(&result);
}

#[test]
fn address_with_userinfo_or_control_bytes_is_refused_without_connecting() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("local address").port();
    listener
        .set_nonblocking(true)
        .expect("nonblocking listener");
    for address in [
        format!("http://user:secret@127.0.0.1:{port}/"),
        format!("http://127.0.0.1:{port}/a\r\nX-Injected: 1"),
        format!("http://127.0.0.1:{port}/a b"),
        "http:///no-host".to_string(),
        "http://127.0.0.1:99999/".to_string(),
    ] {
        let result = fetch(&address, &quick());
        assert_unreachable(&result);
    }
    match listener.accept() {
        Err(e) if e.kind() == ErrorKind::WouldBlock => {}
        other => panic!("a connection was attempted: {other:?}"),
    }
}
