//! Adversary cases of `story:tls-test-server`: is the server strict enough to be an oracle for
//! the client, does each mode do on the wire what its name says, and is the server itself safe
//! to run (no panic on garbage, every connection stopped on drop, an accept loop that sleeps).
//! Some cases drive the server with a hand-built client over the public modules of `surf-tls`;
//! some put a tampering relay between the `surf-tls` client and the server. Every socket has a
//! read timeout; no case bounds wall time.

use std::io::{ErrorKind, Read, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use surf_tls::connection::Connection;
use surf_tls::crypto::{EphemeralKey, Group, Random};
use surf_tls::error::{AlertDescription, Error, Violation};
use surf_tls::key_schedule::{self, CipherSuite};
use surf_tls::record::{Message, RecordLayer};
use surf_tls::transcript::Transcript;
use surf_tls::verify::LeafKey;
use surf_tls_test_server::{Config, Mode, Received, Seen, TestServer, leaf_key};

use crate::common::{self, Accepting, HOST};

const TIMEOUT: Duration = Duration::from_secs(1);
const SEEN_WITHIN: Duration = Duration::from_secs(3);

// ---------------------------------------------------------------------------------------------
// A ClientHello built by hand.

fn vec8(out: &mut Vec<u8>, data: &[u8]) {
    out.push(u8::try_from(data.len()).expect("vec8"));
    out.extend_from_slice(data);
}

fn vec16(out: &mut Vec<u8>, data: &[u8]) {
    out.extend_from_slice(&u16::try_from(data.len()).expect("vec16").to_be_bytes());
    out.extend_from_slice(data);
}

fn extension(out: &mut Vec<u8>, kind: u16, data: &[u8]) {
    out.extend_from_slice(&kind.to_be_bytes());
    vec16(out, data);
}

fn u16s(values: &[u16]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_be_bytes()).collect()
}

fn handshake_message(kind: u8, body: &[u8]) -> Vec<u8> {
    let len = u32::try_from(body.len()).expect("u24").to_be_bytes();
    let mut out = vec![kind, len[1], len[2], len[3]];
    out.extend_from_slice(body);
    out
}

fn record(content_type: u8, body: &[u8]) -> Vec<u8> {
    let mut out = vec![content_type, 3, 1];
    vec16(&mut out, body);
    out
}

/// What varies in a hand-built hello.
struct Hello<'a> {
    session_id: &'a [u8],
    compression: &'a [u8],
    supported_versions: bool,
    schemes: &'a [u16],
    share: &'a [u8],
}

impl Hello<'_> {
    fn with_share(share: &[u8]) -> Hello<'_> {
        Hello {
            session_id: &[],
            compression: &[0],
            supported_versions: true,
            schemes: &[0x0403, 0x0503, 0x0807, 0x0804, 0x0805, 0x0806],
            share,
        }
    }
}

/// A ClientHello message (its header included) as our client offers: the three suites, x25519
/// and secp256r1, the schemes of item 14, ALPN `http/1.1`, `server_name` `localhost`.
fn client_hello(h: &Hello<'_>) -> Vec<u8> {
    let mut body = vec![3, 3];
    body.extend_from_slice(&[0x11; 32]);
    vec8(&mut body, h.session_id);
    vec16(&mut body, &u16s(&[0x1301, 0x1302, 0x1303]));
    vec8(&mut body, h.compression);
    let mut ext = Vec::new();
    let mut name = vec![0];
    vec16(&mut name, HOST.as_bytes());
    let mut names = Vec::new();
    vec16(&mut names, &name);
    extension(&mut ext, 0, &names);
    let mut groups = Vec::new();
    vec16(&mut groups, &u16s(&[0x001d, 0x0017]));
    extension(&mut ext, 10, &groups);
    let mut schemes = Vec::new();
    vec16(&mut schemes, &u16s(h.schemes));
    extension(&mut ext, 13, &schemes);
    let mut protocol = Vec::new();
    vec8(&mut protocol, b"http/1.1");
    let mut alpn = Vec::new();
    vec16(&mut alpn, &protocol);
    extension(&mut ext, 16, &alpn);
    if h.supported_versions {
        let mut versions = Vec::new();
        vec8(&mut versions, &u16s(&[0x0304]));
        extension(&mut ext, 43, &versions);
    }
    let mut entry = u16s(&[0x001d]);
    vec16(&mut entry, h.share);
    let mut shares = Vec::new();
    vec16(&mut shares, &entry);
    extension(&mut ext, 51, &shares);
    vec16(&mut body, &ext);
    handshake_message(1, &body)
}

fn x25519() -> EphemeralKey {
    EphemeralKey::generate(Group::X25519, &Random::system()).expect("a key")
}

/// One record read off a raw socket: its type and content; `None` at the end or a timeout.
fn read_record(stream: &mut TcpStream) -> Option<(u8, Vec<u8>)> {
    let mut header = [0u8; 5];
    stream.read_exact(&mut header).ok()?;
    let mut body = vec![0u8; usize::from(u16::from_be_bytes([header[3], header[4]]))];
    stream.read_exact(&mut body).ok()?;
    Some((header[0], body))
}

/// Reads until the peer closes: the bytes read, or the kind of another end (a timeout).
fn until_closed(stream: &mut TcpStream) -> Result<usize, ErrorKind> {
    let mut total = 0;
    let mut buf = [0u8; 4096];
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

fn start(mode: Mode) -> TestServer {
    let config = Config {
        mode,
        ..Config::default()
    };
    TestServer::start(config).expect("the server starts")
}

fn one_seen(server: &TestServer) -> Seen {
    let seen = server.wait_seen(1, SEEN_WITHIN);
    assert_eq!(seen.len(), 1, "the server ended one connection: {seen:?}");
    seen.into_iter().next().expect("one")
}

// ---------------------------------------------------------------------------------------------
// Question 1: the server as an oracle of the ClientHello.

/// RFC 8446 section 4.1.2: `legacy_compression_methods` of a TLS 1.3 ClientHello "MUST contain
/// exactly one byte, set to zero"; on any other value "the server MUST abort the handshake with
/// an illegal_parameter alert". The server answers with a ServerHello instead, so a client that
/// offers compression passes every test of the server.
#[test]
fn adversary_hello_with_compression_other_than_null_is_refused() {
    let server = start(Mode::Normal);
    let key = x25519();
    // The control: the null method alone gets a ServerHello.
    let mut good = common::socket(&server, TIMEOUT);
    let hello = client_hello(&Hello::with_share(key.public_key()));
    good.write_all(&record(22, &hello)).expect("write");
    let first = read_record(&mut good).expect("a record");
    assert_eq!((first.0, first.1[0]), (22, 2), "a ServerHello to [0]");

    for compression in [&[1u8][..], &[0, 1][..]] {
        let mut stream = common::socket(&server, TIMEOUT);
        let hello = client_hello(&Hello {
            compression,
            ..Hello::with_share(key.public_key())
        });
        stream.write_all(&record(22, &hello)).expect("write");
        let first = read_record(&mut stream).map(|(t, b)| (t, b.first().copied()));
        assert_eq!(
            first,
            Some((21, Some(2))),
            "compression {compression:?}: a fatal alert first (RFC 8446 4.1.2), got {first:?}"
        );
    }
}

/// A TLS 1.3 server answers a hello without `supported_versions` with `protocol_version`
/// before any ServerHello (the client offers TLS 1.3 only there).
#[test]
fn adversary_hello_without_supported_versions_gets_protocol_version() {
    let server = start(Mode::Normal);
    let key = x25519();
    let mut stream = common::socket(&server, TIMEOUT);
    let hello = client_hello(&Hello {
        supported_versions: false,
        ..Hello::with_share(key.public_key())
    });
    stream.write_all(&record(22, &hello)).expect("write");
    assert_eq!(
        read_record(&mut stream),
        Some((21, vec![2, AlertDescription::PROTOCOL_VERSION.0]))
    );
}

/// `Config::scheme`: "the hello must offer it in `signature_algorithms`, else the server sends
/// `handshake_failure`". A hello without ecdsa_secp256r1_sha256 gets that alert first.
#[test]
fn adversary_hello_without_the_configured_scheme_gets_handshake_failure() {
    let server = start(Mode::Normal);
    let key = x25519();
    let mut stream = common::socket(&server, TIMEOUT);
    let hello = client_hello(&Hello {
        schemes: &[0x0503, 0x0807],
        ..Hello::with_share(key.public_key())
    });
    stream.write_all(&record(22, &hello)).expect("write");
    assert_eq!(
        read_record(&mut stream),
        Some((21, vec![2, AlertDescription::HANDSHAKE_FAILURE.0]))
    );
}

// ---------------------------------------------------------------------------------------------
// Question 1: the server as an oracle of the client's flight after the ServerHello.

/// What a hand-built client sends between the server's Finished and its own.
#[derive(Clone, Copy, Debug)]
enum Before {
    Nothing,
    /// A `change_cipher_spec` record holding this byte.
    ChangeCipherSpec(u8),
    /// A record of application data under the client's handshake key.
    ApplicationData,
}

fn server_share(server_hello: &[u8]) -> (u16, Vec<u8>) {
    let body = &server_hello[4..];
    let sid = usize::from(body[34]);
    let at = 35 + sid;
    let suite = u16::from_be_bytes([body[at], body[at + 1]]);
    let mut ext = &body[at + 5..];
    while ext.len() >= 4 {
        let kind = u16::from_be_bytes([ext[0], ext[1]]);
        let len = usize::from(u16::from_be_bytes([ext[2], ext[3]]));
        let data = &ext[4..4 + len];
        if kind == 51 {
            let key_len = usize::from(u16::from_be_bytes([data[2], data[3]]));
            return (suite, data[4..4 + key_len].to_vec());
        }
        ext = &ext[4 + len..];
    }
    panic!("no key_share in the ServerHello");
}

/// A client built from the public modules of `surf-tls`: the hello, the server's flight read
/// and checked by nothing, `before`, then its Finished (one bit flipped when `flip`); then it
/// shuts its write side and reads to the end. What the server saw.
fn hand_built_client(before: Before, flip: bool) -> Seen {
    let server = start(Mode::Normal);
    let key = x25519();
    let hello = client_hello(&Hello::with_share(key.public_key()));
    let mut layer = RecordLayer::new(common::socket(&server, TIMEOUT));
    layer.write_handshake(&hello).expect("the hello");
    let mut transcript = Transcript::new();
    transcript.add(&hello);
    let next = |layer: &mut RecordLayer<TcpStream>| loop {
        match layer.read_message().expect("a message") {
            Message::Handshake(m) => return m,
            Message::ChangeCipherSpec(_) => {}
            other => panic!("{other:?}"),
        }
    };
    let server_hello = next(&mut layer);
    let (code, public) = server_share(&server_hello);
    let suite = CipherSuite::from_code(code).expect("a suite");
    assert!(transcript.select(suite.hash()));
    transcript.add(&server_hello);
    let shared = key.agree(&public).expect("agree");
    let early = key_schedule::early_secret(suite.hash(), None);
    let handshake = key_schedule::handshake_secret(&early, shared.as_ref()).expect("hs");
    let hash = transcript.current().expect("hash");
    let secrets = key_schedule::handshake_traffic_secrets(&handshake, hash.as_ref()).expect("s");
    layer.set_read_keys(suite, &secrets.server).expect("keys");
    for _ in 0..4 {
        let message = next(&mut layer);
        transcript.add(&message);
    }
    let hash = transcript.current().expect("hash");
    layer.set_write_keys(suite, &secrets.client).expect("keys");
    match before {
        Before::Nothing => {}
        Before::ChangeCipherSpec(byte) => layer
            .get_mut()
            .write_all(&[20, 3, 3, 0, 1, byte])
            .expect("write"),
        Before::ApplicationData => layer.write_application_data(b"early").expect("write"),
    }
    let verify_data = key_schedule::verify_data(&secrets.client, hash.as_ref()).expect("vd");
    let mut verify_data = verify_data.as_ref().to_vec();
    if flip {
        verify_data[0] ^= 0x01;
    }
    // The server may have ended the connection on what came before: errors of the last two
    // writes are its answer, read back from what it saw.
    let _ = layer.write_handshake(&handshake_message(20, &verify_data));
    let mut stream = layer.into_inner();
    let _ = stream.shutdown(Shutdown::Write);
    let _ = until_closed(&mut stream);
    one_seen(&server)
}

/// The control: the hand-built client's own Finished is taken, the ticket sent.
#[test]
fn adversary_hand_built_client_completes() {
    let seen = hand_built_client(Before::Nothing, false);
    assert!(seen.ticket_sent, "{seen:?}");
}

/// A client Finished with one bit flipped is refused with `decrypt_error` (RFC 8446 4.4.4):
/// no test of the server sends one, so its check is otherwise untested.
#[test]
fn adversary_client_finished_with_a_flipped_bit_is_refused() {
    let seen = hand_built_client(Before::Nothing, true);
    assert!(!seen.ticket_sent, "{seen:?}");
    assert_eq!(seen.end, "the server sent decrypt_error");
}

/// Application data before the client's Finished is `unexpected_message`.
#[test]
fn adversary_application_data_before_finished_is_refused() {
    let seen = hand_built_client(Before::ApplicationData, false);
    assert!(!seen.ticket_sent, "{seen:?}");
    assert_eq!(seen.end, "the server sent unexpected_message");
}

/// RFC 8446 section 5: a `change_cipher_spec` of any value but 0x01 MUST be answered with
/// `unexpected_message`. The server drops every `change_cipher_spec` before the client's
/// Finished, whatever it holds, so a client that writes a wrong one passes.
#[test]
fn adversary_change_cipher_spec_of_another_value_is_refused() {
    let seen = hand_built_client(Before::ChangeCipherSpec(0x02), false);
    assert!(
        !seen.ticket_sent && seen.end.contains("unexpected_message"),
        "a change_cipher_spec of 0x02 was dropped and the handshake went on: {seen:?}"
    );
}

/// RFC 8446 section 5.1: a record carries at most 2^14 bytes, a literal here so that a limit
/// the client and the server share cannot drift with them. A request of 20,000 bytes reaches
/// the server in records of at most 16,384 bytes, and is answered.
#[test]
fn adversary_long_request_arrives_in_records_of_at_most_2_14() {
    let server = start(Mode::Normal);
    let mut connection = common::connect(&server, "ok-ecdsa-p256").expect("handshake");
    let fill = "a".repeat(20_000);
    let request = format!("GET / HTTP/1.1\r\nHost: {HOST}\r\nX-Fill: {fill}\r\n\r\n");
    connection.write_data(request.as_bytes()).expect("write");
    let mut response = Vec::new();
    let mut buf = [0u8; 4096];
    let end = loop {
        match connection.read_data(&mut buf) {
            Ok(0) => break Ok(()),
            Ok(n) => response.extend_from_slice(&buf[..n]),
            Err(e) => break Err(e),
        }
    };
    assert_eq!(end, Ok(()));
    assert_eq!(common::body(&response), b"ok");
    drop(connection);
    let seen = one_seen(&server);
    let lengths: Vec<usize> = seen
        .after_handshake
        .iter()
        .filter_map(|r| match r {
            Received::ApplicationData(n) => Some(*n),
            _ => None,
        })
        .collect();
    assert!(
        lengths.iter().all(|n| *n <= 16_384),
        "records over 2^14: {lengths:?}"
    );
    assert_eq!(lengths.iter().sum::<usize>(), request.len(), "{lengths:?}");
}

// ---------------------------------------------------------------------------------------------
// Question 1: checks of the client no mode of the server reaches, reached through a relay.

/// A relay between a client and `upstream` that applies `patch` to the first record the
/// server sends and copies everything else.
fn relay(upstream: SocketAddr, patch: fn(&mut [u8])) -> (SocketAddr, JoinHandle<()>) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind");
    let addr = listener.local_addr().expect("addr");
    let handle = thread::spawn(move || {
        let (client, _) = listener.accept().expect("accept");
        let server = TcpStream::connect(upstream).expect("connect");
        for s in [&client, &server] {
            s.set_read_timeout(Some(TIMEOUT)).expect("timeout");
            s.set_nodelay(true).expect("nodelay");
        }
        let (mut c_read, mut s_write) = (
            client.try_clone().expect("c"),
            server.try_clone().expect("s"),
        );
        let up = thread::spawn(move || {
            let mut buf = [0u8; 4096];
            while let Ok(n) = c_read.read(&mut buf) {
                if n == 0 || s_write.write_all(&buf[..n]).is_err() {
                    break;
                }
            }
            let _ = s_write.shutdown(Shutdown::Write);
        });
        let (mut s_read, mut c_write) = (server, client);
        let mut header = [0u8; 5];
        if s_read.read_exact(&mut header).is_ok() {
            let mut first = header.to_vec();
            let mut body = vec![0u8; usize::from(u16::from_be_bytes([header[3], header[4]]))];
            if s_read.read_exact(&mut body).is_ok() {
                first.extend_from_slice(&body);
                patch(&mut first);
                let _ = c_write.write_all(&first);
                let mut buf = [0u8; 4096];
                while let Ok(n) = s_read.read(&mut buf) {
                    if n == 0 || c_write.write_all(&buf[..n]).is_err() {
                        break;
                    }
                }
            }
        }
        let _ = c_write.shutdown(Shutdown::Both);
        let _ = up.join();
    });
    (addr, handle)
}

/// The client over a relay that patches the server's first record: how its handshake ends.
fn through_relay(server: &TestServer, case: &str, patch: fn(&mut [u8])) -> Result<(), Error> {
    let (addr, handle) = relay(server.addr(), patch);
    let stream = TcpStream::connect(addr).expect("connect");
    stream.set_read_timeout(Some(TIMEOUT)).expect("timeout");
    stream.set_nodelay(true).expect("nodelay");
    let outcome = Connection::new(stream, HOST, &common::accepting(case), common::now()).map(drop);
    handle.join().expect("the relay ends");
    outcome
}

/// The first byte of the echoed `legacy_session_id` of the ServerHello: record header 5,
/// handshake header 4, version 2, random 32, length 1.
fn flip_session_id(first: &mut [u8]) {
    first[44] ^= 0x01;
}

/// RFC 8446 4.1.3: a ServerHello whose `legacy_session_id_echo` differs ends the client with
/// `illegal_parameter`. The server always echoes, so no test of the server reaches the check.
#[test]
fn adversary_client_refuses_a_wrong_session_id_echo() {
    let server = start(Mode::Normal);
    assert_eq!(
        through_relay(&server, "ok-ecdsa-p256", flip_session_id),
        Err(Error::AlertSent {
            description: AlertDescription::ILLEGAL_PARAMETER,
            violation: Violation::SessionIdMismatch,
        })
    );
}

/// The last 8 bytes of the ServerHello's random set to the TLS 1.2 downgrade value.
fn downgrade_value(first: &mut [u8]) {
    first[35..43].copy_from_slice(b"DOWNGRD\x01");
}

/// RFC 8446 4.1.3: a TLS 1.2 ServerHello carrying the downgrade value ends the client with
/// `illegal_parameter`, not with the plain version refusal of kind (c).
#[test]
fn adversary_client_refuses_the_downgrade_value() {
    let mut config = Config::new("ok-rsa2048", surf_tls::crypto::Scheme::RsaPssRsaeSha256);
    config.mode = Mode::SelectsTls12;
    let server = TestServer::start(config).expect("the server starts");
    assert_eq!(
        through_relay(&server, "ok-rsa2048", downgrade_value),
        Err(Error::AlertSent {
            description: AlertDescription::ILLEGAL_PARAMETER,
            violation: Violation::Downgrade,
        })
    );
}

/// RFC 8446 4.4.3: a CertificateVerify that the leaf's key does not verify ends the client with
/// `decrypt_error`. No mode sends a bad signature; a verifier double that answers another
/// P-256 key gives the client one.
#[test]
fn adversary_client_refuses_a_certificate_verify_of_another_key() {
    let server = start(Mode::Normal);
    let other = EphemeralKey::generate(Group::Secp256r1, &Random::system()).expect("a key");
    let leaf = leaf_key("ok-ecdsa-p256").expect("the leaf");
    assert_ne!(leaf.bytes, other.public_key());
    let double = Accepting(LeafKey {
        kind: leaf.kind,
        bytes: other.public_key().to_vec(),
    });
    let stream = common::socket(&server, TIMEOUT);
    let outcome = Connection::new(stream, HOST, &double, common::now()).map(drop);
    assert_eq!(
        outcome,
        Err(Error::AlertSent {
            description: AlertDescription::DECRYPT_ERROR,
            violation: Violation::BadCertificateVerify,
        })
    );
}

// ---------------------------------------------------------------------------------------------
// Question 3: the modes on the wire.

/// `NeverAnswer` "accepts and reads, sends nothing" (story, Change): also to a client whose
/// first bytes are not a record.
#[test]
fn adversary_never_answer_sends_nothing_to_garbage() {
    let server = start(Mode::NeverAnswer);
    let mut stream = common::socket(&server, Duration::from_secs(3));
    stream
        .write_all(&[0xde, 0xad, 0xbe, 0xef, 0x00, 0x01, 0x02, 0x03, 0xff, 0xfe])
        .expect("write");
    assert_eq!(
        until_closed(&mut stream),
        Ok(0),
        "bytes sent in NeverAnswer"
    );
}

/// `StallAfterServerHello` (`Mode` doc): the ServerHello and, to a hello with a session id of
/// compatibility mode, the `change_cipher_spec` after it; then nothing.
#[test]
fn adversary_stall_after_server_hello_sends_change_cipher_spec_only_with_a_session_id() {
    let server = start(Mode::StallAfterServerHello);
    for (session_id, want) in [(&[0x22; 32][..], vec![22, 20]), (&[][..], vec![22])] {
        let key = x25519();
        let mut stream = common::socket(&server, Duration::from_millis(300));
        let hello = client_hello(&Hello {
            session_id,
            ..Hello::with_share(key.public_key())
        });
        stream.write_all(&record(22, &hello)).expect("write");
        let mut types = Vec::new();
        while let Some((content_type, _)) = read_record(&mut stream) {
            types.push(content_type);
        }
        assert_eq!(
            types,
            want,
            "the records sent after a hello with a session id of {} bytes",
            session_id.len()
        );
    }
}

/// A client that writes one byte per segment gets what one that writes its flights whole gets:
/// a completed handshake and the body in `Normal`, the alert of kind (a), and `BadFinished`.
#[test]
fn adversary_one_byte_segments_change_nothing() {
    struct Trickle(TcpStream);
    impl Read for Trickle {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            self.0.read(buf)
        }
    }
    impl Write for Trickle {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            match buf.first() {
                Some(b) => self.0.write(std::slice::from_ref(b)),
                None => Ok(0),
            }
        }
        fn flush(&mut self) -> std::io::Result<()> {
            self.0.flush()
        }
    }
    let server = start(Mode::Normal);
    let stream = Trickle(common::socket(&server, TIMEOUT));
    let mut connection = Connection::new(
        stream,
        HOST,
        &common::accepting("ok-ecdsa-p256"),
        common::now(),
    )
    .expect("the handshake completes");
    let (response, end) = common::get(&mut connection, "/");
    assert_eq!((end, common::body(&response)), (Ok(()), &b"ok"[..]));

    for (mode, want) in [
        (
            Mode::Tls12ReadsSupportedVersions,
            Error::AlertReceived {
                description: AlertDescription::PROTOCOL_VERSION,
                before_server_hello: true,
            },
        ),
        (
            Mode::BadFinished,
            Error::AlertSent {
                description: AlertDescription::DECRYPT_ERROR,
                violation: Violation::BadFinished,
            },
        ),
    ] {
        let server = start(mode);
        let stream = Trickle(common::socket(&server, TIMEOUT));
        let outcome = Connection::new(
            stream,
            HOST,
            &common::accepting("ok-ecdsa-p256"),
            common::now(),
        )
        .map(drop);
        assert_eq!(outcome, Err(want), "{mode:?}");
    }
}

// ---------------------------------------------------------------------------------------------
// Question 4: the server itself.

/// A fixed sequence of bytes (a linear congruential generator, fixed seed).
fn garbage(seed: u32, len: usize) -> Vec<u8> {
    let mut state = seed.wrapping_mul(2_654_435_761).wrapping_add(1);
    (0..len)
        .map(|_| {
            state = state.wrapping_mul(1_103_515_245).wrapping_add(12_345);
            (state >> 16) as u8
        })
        .collect()
}

/// No input kills a connection's thread: raw garbage of each length 0 to 600, a handshake
/// record holding a ClientHello header and garbage of each length 0 to 600, and every prefix
/// of a valid ClientHello framed as whole. Each connection ends with what the server saw (a
/// thread that panics records nothing), and the server serves a client after them.
#[test]
fn adversary_garbage_of_each_length_ends_every_connection() {
    let server = start(Mode::Normal);
    let key = x25519();
    let valid = client_hello(&Hello::with_share(key.public_key()));
    let mut inputs: Vec<Vec<u8>> = Vec::new();
    for len in 0..=600 {
        inputs.push(garbage(len as u32, len));
        let body = garbage(1000 + len as u32, len);
        inputs.push(record(22, &handshake_message(1, &body)));
    }
    for cut in 4..valid.len() {
        let body = &valid[4..cut];
        inputs.push(record(22, &handshake_message(1, body)));
    }
    for input in &inputs {
        let mut stream = common::socket(&server, TIMEOUT);
        // The server may have closed already, after its alert: errors of these two are its
        // answer, not the test's.
        let _ = stream.write_all(input);
        let _ = stream.shutdown(Shutdown::Write);
        assert!(until_closed(&mut stream).is_ok(), "closed after {input:?}");
    }
    let seen = server.wait_seen(inputs.len(), SEEN_WITHIN);
    assert_eq!(seen.len(), inputs.len(), "connections ended");
    assert!(seen.iter().all(|s| !s.server_hello_sent));
    let mut connection = common::connect(&server, "ok-ecdsa-p256").expect("served after");
    let (response, _) = common::get(&mut connection, "/");
    assert_eq!(common::body(&response), b"ok");
}

/// In every mode, a drop with 20 open connections (10 that sent a hello, 10 silent) stops each
/// of them: every socket reads its end.
#[test]
fn adversary_drop_stops_twenty_connections_in_every_mode() {
    let key = x25519();
    let hello = record(22, &client_hello(&Hello::with_share(key.public_key())));
    for mode in [
        Mode::Normal,
        Mode::Tls12ReadsSupportedVersions,
        Mode::Tls12IgnoresSupportedVersions,
        Mode::SelectsTls12,
        Mode::StallAfterServerHello,
        Mode::NeverAnswer,
        Mode::OversizedCertificate,
        Mode::BadFinished,
        Mode::NoCloseNotify,
    ] {
        let server = start(mode);
        let mut sockets: Vec<TcpStream> = (0..20)
            .map(|i| {
                let mut s = common::socket(&server, TIMEOUT);
                if i % 2 == 0 {
                    s.write_all(&hello).expect("write");
                }
                s
            })
            .collect();
        let mut polls = 0;
        while server.accepted() < 20 {
            polls += 1;
            assert!(polls < 200, "{mode:?}: accepted {}", server.accepted());
            thread::sleep(Duration::from_millis(5));
        }
        drop(server);
        let closed = sockets
            .iter_mut()
            .map(|s| until_closed(s).is_ok())
            .filter(|closed| *closed)
            .count();
        assert_eq!(closed, 20, "{mode:?}: sockets closed by the drop");
    }
}

/// The accepting thread sleeps while nobody connects: its context switches over 1 s of idle
/// (Linux, `/proc/self/task`; each test is a process of its own under nextest). The server
/// binds 127.0.0.1 only.
#[cfg(target_os = "linux")]
#[test]
fn adversary_idle_accept_loop_does_not_wake() {
    fn switches() -> u64 {
        let mut total = 0;
        for task in std::fs::read_dir("/proc/self/task")
            .expect("tasks")
            .flatten()
        {
            let comm = std::fs::read_to_string(task.path().join("comm")).unwrap_or_default();
            if comm.trim() != "tls-test-server" {
                continue;
            }
            let status = std::fs::read_to_string(task.path().join("status")).unwrap_or_default();
            for line in status.lines() {
                if let Some(v) = line
                    .strip_prefix("voluntary_ctxt_switches:")
                    .or_else(|| line.strip_prefix("nonvoluntary_ctxt_switches:"))
                {
                    total += v.trim().parse::<u64>().unwrap_or(0);
                }
            }
        }
        total
    }
    let server = start(Mode::Normal);
    assert!(server.addr().ip().is_loopback());
    thread::sleep(Duration::from_millis(50));
    let before = switches();
    thread::sleep(Duration::from_secs(1));
    let woke = switches() - before;
    assert!(woke <= 2, "the idle accept loop woke {woke} times in 1 s");
}
