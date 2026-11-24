//! Rule R9 of `story:agent-cookie-jar` for a redirect chain, draft-ietf-httpbis-rfc6265bis-22
//! section 5.2: a hop is same-site only if every address of the chain before it has the hop's
//! scheme and host; once the chain has been on another site, every later hop is cross-site, also
//! for a navigation without an initiator. A cross-site hop gets no `Strict` cookie, and a `Lax`
//! one only when its method is `GET` (R8).
//!
//! The jar holds `strict=1` (`SameSite=Strict`) and `lax=1` (`SameSite=Lax`) of `127.0.0.1`. One
//! server on 127.0.0.1 answers `/away` with a 302 and `/away307` with a 307 to `/echo` on
//! 127.0.0.1, `/stay` with a 302 to the same, and anything else with a 200; `a.example.test` is
//! 127.0.0.1 too. Every cookie is a constant of the test.

use std::io::{Read, Write};
use std::net::{Ipv4Addr, Shutdown, SocketAddr, TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime};

use surf_cookie::Jar;
use surf_fetch::{send_through, Failure, Method, Network, Request};

use crate::redirects::limits;

/// The target and the `Cookie` lines of one request head.
fn hop_of(bytes: &[u8]) -> (String, Vec<String>) {
    let text = String::from_utf8_lossy(bytes);
    let target = text.split(' ').nth(1).unwrap_or_default().to_string();
    let cookies = text
        .split("\r\n")
        .filter(|line| {
            line.split_once(':')
                .is_some_and(|(name, _)| name.eq_ignore_ascii_case("cookie"))
        })
        .map(|line| line.to_string())
        .collect();
    (target, cookies)
}

/// The target and the `Cookie` lines of each request the server read, in order.
type Hops = Arc<Mutex<Vec<(String, Vec<String>)>>>;

/// The server of the module and the hops it read.
fn server() -> (u16, Hops) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("address").port();
    let hops = Arc::new(Mutex::new(Vec::new()));
    let kept = Arc::clone(&hops);
    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
            let mut bytes = Vec::new();
            let mut buffer = [0u8; 4096];
            while !bytes.windows(4).any(|w| w == b"\r\n\r\n") {
                match stream.read(&mut buffer) {
                    Ok(0) | Err(_) => break,
                    Ok(read) => bytes.extend_from_slice(&buffer[..read]),
                }
            }
            let hop = hop_of(&bytes);
            let back = format!("http://127.0.0.1:{port}/echo");
            let reply = match hop.0.as_str() {
                "/away" | "/stay" => {
                    format!("HTTP/1.1 302 Found\r\nLocation: {back}\r\nContent-Length: 0\r\n\r\n")
                }
                "/away307" => format!(
                    "HTTP/1.1 307 Temporary Redirect\r\nLocation: {back}\r\nContent-Length: 0\r\n\r\n"
                ),
                _ => "HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n".to_string(),
            };
            kept.lock().expect("hops").push(hop);
            let _ = stream.write_all(reply.as_bytes());
            let _ = stream.shutdown(Shutdown::Write);
        }
    });
    (port, hops)
}

/// Every name is 127.0.0.1.
struct Loopback;

impl Network for Loopback {
    fn resolve(&self, _: &str, port: u16, _: Duration) -> Result<Vec<SocketAddr>, Failure> {
        Ok(vec![SocketAddr::from((Ipv4Addr::LOCALHOST, port))])
    }

    fn connect(&self, address: &SocketAddr, timeout: Duration) -> std::io::Result<TcpStream> {
        TcpStream::connect_timeout(address, timeout)
    }
}

/// The hops of one fetch of `address` with `method` from `initiator`, after a jar got
/// `strict=1` and `lax=1` from 127.0.0.1.
fn chain(address: &str, method: Method, initiator: Option<&str>) -> Vec<(String, Vec<String>)> {
    let hops = &SERVER.1;
    let jar = Jar::new();
    let home = surf_url::Url::parse("http://127.0.0.1/", None).expect("parses");
    for set_cookie in [
        "strict=1; SameSite=Strict; Path=/",
        "lax=1; SameSite=Lax; Path=/",
    ] {
        jar.receive(&home, set_cookie, SystemTime::now());
    }
    let (body, media_type) = match method {
        Method::Post => (Some(&b"x=1"[..]), Some("application/x-www-form-urlencoded")),
        Method::Get => (None, None),
    };
    let request = Request {
        address,
        method,
        body,
        media_type,
        anchors: None,
        jar: Some(&jar),
        initiator,
    };
    let result = send_through(&request, &limits(), &Loopback);
    assert!(result.is_ok(), "{address}: {result:?}");
    std::mem::take(&mut *hops.lock().expect("hops"))
}

static SERVER: std::sync::LazyLock<(u16, Hops)> = std::sync::LazyLock::new(server);

fn hop(target: &str, cookie: Option<&str>) -> (String, Vec<String>) {
    (
        target.to_string(),
        cookie.map(|c| format!("Cookie: {c}")).into_iter().collect(),
    )
}

const HOME: &str = "http://127.0.0.1:1";

#[test]
fn a_chain_that_leaves_the_site_and_comes_back_is_cross_site_from_then_on() {
    let port = SERVER.0;
    let away = format!("http://a.example.test:{port}/away");
    let away307 = format!("http://a.example.test:{port}/away307");
    // A link of a page of 127.0.0.1 to another site that redirects back, and the same typed.
    for initiator in [Some(HOME), None] {
        assert_eq!(
            chain(&away, Method::Get, initiator),
            [hop("/away", None), hop("/echo", Some("lax=1"))],
            "302 and GET from {initiator:?}"
        );
        // A form posted to another site that redirects back with 307: still a POST.
        assert_eq!(
            chain(&away307, Method::Post, initiator),
            [hop("/away307", None), hop("/echo", None)],
            "307 and POST from {initiator:?}"
        );
    }
}

#[test]
fn a_chain_that_stays_on_the_site_is_same_site() {
    let port = SERVER.0;
    let stay = format!("http://127.0.0.1:{port}/stay");
    for initiator in [Some(HOME), None] {
        assert_eq!(
            chain(&stay, Method::Get, initiator),
            [
                hop("/stay", Some("strict=1; lax=1")),
                hop("/echo", Some("strict=1; lax=1"))
            ],
            "from {initiator:?}"
        );
    }
    // From another site: cross-site on every hop, as before.
    assert_eq!(
        chain(&stay, Method::Get, Some("http://a.example.test:1")),
        [hop("/stay", Some("lax=1")), hop("/echo", Some("lax=1"))]
    );
}
