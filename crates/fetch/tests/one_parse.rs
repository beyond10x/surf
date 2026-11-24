//! Rule R4 of `story:agent-cookie-jar`, no cookie reaches another host: a fetch with a jar parses
//! its address once, and the connection, the `Host` field and the jar's choice of cookies all
//! come from that one parse. The addresses below are those on which two parsers could read
//! different hosts: a backslash, upper case, a trailing dot, user information, an empty port,
//! percent-encoded bytes in the host, an IPv4 address in hexadecimal, a tab or a newline.
//!
//! The jar holds one cookie per host, `h=<tag>`; a request that carries one names the host the jar
//! was asked for, and the network records the host the connection went to. Every name and every
//! port lead to one server on 127.0.0.1 that records each request head and answers `200`. Every
//! cookie is a constant of the test.

use std::io::{Read, Write};
use std::net::{IpAddr, Ipv4Addr, Shutdown, SocketAddr, TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime};

use surf_cookie::Jar;
use surf_fetch::{send_through, Failure, Method, Network, Request};

use crate::redirects::limits;

/// The cookie of each host the jar holds, and the host it belongs to.
const COOKIES: [(&str, &str); 4] = [
    ("http://a.example.test/", "h=a"),
    ("http://b.example.test/", "h=b"),
    ("http://a.example.test./", "h=a-dot"),
    ("http://127.0.0.1/", "h=ip"),
];

/// The host a cookie of [`COOKIES`] belongs to.
fn host_of(cookie: &str) -> &'static str {
    match cookie {
        "h=a" => "a.example.test",
        "h=b" => "b.example.test",
        "h=a-dot" => "a.example.test.",
        "h=ip" => "127.0.0.1",
        _ => panic!("a cookie the test did not set: {cookie:?}"),
    }
}

/// One request head the server read: its `Host` field and its `Cookie` lines.
struct Head {
    host: String,
    cookies: Vec<String>,
}

fn head_of(bytes: &[u8]) -> Head {
    let text = String::from_utf8_lossy(bytes);
    let mut head = Head {
        host: String::new(),
        cookies: Vec::new(),
    };
    for line in text.split("\r\n").skip(1) {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        if name.eq_ignore_ascii_case("host") {
            head.host = value.trim().to_string();
        } else if name.eq_ignore_ascii_case("cookie") {
            head.cookies.push(value.trim().to_string());
        }
    }
    head
}

/// A server on 127.0.0.1 that answers every connection `200` and keeps each request head.
fn server() -> (u16, Arc<Mutex<Vec<Head>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("address").port();
    let heads = Arc::new(Mutex::new(Vec::new()));
    let kept = Arc::clone(&heads);
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
            kept.lock().expect("heads").push(head_of(&bytes));
            let _ = stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n");
            let _ = stream.shutdown(Shutdown::Write);
        }
    });
    (port, heads)
}

/// Every name and every port lead to the server; the host of each connection is kept: the name
/// resolved, or the IP address dialled when there was no name.
struct ToServer {
    port: u16,
    connected: Mutex<Vec<String>>,
}

impl Network for ToServer {
    fn resolve(&self, host: &str, _: u16, _: Duration) -> Result<Vec<SocketAddr>, Failure> {
        self.connected
            .lock()
            .expect("hosts")
            .push(host.to_ascii_lowercase());
        Ok(vec![SocketAddr::from((Ipv4Addr::LOCALHOST, self.port))])
    }

    fn connect(&self, address: &SocketAddr, timeout: Duration) -> std::io::Result<TcpStream> {
        let mut connected = self.connected.lock().expect("hosts");
        if connected.is_empty() {
            connected.push(address.ip().to_string());
        }
        drop(connected);
        let to = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), self.port);
        TcpStream::connect_timeout(&to, timeout)
    }
}

/// What one fetch of `address` did: whether it ended in a response, the host of its connection,
/// the `Host` field and the `Cookie` lines the server read.
struct Outcome {
    ok: bool,
    connected: Vec<String>,
    heads: Vec<Head>,
}

fn fetch(address: &str, port: u16, heads: &Mutex<Vec<Head>>) -> Outcome {
    let jar = Jar::new();
    for (from, set_cookie) in COOKIES {
        let url = surf_url::Url::parse(from, None).expect("a test address parses");
        jar.receive(&url, &format!("{set_cookie}; Path=/"), SystemTime::now());
    }
    let network = ToServer {
        port,
        connected: Mutex::new(Vec::new()),
    };
    let request = Request {
        address,
        method: Method::Get,
        body: None,
        media_type: None,
        anchors: None,
        jar: Some(&jar),
        initiator: None,
    };
    let ok = send_through(&request, &limits(), &network).is_ok();
    let connected = network.connected.into_inner().expect("hosts");
    let heads = std::mem::take(&mut *heads.lock().expect("heads"));
    Outcome {
        ok,
        connected,
        heads,
    }
}

/// The host of a `Host` field: without the port, in lower case.
fn field_host(field: &str) -> String {
    let host = match field.rsplit_once(':') {
        Some((host, port)) if port.bytes().all(|b| b.is_ascii_digit()) => host,
        _ => field,
    };
    host.to_ascii_lowercase()
}

#[test]
fn the_jar_is_asked_for_the_host_the_connection_goes_to() {
    let (port, heads) = server();
    // The address, then the cookie that request carries (`None`: none), or `Err` for an
    // address refused before any connection.
    let cases: Vec<(String, Result<Option<&str>, ()>)> = vec![
        (
            format!("http://a.example.test\\b.example.test:{port}/echo"),
            Ok(Some("h=a")),
        ),
        (
            format!("http://b.example.test\\@a.example.test:{port}/echo"),
            Err(()),
        ),
        (
            format!("http://A.EXAMPLE.TEST:{port}/echo"),
            Ok(Some("h=a")),
        ),
        (
            format!("http://a.example.test.:{port}/echo"),
            Ok(Some("h=a-dot")),
        ),
        (format!("http://u:p@a.example.test:{port}/echo"), Err(())),
        (
            format!("http://b.example.test@a.example.test:{port}/echo"),
            Err(()),
        ),
        ("http://a.example.test:/echo".to_string(), Ok(Some("h=a"))),
        (
            format!("http://a%2Eexample.test:{port}/echo"),
            Ok(Some("h=a")),
        ),
        (
            format!("http://%61.example.test:{port}/echo"),
            Ok(Some("h=a")),
        ),
        (
            format!("http://b.example.test%5C.a.example.test:{port}/echo"),
            Ok(None),
        ),
        (format!("http://0x7f.0.0.1:{port}/echo"), Ok(Some("h=ip"))),
        (
            format!("http://a.example.test\t.b.example.test:{port}/echo"),
            Err(()),
        ),
        (format!("http://a.example.test\n:{port}/echo"), Err(())),
    ];
    for (address, expected) in &cases {
        let outcome = fetch(address, port, &heads);
        match expected {
            Err(()) => {
                assert!(!outcome.ok, "{address:?} was fetched");
                assert!(outcome.heads.is_empty(), "{address:?} reached the server");
                continue;
            }
            Ok(cookie) => {
                assert!(outcome.ok, "{address:?} was not fetched");
                assert_eq!(outcome.heads.len(), 1, "{address:?}: one request");
                assert_eq!(outcome.connected.len(), 1, "{address:?}: one connection");
                let connected = &outcome.connected[0];
                let head = &outcome.heads[0];
                assert_eq!(
                    field_host(&head.host),
                    *connected,
                    "{address:?}: the Host field names another host than the connection's"
                );
                let sent: Vec<&str> = head.cookies.iter().map(String::as_str).collect();
                for sent in &sent {
                    assert_eq!(
                        host_of(sent),
                        connected.as_str(),
                        "{address:?}: the jar was asked for another host than the connection's"
                    );
                }
                assert_eq!(
                    sent,
                    cookie.iter().copied().collect::<Vec<_>>(),
                    "{address:?}"
                );
            }
        }
    }
}
