//! A TLS 1.3 test server on 127.0.0.1 for end-to-end tests of surf (test tooling, never
//! shipped; `story:tls-test-server`, docs/design/tls.md items 6, 7, 12 and 14).
//!
//! [`TestServer::start`] binds 127.0.0.1 port 0 and serves each accepted connection on a thread
//! of its own: the server side of RFC 8446 section 4 as far as the tests need it, built from
//! the public modules `record`, `key_schedule` and `transcript` of `surf-tls` and the signing
//! of its `crypto` (feature `test-tooling`), with the server's messages written here; then one
//! HTTP/1.1 response per connection, looked up by path in [`Config::responses`]. The
//! certificates and keys are those of `crates/tls-test-certs/certs/`.
//!
//! On order ([`Mode`]) it misbehaves as the negative cases need: the three kinds of a server
//! limited to TLS 1.2 (item 7), a stall, silence, an oversized Certificate, a bad Finished, a
//! response without `close_notify`.
//!
//! What the server saw and did on each connection is read back with [`TestServer::wait_seen`]
//! ([`Seen`]). A connection whose client sends nothing for [`IDLE`] is closed. Dropping the
//! server stops every connection and joins every thread it started.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod conn;
mod wire;

use std::collections::BTreeMap;
use std::io;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use surf_tls::crypto::{Group, Scheme, SigningKey};
use surf_tls::error::AlertDescription;
use surf_tls::key_schedule::CipherSuite;
use surf_tls::verify::LeafKey;
use surf_tls::x509::Certificate;

/// How long a connection may go without a byte from the client before the server closes it.
pub const IDLE: Duration = Duration::from_secs(2);

/// How the server answers (`story:tls-test-server`, Change).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// The handshake, one `NewSessionTicket` after the client's Finished (as servers commonly
    /// do), the response, `close_notify`.
    Normal,
    /// Kind (a) of item 7: a server limited to TLS 1.2 that reads `supported_versions` and,
    /// finding no TLS 1.2 there, sends `protocol_version` before any ServerHello (as OpenSSL
    /// 1.1.1 and later limited to TLS 1.2 do; item 12).
    Tls12ReadsSupportedVersions,
    /// Kind (b): a server limited to TLS 1.2 that chooses by `legacy_version` and holds TLS 1.2
    /// suites only; to a hello that offers none it sends `handshake_failure` before any
    /// ServerHello.
    Tls12IgnoresSupportedVersions,
    /// Kind (c), hostile: whatever the hello, a TLS 1.2 ServerHello selecting 0xC02F without a
    /// downgrade value.
    SelectsTls12,
    /// The ServerHello, then nothing.
    StallAfterServerHello,
    /// Accepts and reads raw bytes, past the record layer; sends nothing, whatever arrives
    /// (bytes that are no record among them). A ClientHello in the first record is read for
    /// its `server_name` and ALPN.
    NeverAnswer,
    /// A Certificate whose header declares 16 MiB less one byte (2^24 - 1, the most a uint24
    /// holds); only the header is sent.
    OversizedCertificate,
    /// A Finished whose `verify_data` has one bit flipped.
    BadFinished,
    /// The response without `Content-Length`, its body delimited by the end of the
    /// connection, which is closed without `close_notify`.
    NoCloseNotify,
}

/// One HTTP/1.1 response.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Response {
    /// The status code.
    pub status: u16,
    /// The reason phrase.
    pub reason: String,
    /// The header fields, in order; `Content-Length` and `Connection` are added by the server.
    pub fields: Vec<(String, String)>,
    /// The body.
    pub body: Vec<u8>,
}

impl Response {
    /// `200 OK` with `body`, `Content-Type: text/plain`.
    pub fn ok(body: &[u8]) -> Response {
        Response {
            status: 200,
            reason: "OK".to_string(),
            fields: vec![("Content-Type".to_string(), "text/plain".to_string())],
            body: body.to_vec(),
        }
    }

    /// `404 Not Found`, the answer to a path the map does not hold.
    pub fn not_found() -> Response {
        Response {
            status: 404,
            reason: "Not Found".to_string(),
            fields: vec![("Content-Type".to_string(), "text/plain".to_string())],
            body: b"not found".to_vec(),
        }
    }

    /// The bytes of the response, with `Content-Length` when `with_length`, else delimited
    /// by the end of the connection.
    fn to_bytes(&self, with_length: bool) -> Vec<u8> {
        let mut out = format!("HTTP/1.1 {} {}\r\n", self.status, self.reason).into_bytes();
        for (name, value) in &self.fields {
            out.extend_from_slice(format!("{name}: {value}\r\n").as_bytes());
        }
        if with_length {
            out.extend_from_slice(format!("Content-Length: {}\r\n", self.body.len()).as_bytes());
        }
        out.extend_from_slice(b"Connection: close\r\n\r\n");
        out.extend_from_slice(&self.body);
        out
    }
}

/// What the server is started with.
#[derive(Clone, Debug)]
pub struct Config {
    /// The certificate case of `crates/tls-test-certs/certs/`: its `chain.pem` is sent and its
    /// `leaf.pk8` signs.
    pub case: String,
    /// The scheme the CertificateVerify is signed with; the hello must offer it in
    /// `signature_algorithms`, else the server sends `handshake_failure`.
    pub scheme: Scheme,
    /// The suites the server accepts, in its order of preference.
    pub suites: Vec<CipherSuite>,
    /// The groups the server accepts, in its order of preference.
    pub groups: Vec<Group>,
    /// Ask for secp256r1 by HelloRetryRequest, whatever share the first hello holds.
    pub hello_retry_request: bool,
    /// How the server answers.
    pub mode: Mode,
    /// The response of each path; a path not here gets [`Response::not_found`].
    pub responses: BTreeMap<String, Response>,
}

impl Config {
    /// The case `case` signing with `scheme`; every suite and group, no HelloRetryRequest,
    /// [`Mode::Normal`], and `/` answered `200 OK` with the body `ok`.
    pub fn new(case: &str, scheme: Scheme) -> Config {
        let mut responses = BTreeMap::new();
        responses.insert("/".to_string(), Response::ok(b"ok"));
        Config {
            case: case.to_string(),
            scheme,
            suites: CipherSuite::ALL.to_vec(),
            groups: vec![Group::X25519, Group::Secp256r1],
            hello_retry_request: false,
            mode: Mode::Normal,
            responses,
        }
    }
}

impl Default for Config {
    /// The case `ok-ecdsa-p256` with ecdsa_secp256r1_sha256.
    fn default() -> Self {
        Config::new("ok-ecdsa-p256", Scheme::EcdsaSecp256r1Sha256)
    }
}

/// A record the server read after the client's Finished.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Received {
    /// Application data, its length.
    ApplicationData(usize),
    /// An alert.
    Alert(AlertDescription),
    /// A handshake message, its type.
    Handshake(u8),
    /// A `change_cipher_spec`.
    ChangeCipherSpec,
}

/// What the server saw and did on one connection, for the test to read back.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Seen {
    /// The `server_name` of the first ClientHello.
    pub server_name: Option<String>,
    /// The ALPN protocols the first ClientHello offered.
    pub alpn: Vec<String>,
    /// The TLS 1.3 suite selected.
    pub suite: Option<CipherSuite>,
    /// The group of the key exchange.
    pub group: Option<Group>,
    /// A HelloRetryRequest was sent.
    pub hello_retry_request_sent: bool,
    /// A ServerHello was sent (of TLS 1.3, or of TLS 1.2 in the modes that send one).
    pub server_hello_sent: bool,
    /// The scheme the CertificateVerify was signed with.
    pub scheme: Option<Scheme>,
    /// A NewSessionTicket was sent.
    pub ticket_sent: bool,
    /// The request line read.
    pub request: Option<String>,
    /// Every record read after the client's Finished, in order.
    pub after_handshake: Vec<Received>,
    /// Every alert the client sent, in order.
    pub alerts: Vec<AlertDescription>,
    /// How the connection ended, in words.
    pub end: String,
}

/// What every connection of a server reads: the config, the certificates, the key.
pub(crate) struct Setup {
    config: Config,
    chain: Vec<Vec<u8>>,
    key: SigningKey,
}

struct Shared {
    stop: AtomicBool,
    accepted: AtomicUsize,
    seen: Mutex<Vec<Seen>>,
    ended: Condvar,
}

/// A TLS 1.3 server on 127.0.0.1; stopped, its threads joined, when dropped.
pub struct TestServer {
    addr: SocketAddr,
    shared: Arc<Shared>,
    acceptor: Option<JoinHandle<()>>,
}

impl TestServer {
    /// Reads the case's certificates and key, binds 127.0.0.1 port 0 and starts accepting.
    pub fn start(config: Config) -> io::Result<TestServer> {
        let chain = chain(&config.case)?;
        let pkcs8 = std::fs::read(certs_dir().join(&config.case).join("leaf.pk8"))?;
        let key = conn::signing_key(config.scheme, &pkcs8).map_err(io::Error::other)?;
        let setup = Arc::new(Setup { config, chain, key });
        let listener = TcpListener::bind(("127.0.0.1", 0))?;
        let addr = listener.local_addr()?;
        let shared = Arc::new(Shared {
            stop: AtomicBool::new(false),
            accepted: AtomicUsize::new(0),
            seen: Mutex::new(Vec::new()),
            ended: Condvar::new(),
        });
        let acceptor = {
            let shared = Arc::clone(&shared);
            thread::Builder::new()
                .name("tls-test-server".to_string())
                .spawn(move || accept(listener, &shared, &setup))?
        };
        Ok(TestServer {
            addr,
            shared,
            acceptor: Some(acceptor),
        })
    }

    /// The address the server listens on.
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    /// The port the server listens on.
    pub fn port(&self) -> u16 {
        self.addr.port()
    }

    /// The connections accepted so far.
    pub fn accepted(&self) -> usize {
        self.shared.accepted.load(Ordering::SeqCst)
    }

    /// What the server saw on each connection it has ended so far, in the order they ended.
    pub fn seen(&self) -> Vec<Seen> {
        match self.shared.seen.lock() {
            Ok(seen) => seen.clone(),
            Err(poisoned) => poisoned.into_inner().clone(),
        }
    }

    /// Waits until the server has ended `count` connections, at most `within`; then what it
    /// saw on each, in the order they ended.
    pub fn wait_seen(&self, count: usize, within: Duration) -> Vec<Seen> {
        let deadline = Instant::now() + within;
        let mut seen = match self.shared.seen.lock() {
            Ok(seen) => seen,
            Err(poisoned) => poisoned.into_inner(),
        };
        while seen.len() < count {
            let now = Instant::now();
            if now >= deadline {
                break;
            }
            seen = match self.shared.ended.wait_timeout(seen, deadline - now) {
                Ok((seen, _)) => seen,
                Err(poisoned) => poisoned.into_inner().0,
            };
        }
        seen.clone()
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.shared.stop.store(true, Ordering::SeqCst);
        // Wakes the accepting thread out of `accept`.
        let _ = TcpStream::connect_timeout(&self.addr, Duration::from_secs(1));
        if let Some(acceptor) = self.acceptor.take() {
            let _ = acceptor.join();
        }
    }
}

/// Accepts until the server stops; each connection on a thread of its own, all joined before
/// it returns.
fn accept(listener: TcpListener, shared: &Arc<Shared>, setup: &Arc<Setup>) {
    let mut workers: Vec<JoinHandle<()>> = Vec::new();
    for stream in listener.incoming() {
        if shared.stop.load(Ordering::SeqCst) {
            break;
        }
        let stream = match stream {
            Ok(stream) => stream,
            Err(_) => {
                thread::sleep(Duration::from_millis(10));
                continue;
            }
        };
        shared.accepted.fetch_add(1, Ordering::SeqCst);
        let (shared_c, setup_c) = (Arc::clone(shared), Arc::clone(setup));
        let worker = thread::Builder::new()
            .name("tls-test-server-connection".to_string())
            .spawn(move || {
                let seen = conn::serve(stream, &setup_c, &shared_c.stop);
                match shared_c.seen.lock() {
                    Ok(mut all) => all.push(seen),
                    Err(poisoned) => poisoned.into_inner().push(seen),
                }
                shared_c.ended.notify_all();
            });
        match worker {
            Ok(worker) => workers.push(worker),
            Err(_) => continue,
        }
        let (done, running): (Vec<_>, Vec<_>) =
            workers.into_iter().partition(JoinHandle::is_finished);
        for worker in done {
            let _ = worker.join();
        }
        workers = running;
    }
    for worker in workers {
        let _ = worker.join();
    }
}

/// The directory of the test certificates, `crates/tls-test-certs/certs/`.
pub fn certs_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../tls-test-certs/certs")
}

/// The certificates of a case's `chain.pem`, the leaf first.
pub fn chain(case: &str) -> io::Result<Vec<Vec<u8>>> {
    let text = std::fs::read(certs_dir().join(case).join("chain.pem"))?;
    let blocks = surf_tls::pem::parse(&text).map_err(io::Error::other)?;
    Ok(blocks.into_iter().map(|b| b.data).collect())
}

/// The key of a case's leaf, as the certificate reader of `surf-tls` returns it: what a
/// verifier double answers.
pub fn leaf_key(case: &str) -> io::Result<LeafKey> {
    let chain = chain(case)?;
    let leaf = chain
        .first()
        .ok_or_else(|| io::Error::other(format!("{case}: chain.pem holds no certificate")))?;
    let cert = Certificate::from_der(leaf).map_err(io::Error::other)?;
    Ok(LeafKey::from_certificate(&cert))
}
