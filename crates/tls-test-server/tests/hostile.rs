//! Clients hostile toward the server (story `tls-test-server`, acceptance 4): garbage, silence,
//! and a server dropped while a connection stalls.

use std::io::{ErrorKind, Read, Write};
use std::net::TcpStream;
use std::thread;
use std::time::{Duration, Instant};

use surf_tls_test_server::{Config, IDLE, TestServer};

use crate::common;

/// Reads `stream` until the server closes it: `Ok` with the bytes read at the end of the
/// stream or a reset, `Err` with the kind of any other end (a timeout among them).
fn until_closed(stream: &mut TcpStream) -> Result<usize, ErrorKind> {
    let mut total = 0;
    let mut buf = [0u8; 512];
    loop {
        match stream.read(&mut buf) {
            Ok(0) => return Ok(total),
            Ok(n) => total += n,
            Err(e)
                if matches!(
                    e.kind(),
                    ErrorKind::ConnectionReset | ErrorKind::ConnectionAborted
                ) =>
            {
                return Ok(total);
            }
            Err(e) => return Err(e.kind()),
        }
    }
}

/// Waits until the server has accepted `count` connections, at most 1 s.
fn accepted(server: &TestServer, count: usize) {
    let deadline = Instant::now() + Duration::from_secs(1);
    while server.accepted() < count {
        assert!(Instant::now() < deadline, "the server accepted {count}");
        thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn garbage_is_closed_and_the_next_client_served() {
    let server = TestServer::start(Config::default()).expect("the server starts");
    let mut garbage = common::socket(&server, Duration::from_secs(1));
    let started = Instant::now();
    garbage
        .write_all(&[0xde, 0xad, 0xbe, 0xef, 0x00, 0x01, 0x02, 0x03, 0xff, 0xfe])
        .expect("write the garbage");
    assert!(until_closed(&mut garbage).is_ok(), "the server closed it");
    assert!(started.elapsed() < Duration::from_secs(1));

    let mut connection = common::connect(&server, "ok-ecdsa-p256").expect("the next completes");
    let (response, end) = common::get(&mut connection, "/");
    assert_eq!(end, Ok(()));
    assert_eq!(common::body(&response), b"ok");
    assert_eq!(server.accepted(), 2);
}

#[test]
fn silent_client_is_closed_after_two_seconds() {
    let server = TestServer::start(Config::default()).expect("the server starts");
    let mut silent = common::socket(&server, Duration::from_secs(3));
    let started = Instant::now();
    assert_eq!(until_closed(&mut silent), Ok(0), "closed, nothing sent");
    let took = started.elapsed();
    assert_eq!(IDLE, Duration::from_secs(2));
    assert!(
        took >= IDLE - Duration::from_millis(100),
        "closed early: {took:?}"
    );
    assert!(took < Duration::from_secs(3), "closed late: {took:?}");
}

#[test]
fn drop_while_a_connection_stalls_joins_within_three_seconds() {
    let server = TestServer::start(Config::default()).expect("the server starts");
    let mut stalled = common::socket(&server, Duration::from_secs(1));
    accepted(&server, 1);
    let started = Instant::now();
    drop(server);
    let took = started.elapsed();
    assert!(took < Duration::from_secs(3), "the drop took {took:?}");
    // Below the idle bound: the drop stopped the connection, its idle timeout did not.
    assert!(
        took < IDLE / 2,
        "the drop waited for the idle timeout: {took:?}"
    );
    assert_eq!(until_closed(&mut stalled), Ok(0), "its socket is closed");
}
