//! Case T1 of `story:agent-cookie-jar`: the jar is used below the `Transport`, inside
//! `fetch_http`, so the `RequestStarted` handed to a transport and the `Answer` it returns never
//! hold a cookie. A server of the test on 127.0.0.1 port 0 sets the cookie on `/set` and records
//! the head of `/echo`; the cookie's value is 32 hexadecimal digits drawn for this run.

use std::cell::RefCell;
use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener};
use std::thread;
use std::time::Duration;

use surf::{Answer, Browser, Method, RequestContext, RequestStarted, answer, fetch_http};
use surf_cookie::Jar;
use surf_ports::ComponentPorts;
use surf_types::navigation::Navigate;
use surf_types::primitives::Uuid;
use surf_types::shell::TabId;
use surf_types::url::Url;

/// 32 hexadecimal digits, drawn from the standard library's random hasher keys.
fn digits() -> String {
    (0..2)
        .map(|_| format!("{:016x}", RandomState::new().build_hasher().finish()))
        .collect()
}

/// Serves two connections on `listener`: `/set` answers `Set-Cookie: <cookie>; Path=/`, any
/// other target `200`. Returns the head of each request, in order.
fn serve_two(listener: TcpListener, cookie: String) -> thread::JoinHandle<Vec<String>> {
    thread::spawn(move || {
        let mut heads = Vec::new();
        for _ in 0..2 {
            let (mut stream, _) = listener.accept().expect("accept");
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .expect("read timeout");
            let mut head = Vec::new();
            let mut buf = [0u8; 4096];
            while !head.windows(4).any(|w| w == b"\r\n\r\n") {
                match stream.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => head.extend_from_slice(&buf[..n]),
                }
            }
            let head = String::from_utf8_lossy(&head).into_owned();
            let set = if head.starts_with("GET /set ") {
                format!("Set-Cookie: {cookie}; Path=/\r\n")
            } else {
                String::new()
            };
            let reply = format!(
                "HTTP/1.1 200 OK\r\n{set}Content-Type: text/plain\r\nContent-Length: 2\r\n\r\nok"
            );
            let _ = stream.write_all(reply.as_bytes());
            let _ = stream.shutdown(Shutdown::Write);
            heads.push(head);
        }
        heads
    })
}

/// A `GET` navigation of a tab no shell holds to `address`, without an initiator.
fn get(address: &str) -> Navigate {
    Navigate {
        tab_id: TabId(Uuid("6f1c2d3e-4b5a-4c6d-8e7f-001122334477".to_owned())),
        url: Url(address.to_owned()),
        method: Method::Get,
        body: None,
        body_type: None,
        initiator: None,
    }
}

#[test]
fn a_transport_sees_no_cookie() {
    let digits = digits();
    let cookie = format!("zqsid={digits}");
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let local = listener.local_addr().expect("address");
    let server = serve_two(listener, cookie.clone());
    let jar = Jar::new();
    let seen: RefCell<Vec<String>> = RefCell::default();
    let transport = |request: &RequestStarted| -> Answer {
        let context = RequestContext {
            anchors: None,
            jar: Some(&jar),
        };
        let answered = answer(fetch_http(request, &surf_fetch::Limits::default(), context));
        seen.borrow_mut().push(format!("{request:?}"));
        seen.borrow_mut().push(format!("{answered:?}"));
        answered
    };
    let mut browser = Browser::from_ports(ComponentPorts::new(), transport);
    for path in ["/set", "/echo"] {
        browser
            .navigator_mut()
            .navigate(get(&format!("http://{local}{path}")))
            .expect("navigate");
        browser.run().expect("run");
    }
    drop(browser);
    let heads = server.join().expect("the server read two requests");
    let cookie_lines: Vec<&str> = heads[1]
        .split("\r\n")
        .filter(|line| {
            line.split_once(':')
                .is_some_and(|(name, _)| name.eq_ignore_ascii_case("cookie"))
        })
        .collect();
    assert!(
        cookie_lines == [format!("Cookie: {cookie}")],
        "/echo did not receive the cookie of /set: {} Cookie line(s)",
        cookie_lines.len()
    );
    let seen = seen.into_inner();
    assert_eq!(seen.len(), 4, "two requests and two answers");
    for text in &seen {
        assert!(
            !text.contains("zqsid") && !text.contains(&digits),
            "a Debug text handed through the transport holds the cookie"
        );
    }
}
