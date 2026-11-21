//! Review 1, security reviewer: conformance checks of `surf-fetch` against the bounds its crate
//! documentation states, through the public API only. Every server is started here on
//! `127.0.0.1` port 0 and ends on its own within `BOUND`.

use std::io::{ErrorKind, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use surf_fetch::{fetch, Failure, Limits, Response};

const BOUND: Duration = Duration::from_secs(10);

struct Server {
    port: u16,
    handle: JoinHandle<Vec<u8>>,
}

/// Accept one connection, read the request header section, hand the stream to `respond`, then
/// read until the client closes. Returns the request bytes.
fn serve<F>(respond: F) -> Server
where
    F: FnOnce(&mut TcpStream) + Send + 'static,
{
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
                Err(_) => return Vec::new(),
            }
        };
        drop(listener);
        stream.set_nonblocking(false).expect("blocking");
        stream.set_read_timeout(Some(BOUND)).expect("timeout");
        stream.set_write_timeout(Some(BOUND)).expect("timeout");
        let mut request = Vec::new();
        let mut buf = [0u8; 1024];
        while !request.windows(4).any(|w| w == b"\r\n\r\n") && request.len() < 64 * 1024 {
            match stream.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => request.extend_from_slice(&buf[..n]),
            }
        }
        respond(&mut stream);
        let _ = stream.flush();
        while let Ok(n) = stream.read(&mut buf) {
            if n == 0 {
                break;
            }
        }
        request
    });
    Server { port, handle }
}

fn reply(bytes: Vec<u8>) -> Server {
    serve(move |s| {
        let _ = s.write_all(&bytes);
    })
}

fn limits() -> Limits {
    Limits {
        connect_timeout: Duration::from_secs(2),
        read_timeout: Duration::from_millis(500),
        max_header_bytes: 1024,
        max_body_bytes: 64,
    }
}

fn get(server: &Server, limits: &Limits) -> Result<Response, Failure> {
    fetch(&format!("http://127.0.0.1:{}/", server.port), limits)
}

fn malformed(result: &Result<Response, Failure>, case: &str) {
    assert!(
        matches!(result, Err(Failure::MalformedResponse(_))),
        "{case}: expected MalformedResponse, got {result:?}"
    );
}

fn run(bytes: &[u8]) -> Result<Response, Failure> {
    let server = reply(bytes.to_vec());
    let result = get(&server, &limits());
    server.handle.join().expect("server");
    result
}

// ---- invariant 2: what an address can put into the request -----------------------------------

/// A listener nothing may connect to: every address below must be refused before connecting.
#[test]
fn review1_addresses_with_injected_bytes_bad_ports_or_userinfo_are_refused_without_connecting() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    listener.set_nonblocking(true).expect("nonblocking");
    let p = listener.local_addr().expect("address").port();
    let wrapped = u32::from(p) + 65536;
    let addresses = [
        format!("http://127.0.0.1:{p}/a b"),
        format!("http://127.0.0.1:{p}/a\r\nX-Injected: 1"),
        format!("http://127.0.0.1:{p}/a\nX-Injected: 1"),
        format!("http://127.0.0.1:{p}/a\rX-Injected: 1"),
        format!("http://127.0.0.1:{p}/\t"),
        format!("http://127.0.0.1:{p}/\0"),
        format!("http://127.0.0.1:{p}/\x7f"),
        format!("http://127.0.0.1:{p}/\u{85}"),
        format!("http://127.0.0.1:{p}/\u{2028}"),
        format!("http://127.0.0.1:{p}/\u{e9}"),
        format!("http://127.0.0.1\r\nX: 1:{p}/"),
        format!("http://127.0.0.1:{p}\r\n/"),
        format!("http://127.0.0.1:{p} /"),
        format!("http://127.0.0.1 :{p}/"),
        format!("http://user@127.0.0.1:{p}/"),
        format!("http://user:pw@127.0.0.1:{p}/"),
        format!("http://127.0.0.1:{p}@127.0.0.1:{p}/"),
        format!("http://x\\@127.0.0.1:{p}/"),
        format!("http://127.0.0.1:{p}\\x/"),
        format!("http:///127.0.0.1:{p}/"),
        format!("http:\\\\127.0.0.1:{p}/"),
        format!("http://127.0.0.1:{wrapped}/"),
        format!("http://127.0.0.1:+{p}/"),
        format!("http://127.0.0.1:-{p}/"),
        format!("http://127.0.0.1:{p}:{p}/"),
        format!("http://127.0.0.1:0x{p:x}/"),
        "http://127.0.0.1:0/".to_string(),
        "http://127.0.0.1:00/".to_string(),
        "http://127.0.0.1:65536/".to_string(),
        "http://127.0.0.1:4294967296/".to_string(),
        format!("http://[::1/:{p}"),
        format!("http://[::1]x:{p}/"),
    ];
    for address in &addresses {
        let result = fetch(address, &limits());
        assert!(
            matches!(&result, Err(Failure::Unreachable(d)) if !d.contains("connecting") && !d.contains("resolve")),
            "{address:?}: expected a refusal before connecting, got {result:?}"
        );
        assert!(
            matches!(listener.accept(), Err(e) if e.kind() == ErrorKind::WouldBlock),
            "{address:?} connected"
        );
    }
}

/// Every request this crate sends is exactly the request line, `Host` and `Connection: close`,
/// with printable ASCII between the line terminators.
#[test]
fn review1_request_bytes_are_exactly_the_three_lines_and_printable() {
    let cases: [(&str, &str, &str); 6] = [
        ("http://127.0.0.1:{p}", "/", "127.0.0.1:{p}"),
        ("HTTP://127.0.0.1:{p}?a=%0d%0a", "/?a=%0d%0a", "127.0.0.1:{p}"),
        ("http://127.0.0.1:{p}/x%20y#frag", "/x%20y", "127.0.0.1:{p}"),
        ("http://127.0.0.1:{p}#@evil.invalid/", "/", "127.0.0.1:{p}"),
        ("http://127.0.0.1:0{p}/p", "/p", "127.0.0.1:0{p}"),
        ("http://127.0.0.1:{p}/a?b#c?d", "/a?b", "127.0.0.1:{p}"),
    ];
    for (address, target, host) in cases {
        let server = reply(b"HTTP/1.1 204 No Content\r\n\r\n".to_vec());
        let p = server.port.to_string();
        let address = address.replace("{p}", &p);
        let result = fetch(&address, &limits());
        let request = server.handle.join().expect("server");
        assert!(result.is_ok(), "{address:?}: {result:?}");
        let expected = format!(
            "GET {target} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
            host.replace("{p}", &p)
        );
        assert_eq!(String::from_utf8_lossy(&request), expected, "{address:?}");
    }
}

/// RFC 3986 section 3.2.2: an IP-literal in brackets is an IPv6 address or IPvFuture. A
/// bracketed IPv4 address or name is not a host, so the address is refused before connecting
/// and is never sent as `Host`.
#[test]
fn review1_bracketed_host_that_is_not_ipv6_is_refused_without_connecting() {
    for host in ["[127.0.0.1]", "[localhost]"] {
        let server = reply(b"HTTP/1.1 204 No Content\r\n\r\n".to_vec());
        let address = format!("http://{host}:{}/", server.port);
        let result = fetch(&address, &limits());
        let request = server.handle.join().expect("server");
        assert!(
            matches!(&result, Err(Failure::Unreachable(_))) && request.is_empty(),
            "{address:?}: connected and sent {:?}, result {result:?}",
            String::from_utf8_lossy(&request)
        );
    }
}

// ---- invariant 1: what a server can make a fetch do ------------------------------------------

#[test]
fn review1_endless_interim_responses_end_at_the_header_limit() {
    let server = serve(|s| {
        for _ in 0..100_000 {
            if s.write_all(b"HTTP/1.1 100 Continue\r\n\r\n").is_err() {
                return;
            }
        }
    });
    let start = Instant::now();
    let result = get(&server, &limits());
    malformed(&result, "endless 1xx");
    assert!(start.elapsed() < Duration::from_secs(3));
    server.handle.join().expect("server");
}

#[test]
fn review1_endless_header_line_and_endless_fields_end_at_the_header_limit() {
    for unit in [&b"a"[..], b"X: y\r\n", b"X: y\n", b" folded\r\n"] {
        let unit = unit.to_vec();
        let server = serve(move |s| {
            if s.write_all(b"HTTP/1.1 200 OK\r\nA: b\r\n").is_err() {
                return;
            }
            for _ in 0..200_000 {
                if s.write_all(&unit).is_err() {
                    return;
                }
            }
        });
        let start = Instant::now();
        let result = get(&server, &limits());
        malformed(&result, "endless header");
        assert!(start.elapsed() < Duration::from_secs(3));
        server.handle.join().expect("server");
    }
}

#[test]
fn review1_content_length_values_that_are_not_one_decimal_number_are_malformed() {
    for value in [
        "-1",
        "+5",
        "0x5",
        "5 5",
        "5,6",
        "5;",
        "",
        "18446744073709551616",
        "99999999999999999999999999999999",
        "\u{663}",
    ] {
        let bytes = format!("HTTP/1.1 200 OK\r\nContent-Length: {value}\r\n\r\nhello");
        malformed(&run(bytes.as_bytes()), value);
    }
    // In range for u64, far over the body limit: refused before any body byte is awaited.
    let start = Instant::now();
    malformed(
        &run(b"HTTP/1.1 200 OK\r\nContent-Length: 18446744073709551615\r\n\r\n"),
        "u64::MAX",
    );
    assert!(start.elapsed() < Duration::from_millis(400));
    malformed(
        &run(b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\nContent-Length: 6\r\n\r\nhello!"),
        "differing duplicates",
    );
}

#[test]
fn review1_transfer_encoding_rules() {
    // Transfer-Encoding overrides Content-Length (RFC 9112 section 6.3, rule 3).
    assert_eq!(
        run(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nTransfer-Encoding: chunked\r\n\r\n5\r\nhello\r\n0\r\n\r\n")
            .map(|r| r.body),
        Ok(b"hello".to_vec())
    );
    for te in [
        "Transfer-Encoding: chunked\r\nTransfer-Encoding: chunked",
        "Transfer-Encoding: chunked, chunked",
        "Transfer-Encoding: gzip, chunked",
        "Transfer-Encoding: chunked, identity",
        "Transfer-Encoding: identity",
        "Transfer-Encoding:",
        "Transfer-Encoding: chunked\r\n , gzip",
    ] {
        let bytes = format!("HTTP/1.1 200 OK\r\n{te}\r\n\r\n5\r\nhello\r\n0\r\n\r\n");
        malformed(&run(bytes.as_bytes()), te);
    }
}

#[test]
fn review1_body_at_the_limit_is_a_response_and_one_byte_over_is_malformed() {
    let at = vec![b'x'; 64];
    let over = vec![b'x'; 65];
    let by_length = |body: &[u8]| {
        let mut b = format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n", body.len()).into_bytes();
        b.extend_from_slice(body);
        b
    };
    let by_close = |body: &[u8]| {
        let mut b = b"HTTP/1.1 200 OK\r\n\r\n".to_vec();
        b.extend_from_slice(body);
        b
    };
    let by_chunks = |body: &[u8]| {
        let mut b = b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n".to_vec();
        let (one, two) = body.split_at(30);
        for part in [one, two] {
            b.extend_from_slice(format!("{:x}\r\n", part.len()).as_bytes());
            b.extend_from_slice(part);
            b.extend_from_slice(b"\r\n");
        }
        b.extend_from_slice(b"0\r\n\r\n");
        b
    };
    for (framing, build) in [
        ("length", &by_length as &dyn Fn(&[u8]) -> Vec<u8>),
        ("close", &by_close),
        ("chunked", &by_chunks),
    ] {
        let server = serve({
            let bytes = build(&at);
            move |s| {
                let _ = s.write_all(&bytes);
                let _ = s.shutdown(std::net::Shutdown::Write);
            }
        });
        assert_eq!(get(&server, &limits()).map(|r| r.body), Ok(at.clone()), "{framing}");
        server.handle.join().expect("server");
        let server = serve({
            let bytes = build(&over);
            move |s| {
                let _ = s.write_all(&bytes);
                let _ = s.shutdown(std::net::Shutdown::Write);
            }
        });
        malformed(&get(&server, &limits()), framing);
        server.handle.join().expect("server");
    }
}

#[test]
fn review1_bare_cr_anywhere_in_the_head_or_chunk_lines_is_malformed() {
    for bytes in [
        &b"HTTP/1.1 200 OK\r\r\nContent-Length: 0\r\n\r\n"[..],
        b"HTTP/1.1 200\rOK\r\nContent-Length: 0\r\n\r\n",
        b"HTTP/1.1 200 OK\r\nA: b\rc\r\nContent-Length: 0\r\n\r\n",
        b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\r\n",
        b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r",
        b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n5\r\r\nhello\r\n0\r\n\r\n",
        b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n5;a=\rb\r\nhello\r\n0\r\n\r\n",
        b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n5\r\nhello\r\r0\r\n\r\n",
    ] {
        let result = run(bytes);
        assert!(
            matches!(result, Err(Failure::MalformedResponse(_)) | Err(Failure::Unreachable(_))),
            "{:?}: {result:?}",
            String::from_utf8_lossy(bytes)
        );
        assert!(result.is_err(), "{:?} accepted", String::from_utf8_lossy(bytes));
    }
}

#[test]
fn review1_chunk_size_lines_that_are_not_a_hexadecimal_size_are_malformed() {
    for line in ["-1", "+5", "0x5", " 5", "", ";a", "5 5", "FFFFFFFFFFFFFFFFF", "\u{663}"] {
        let bytes = format!(
            "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n{line}\r\nhello\r\n0\r\n\r\n"
        );
        malformed(&run(bytes.as_bytes()), line);
    }
    let long = format!(
        "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n5;{}\r\nhello\r\n0\r\n\r\n",
        "e".repeat(1100)
    );
    malformed(&run(long.as_bytes()), "1100-byte extension");
    let endless = serve(|s| {
        let _ = s.write_all(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n5;");
        for _ in 0..100_000 {
            if s.write_all(b"eeeeeeee").is_err() {
                return;
            }
        }
    });
    malformed(&get(&endless, &limits()), "endless extension");
    endless.handle.join().expect("server");
}

// ---- findings ---------------------------------------------------------------------------------

/// The crate documentation: "Time is bounded per read only: there is no deadline for the whole
/// fetch". A server that sends one byte inside every read timeout keeps one fetch alive for
/// (header bytes + body bytes) read timeouts: at the defaults, 64 KiB x 30 s of header and
/// 16 MiB x 30 s of body. This case asks for a whole-fetch bound of five read timeouts and drips
/// a 40-byte body at one byte per 0.5 read timeout.
#[test]
fn review1_a_drip_fed_response_ends_within_a_bound_of_the_read_timeout() {
    let read_timeout = Duration::from_millis(200);
    let server = serve(|s| {
        let _ = s.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 40\r\n\r\n");
        for _ in 0..40 {
            thread::sleep(Duration::from_millis(100));
            if s.write_all(b"x").is_err() {
                return;
            }
        }
    });
    let limits = Limits {
        read_timeout,
        ..limits()
    };
    let start = Instant::now();
    let result = get(&server, &limits);
    let elapsed = start.elapsed();
    server.handle.join().expect("server");
    assert!(
        elapsed < read_timeout * 5,
        "one fetch took {elapsed:?} with a read timeout of {read_timeout:?} and ended {:?}",
        result.map(|r| r.body.len())
    );
}

/// The crate documentation: "Memory held at once is at most the header limit plus the body
/// limit plus a few KiB". The body buffer a chunked response leaves behind is measured: the
/// server picks the first piece (4095 bytes) so that amortised growth doubles past the limit.
#[test]
fn review1_body_allocation_stays_within_the_body_limit_plus_a_few_kib() {
    let max_body_bytes = 64 * 1024;
    let server = serve(move |s| {
        let mut first = b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n10000\r\n".to_vec();
        first.extend(std::iter::repeat_n(b'a', 4095));
        if s.write_all(&first).is_err() {
            return;
        }
        let _ = s.flush();
        let mut left = max_body_bytes - 4095;
        while left > 0 {
            thread::sleep(Duration::from_millis(20));
            let n = left.min(4095);
            if s.write_all(&vec![b'a'; n]).is_err() {
                return;
            }
            let _ = s.flush();
            left -= n;
        }
        let _ = s.write_all(b"\r\n0\r\n\r\n");
    });
    let limits = Limits {
        read_timeout: Duration::from_secs(2),
        max_body_bytes,
        ..limits()
    };
    let response = get(&server, &limits).expect("a response at the limit");
    server.handle.join().expect("server");
    assert_eq!(response.body.len(), max_body_bytes);
    assert!(
        response.body.capacity() <= max_body_bytes + 9 * 1024,
        "body limit {max_body_bytes} bytes, body buffer allocated {} bytes",
        response.body.capacity()
    );
}

/// The crate documentation: "no input makes this crate panic". `Limits::max_body_bytes` is a
/// public field; with `usize::MAX` (no body limit) a close-delimited body whose first byte
/// arrives after the header section is read at `crates/fetch/src/lib.rs:616`,
/// `max - body.len() + 1`, with an empty body.
#[test]
fn review1_unlimited_body_with_a_close_delimited_response_does_not_panic() {
    let server = serve(|s| {
        let _ = s.write_all(b"HTTP/1.1 200 OK\r\n\r\n");
        let _ = s.flush();
        thread::sleep(Duration::from_millis(100));
        let _ = s.write_all(b"hello");
        let _ = s.shutdown(std::net::Shutdown::Write);
    });
    let limits = Limits {
        max_body_bytes: usize::MAX,
        ..limits()
    };
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| get(&server, &limits)));
    server.handle.join().expect("server");
    assert!(
        matches!(&result, Ok(Ok(r)) if r.body == b"hello"),
        "fetch panicked or failed: {:?}",
        result.as_ref().map_err(|p| p
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string())))
    );
}
