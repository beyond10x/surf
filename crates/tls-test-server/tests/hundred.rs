//! 100 handshakes in one test (story `tls-test-server`, acceptance 5), each with one GET read
//! to its end, under 5 s together. The time of one handshake is printed for the fetch and
//! session stories to plan within the test step.

use std::time::{Duration, Instant};

use surf_tls_test_server::{Config, TestServer};

use crate::common;

const HANDSHAKES: u32 = 100;

#[test]
fn hundred_handshakes_under_five_seconds() {
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
    assert_eq!(server.accepted(), HANDSHAKES as usize);
    eprintln!(
        "hundred: {HANDSHAKES} handshakes with a GET in {took:?}, one in {:?}",
        took / HANDSHAKES
    );
    assert!(took < Duration::from_secs(5), "took {took:?}");
}
