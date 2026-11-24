//! `story:agent-cookie-jar`: the `surf` binary has no cookie jar. A redirect that sets a cookie
//! is followed without a `Cookie` field, and no output line holds the cookie. The server is the
//! test's own on 127.0.0.1 port 0; the binary is killed if it runs longer than `RUN_BOUND`.
//! The cookie is a constant of the test.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

/// The longest the binary may run before the test kills it and fails.
const RUN_BOUND: Duration = Duration::from_secs(20);

/// The longest the server waits for a request.
const BOUND: Duration = Duration::from_secs(5);

#[test]
fn surf_sends_no_cookie_a_redirect_set() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind 127.0.0.1:0");
    let port = listener.local_addr().expect("local address").port();
    let (heads, read) = mpsc::channel();
    let server = thread::spawn(move || {
        for stream in listener.incoming().take(2) {
            let Ok(mut stream) = stream else { continue };
            let _ = stream.set_read_timeout(Some(BOUND));
            let mut head = Vec::new();
            let mut buffer = [0u8; 1024];
            while !head.windows(4).any(|w| w == b"\r\n\r\n") && head.len() < 16 * 1024 {
                match stream.read(&mut buffer) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => head.extend_from_slice(&buffer[..n]),
                }
            }
            let reply = if head.starts_with(b"GET /a ") {
                "HTTP/1.1 302 Found\r\nLocation: /b\r\nSet-Cookie: zqcli=k7q9w2x5; Path=/\r\n\
                 Content-Length: 0\r\nConnection: close\r\n\r\n"
            } else {
                "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: 13\r\n\
                 Connection: close\r\n\r\n<p>a page</p>"
            };
            let _ = stream.write_all(reply.as_bytes());
            let _ = heads.send(String::from_utf8_lossy(&head).into_owned());
        }
    });
    let child = Command::new(env!("CARGO_BIN_EXE_surf"))
        .arg(format!("http://127.0.0.1:{port}/a"))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn surf");
    let pid = child.id();
    let (sender, receiver) = mpsc::channel();
    let waiter = thread::spawn(move || {
        let _ = sender.send(child.wait_with_output());
    });
    let output = match receiver.recv_timeout(RUN_BOUND) {
        Ok(output) => output.expect("surf output"),
        Err(_) => {
            let _ = Command::new("kill").args(["-9", &pid.to_string()]).status();
            let _ = waiter.join();
            panic!("surf ran longer than {RUN_BOUND:?}");
        }
    };
    waiter.join().expect("the waiting thread");
    server.join().expect("the server thread");
    let heads: Vec<String> = read.try_iter().collect();
    assert_eq!(heads.len(), 2, "two requests: /a and /b");
    assert!(heads[1].starts_with("GET /b "), "the second request is /b");
    let cookie_lines = heads[1]
        .split("\r\n")
        .filter(|line| {
            line.split_once(':')
                .is_some_and(|(name, _)| name.eq_ignore_ascii_case("cookie"))
        })
        .count();
    assert_eq!(cookie_lines, 0, "the request for /b carried a Cookie field");
    for stream in [&output.stdout, &output.stderr] {
        let text = String::from_utf8_lossy(stream);
        assert!(
            !text.contains("zqcli") && !text.contains("k7q9w2x5"),
            "an output line holds the cookie"
        );
    }
}
