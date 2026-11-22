//! Clients hostile toward the server (story `tls-test-server`, acceptance 4): garbage, silence,
//! and a server dropped while a connection stalls; and a hello the server must refuse, made
//! from the hello the client of `surf-tls` sends.

use std::io::{ErrorKind, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::{Duration, Instant};

use surf_tls::connection::Connection;
use surf_tls::error::AlertDescription;
use surf_tls_test_server::{Config, IDLE, Received, TestServer};

use crate::common::{self, HOST};

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

/// The first record the client of `surf-tls` writes, its ClientHello, taken off a listener of
/// the test's own that answers nothing.
fn client_hello_record() -> Vec<u8> {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind");
    let addr = listener.local_addr().expect("addr");
    let client = thread::spawn(move || {
        let stream = TcpStream::connect(addr).expect("connect");
        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .expect("read timeout");
        let _ = Connection::new(
            stream,
            HOST,
            &common::accepting("ok-ecdsa-p256"),
            common::now(),
        );
    });
    let (mut stream, _) = listener.accept().expect("accept");
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .expect("read timeout");
    let mut record = vec![0u8; 5];
    stream.read_exact(&mut record).expect("a record header");
    let len = usize::from(u16::from_be_bytes([record[3], record[4]]));
    record.resize(5 + len, 0);
    stream.read_exact(&mut record[5..]).expect("the record");
    drop(stream);
    client.join().expect("the client ends");
    assert_eq!(
        (record[0], record[5]),
        (22, 1),
        "a ClientHello in one record"
    );
    record
}

/// `byte` inserted at `at` of a record holding one handshake message: both lengths raised.
fn insert(record: &mut Vec<u8>, at: usize, byte: u8) {
    record.insert(at, byte);
    let len = u16::from_be_bytes([record[3], record[4]]) + 1;
    record[3..5].copy_from_slice(&len.to_be_bytes());
    let body = u32::from_be_bytes([0, record[6], record[7], record[8]]) + 1;
    record[6..9].copy_from_slice(&body.to_be_bytes()[1..]);
}

/// The first record the server answers `record` with.
fn answer(server: &TestServer, record: &[u8]) -> (u8, Vec<u8>) {
    let mut stream = common::socket(server, Duration::from_secs(10));
    stream.write_all(record).expect("write the hello");
    let mut header = [0u8; 5];
    stream.read_exact(&mut header).expect("a record header");
    let mut body = vec![0u8; usize::from(u16::from_be_bytes([header[3], header[4]]))];
    stream.read_exact(&mut body).expect("the record");
    (header[0], body)
}

/// RFC 8446 section 4.1.2: `legacy_session_id<0..32>`. The client's own hello is answered
/// with a ServerHello; the same hello with a session id of 33 bytes with `decode_error`.
#[test]
fn hello_with_a_session_id_over_32_bytes_gets_decode_error() {
    let server = TestServer::start(Config::default()).expect("the server starts");
    let mut hello = client_hello_record();
    // Record header 5, handshake header 4, legacy_version 2, random 32: the session id's length.
    let at = 5 + 4 + 2 + 32;
    assert_eq!(hello[at], 32, "the client sends a session id of 32 bytes");
    assert_eq!(
        answer(&server, &hello).0,
        22,
        "a ServerHello to the hello as sent"
    );
    hello[at] = 33;
    insert(&mut hello, at + 1, 0x33);
    assert_eq!(
        answer(&server, &hello),
        (21, vec![2, AlertDescription::DECODE_ERROR.0])
    );
}

/// A `change_cipher_spec` record of the one byte 0x01, unprotected.
const CHANGE_CIPHER_SPEC: [u8; 6] = [20, 3, 3, 0, 1, 1];

/// RFC 8446 section 5: a `change_cipher_spec` before the first ClientHello is an unexpected
/// record, `unexpected_message`, even of the value 0x01.
#[test]
fn change_cipher_spec_before_the_hello_is_refused() {
    let server = TestServer::start(Config::default()).expect("the server starts");
    assert_eq!(
        answer(&server, &CHANGE_CIPHER_SPEC),
        (21, vec![2, AlertDescription::UNEXPECTED_MESSAGE.0])
    );
}

/// RFC 8446 section 5: a `change_cipher_spec` after the client's Finished is an unexpected
/// record, `unexpected_message`, even of the value 0x01. Written on the socket under the
/// client's connection, before its request.
#[test]
fn change_cipher_spec_after_the_finished_is_refused() {
    let server = TestServer::start(Config::default()).expect("the server starts");
    let stream = common::socket(&server, common::READ_TIMEOUT);
    let mut raw = stream.try_clone().expect("a second handle of the socket");
    let connection = Connection::new(
        stream,
        HOST,
        &common::accepting("ok-ecdsa-p256"),
        common::now(),
    )
    .expect("the handshake completes");
    raw.write_all(&CHANGE_CIPHER_SPEC).expect("write");
    drop(connection);
    let seen = server.wait_seen(1, Duration::from_secs(10));
    assert_eq!(seen.len(), 1, "the server ended one connection: {seen:?}");
    assert_eq!(seen[0].after_handshake[..1], [Received::ChangeCipherSpec]);
    assert_eq!(seen[0].end, "the server sent unexpected_message");
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

/// Set in the child process of [`drop_returns_when_the_wake_up_connect_fails`].
const STARVED: &str = "SURF_TLS_TEST_SERVER_STARVED";

/// The drop wakes the accepting thread with a connection of its own; when that cannot be made,
/// a join would wait forever. The test runs itself again in a child process whose limit of
/// file descriptors `sh` lowers to 64 (`ulimit -n`); there the server starts, every
/// descriptor left is taken, and the drop must return. The child's own result is read back,
/// so a filter that ran nothing fails.
#[cfg(target_os = "linux")]
#[test]
fn drop_returns_when_the_wake_up_connect_fails() {
    const NAME: &str = "hostile::drop_returns_when_the_wake_up_connect_fails";
    if std::env::var_os(STARVED).is_none() {
        let exe = std::env::current_exe().expect("the test binary");
        let out = std::process::Command::new("sh")
            .arg("-c")
            .arg("ulimit -n 64 && exec \"$0\" --exact \"$1\" --nocapture --test-threads 1")
            .arg(&exe)
            .arg(NAME)
            .env(STARVED, "1")
            .output()
            .expect("sh runs");
        let stdout = String::from_utf8_lossy(&out.stdout);
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            out.status.success() && stdout.contains("test result: ok. 1 passed"),
            "the child: {:?}\n{stdout}\n{stderr}",
            out.status
        );
        return;
    }
    let server = TestServer::start(Config::default()).expect("the server starts");
    let addr = server.addr();
    // One connection first, ended: after it the accepting thread is back in `accept`, which
    // on Linux takes its descriptor before it waits. Were every descriptor taken before it
    // got there, its `accept` would fail at once and the thread would see the stop flag by
    // itself.
    drop(TcpStream::connect(addr).expect("connect"));
    assert_eq!(server.wait_seen(1, Duration::from_secs(10)).len(), 1);
    thread::sleep(Duration::from_millis(50));
    let mut held = Vec::new();
    while let Ok(file) = std::fs::File::open("/dev/null") {
        held.push(file);
    }
    assert!(
        TcpStream::connect(addr).is_err(),
        "a connection was made with {} descriptors taken",
        held.len()
    );
    let (returned, dropped) = std::sync::mpsc::channel();
    thread::spawn(move || {
        drop(server);
        let _ = returned.send(());
    });
    // A bound against a hang only: a drop that joins the blocked thread never returns.
    let outcome = dropped.recv_timeout(Duration::from_secs(10));
    drop(held);
    assert_eq!(outcome, Ok(()), "the drop returned");
}
