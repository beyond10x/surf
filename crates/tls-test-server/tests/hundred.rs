//! 100 handshakes in one test (story `tls-test-server`, acceptance 5), each with one GET read
//! to its end. The test asserts counts, not time: a wall-time bound failed on a loaded host
//! (the same test took 0.077 s to 2.3 s). It prints its time and the time of one handshake for
//! the fetch and session stories to plan within the test step; the runner ends it at 60 s.

use std::time::{Duration, Instant};

use surf_tls::error::AlertDescription;
use surf_tls_test_server::{Config, Received, TestServer};

use crate::common;

const HANDSHAKES: u32 = 100;

/// How long the test waits for the server to end its side of the last connections.
const SEEN_WITHIN: Duration = Duration::from_secs(10);

#[test]
fn hundred_handshakes() {
    let server = TestServer::start(Config::default()).expect("the server starts");
    let started = Instant::now();
    for _ in 0..HANDSHAKES {
        let mut connection =
            common::connect(&server, "ok-ecdsa-p256").expect("the handshake completes");
        let (response, end) = common::get(&mut connection, "/");
        assert_eq!(end, Ok(()));
        assert_eq!(common::body(&response), b"ok");
    }
    let took = started.elapsed();
    let count = HANDSHAKES as usize;
    assert_eq!(server.accepted(), count);
    let seen = server.wait_seen(count, SEEN_WITHIN);
    assert_eq!(seen.len(), count, "the server ended every connection");
    for (i, s) in seen.iter().enumerate() {
        assert!(s.ticket_sent, "connection {i}: a NewSessionTicket: {s:?}");
        assert_eq!(
            s.request.as_deref(),
            Some("GET / HTTP/1.1"),
            "connection {i}"
        );
        assert!(
            matches!(
                s.after_handshake.as_slice(),
                [
                    Received::ApplicationData(_),
                    Received::Alert(AlertDescription::CLOSE_NOTIFY)
                ]
            ),
            "connection {i}: {:?}",
            s.after_handshake
        );
    }
    eprintln!(
        "hundred: {HANDSHAKES} handshakes with a GET in {took:?}, one in {:?}",
        took / HANDSHAKES
    );
}
