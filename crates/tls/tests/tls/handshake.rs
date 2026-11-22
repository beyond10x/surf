//! The client's handshake of `surf-tls` (`story:tls-handshake`), each case from server bytes
//! crafted in memory: our ClientHello against `tests/data/client-hello.golden` and
//! `client-hello-retry.golden`, the RFC 8448 traces of sections 3, 5 and 7 through the test
//! mode `GivenClientHello`, the handshakes that complete, the close, and the two tables
//! `rfc8446-client-musts.txt` and `rfc8446-client-fields.txt`, whose every row names its tests.
//!
//! The server of these tests is [`Server`]: it writes the server's records from the client's
//! own hellos with the key schedule of `story:tls-record`, signs its CertificateVerify with the
//! key of a case of `crates/tls-test-certs`, and reads the client's records back. The verifier
//! is a double ([`Double`]) that answers as the test orders. `handshake_hostile.rs` and
//! `flights.rs` use both.

#![forbid(unsafe_code)]

use std::cell::RefCell;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use surf_tls::connection::Connection;
use surf_tls::crypto::{self, AeadKey, EphemeralKey, Group, Random, Scheme, SigningKey};
use surf_tls::der::Time;
use surf_tls::error::{AlertDescription, Error, Violation};
use surf_tls::handshake::trace::{GivenClientHello, Memory};
use surf_tls::key_schedule::{self, CipherSuite, Secret, TrafficSecrets};
use surf_tls::path::PathVerifier;
use surf_tls::pem;
use surf_tls::transcript::Transcript;
use surf_tls::verify::{CertificateRejection, LeafKey, ServerCertVerifier};
use surf_tls::x509::Certificate;

use crate::rfc8448::Pipe;

// ---------------------------------------------------------------------------------------------
// Encoding.

pub fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn from_hex(text: &str) -> Vec<u8> {
    let digits: Vec<u8> = text
        .bytes()
        .filter(|b| b.is_ascii_hexdigit())
        .map(|b| match b {
            b'0'..=b'9' => b - b'0',
            b'a'..=b'f' => b - b'a' + 10,
            _ => b - b'A' + 10,
        })
        .collect();
    assert!(
        digits.len().is_multiple_of(2),
        "an even number of hex digits"
    );
    digits.chunks(2).map(|p| p[0] << 4 | p[1]).collect()
}

/// A handshake message of `msg_type` around `body`.
pub fn msg(msg_type: u8, body: &[u8]) -> Vec<u8> {
    let len = body.len();
    let mut out = vec![msg_type, (len >> 16) as u8, (len >> 8) as u8, len as u8];
    out.extend_from_slice(body);
    out
}

pub fn v8(bytes: &[u8]) -> Vec<u8> {
    let mut out = vec![bytes.len() as u8];
    out.extend_from_slice(bytes);
    out
}

pub fn v16(bytes: &[u8]) -> Vec<u8> {
    let mut out = (bytes.len() as u16).to_be_bytes().to_vec();
    out.extend_from_slice(bytes);
    out
}

pub fn v24(bytes: &[u8]) -> Vec<u8> {
    let len = bytes.len();
    let mut out = vec![(len >> 16) as u8, (len >> 8) as u8, len as u8];
    out.extend_from_slice(bytes);
    out
}

/// An extension block with its 2-byte length.
pub fn block(extensions: &[(u16, Vec<u8>)]) -> Vec<u8> {
    let mut inner = Vec::new();
    for (t, data) in extensions {
        inner.extend_from_slice(&t.to_be_bytes());
        inner.extend_from_slice(&v16(data));
    }
    v16(&inner)
}

/// A record without protection.
pub fn record(content_type: u8, content: &[u8]) -> Vec<u8> {
    let mut out = vec![content_type, 3, 3];
    out.extend_from_slice(&(content.len() as u16).to_be_bytes());
    out.extend_from_slice(content);
    out
}

/// The records of `bytes`: each its outer type and its body.
pub fn records(bytes: &[u8]) -> Vec<(u8, Vec<u8>)> {
    let mut out = Vec::new();
    let mut rest = bytes;
    while rest.len() >= 5 {
        let len = usize::from(u16::from_be_bytes([rest[3], rest[4]]));
        let end = (5 + len).min(rest.len());
        out.push((rest[0], rest[5..end].to_vec()));
        rest = &rest[end..];
    }
    out
}

// ---------------------------------------------------------------------------------------------
// A ClientHello read by the tests, independently of the crate's own reader.

pub struct Hello {
    pub legacy_version: u16,
    pub random: Vec<u8>,
    pub session_id: Vec<u8>,
    pub suites: Vec<u16>,
    pub compression: Vec<u8>,
    pub extensions: Vec<(u16, Vec<u8>)>,
}

impl Hello {
    pub fn parse(message: &[u8]) -> Hello {
        assert_eq!(message[0], 1, "a ClientHello");
        let len =
            usize::from(message[1]) << 16 | usize::from(message[2]) << 8 | usize::from(message[3]);
        assert_eq!(len + 4, message.len(), "the length of the ClientHello");
        let b = &message[4..];
        let legacy_version = u16::from_be_bytes([b[0], b[1]]);
        let random = b[2..34].to_vec();
        let sid_len = usize::from(b[34]);
        let session_id = b[35..35 + sid_len].to_vec();
        let mut at = 35 + sid_len;
        let suites_len = usize::from(u16::from_be_bytes([b[at], b[at + 1]]));
        let suites = b[at + 2..at + 2 + suites_len]
            .chunks(2)
            .map(|c| u16::from_be_bytes([c[0], c[1]]))
            .collect();
        at += 2 + suites_len;
        let comp_len = usize::from(b[at]);
        let compression = b[at + 1..at + 1 + comp_len].to_vec();
        at += 1 + comp_len;
        let ext_len = usize::from(u16::from_be_bytes([b[at], b[at + 1]]));
        at += 2;
        assert_eq!(at + ext_len, b.len(), "the extensions end the hello");
        let mut extensions = Vec::new();
        while at < b.len() {
            let t = u16::from_be_bytes([b[at], b[at + 1]]);
            let l = usize::from(u16::from_be_bytes([b[at + 2], b[at + 3]]));
            extensions.push((t, b[at + 4..at + 4 + l].to_vec()));
            at += 4 + l;
        }
        Hello {
            legacy_version,
            random,
            session_id,
            suites,
            compression,
            extensions,
        }
    }

    pub fn extension(&self, t: u16) -> Option<&[u8]> {
        self.extensions
            .iter()
            .find(|(e, _)| *e == t)
            .map(|(_, d)| d.as_slice())
    }

    /// The key shares: group and public key.
    pub fn shares(&self) -> Vec<(u16, Vec<u8>)> {
        let data = self.extension(51).expect("key_share");
        let mut out = Vec::new();
        let mut at = 2;
        while at < data.len() {
            let group = u16::from_be_bytes([data[at], data[at + 1]]);
            let l = usize::from(u16::from_be_bytes([data[at + 2], data[at + 3]]));
            out.push((group, data[at + 4..at + 4 + l].to_vec()));
            at += 4 + l;
        }
        out
    }
}

// ---------------------------------------------------------------------------------------------
// The client under test.

pub const HOST: &str = "localhost";

/// The time of the library tests (docs/design/tls.md item 13).
pub fn now() -> Time {
    Time::from_utc(2026, 11, 22, 0, 0, 0).expect("a time")
}

/// The client's draws: its random, its session id, its x25519 key, its secp256r1 key.
pub static DRAWS: [&[u8]; 4] = [&[0x11; 32], &[0x22; 32], &[0x33; 32], &[0x44; 32]];

/// The client over `pipe`, with the fixed draws, for `host`.
pub fn connect_to<'p>(
    pipe: &'p mut Pipe,
    host: &str,
    verifier: &dyn ServerCertVerifier,
) -> Result<Connection<&'p mut Pipe>, Error> {
    Connection::with_random(pipe, host, verifier, now(), &Random::fixed_sequence(&DRAWS))
}

pub fn connect<'p>(
    pipe: &'p mut Pipe,
    verifier: &dyn ServerCertVerifier,
) -> Result<Connection<&'p mut Pipe>, Error> {
    connect_to(pipe, HOST, verifier)
}

/// A verifier that answers as ordered and keeps what it was given.
pub struct Double {
    pub answer: Result<LeafKey, CertificateRejection>,
    pub seen: RefCell<Vec<Vec<Vec<u8>>>>,
    pub hosts: RefCell<Vec<String>>,
}

impl Double {
    pub fn accepting(case: &str) -> Double {
        Double::answering(Ok(leaf_key(case)))
    }

    pub fn answering(answer: Result<LeafKey, CertificateRejection>) -> Double {
        Double {
            answer,
            seen: RefCell::new(Vec::new()),
            hosts: RefCell::new(Vec::new()),
        }
    }
}

impl ServerCertVerifier for Double {
    fn verify(
        &self,
        certificates: &[&[u8]],
        host: &str,
        _now: Time,
    ) -> Result<LeafKey, CertificateRejection> {
        self.seen
            .borrow_mut()
            .push(certificates.iter().map(|c| c.to_vec()).collect());
        self.hosts.borrow_mut().push(host.to_string());
        self.answer.clone()
    }
}

// ---------------------------------------------------------------------------------------------
// The cases of crates/tls-test-certs.

pub fn certs_dir() -> PathBuf {
    crate_dir().join("../tls-test-certs/certs")
}

/// The certificates of a case's `chain.pem`, the leaf first.
pub fn chain(case: &str) -> Vec<Vec<u8>> {
    let text = fs::read(certs_dir().join(case).join("chain.pem")).expect("read chain.pem");
    pem::parse(&text)
        .expect("PEM")
        .into_iter()
        .map(|b| b.data)
        .collect()
}

/// The key of a case's leaf as the certificate reader returns it.
pub fn leaf_key(case: &str) -> LeafKey {
    let chain = chain(case);
    let cert = Certificate::from_der(&chain[0]).expect("the leaf reads");
    LeafKey::from_certificate(&cert)
}

/// The scheme the server signs its CertificateVerify in for a case.
pub fn scheme_of(case: &str) -> Scheme {
    match case {
        "ok-ecdsa-p256" => Scheme::EcdsaSecp256r1Sha256,
        "ok-ecdsa-p384" => Scheme::EcdsaSecp384r1Sha384,
        "ok-ed25519" => Scheme::Ed25519,
        "ok-rsa2048" | "ok-rsa-pss" => Scheme::RsaPssRsaeSha256,
        other => panic!("no scheme for {other}"),
    }
}

// ---------------------------------------------------------------------------------------------
// The server of the tests.

/// The fields of a ServerHello or HelloRetryRequest, to write as given.
#[derive(Clone)]
pub struct Sh {
    pub legacy_version: u16,
    pub random: [u8; 32],
    pub session_id: Vec<u8>,
    pub suite: u16,
    pub compression: u8,
    pub extensions: Vec<(u16, Vec<u8>)>,
}

impl Sh {
    pub fn encode(&self) -> Vec<u8> {
        let mut body = self.legacy_version.to_be_bytes().to_vec();
        body.extend_from_slice(&self.random);
        body.extend_from_slice(&v8(&self.session_id));
        body.extend_from_slice(&self.suite.to_be_bytes());
        body.push(self.compression);
        body.extend_from_slice(&block(&self.extensions));
        msg(2, &body)
    }

    pub fn without(mut self, t: u16) -> Sh {
        self.extensions.retain(|(e, _)| *e != t);
        self
    }

    pub fn with(mut self, t: u16, data: Vec<u8>) -> Sh {
        self.extensions.push((t, data));
        self
    }

    pub fn replace(mut self, t: u16, data: Vec<u8>) -> Sh {
        for (e, d) in &mut self.extensions {
            if *e == t {
                *d = data.clone();
            }
        }
        self
    }
}

/// The random of a HelloRetryRequest.
pub fn hrr_random() -> [u8; 32] {
    let digest = crypto::hash(crypto::HashAlgorithm::Sha256, b"HelloRetryRequest");
    let mut out = [0u8; 32];
    out.copy_from_slice(digest.as_ref());
    out
}

pub fn group_code(group: Group) -> u16 {
    match group {
        Group::X25519 => 0x001d,
        Group::Secp256r1 => 0x0017,
    }
}

const SERVER_KEY: [u8; 32] = [0x55; 32];

fn server_key(group: Group) -> EphemeralKey {
    EphemeralKey::generate(group, &Random::fixed_slice(&SERVER_KEY)).expect("a server key")
}

/// What the client wrote, decoded: each record's type (the inner one when protected), its
/// content, and whether it was protected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Written {
    pub content_type: u8,
    pub content: Vec<u8>,
    pub protected: bool,
}

/// The server side of a test: the records the client will read, written step by step.
pub struct Server {
    pub suite: CipherSuite,
    pub case: String,
    pub host: String,
    /// The bytes the client will read.
    pub input: Vec<u8>,
    pub transcript: Transcript,
    /// The client's hellos so far.
    pub hellos: Vec<Vec<u8>>,
    pub handshake: Option<TrafficSecrets>,
    master: Option<Secret>,
    pub application: Option<TrafficSecrets>,
    /// The server's current write secret and the sequence number of its next record.
    write: Option<(Secret, u64)>,
    /// Handshake messages pushed and not yet written.
    pending: Vec<u8>,
    /// The application secret to write under after the pending record.
    switch: Option<Secret>,
    /// The server's Finished was pushed: messages after it stay out of the transcript.
    done: bool,
    /// The transcript hash after the server's Finished, and after an empty client
    /// Certificate too.
    pub finished_hash: Option<(crypto::Digest, crypto::Digest)>,
}

/// The client's output for `input` with the fixed draws; the connection is given up without
/// its `close_notify`.
pub fn client_written(input: &[u8]) -> Vec<u8> {
    client_written_to(HOST, input)
}

/// As [`client_written`], for `host`.
pub fn client_written_to(host: &str, input: &[u8]) -> Vec<u8> {
    let mut pipe = Pipe::with_input(input.to_vec());
    let verifier = Double::accepting("ok-ecdsa-p256");
    if let Ok(connection) = connect_to(&mut pipe, host, &verifier) {
        connection.into_inner();
    }
    pipe.output
}

/// The handshake messages of the unprotected handshake records at the start of `written`.
pub fn hellos_in(written: &[u8]) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    for (t, body) in records(written) {
        match t {
            22 => out.push(body),
            20 => {}
            _ => break,
        }
    }
    out
}

impl Server {
    pub fn new(suite: CipherSuite) -> Server {
        Server::with_case(suite, "ok-ecdsa-p256")
    }

    pub fn with_case(suite: CipherSuite, case: &str) -> Server {
        Server::for_host(suite, case, HOST)
    }

    pub fn for_host(suite: CipherSuite, case: &str, host: &str) -> Server {
        let first = hellos_in(&client_written_to(host, &[]));
        assert_eq!(first.len(), 1, "one ClientHello before any answer");
        let mut transcript = Transcript::new();
        transcript.add(&first[0]);
        Server {
            suite,
            case: case.to_string(),
            host: host.to_string(),
            input: Vec::new(),
            transcript,
            hellos: first,
            handshake: None,
            master: None,
            application: None,
            write: None,
            pending: Vec::new(),
            switch: None,
            done: false,
            finished_hash: None,
        }
    }

    pub fn last_hello(&self) -> Hello {
        Hello::parse(self.hellos.last().expect("a hello"))
    }

    /// A ServerHello that answers the last hello with a share of `group`.
    pub fn server_hello(&self, group: Group) -> Sh {
        let key = server_key(group);
        let mut share = group_code(group).to_be_bytes().to_vec();
        share.extend_from_slice(&v16(key.public_key()));
        Sh {
            legacy_version: 0x0303,
            random: [0x77; 32],
            session_id: self.last_hello().session_id,
            suite: self.suite.code(),
            compression: 0,
            extensions: vec![(43, vec![3, 4]), (51, share)],
        }
    }

    /// A HelloRetryRequest for `group` (if any) with `cookie` (if any).
    pub fn hello_retry_request(&self, group: Option<u16>, cookie: Option<&[u8]>) -> Sh {
        let mut extensions = vec![(43, vec![3, 4])];
        if let Some(group) = group {
            extensions.push((51, group.to_be_bytes().to_vec()));
        }
        if let Some(cookie) = cookie {
            extensions.push((44, v16(cookie)));
        }
        Sh {
            legacy_version: 0x0303,
            random: hrr_random(),
            session_id: self.last_hello().session_id,
            suite: self.suite.code(),
            compression: 0,
            extensions,
        }
    }

    /// A record without protection.
    pub fn plain(&mut self, content_type: u8, content: &[u8]) {
        self.input.extend_from_slice(&record(content_type, content));
    }

    /// The HelloRetryRequest `sh`: its record, the transcript of RFC 8446 section 4.4.1, and
    /// the client's second hello, read from what the client writes for it.
    pub fn send_retry(&mut self, sh: &Sh) {
        let message = sh.encode();
        self.plain(22, &message);
        self.transcript
            .hello_retry_request(self.suite.hash(), &message)
            .expect("one retry");
        let hellos = hellos_in(&client_written_to(&self.host, &self.input));
        assert_eq!(hellos.len(), 2, "the second hello");
        self.transcript.add(&hellos[1]);
        self.hellos.push(hellos[1].clone());
    }

    /// The ServerHello `sh` in its record, the transcript, and the handshake keys of the share
    /// it carries.
    pub fn send_hello(&mut self, sh: &Sh) {
        let message = sh.encode();
        self.plain(22, &message);
        assert!(self.transcript.select(self.suite.hash()));
        self.transcript.add(&message);
        let share = sh
            .extensions
            .iter()
            .find(|(t, _)| *t == 51)
            .map(|(_, d)| d.clone())
            .expect("a key share");
        let code = u16::from_be_bytes([share[0], share[1]]);
        let group = if code == 0x001d {
            Group::X25519
        } else {
            Group::Secp256r1
        };
        let peer = self
            .last_hello()
            .shares()
            .into_iter()
            .find(|(g, _)| *g == code)
            .map(|(_, k)| k)
            .expect("the client's share");
        let shared = server_key(group).agree(&peer).expect("agree");
        let early = key_schedule::early_secret(self.suite.hash(), None);
        let handshake = key_schedule::handshake_secret(&early, shared.as_ref()).expect("hs");
        let hash = self.transcript.current().expect("hash");
        let secrets =
            key_schedule::handshake_traffic_secrets(&handshake, hash.as_ref()).expect("traffic");
        self.master = Some(key_schedule::master_secret(&handshake).expect("master"));
        self.write = Some((secrets.server.clone(), 0));
        self.handshake = Some(secrets);
    }

    /// The default ServerHello with an x25519 share, sent.
    pub fn hello(&mut self) {
        let sh = self.server_hello(Group::X25519);
        self.send_hello(&sh);
    }

    /// A handshake message pushed into the next protected record; it joins the transcript
    /// until the server's Finished.
    pub fn push(&mut self, message: &[u8]) {
        if !self.done {
            self.transcript.add(message);
        }
        self.pending.extend_from_slice(message);
    }

    /// The pushed messages in protected records of at most 2^14 bytes each.
    pub fn flush(&mut self) {
        let pending = std::mem::take(&mut self.pending);
        for fragment in pending.chunks(16_384) {
            self.protected(22, fragment);
        }
        if let Some(next) = self.switch.take() {
            self.write = Some((next, 0));
        }
    }

    /// One protected record of `content` and its type, under the server's current key.
    pub fn protected(&mut self, content_type: u8, content: &[u8]) {
        let (secret, sequence) = self.write.as_mut().expect("server keys");
        let mut inner = content.to_vec();
        inner.push(content_type);
        let sealed = seal(self.suite, secret, *sequence, &inner);
        *sequence += 1;
        self.input.extend_from_slice(&sealed);
    }

    pub fn encrypted_extensions(&self, extensions: &[(u16, Vec<u8>)]) -> Vec<u8> {
        msg(8, &block(extensions))
    }

    /// The case's chain in a Certificate message, each entry without extensions.
    pub fn certificate(&self) -> Vec<u8> {
        certificate_message(&chain(&self.case), &[])
    }

    /// The CertificateVerify over the transcript so far, signed with the case's key.
    pub fn certificate_verify(&self) -> Vec<u8> {
        let scheme = scheme_of(&self.case);
        let hash = self.transcript.current().expect("hash");
        let mut content = vec![0x20; 64];
        content.extend_from_slice(b"TLS 1.3, server CertificateVerify");
        content.push(0);
        content.extend_from_slice(hash.as_ref());
        let pk8 = fs::read(certs_dir().join(&self.case).join("leaf.pk8")).expect("leaf.pk8");
        let random = Random::fixed_byte(0x5a);
        let key = SigningKey::from_pkcs8(scheme, &pk8, &random).expect("the case's key");
        let signature = key.sign(&content, &random).expect("sign");
        let mut body = scheme.code().to_be_bytes().to_vec();
        body.extend_from_slice(&v16(&signature));
        msg(15, &body)
    }

    /// The server's Finished over the transcript so far.
    pub fn finished(&self) -> Vec<u8> {
        let secrets = self.handshake.as_ref().expect("handshake secrets");
        let hash = self.transcript.current().expect("hash");
        let data = key_schedule::verify_data(&secrets.server, hash.as_ref()).expect("verify");
        msg(20, data.as_ref())
    }

    /// Pushes the Finished, derives the application secrets, and writes under them after the
    /// next flush.
    pub fn push_finished(&mut self) {
        let finished = self.finished();
        self.push(&finished);
        self.done = true;
        let hash = self.transcript.current().expect("hash");
        let master = self.master.clone().expect("master");
        let application =
            key_schedule::application_traffic_secrets(&master, hash.as_ref()).expect("app");
        self.switch = Some(application.server.clone());
        self.application = Some(application);
        self.transcript.add(&certificate_message(&[], &[]));
        let with_certificate = self.transcript.current().expect("hash");
        self.finished_hash = Some((hash, with_certificate));
    }

    /// EncryptedExtensions without extensions, Certificate, CertificateVerify, Finished, in one
    /// record.
    pub fn flight(&mut self) {
        let ee = self.encrypted_extensions(&[]);
        self.push(&ee);
        let certificate = self.certificate();
        self.push(&certificate);
        let cv = self.certificate_verify();
        self.push(&cv);
        self.push_finished();
        self.flush();
    }

    /// A whole handshake: ServerHello, then [`Server::flight`].
    pub fn handshake(suite: CipherSuite) -> Server {
        let mut server = Server::new(suite);
        server.hello();
        server.flight();
        server
    }

    /// Application data under the server's current key.
    pub fn data(&mut self, data: &[u8]) {
        self.protected(23, data);
    }

    /// A protected alert.
    pub fn alert(&mut self, level: u8, description: u8) {
        self.protected(21, &[level, description]);
    }

    /// A KeyUpdate under the current key; the server writes under the next one after it.
    pub fn key_update(&mut self, update_requested: u8) {
        self.protected(22, &msg(24, &[update_requested]));
        let (secret, _) = self.write.take().expect("keys");
        let next = key_schedule::next_traffic_secret(&secret).expect("next");
        self.write = Some((next, 0));
    }

    /// The client's records, opened with the client's keys as they change: the handshake key
    /// until its Finished, the application key after, the next one after each KeyUpdate.
    pub fn client_records(&self, written: &[u8]) -> Vec<Written> {
        let mut out = Vec::new();
        let mut key: Option<(Secret, u64)> = self.handshake.as_ref().map(|h| (h.client.clone(), 0));
        for (t, body) in records(written) {
            if t != 23 {
                out.push(Written {
                    content_type: t,
                    content: body,
                    protected: false,
                });
                continue;
            }
            let (secret, sequence) = key.as_mut().expect("client keys");
            let inner = open(self.suite, secret, *sequence, &written_record(t, &body))
                .expect("the client's record opens");
            *sequence += 1;
            let at = inner.iter().rposition(|&b| b != 0).expect("a type");
            let content_type = inner[at];
            let content = inner[..at].to_vec();
            if content_type == 22 {
                if contains_message(&content, 20) {
                    let app = self.application.as_ref().expect("application secrets");
                    key = Some((app.client.clone(), 0));
                } else if contains_message(&content, 24) {
                    let (secret, _) = key.take().expect("keys");
                    key = Some((key_schedule::next_traffic_secret(&secret).expect("next"), 0));
                }
            }
            out.push(Written {
                content_type,
                content,
                protected: true,
            });
        }
        out
    }

    /// The client Finished the transcript calls for.
    pub fn expected_client_finished(&self, with_certificate: bool) -> Vec<u8> {
        let (plain, certified) = self.finished_hash.as_ref().expect("the server's Finished");
        let hash = if with_certificate { certified } else { plain };
        let secrets = self.handshake.as_ref().expect("secrets");
        msg(
            20,
            key_schedule::verify_data(&secrets.client, hash.as_ref())
                .expect("verify")
                .as_ref(),
        )
    }
}

/// A Certificate message of `certificates`, each entry with the extensions `entry`.
pub fn certificate_message(certificates: &[Vec<u8>], entry: &[(u16, Vec<u8>)]) -> Vec<u8> {
    let mut list = Vec::new();
    for c in certificates {
        list.extend_from_slice(&v24(c));
        list.extend_from_slice(&block(entry));
    }
    let mut body = vec![0];
    body.extend_from_slice(&v24(&list));
    msg(11, &body)
}

fn contains_message(content: &[u8], msg_type: u8) -> bool {
    let mut at = 0;
    while at + 4 <= content.len() {
        if content[at] == msg_type {
            return true;
        }
        let len = usize::from(content[at + 1]) << 16
            | usize::from(content[at + 2]) << 8
            | usize::from(content[at + 3]);
        at += 4 + len;
    }
    false
}

fn written_record(t: u8, body: &[u8]) -> Vec<u8> {
    let mut out = vec![t, 3, 3];
    out.extend_from_slice(&(body.len() as u16).to_be_bytes());
    out.extend_from_slice(body);
    out
}

fn nonce(suite: CipherSuite, secret: &Secret, sequence: u64) -> (AeadKey, [u8; 12]) {
    let keys = key_schedule::traffic_key(suite, secret).expect("keys");
    let key = AeadKey::new(suite.aead(), keys.key()).expect("key");
    let mut nonce = *keys.iv();
    for (i, b) in sequence.to_be_bytes().iter().enumerate() {
        nonce[4 + i] ^= b;
    }
    (key, nonce)
}

/// A protected record of the inner plaintext `inner` under `secret` at `sequence`.
pub fn seal(suite: CipherSuite, secret: &Secret, sequence: u64, inner: &[u8]) -> Vec<u8> {
    let (key, nonce) = nonce(suite, secret, sequence);
    let len = (inner.len() + 16) as u16;
    let mut record = vec![23, 3, 3];
    record.extend_from_slice(&len.to_be_bytes());
    let mut body = inner.to_vec();
    key.seal(&nonce, &record, &mut body).expect("seal");
    record.extend_from_slice(&body);
    record
}

/// The inner plaintext of the protected record `record` under `secret` at `sequence`.
pub fn open(suite: CipherSuite, secret: &Secret, sequence: u64, record: &[u8]) -> Option<Vec<u8>> {
    let (key, nonce) = nonce(suite, secret, sequence);
    let mut body = record[5..].to_vec();
    let opened = key.open(&nonce, &record[..5], &mut body).ok()?;
    Some(opened.to_vec())
}

/// Runs the client over the server's input: the handshake's error, if any, and every byte the
/// client wrote (a connection that completes is given up without `close_notify`).
pub fn outcome(input: &[u8], verifier: &dyn ServerCertVerifier) -> (Result<(), Error>, Vec<u8>) {
    outcome_to(HOST, input, verifier)
}

/// As [`outcome`], for `host`.
pub fn outcome_to(
    host: &str,
    input: &[u8],
    verifier: &dyn ServerCertVerifier,
) -> (Result<(), Error>, Vec<u8>) {
    let mut pipe = Pipe::with_input(input.to_vec());
    let result = connect_to(&mut pipe, host, verifier).map(|c| {
        c.into_inner();
    });
    (result, pipe.output)
}

/// `AlertSent` of `description` and `violation`.
pub fn sent(description: AlertDescription, violation: Violation) -> Error {
    Error::AlertSent {
        description,
        violation,
    }
}

/// The last record the client wrote is the fatal alert `description`, protected or not.
pub fn last_alert(server: &Server, written: &[u8]) -> Option<(u8, u8)> {
    let last = server.client_records(written).pop()?;
    (last.content_type == 21).then(|| (last.content[0], last.content[1]))
}

// ---------------------------------------------------------------------------------------------
// The ClientHello.

fn golden(name: &str) -> Vec<u8> {
    let text =
        fs::read_to_string(crate_dir().join("tests/data").join(name)).expect("the golden file");
    from_hex(&text)
}

/// Checks the hello surf writes for `host`, field by field, against RFC 8446 section 4.1.2
/// and item 14; the record holds it whole.
fn check_own_hello(record_bytes: &[u8], server_name: Option<&str>, second: bool) -> Hello {
    assert_eq!(
        &record_bytes[..3],
        &[22, 3, 3],
        "a handshake record of 0x0303"
    );
    let hello = Hello::parse(&record_bytes[5..]);
    assert_eq!(hello.legacy_version, 0x0303);
    assert_eq!(hello.random, vec![0x11; 32], "the random from the source");
    assert_eq!(hello.session_id, vec![0x22; 32], "a new 32-byte session id");
    assert_eq!(
        hello.suites,
        vec![0x1301, 0x1302, 0x1303],
        "the three TLS 1.3 suites only"
    );
    assert_eq!(hello.compression, vec![0], "exactly one byte 0");
    let mut types: Vec<u16> = hello.extensions.iter().map(|(t, _)| *t).collect();
    let mut expected: Vec<u16> = vec![10, 13, 50, 16, 43, 51];
    if server_name.is_some() {
        expected.insert(0, 0);
    }
    if second {
        expected.push(44);
    }
    assert_eq!(types, expected, "the extensions, in order");
    types.sort_unstable();
    types.dedup();
    assert_eq!(types.len(), expected.len(), "no extension twice");
    match server_name {
        Some(name) => {
            let mut entry = vec![0];
            entry.extend_from_slice(&v16(name.as_bytes()));
            assert_eq!(hello.extension(0), Some(&v16(&entry)[..]));
        }
        None => assert_eq!(hello.extension(0), None),
    }
    assert_eq!(hello.extension(10), Some(&v16(&[0, 0x1d, 0, 0x17])[..]));
    let sig: Vec<u8> = [0x0403u16, 0x0503, 0x0807, 0x0804, 0x0805, 0x0806]
        .iter()
        .flat_map(|s| s.to_be_bytes())
        .collect();
    assert_eq!(hello.extension(13), Some(&v16(&sig)[..]));
    let mut cert = sig.clone();
    for s in [0x0401u16, 0x0501, 0x0601] {
        cert.extend_from_slice(&s.to_be_bytes());
    }
    assert_eq!(hello.extension(50), Some(&v16(&cert)[..]));
    assert_eq!(hello.extension(16), Some(&v16(&v8(b"http/1.1"))[..]));
    assert_eq!(hello.extension(43), Some(&[2, 3, 4][..]), "0x0304 only");
    for absent in [41, 42, 45] {
        assert_eq!(hello.extension(absent), None, "no PSK, no early data");
    }
    hello
}

/// Acceptance 1: our ClientHello with a fixed random source is byte for byte
/// `client-hello.golden`; with the host 127.0.0.1 (and [::1]) it has no `server_name`.
#[test]
fn client_hello_golden() {
    let written = client_written(&[]);
    let first = &records_raw(&written)[0];
    let hello = check_own_hello(first, Some(HOST), false);
    let shares = hello.shares();
    assert_eq!(shares.len(), 1, "one share");
    assert_eq!(shares[0].0, 0x001d, "x25519");
    assert_eq!(hello.extension(44), None, "no cookie in a first hello");
    assert_eq!(
        to_hex(first),
        to_hex(&golden("client-hello.golden")),
        "the ClientHello record against client-hello.golden"
    );

    for host in ["127.0.0.1", "[::1]", "::1"] {
        let mut pipe = Pipe::default();
        let verifier = Double::accepting("ok-ecdsa-p256");
        assert_eq!(
            connect_to(&mut pipe, host, &verifier).map(|_| ()),
            Err(Error::ConnectionClosed)
        );
        check_own_hello(&records_raw(&pipe.output)[0], None, false);
    }
    let mut pipe = Pipe::default();
    let verifier = Double::accepting("ok-ecdsa-p256");
    let _ = connect_to(&mut pipe, "example.org.", &verifier);
    check_own_hello(&records_raw(&pipe.output)[0], Some("example.org"), false);
}

/// The records of `bytes`, each whole.
pub fn records_raw(bytes: &[u8]) -> Vec<Vec<u8>> {
    let mut out = Vec::new();
    let mut rest = bytes;
    while rest.len() >= 5 {
        let len = usize::from(u16::from_be_bytes([rest[3], rest[4]]));
        let end = (5 + len).min(rest.len());
        out.push(rest[..end].to_vec());
        rest = &rest[end..];
    }
    out
}

/// Acceptance 6.1: after a HelloRetryRequest for secp256r1 with a 40-byte cookie, the second
/// ClientHello holds that cookie, one secp256r1 share and the other extensions unchanged, in a
/// record of 0x0303, byte for byte `client-hello-retry.golden`.
#[test]
fn retry_second_hello() {
    let mut server = Server::new(CipherSuite::Aes128GcmSha256);
    let cookie: Vec<u8> = (0..40).collect();
    let hrr = server.hello_retry_request(Some(0x0017), Some(&cookie));
    server.send_retry(&hrr);
    let written = client_written(&server.input);
    let raw = records_raw(&written);
    assert_eq!(raw.len(), 2, "two hellos and nothing between them");
    let first = Hello::parse(&raw[0][5..]);
    let second = check_own_hello(&raw[1], Some(HOST), true);
    assert_eq!(
        second.extension(44),
        Some(&v16(&cookie)[..]),
        "the cookie copied"
    );
    let shares = second.shares();
    assert_eq!(shares.len(), 1);
    assert_eq!(shares[0].0, 0x0017, "one secp256r1 share");
    assert_eq!(shares[0].1.len(), 65, "an uncompressed point");
    for (t, data) in &first.extensions {
        if *t != 51 {
            assert_eq!(
                second.extension(*t),
                Some(&data[..]),
                "extension {t} unchanged"
            );
        }
    }
    assert_eq!(first.random, second.random);
    assert_eq!(first.session_id, second.session_id);
    assert_eq!(
        to_hex(&raw[1]),
        to_hex(&golden("client-hello-retry.golden")),
        "the second ClientHello record against client-hello-retry.golden"
    );
    // And the handshake completes over secp256r1.
    let sh = server.server_hello(Group::Secp256r1);
    server.send_hello(&sh);
    server.flight();
    let (result, written) = outcome(&server.input, &Double::accepting("ok-ecdsa-p256"));
    assert_eq!(result, Ok(()));
    let client = server.client_records(&written);
    let finished = client.iter().find(|w| w.protected).expect("the Finished");
    assert_eq!(finished.content, server.expected_client_finished(false));
}

// ---------------------------------------------------------------------------------------------
// Handshakes that complete.

/// Each suite completes, with the ECDSA P-256 case; the client's flight is a
/// `change_cipher_spec` and its Finished, and its data opens under its application key.
#[test]
fn handshake_completes_with_each_suite() {
    for suite in CipherSuite::ALL {
        let mut server = Server::handshake(suite);
        server.data(b"ok");
        let mut pipe = Pipe::with_input(server.input.clone());
        let verifier = Double::accepting("ok-ecdsa-p256");
        let mut connection = connect(&mut pipe, &verifier).expect("the handshake completes");
        assert_eq!(connection.suite(), suite);
        let mut buf = [0u8; 16];
        let n = connection.read_data(&mut buf).expect("data");
        assert_eq!(&buf[..n], b"ok");
        connection.write_data(b"GET").expect("write");
        connection.into_inner();
        let client = server.client_records(&pipe.output);
        let kinds: Vec<(u8, bool)> = client
            .iter()
            .map(|w| (w.content_type, w.protected))
            .collect();
        assert_eq!(
            kinds,
            vec![(22, false), (20, false), (22, true), (23, true)],
            "{suite:?}"
        );
        assert_eq!(
            client[1].content,
            vec![1],
            "the change_cipher_spec of compatibility mode"
        );
        assert_eq!(client[2].content, server.expected_client_finished(false));
        assert_eq!(client[3].content, b"GET");
        assert_eq!(verifier.hosts.borrow().as_slice(), &[HOST.to_string()]);
    }
}

/// Each key kind of the cases signs a CertificateVerify the client accepts: RSA-PSS, ECDSA
/// P-256 and P-384, Ed25519.
#[test]
fn handshake_completes_with_each_key() {
    for case in ["ok-rsa2048", "ok-ecdsa-p256", "ok-ecdsa-p384", "ok-ed25519"] {
        let mut server = Server::with_case(CipherSuite::Aes128GcmSha256, case);
        server.hello();
        server.flight();
        let (result, _) = outcome(&server.input, &Double::accepting(case));
        assert_eq!(result, Ok(()), "{case}");
    }
}

/// The seam of the handshake and the path verifier: a whole handshake through the shipped
/// `PathVerifier`, whose one anchor is the test root of `ok-ecdsa-p256` added with `add_root`.
/// The right host completes and reads data; a wrong host ends with `NameMismatch` and the
/// fatal alert that `CertificateRejection::alert()` names.
#[test]
fn handshake_through_the_path_verifier() {
    let case = "ok-ecdsa-p256";
    let root = fs::read(certs_dir().join(case).join("root.pem")).expect("root.pem");
    let root = pem::parse(&root).expect("PEM");
    let mut verifier = PathVerifier::new(Vec::new());
    verifier
        .add_root(&root[0].data)
        .expect("the test root is an anchor");

    let mut server = Server::with_case(CipherSuite::Aes128GcmSha256, case);
    server.hello();
    server.flight();
    server.data(b"ok");
    let mut pipe = Pipe::with_input(server.input.clone());
    let mut connection = connect(&mut pipe, &verifier).expect("the right host completes");
    let mut buf = [0u8; 16];
    let n = connection.read_data(&mut buf).expect("data");
    assert_eq!(&buf[..n], b"ok");
    connection.into_inner();

    let host = "other.test";
    let mut server = Server::for_host(CipherSuite::Aes128GcmSha256, case, host);
    server.hello();
    server.flight();
    let (result, written) = outcome_to(host, &server.input, &verifier);
    let rejection = CertificateRejection::NameMismatch {
        host: host.to_string(),
    };
    let alert = rejection.alert();
    assert_eq!(alert, AlertDescription::BAD_CERTIFICATE);
    assert_eq!(result, Err(Error::Certificate(rejection)));
    assert_eq!(last_alert(&server, &written), Some((2, alert.0)));
}

/// A ServerHello of `legacy_version` 0x0301 with `supported_versions` 0x0304 completes: the
/// legacy version is ignored once `supported_versions` is present (RFC 8446 section 4.2.1).
#[test]
fn legacy_version_ignored() {
    let mut server = Server::new(CipherSuite::Aes128GcmSha256);
    let mut sh = server.server_hello(Group::X25519);
    sh.legacy_version = 0x0301;
    server.send_hello(&sh);
    server.flight();
    let (result, _) = outcome(&server.input, &Double::accepting("ok-ecdsa-p256"));
    assert_eq!(result, Ok(()));
}

/// EncryptedExtensions with `supported_groups` x448 is read and ignored (RFC 8446 section
/// 4.2.7), and so are an empty `server_name` and ALPN `http/1.1`.
#[test]
fn server_supported_groups_ignored() {
    let mut server = Server::new(CipherSuite::Aes128GcmSha256);
    server.hello();
    let ee = server.encrypted_extensions(&[
        (10, v16(&[0, 0x1e])),
        (0, vec![]),
        (16, v16(&v8(b"http/1.1"))),
    ]);
    server.push(&ee);
    let certificate = server.certificate();
    server.push(&certificate);
    let cv = server.certificate_verify();
    server.push(&cv);
    server.push_finished();
    server.flush();
    let (result, _) = outcome(&server.input, &Double::accepting("ok-ecdsa-p256"));
    assert_eq!(result, Ok(()));
}

/// Acceptance 6.2: a flight with a CertificateRequest (empty context, `signature_algorithms`,
/// an unknown extension 0x1234): the client writes a Certificate with an empty list and its
/// Finished, no CertificateVerify, and the handshake completes.
#[test]
fn certificate_request_answered_empty() {
    let mut server = Server::new(CipherSuite::Aes128GcmSha256);
    server.hello();
    let ee = server.encrypted_extensions(&[]);
    server.push(&ee);
    let mut body = v8(&[]);
    body.extend_from_slice(&block(&[(13, v16(&[4, 3])), (0x1234, vec![1, 2, 3])]));
    server.push(&msg(13, &body));
    let certificate = server.certificate();
    server.push(&certificate);
    let cv = server.certificate_verify();
    server.push(&cv);
    server.push_finished();
    server.flush();
    let (result, written) = outcome(&server.input, &Double::accepting("ok-ecdsa-p256"));
    assert_eq!(result, Ok(()));
    let client = server.client_records(&written);
    let flight = client
        .iter()
        .find(|w| w.protected)
        .expect("the client's flight");
    let mut expected = msg(11, &[0, 0, 0, 0]);
    expected.extend_from_slice(&server.expected_client_finished(true));
    assert_eq!(
        flight.content, expected,
        "an empty Certificate, then Finished"
    );
    assert!(
        !contains_message(&flight.content, 15),
        "no CertificateVerify"
    );
}

/// Acceptance 6.4: the verifier receives the entries in the order sent, the leaf first, with
/// the host.
#[test]
fn leaf_is_first_entry() {
    let mut server = Server::new(CipherSuite::Aes128GcmSha256);
    server.hello();
    let ee = server.encrypted_extensions(&[]);
    server.push(&ee);
    let sent_chain = chain("ok-ecdsa-p256");
    assert!(sent_chain.len() >= 2, "a leaf and its issuer");
    let mut entries = sent_chain.clone();
    entries.push(b"a third entry, not DER".to_vec());
    server.push(&certificate_message(&entries, &[]));
    let cv = server.certificate_verify();
    server.push(&cv);
    server.push_finished();
    server.flush();
    let verifier = Double::accepting("ok-ecdsa-p256");
    let (result, _) = outcome(&server.input, &verifier);
    assert_eq!(result, Ok(()));
    assert_eq!(verifier.seen.borrow().as_slice(), &[entries]);
}

/// Acceptance 6.7, second half: `close` writes one `close_notify` under the application key
/// and shuts the write side; so does the drop of a connection after its handshake.
#[test]
fn close_sends_close_notify() {
    let server = Server::handshake(CipherSuite::Aes256GcmSha384);
    let mut pipe = Pipe::with_input(server.input.clone());
    let verifier = Double::accepting("ok-ecdsa-p256");
    let mut connection = connect(&mut pipe, &verifier).expect("handshake");
    connection.close().expect("close");
    connection.close().expect("a second close writes nothing");
    assert_eq!(
        connection.write_data(b"late"),
        Err(Error::Io(std::io::ErrorKind::BrokenPipe)),
        "the write side is shut"
    );
    drop(connection);
    let client = server.client_records(&pipe.output);
    let last = client.last().expect("records");
    assert_eq!((last.content_type, last.protected), (21, true));
    assert_eq!(last.content, vec![1, 0], "warning close_notify");
    assert_eq!(
        client.iter().filter(|w| w.content_type == 21).count(),
        1,
        "one"
    );

    let mut pipe = Pipe::with_input(server.input.clone());
    drop(connect(&mut pipe, &verifier).expect("handshake"));
    let client = server.client_records(&pipe.output);
    let last = client.last().expect("records");
    assert_eq!(
        (last.content_type, last.protected, last.content.clone()),
        (21, true, vec![1, 0])
    );
}

/// The record limit of an AES-GCM key: a write that would leave no record of the key for a
/// KeyUpdate sends one first, under the old key, and writes on under the next; reached through
/// the sequence setter, not by writing millions of records.
#[test]
fn key_update_before_record_limit() {
    let server = Server::handshake(CipherSuite::Aes128GcmSha256);
    let mut pipe = Pipe::with_input(server.input.clone());
    let verifier = Double::accepting("ok-ecdsa-p256");
    let mut connection = connect(&mut pipe, &verifier).expect("handshake");
    let handshake_bytes = connection.get_ref().output.len();
    connection.set_write_sequence(surf_tls::record::AES_GCM_RECORD_LIMIT - 2);
    connection
        .write_data(b"a")
        .expect("one record left after it");
    connection.write_data(b"b").expect("a key update first");
    connection.into_inner();
    let app = server.application.as_ref().expect("app");
    let tail = &pipe.output[handshake_bytes..];
    let raw = records_raw(tail);
    assert_eq!(raw.len(), 3, "a, the KeyUpdate, b");
    let limit = surf_tls::record::AES_GCM_RECORD_LIMIT;
    let suite = CipherSuite::Aes128GcmSha256;
    let a = open(suite, &app.client, limit - 2, &raw[0]).expect("a under the old key");
    assert_eq!(a, b"a\x17");
    let update = open(suite, &app.client, limit - 1, &raw[1]).expect("the last record of the key");
    assert_eq!(
        update,
        vec![24, 0, 0, 1, 0, 22],
        "KeyUpdate, update_not_requested"
    );
    let next = key_schedule::next_traffic_secret(&app.client).expect("next");
    let b = open(suite, &next, 0, &raw[2]).expect("b under the next key");
    assert_eq!(b, b"b\x17");
}

// ---------------------------------------------------------------------------------------------
// RFC 8448 through GivenClientHello.

struct TraceStep {
    heading: String,
    blocks: Vec<(String, Vec<u8>)>,
}

impl TraceStep {
    fn block(&self, label: &str) -> &[u8] {
        self.blocks
            .iter()
            .find(|(l, _)| l == label)
            .map(|(_, b)| b.as_slice())
            .unwrap_or_else(|| panic!("{} has no {label}", self.heading))
    }
}

/// The steps of section `number` of `rfc8448.txt`, each with its labelled hex blocks.
fn trace_steps(number: u32) -> Vec<TraceStep> {
    let text = fs::read_to_string(crate_dir().join("tests/data/rfc8448/rfc8448.txt"))
        .expect("rfc8448.txt");
    let mut steps: Vec<TraceStep> = Vec::new();
    let mut section = 0;
    let mut open: Option<(String, Vec<u8>)> = None;
    for raw in text.lines() {
        let line = raw.trim_end_matches('\u{c}');
        if line.trim().is_empty() || line.starts_with("Thomson ") || line.starts_with("RFC 8448 ") {
            continue;
        }
        let hex: Option<Vec<u8>> = line
            .split_whitespace()
            .map(|t| {
                (t.len() == 2)
                    .then(|| u8::from_str_radix(t, 16).ok())
                    .flatten()
            })
            .collect();
        if open.is_some() && line.starts_with("         ") && hex.is_some() {
            if let (Some((_, bytes)), Some(more)) = (open.as_mut(), hex) {
                bytes.extend(more);
            }
            continue;
        }
        if let Some(done) = open.take()
            && section == number
            && let Some(step) = steps.last_mut()
        {
            step.blocks.push(done);
        }
        if line.chars().next().is_some_and(|c| c.is_ascii_digit()) {
            if let Some((n, _)) = line.split_once(".  ")
                && let Ok(n) = n.parse()
            {
                section = n;
            }
            continue;
        }
        if section != number {
            continue;
        }
        if line.starts_with("   {client}") || line.starts_with("   {server}") {
            steps.push(TraceStep {
                heading: line.trim().trim_end_matches(':').to_string(),
                blocks: Vec::new(),
            });
            continue;
        }
        if line.starts_with("      ")
            && !line.starts_with("       ")
            && let Some((label, value)) = line.trim().split_once(":  ")
            && let Some((name, _)) = label.split_once(" (")
        {
            let bytes = value
                .split_whitespace()
                .filter_map(|t| u8::from_str_radix(t, 16).ok().filter(|_| t.len() == 2))
                .collect();
            open = Some((name.to_string(), bytes));
        }
    }
    steps
}

/// The answer of the trace for its CertificateVerify: the signature the trace's server
/// constructed, over content of the right form (its RSA-1024 key is refused by the seam).
struct TraceSignature {
    signature: Vec<u8>,
    hash_len: usize,
    calls: RefCell<usize>,
}

impl crypto::SignatureVerifier for TraceSignature {
    fn verify(
        &self,
        scheme: Scheme,
        _public_key: &[u8],
        message: &[u8],
        signature: &[u8],
    ) -> Result<(), crypto::CryptoError> {
        *self.calls.borrow_mut() += 1;
        let mut prefix = vec![0x20; 64];
        prefix.extend_from_slice(b"TLS 1.3, server CertificateVerify\0");
        let form = message.starts_with(&prefix) && message.len() == prefix.len() + self.hash_len;
        if scheme == Scheme::RsaPssRsaeSha256 && form && signature == self.signature.as_slice() {
            Ok(())
        } else {
            Err(crypto::CryptoError::BadSignature)
        }
    }
}

/// Runs section `number` as the client: the given hellos, their private keys, the server's
/// records. The client's Finished record equals the trace's; returns the steps, the memory and
/// the connection's reads for the caller.
fn trace_client(number: u32) {
    let steps = trace_steps(number);
    let hellos: Vec<Vec<u8>> = steps
        .iter()
        .filter(|s| s.heading == "{client}  construct a ClientHello handshake message")
        .map(|s| s.blocks[0].1.clone())
        .collect();
    let private_keys: Vec<Vec<u8>> = steps
        .iter()
        .filter(|s| s.heading.starts_with("{client}  create an ephemeral"))
        .map(|s| s.block("private key").to_vec())
        .collect();
    let server_records: Vec<&TraceStep> = steps
        .iter()
        .filter(|s| s.heading.starts_with("{server}  send ") && s.heading.ends_with(" record"))
        .collect();
    let server_flight: Vec<u8> = server_records
        .iter()
        .flat_map(|s| s.block("complete record").to_vec())
        .collect();
    let client_records: Vec<&TraceStep> = steps
        .iter()
        .filter(|s| s.heading.starts_with("{client}  send ") && s.heading.ends_with(" record"))
        .collect();
    let cv = steps
        .iter()
        .find(|s| s.heading == "{server}  construct a CertificateVerify handshake message")
        .expect("a CertificateVerify")
        .blocks[0]
        .1
        .clone();
    let signature = TraceSignature {
        signature: cv[8..].to_vec(),
        hash_len: 32,
        calls: RefCell::new(0),
    };
    let leaf = Double::answering(Ok(LeafKey {
        kind: crypto::PublicKeyKind::Rsa { bits: 1024 },
        bytes: b"the trace's RSA-1024 key, checked by its answer".to_vec(),
    }));
    let given = GivenClientHello {
        hellos: hellos.clone(),
        private_keys,
        server_flight,
        host: "server",
        verifier: &leaf,
        signatures: &signature,
        now: now(),
    };
    let mut memory = Memory::default();
    let mut connection = given
        .run(&mut memory)
        .expect("the trace's handshake completes");
    assert_eq!(
        *signature.calls.borrow(),
        1,
        "the CertificateVerify checked by its answer"
    );

    // What the client wrote: its hellos, a change_cipher_spec in section 7, its Finished.
    let written = records_raw(connection.get_ref().written());
    let expected_hellos = if number == 5 { 2 } else { 1 };
    for (i, hello) in hellos.iter().enumerate() {
        assert_eq!(&written[i][5..], hello.as_slice(), "hello {i} as given");
    }
    let mut at = expected_hellos;
    if number == 7 {
        assert_eq!(
            written[at],
            vec![20, 3, 3, 0, 1, 1],
            "the client's change_cipher_spec"
        );
        at += 1;
    }
    let finished = client_records
        .iter()
        .filter(|s| s.heading == "{client}  send handshake record")
        .nth(expected_hellos)
        .expect("the client's Finished record");
    assert_eq!(
        to_hex(&written[at]),
        to_hex(finished.block("complete record")),
        "section {number}: the client Finished equals the trace"
    );
    assert_eq!(
        written.len(),
        at + 1,
        "nothing else before the server's data"
    );

    // Section 3: records after Finished under the application keys.
    if number == 3 {
        let mut buf = [0u8; 64];
        let n = connection.read_data(&mut buf).expect("the server's data");
        let server_data = server_records
            .iter()
            .find(|s| s.heading == "{server}  send application_data record")
            .expect("data");
        assert_eq!(
            &buf[..n],
            server_data.block("payload"),
            "the ticket dropped, the data read"
        );
        let client_data = client_records
            .iter()
            .find(|s| s.heading == "{client}  send application_data record")
            .expect("data");
        connection
            .write_data(client_data.block("payload"))
            .expect("write");
        connection.close().expect("close");
        assert_eq!(
            connection.read_data(&mut buf),
            Ok(0),
            "the server's close_notify"
        );
        let written = records_raw(connection.get_ref().written());
        assert_eq!(
            to_hex(&written[at + 1]),
            to_hex(client_data.block("complete record")),
            "the client's data record"
        );
        let alert = client_records
            .iter()
            .find(|s| s.heading == "{client}  send alert record")
            .expect("alert");
        assert_eq!(
            to_hex(&written[at + 2]),
            to_hex(alert.block("complete record")),
            "the client's close_notify record"
        );
    }
}

/// Acceptance 2, RFC 8448 section 3 (the simple 1-RTT handshake): with its ClientHello given,
/// the client Finished, the application data and the close_notify equal the trace.
#[test]
fn rfc8448_section_3() {
    trace_client(3);
}

/// Acceptance 2, section 5 (HelloRetryRequest): both given hellos, the x25519 and the P-256
/// key; the client Finished equals the trace.
#[test]
fn rfc8448_section_5() {
    trace_client(5);
}

/// Acceptance 2, section 7 (compatibility mode): the server's `change_cipher_spec` dropped, the
/// client's written before its Finished, and the Finished equals the trace.
#[test]
fn rfc8448_section_7() {
    trace_client(7);
}

// ---------------------------------------------------------------------------------------------
// The two tables.

/// Checks a table: four columns on every line, and every entry of the last one a test of this
/// crate (`tests/tls/<file>.rs::<function>` with `#[test]` above it), `INDEX:<case>` of
/// `crates/tls-test-certs/certs/INDEX`, `wycheproof:<file>` of `tests/data/wycheproof/`, or
/// `not reachable: <why>`. The number of rows, or one problem per line.
pub fn check_table(
    text: &str,
    read: &dyn Fn(&str) -> Option<String>,
    index: &str,
    wycheproof: &[String],
) -> Result<usize, Vec<String>> {
    let mut problems = Vec::new();
    let mut rows = 0;
    let mut sources: HashMap<String, Option<String>> = HashMap::new();
    let cases: Vec<&str> = index
        .lines()
        .filter_map(|l| l.split_whitespace().next())
        .collect();
    for (i, line) in text.lines().enumerate() {
        let n = i + 1;
        if line.trim().is_empty() {
            continue;
        }
        rows += 1;
        let columns: Vec<&str> = line.split(" | ").collect();
        if columns.len() != 4 {
            problems.push(format!("line {n}: {} columns, not 4", columns.len()));
            continue;
        }
        for entry in columns[3].split(", ").map(str::trim) {
            if entry.contains('*') {
                problems.push(format!("line {n}: a wildcard: {entry}"));
            } else if let Some(why) = entry.strip_prefix("not reachable: ") {
                if why.trim().is_empty() {
                    problems.push(format!("line {n}: not reachable without a reason"));
                }
            } else if let Some(case) = entry.strip_prefix("INDEX:") {
                if !cases.contains(&case) {
                    problems.push(format!("line {n}: INDEX has no case {case}"));
                }
            } else if let Some(file) = entry.strip_prefix("wycheproof:") {
                if !wycheproof.iter().any(|f| f == file) {
                    problems.push(format!("line {n}: no wycheproof file {file}"));
                }
            } else if let Some((path, function)) = entry.split_once("::") {
                let well_formed = path.starts_with("tests/tls/")
                    && path.ends_with(".rs")
                    && !path.contains("..")
                    && !function.is_empty()
                    && function
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '_');
                if !well_formed {
                    problems.push(format!("line {n}: not a test path: {entry}"));
                    continue;
                }
                let source = sources
                    .entry(path.to_string())
                    .or_insert_with(|| read(path));
                match source {
                    None => problems.push(format!("line {n}: no file {path}")),
                    Some(source) if !has_test(source, function) => {
                        problems.push(format!("line {n}: {path} has no #[test] fn {function}("))
                    }
                    Some(_) => {}
                }
            } else {
                problems.push(format!("line {n}: not a test path: {entry}"));
            }
        }
    }
    if problems.is_empty() {
        Ok(rows)
    } else {
        Err(problems)
    }
}

/// Whether `source` holds `fn <function>(` with `#[test]` among the attribute and comment lines
/// right above it.
fn has_test(source: &str, function: &str) -> bool {
    let needle = format!("fn {function}(");
    let lines: Vec<&str> = source.lines().collect();
    for (i, line) in lines.iter().enumerate() {
        let line = line.trim_start();
        if !(line.starts_with(&needle) || line.starts_with(&format!("pub {needle}"))) {
            continue;
        }
        let mut above = i;
        while above > 0 {
            above -= 1;
            let attribute = lines[above].trim();
            if attribute == "#[test]" {
                return true;
            }
            if !(attribute.starts_with("#[") || attribute.starts_with("///")) {
                break;
            }
        }
    }
    false
}

fn read_crate_file(path: &str) -> Option<String> {
    fs::read_to_string(crate_dir().join(path)).ok()
}

fn index_text() -> String {
    fs::read_to_string(certs_dir().join("INDEX")).expect("INDEX")
}

fn wycheproof_files() -> Vec<String> {
    fs::read_dir(crate_dir().join("tests/data/wycheproof"))
        .expect("wycheproof")
        .map(|e| e.expect("entry").file_name().to_string_lossy().into_owned())
        .collect()
}

fn table(name: &str) -> String {
    fs::read_to_string(crate_dir().join("tests/tls").join(name)).expect("the table")
}

/// The line (from 1) of the first line of `text` holding `needle`.
fn line_of(text: &str, needle: &str) -> usize {
    text.lines()
        .position(|l| l.contains(needle))
        .expect("the row")
        + 1
}

/// The checks of one table, and the three copies that must fail naming their line: one
/// function renamed, one `#[test]` removed, one case renamed.
fn check_named_table(name: &str, rows: usize, function: &str, file: &str, case: Option<&str>) {
    let text = table(name);
    let index = index_text();
    let wycheproof = wycheproof_files();
    assert_eq!(
        check_table(&text, &read_crate_file, &index, &wycheproof),
        Ok(rows)
    );

    let entry = format!("{file}::{function}");
    let line = line_of(&text, &entry);
    let renamed = text.replacen(&entry, &format!("{entry}_renamed"), 1);
    assert_eq!(
        check_table(&renamed, &read_crate_file, &index, &wycheproof),
        Err(vec![format!(
            "line {line}: {file} has no #[test] fn {function}_renamed("
        )])
    );

    let without = |path: &str| {
        read_crate_file(path).map(|source| {
            if path == file {
                source.replacen(
                    &format!("#[test]\nfn {function}("),
                    &format!("fn {function}("),
                    1,
                )
            } else {
                source
            }
        })
    };
    assert_ne!(
        without(file),
        read_crate_file(file),
        "the #[test] is removed in the copy"
    );
    let problems = check_table(&text, &without, &index, &wycheproof).expect_err("fails");
    assert!(
        problems
            .iter()
            .all(|p| p.ends_with(&format!("{file} has no #[test] fn {function}("))),
        "{problems:?}"
    );
    assert!(problems.contains(&format!(
        "line {line}: {file} has no #[test] fn {function}("
    )));

    if let Some(case) = case {
        let line = line_of(&text, &format!("INDEX:{case}"));
        let changed = text.replacen(
            &format!("INDEX:{case}"),
            &format!("INDEX:{case}-renamed"),
            1,
        );
        assert_eq!(
            check_table(&changed, &read_crate_file, &index, &wycheproof),
            Err(vec![format!(
                "line {line}: INDEX has no case {case}-renamed"
            )])
        );
    }
    let wildcard = text.replacen(&entry, &format!("{file}::*"), 1);
    assert!(check_table(&wildcard, &read_crate_file, &index, &wycheproof).is_err());
    let three = text.replacen(" | ", " ", 1);
    assert!(check_table(&three, &read_crate_file, &index, &wycheproof).is_err());
}

/// Acceptance 7: `rfc8446-client-musts.txt` has the 76 rows of the table "RFC 8446 client
/// obligations", every entry a test, an INDEX case, a Wycheproof file or a reason.
#[test]
fn musts_rows_name_tests() {
    check_named_table(
        "rfc8446-client-musts.txt",
        76,
        "bad_finished",
        "tests/tls/handshake_hostile.rs",
        Some("md5"),
    );
}

/// Acceptance 8: `rfc8446-client-fields.txt` has the 49 rows of the table "Messages the client
/// receives"; its `tests/tls/record.rs::` entries are checked in the file of `story:tls-record`.
#[test]
fn fields_rows_name_tests() {
    check_named_table(
        "rfc8446-client-fields.txt",
        49,
        "finished_wrong_length",
        "tests/tls/handshake_hostile.rs",
        Some("leaf-v1"),
    );
    let text = table("rfc8446-client-fields.txt");
    assert!(text.contains("tests/tls/record.rs::alert_record_length"));
}
