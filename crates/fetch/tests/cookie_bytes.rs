//! `story:agent-cookie-jar`: a `Set-Cookie` field that is not UTF-8 is no cookie the jar can read.
//! The fetch drops it without a failure and keeps the other fields of the same response. One
//! server of the test on 127.0.0.1 answers `/set` with the raw bytes below and records the
//! `Cookie` lines of `/echo`. Every cookie is a constant of the test.

use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, SystemTime};

use surf_cookie::Jar;
use surf_fetch::{send, Method, Request};

use crate::redirects::limits;

/// The response of `/set`: one field with the bytes 0xff and 0xfe in its value, one of
/// Latin-1 text, and one of UTF-8.
const SET: &[u8] = b"HTTP/1.1 200 OK\r\n\
    Set-Cookie: raw=\xff\xfe; Path=/\r\n\
    Set-Cookie: latin=caf\xe9; Path=/\r\n\
    Set-Cookie: ok=1; Path=/\r\n\
    Content-Length: 0\r\n\r\n";

#[test]
fn a_set_cookie_field_that_is_not_utf8_is_dropped() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("address").port();
    let (sender, cookies) = mpsc::channel();
    let server = thread::spawn(move || {
        for stream in listener.incoming().take(2) {
            let Ok(mut stream) = stream else { continue };
            let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
            let mut head = Vec::new();
            let mut buffer = [0u8; 4096];
            while !head.windows(4).any(|w| w == b"\r\n\r\n") {
                match stream.read(&mut buffer) {
                    Ok(0) | Err(_) => break,
                    Ok(read) => head.extend_from_slice(&buffer[..read]),
                }
            }
            let text = String::from_utf8_lossy(&head).into_owned();
            if text.starts_with("GET /set ") {
                let _ = stream.write_all(SET);
            } else {
                let lines: Vec<String> = text
                    .split("\r\n")
                    .filter(|line| {
                        line.split_once(':')
                            .is_some_and(|(name, _)| name.eq_ignore_ascii_case("cookie"))
                    })
                    .map(str::to_string)
                    .collect();
                let _ = sender.send(lines);
                let _ = stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n");
            }
            let _ = stream.shutdown(Shutdown::Write);
        }
    });
    let jar = Jar::new();
    for path in ["/set", "/echo"] {
        let address = format!("http://127.0.0.1:{port}{path}");
        let request = Request {
            address: &address,
            method: Method::Get,
            body: None,
            media_type: None,
            anchors: None,
            jar: Some(&jar),
            initiator: None,
        };
        let result = send(&request, &limits());
        assert!(result.is_ok(), "{path}: {result:?}");
    }
    server.join().expect("the server thread");
    assert_eq!(jar.count(SystemTime::now()), 1);
    assert_eq!(
        cookies.try_iter().collect::<Vec<_>>(),
        [vec!["Cookie: ok=1".to_string()]]
    );
}
