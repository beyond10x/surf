//! A row whose `server` is `https` reaches its page over TLS only: the lane's fixture server
//! does not serve its site, and the row fails unless `open` loaded the start address on the TLS
//! server. `tasks::every_task` holds what the TLS server saw; these hold the two rules without
//! surfd or a TLS server.

use std::fs;
use std::io::{Read, Write};
use std::net::TcpStream;

use surf_agent_tasks::Verdict;
use surf_agent_tasks::format::{Server, read_tasks};
use surf_agent_tasks::lane::{fixture_server, opened_at};
use surf_agent_tasks::tasks_file;
use surf_answer::{Answer, Head, Page, SessionState, render};

/// The status line of `GET <path>` to `origin` (`http://127.0.0.1:<port>`).
fn status_line(origin: &str, path: &str) -> String {
    let mut stream = TcpStream::connect(origin.strip_prefix("http://").unwrap()).unwrap();
    write!(stream, "GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n").unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    response.lines().next().unwrap_or_default().to_string()
}

#[test]
fn the_fixture_server_serves_no_https_row() {
    let tasks = read_tasks(&fs::read_to_string(tasks_file()).unwrap()).unwrap();
    let server = fixture_server(&tasks).expect("the fixture server listens");
    for task in &tasks {
        let expected = match task.server {
            Server::Http => "HTTP/1.1 200 OK",
            Server::Https => "HTTP/1.1 404 Not Found",
        };
        let path = format!("/{}{}", task.name, task.start);
        assert_eq!(status_line(server.origin(), &path), expected, "{path}");
    }
}

/// The answer of an `open` whose `page` line is `page`.
fn opened(page: Page) -> String {
    let mut answer = Answer::new(Head {
        session: "4b0f6d2e-8c1a-4e5b-9f3d-2a7c6e1b0d94".to_string(),
        state: SessionState::Open,
    });
    answer.page = Some(page);
    render(&answer).unwrap()
}

fn loaded(status: u16, address: &str) -> Page {
    Page::Loaded {
        status,
        media_type: "text/html".to_string(),
        address: address.to_string(),
        title: Some("Notice to visitors".to_string()),
    }
}

#[test]
fn an_https_row_holds_the_address_open_loaded() {
    let address = "https://localhost:41234/notice.html";
    let answer = opened(loaded(200, address));
    assert_eq!(opened_at(address, Some(answer.as_str())), Verdict::Pass);

    let fails = |page: Page, line: &str| {
        let answer = opened(page);
        assert_eq!(
            opened_at(address, Some(answer.as_str())),
            Verdict::Fail(format!("open answered {line}")),
            "{answer}"
        );
    };
    fails(
        loaded(200, "http://127.0.0.1:41235/https/notice.html"),
        "page 200 \"text/html\" \"http://127.0.0.1:41235/https/notice.html\" \"Notice to visitors\"",
    );
    fails(
        loaded(200, "https://localhost:41235/notice.html"),
        "page 200 \"text/html\" \"https://localhost:41235/notice.html\" \"Notice to visitors\"",
    );
    fails(
        loaded(404, address),
        "page 404 \"text/html\" \"https://localhost:41234/notice.html\" \"Notice to visitors\"",
    );
    fails(
        Page::Failed {
            reason: "CertificateNotTrusted".to_string(),
            address: address.to_string(),
        },
        "page failed CertificateNotTrusted \"https://localhost:41234/notice.html\"",
    );
    assert_eq!(
        opened_at(address, None),
        Verdict::Fail("open answered no page line".to_string())
    );
}
