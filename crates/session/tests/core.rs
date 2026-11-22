//! Cases 4, 7, 8, 9, 10 and 11 of `story:agent-session-process`, on the session core with no
//! socket and no process, and two more on the bounds the core keeps: the path shows the last
//! five actions, and an address too long for the status record still gets an answer. Since
//! the story that sends a `POST` (wave 9), that the session's transport sends one.

#![forbid(unsafe_code)]

mod common;

use std::fs;
use std::path::Path;

use common::{golden, opened, path_block, placeholders, request};
use surf_agent::wire::{self, Request};
use surf_session::Session;

const ID: &str = "0b0e4f5a-6c7d-4e8f-9a0b-1c2d3e4f5a6b";

/// The port the addresses of the core cases name; nothing listens on it.
const PORT: u16 = 8000;

/// A transport that answers every address with status 200 and `text/html`.
fn pages(request: &surf::RequestStarted) -> surf::Answer {
    surf::Answer::Response {
        url: request.url.0.clone(),
        status: 200,
        media_type: "text/html".to_string(),
        body: b"<p>a page</p>".to_vec(),
    }
}

fn url(path: &str) -> String {
    format!("http://127.0.0.1:{PORT}{path}")
}

/// The answer lines of `request` and its exit status.
fn call<T: surf::Transport>(session: &mut Session<T>, request: &Request) -> (u8, String) {
    let reply = session.answer(request);
    let link = reply.to_link();
    (link.status, link.text)
}

fn holder(command: &str, token: &str, args: &[(&str, &str)]) -> Request {
    request(command, Some(ID), Some(token), args)
}

#[test]
fn token_differing_in_one_byte_is_refused() {
    let mut session = Session::new(ID, pages).unwrap();
    let (status, text) = call(
        &mut session,
        &request("open", None, None, &[("url", &url("/a"))]),
    );
    assert_eq!(status, 0);
    let token = opened(&text).token;
    let (_, before) = call(&mut session, &holder("look", &token, &[]));

    let flip = |digit: u8| if digit == b'0' { '1' } else { '0' };
    let first = format!("{}{}", flip(token.as_bytes()[0]), &token[1..]);
    let last = format!("{}{}", &token[..63], flip(token.as_bytes()[63]));
    for other in [&first, &last] {
        assert_ne!(other, &token);
        for command in ["look", "close"] {
            let (status, text) = call(&mut session, &holder(command, other, &[]));
            assert_eq!(status, 1, "{command} with {other}");
            assert_eq!(
                text,
                format!("surf 1 session {ID} unknown\ndid refused token\n")
            );
        }
        let (status, text) = call(&mut session, &holder("go", other, &[("url", &url("/x"))]));
        assert_eq!(status, 1);
        assert_eq!(
            text,
            format!("surf 1 session {ID} unknown\ndid refused token\n")
        );
    }

    let (status, after) = call(&mut session, &holder("look", &token, &[]));
    assert_eq!(status, 0, "the session's own token");
    assert_eq!(after, before, "the log after the refusals");
    assert_eq!(
        path_block(&after),
        "path 2 of 2\n  1 open\n  2 go \"http://127.0.0.1:8000/a\"\n"
    );
}

#[test]
fn file_address_is_not_read() {
    let mut session = Session::new(ID, surf_session::fetch).unwrap();
    let (status, text) = call(&mut session, &request("open", None, None, &[]));
    assert_eq!(status, 0);
    let token = opened(&text).token;
    let (status, text) = call(
        &mut session,
        &holder("go", &token, &[("url", "file:///etc/passwd")]),
    );
    assert_eq!(status, 0, "{text}");
    let lines: Vec<&str> = text.lines().collect();
    assert!(
        lines.contains(&"page failed UnsupportedScheme \"file:///etc/passwd\""),
        "{text}"
    );
    assert!(lines.contains(&"  2 go \"file:///etc/passwd\""), "{text}");
}

/// The story that sends a `POST` (wave 9): the session's transport sends a `Post` with its body
/// and media type. The server reads the head and the `Content-Length` bytes of the body before it
/// answers 200.
#[test]
fn fetch_sends_post() {
    use std::io::{Read, Write};

    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let local = listener.local_addr().unwrap();
    let address = format!("http://{local}/form");
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(5)))
            .unwrap();
        let mut bytes = Vec::new();
        let mut buf = [0u8; 4096];
        let end = loop {
            if let Some(end) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                break end + 4;
            }
            match stream.read(&mut buf) {
                Ok(0) | Err(_) => panic!("the request ended inside its head"),
                Ok(n) => bytes.extend_from_slice(&buf[..n]),
            }
        };
        let mut body = bytes.split_off(end);
        let head = String::from_utf8(bytes).unwrap();
        let length: usize = head
            .lines()
            .find_map(|line| line.strip_prefix("Content-Length: "))
            .map_or(0, |value| value.parse().unwrap());
        while body.len() < length {
            match stream.read(&mut buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => body.extend_from_slice(&buf[..n]),
            }
        }
        let _ = stream.write_all(
            b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 2\r\n\r\nok",
        );
        let _ = stream.shutdown(std::net::Shutdown::Write);
        (head, body)
    });
    let request = surf::RequestStarted {
        request_id: surf_types::net::RequestId(surf_types::primitives::Uuid(
            "00000000-0000-4000-8000-000000000081".to_string(),
        )),
        url: surf_types::url::Url(address.clone()),
        method: surf::Method::Post,
        body: Some(b"a=1".to_vec()),
        body_type: Some(surf_types::net::MediaType(
            "application/x-www-form-urlencoded".to_string(),
        )),
    };
    let answer = surf_session::fetch(&request);
    // A transport that sent nothing would leave the server in `accept`: this connection, queued
    // after any the fetch made and closed at once, ends it with an empty head instead.
    let _ = std::net::TcpStream::connect(local);
    let (head, body) = server.join().expect("the server read one request");
    assert!(head.starts_with("POST /form HTTP/1.1\r\n"), "{head:?}");
    assert_eq!(body, b"a=1");
    assert_eq!(
        answer,
        surf::Answer::Response {
            url: address,
            status: 200,
            media_type: "text/plain".to_string(),
            body: b"ok".to_vec(),
        }
    );
}

#[test]
fn outside_values_are_quoted() {
    let address = "http://127.0.0.1:8000/\"q\"\\x\u{e9}";
    let mut session = Session::new(ID, pages).unwrap();
    let (status, text) = call(
        &mut session,
        &request("open", None, None, &[("url", address)]),
    );
    assert_eq!(status, 0, "{text}");
    let quoted = surf_answer::quote(address);
    let lines: Vec<&str> = text.lines().collect();
    assert!(
        lines.contains(&format!("page 200 \"text/html\" {quoted}").as_str()),
        "{text}"
    );
    assert!(
        lines.contains(&format!("  2 go {quoted}").as_str()),
        "{text}"
    );
    assert!(
        lines.contains(&format!("did open {quoted}").as_str()),
        "{text}"
    );
    assert!(
        text.bytes()
            .all(|b| b == b'\n' || (b' '..=b'~').contains(&b))
    );
}

#[test]
fn core_without_a_socket() {
    let mut session = Session::new(ID, pages).unwrap();
    let (status, text) = call(
        &mut session,
        &request("open", None, None, &[("url", &url("/a"))]),
    );
    assert_eq!(status, 0);
    let token = opened(&text).token;
    let mut answers = vec![("open", text)];
    for (name, args) in [
        ("go", vec![("url", url("/b"))]),
        ("look", vec![]),
        ("close", vec![]),
    ] {
        let args: Vec<(&str, &str)> = args.iter().map(|(n, v)| (*n, v.as_str())).collect();
        let (status, text) = call(&mut session, &holder(name, &token, &args));
        assert_eq!(status, 0, "{name}: {text}");
        answers.push((name, text));
    }
    for (name, text) in &answers {
        assert_eq!(
            placeholders(text, ID, &token, PORT),
            golden(&format!("{name}.txt")),
            "the answer of {name}"
        );
        surf_answer::parse(text).expect("the answer parses");
    }
}

#[test]
fn open_without_an_address() {
    let mut session = Session::new(ID, pages).unwrap();
    let (status, text) = call(&mut session, &request("open", None, None, &[]));
    assert_eq!(status, 0);
    let token = opened(&text).token;
    assert_eq!(
        text,
        format!(
            "surf 1 session {ID} open\ntoken {token}\ndid open\npage none\npath 1 of 1\n  1 open\n"
        )
    );
    let (status, text) = call(&mut session, &holder("close", &token, &[]));
    assert_eq!(status, 0);
    assert_eq!(
        text,
        format!(
            "surf 1 session {ID} closed\ndid close\npage none\npath 2 of 2\n  1 open\n  2 close\n"
        )
    );
}

#[test]
fn token_held_only_as_surf_secret_token() {
    let mut session = Session::new(ID, pages).unwrap();
    let (_, text) = call(
        &mut session,
        &request("open", None, None, &[("url", &url("/a"))]),
    );
    let token = opened(&text).token;
    let debug = format!("{session:?} {:#?}", session);
    assert!(
        !debug.contains(&token),
        "the Debug form holds the token: {debug}"
    );

    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut to_hex = Vec::new();
    for entry in fs::read_dir(&src).unwrap() {
        let path = entry.unwrap().path();
        let text = fs::read_to_string(&path).unwrap();
        assert!(
            !text.contains("SessionToken"),
            "{} names SessionToken",
            path.display()
        );
        for (number, line) in text.lines().enumerate() {
            if line.contains("to_hex") {
                to_hex.push(format!(
                    "{}:{}: {}",
                    path.display(),
                    number + 1,
                    line.trim()
                ));
            }
        }
    }
    assert_eq!(
        to_hex.len(),
        1,
        "Token::to_hex is called in one place: {to_hex:#?}"
    );
    assert!(
        to_hex[0].contains("answer.token = Some("),
        "the one call is the token line of open: {}",
        to_hex[0]
    );
}

#[test]
fn path_shows_the_last_five_actions() {
    let mut session = Session::new(ID, pages).unwrap();
    let (_, text) = call(&mut session, &request("open", None, None, &[]));
    let token = opened(&text).token;
    let mut text = String::new();
    for n in 2..=7 {
        let (status, answer) = call(
            &mut session,
            &holder("go", &token, &[("url", &url(&format!("/{n}")))]),
        );
        assert_eq!(status, 0);
        text = answer;
    }
    assert_eq!(
        path_block(&text),
        "path 5 of 7\n  3 go \"http://127.0.0.1:8000/3\"\n  4 go \"http://127.0.0.1:8000/4\"\n  5 go \
         \"http://127.0.0.1:8000/5\"\n  6 go \"http://127.0.0.1:8000/6\"\n  7 go \
         \"http://127.0.0.1:8000/7\"\n"
    );
    let (_, text) = call(&mut session, &request("open", None, None, &[]));
    assert_eq!(
        text,
        format!("surf 1 session {ID} unknown\ndid refused usage\n")
    );
}

#[test]
fn long_address_still_gets_an_answer() {
    // An address asked through the wire comes to about the record's limit at most: a request is
    // at most 64 KiB and quotes it the same way. The address of a loaded page is not bound by
    // the request: after a redirect it is the address the response came from, up to
    // surf_fetch::MAX_ADDRESS_BYTES before quoting and up to four times that after, which a
    // server can make too long for the record (case 24 of tests/redirect.rs). Here the core
    // takes a request that never crossed the wire (as the scenario runner does), whose address
    // is longer than the record holds.
    let address = format!("http://127.0.0.1:8000/{}", "\u{e9}".repeat(20_000));
    let go = Request {
        command: "go".to_string(),
        session: Some(ID.to_string()),
        token: None,
        args: vec![wire::Arg {
            name: "url".to_string(),
            value: address.clone(),
        }],
    };
    assert!(go.encode().is_err(), "the request does not fit the wire");
    let mut session = Session::new(ID, pages).unwrap();
    let (_, text) = call(&mut session, &request("open", None, None, &[]));
    let token = opened(&text).token;
    let reply = session.answer(&holder("go", &token, &[("url", &address)]));
    let link = reply.to_link();
    assert_eq!(link.status, 0, "{:?}", link.reason);
    assert!(link.encode().is_ok(), "the link answer encodes");
    assert_eq!(link.address, None, "the record goes without the address");
    let quoted = surf_answer::quote(&address);
    assert!(
        link.text
            .lines()
            .any(|line| line == format!("page 200 \"text/html\" {quoted}"))
    );
}

#[test]
fn another_identity_with_the_token_is_refused() {
    let mut session = Session::new(ID, pages).unwrap();
    let (_, text) = call(&mut session, &request("open", None, None, &[]));
    let token = opened(&text).token;
    let other = "0b0e4f5a-6c7d-4e8f-9a0b-1c2d3e4f5a6c";
    for named in [Some(other), None] {
        let (status, text) = call(&mut session, &request("look", named, Some(&token), &[]));
        assert_eq!(status, 1, "{named:?}");
        assert_eq!(
            text,
            format!("surf 1 session {ID} unknown\ndid refused token\n")
        );
    }
    let upper = ID.to_ascii_uppercase();
    let (status, _) = call(
        &mut session,
        &request("look", Some(&upper), Some(&token), &[]),
    );
    assert_eq!(
        status, 0,
        "the identity in upper case names the same session"
    );
}
