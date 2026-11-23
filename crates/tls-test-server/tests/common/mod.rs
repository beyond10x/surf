//! What the tests share: the client of `surf-tls` over a socket with a read timeout, the
//! verifier double, and one GET read to its end.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

use surf_tls::connection::Connection;
use surf_tls::der::Time;
use surf_tls::error::Error;
use surf_tls::verify::{CertificateRejection, LeafKey, ServerCertVerifier};
use surf_tls_test_server::{TestServer, leaf_key};

/// The host every test connects for; the certificates of the `ok-*` cases name it.
pub const HOST: &str = "localhost";

/// The read timeout of a client that completes its handshake.
pub const READ_TIMEOUT: Duration = Duration::from_secs(2);

/// The time of the library tests (docs/design/tls.md item 13); the double does not read it.
pub fn now() -> Time {
    Time::from_utc(2026, 11, 22, 0, 0, 0).expect("a time")
}

/// A verifier that accepts any certificates and answers the key of one case's leaf.
pub struct Accepting(pub LeafKey);

impl ServerCertVerifier for Accepting {
    fn verify(
        &self,
        _certificates: &[&[u8]],
        _host: &str,
        _now: Time,
    ) -> Result<LeafKey, CertificateRejection> {
        Ok(self.0.clone())
    }
}

/// The double for `case`.
pub fn accepting(case: &str) -> Accepting {
    Accepting(leaf_key(case).expect("the leaf of the case reads"))
}

/// A socket to `server` with `timeout` on reads and writes, and `TCP_NODELAY`: the client
/// writes its `change_cipher_spec` and its Finished in two writes, and without it the second
/// waits for the server's delayed ACK (about 40 ms a handshake on Linux, measured).
pub fn socket(server: &TestServer, timeout: Duration) -> TcpStream {
    let stream = TcpStream::connect(server.addr()).expect("connect to the test server");
    stream
        .set_read_timeout(Some(timeout))
        .expect("read timeout");
    stream
        .set_write_timeout(Some(timeout))
        .expect("write timeout");
    stream.set_nodelay(true).expect("TCP_NODELAY");
    stream
}

/// The client's handshake with `server`, the double answering the leaf of `case`.
pub fn connect(server: &TestServer, case: &str) -> Result<Connection<TcpStream>, Error> {
    Connection::new(socket(server, READ_TIMEOUT), HOST, &accepting(case), now())
}

/// Writes `GET <path> HTTP/1.1` and reads until the end of the stream: the bytes read and how
/// the reading ended (`Ok` at `close_notify`).
pub fn get<S: Read + Write>(
    connection: &mut Connection<S>,
    path: &str,
) -> (Vec<u8>, Result<(), Error>) {
    let request = format!("GET {path} HTTP/1.1\r\nHost: {HOST}\r\nConnection: close\r\n\r\n");
    if let Err(error) = connection.write_data(request.as_bytes()) {
        return (Vec::new(), Err(error));
    }
    let mut out = Vec::new();
    let mut buf = [0u8; 4096];
    loop {
        match connection.read_data(&mut buf) {
            Ok(0) => return (out, Ok(())),
            Ok(n) => out.extend_from_slice(&buf[..n]),
            Err(error) => return (out, Err(error)),
        }
    }
}

/// The body of a response: the bytes after the first empty line.
pub fn body(response: &[u8]) -> &[u8] {
    match response.windows(4).position(|w| w == b"\r\n\r\n") {
        Some(at) => &response[at + 4..],
        None => &[],
    }
}

/// The status line of a response.
pub fn status_line(response: &[u8]) -> String {
    let end = response
        .windows(2)
        .position(|w| w == b"\r\n")
        .unwrap_or(response.len());
    String::from_utf8_lossy(&response[..end]).into_owned()
}
