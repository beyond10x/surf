//! RFC 8448, the example handshakes of TLS 1.3, replayed against the record layer, the key
//! schedule and the transcript of `surf-tls` (`story:tls-record`). The trace is the RFC's text,
//! unchanged, in `tests/data/rfc8448/`; this file reads its labelled hex blocks.
//!
//! Sections 3 (a simple 1-RTT handshake), 5 (HelloRetryRequest) and 7 (compatibility mode)
//! are replayed from either side: the side under test writes its records from the trace's
//! payloads and its own keys, and the bytes must equal the trace's; it reads the other side's
//! records and must hand up the trace's messages. Every secret is computed here from the
//! private keys and the messages of the trace, and compared with the trace's value.

#![forbid(unsafe_code)]

use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use surf_tls::crypto::{self, EphemeralKey, Group, HashAlgorithm, Random};
use surf_tls::key_schedule::{self, CipherSuite, Secret, TrafficSecrets};
use surf_tls::record::{AlertLevel, Message, RecordLayer};
use surf_tls::transcript::Transcript;

// ---------------------------------------------------------------------------------------------
// Helpers.

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn rfc8448_dir() -> PathBuf {
    crate_dir().join("tests/data/rfc8448")
}

fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Asserts that `actual` holds the bytes of `expected`, compared through the seam's
/// comparison in constant time; the failure names `what` and prints both, which are values
/// of the published trace.
fn same(what: &str, actual: &[u8], expected: &[u8]) {
    assert!(
        crypto::constant_time_eq(actual, expected),
        "{what}: computed {}, the trace has {}",
        to_hex(actual),
        to_hex(expected)
    );
}

/// An in-memory connection: the bytes the peer sent, read from the front, and the bytes
/// written.
#[derive(Default, Debug)]
pub struct Pipe {
    pub input: Vec<u8>,
    pub read: usize,
    pub output: Vec<u8>,
}

impl Pipe {
    pub fn with_input(input: Vec<u8>) -> Self {
        Pipe {
            input,
            read: 0,
            output: Vec::new(),
        }
    }

    pub fn unread(&self) -> usize {
        self.input.len() - self.read
    }
}

impl Read for Pipe {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = buf.len().min(self.input.len() - self.read);
        buf[..n].copy_from_slice(&self.input[self.read..self.read + n]);
        self.read += n;
        Ok(n)
    }
}

impl Write for Pipe {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.output.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

// ---------------------------------------------------------------------------------------------
// The trace: sections, steps and labelled hex blocks.

/// One labelled block, `label (N octets):  hex …`, with its bytes.
struct Block {
    label: String,
    bytes: Vec<u8>,
}

/// One step of a trace, `{client}  <action>:` or `{server}  <action>:`, with its blocks.
struct Step {
    line: usize,
    heading: String,
    blocks: Vec<Block>,
}

impl Step {
    fn block(&self, label: &str) -> Option<&[u8]> {
        self.blocks
            .iter()
            .find(|b| b.label == label)
            .map(|b| b.bytes.as_slice())
    }

    fn need(&self, label: &str) -> &[u8] {
        self.block(label)
            .unwrap_or_else(|| panic!("line {}: {} has no {label}", self.line, self.heading))
    }

    fn client(&self) -> bool {
        self.heading.starts_with("{client}")
    }
}

struct Section {
    number: u32,
    steps: Vec<Step>,
    /// Blocks without an octet count (`salt:  0 (all zero octets)`), not read.
    skipped: usize,
}

impl Section {
    /// The first step with `heading` that holds `label`.
    fn value(&self, heading: &str, label: &str) -> &[u8] {
        self.steps
            .iter()
            .filter(|s| s.heading == heading)
            .find_map(|s| s.block(label))
            .unwrap_or_else(|| panic!("section {}: no step {heading} with {label}", self.number))
    }

    fn blocks(&self) -> usize {
        self.steps.iter().map(|s| s.blocks.len()).sum()
    }
}

fn hex_token(token: &str) -> Option<u8> {
    if token.len() == 2 {
        u8::from_str_radix(token, 16).ok()
    } else {
        None
    }
}

fn hex_line(text: &str) -> Option<Vec<u8>> {
    let tokens: Vec<&str> = text.split_whitespace().collect();
    if tokens.is_empty() {
        return None;
    }
    tokens.into_iter().map(hex_token).collect()
}

/// A line of the page layout: blank, a form feed, a footer or a header.
fn page_line(line: &str) -> bool {
    line.trim().is_empty() || line.starts_with("Thomson ") || line.starts_with("RFC 8448 ")
}

struct Open {
    line: usize,
    label: String,
    count: usize,
    bytes: Vec<u8>,
}

fn close(open: &mut Option<Open>, sections: &mut [Section]) {
    if let Some(block) = open.take() {
        assert_eq!(
            block.bytes.len(),
            block.count,
            "line {}: {} holds {} bytes, its label says {}",
            block.line,
            block.label,
            block.bytes.len(),
            block.count
        );
        let step = sections
            .last_mut()
            .and_then(|s| s.steps.last_mut())
            .expect("a block inside a step");
        step.blocks.push(Block {
            label: block.label,
            bytes: block.bytes,
        });
    }
}

/// Every section of `text`, with its steps and their blocks. A block's byte count is checked
/// against the count its label states.
fn parse(text: &str) -> Vec<Section> {
    let mut sections: Vec<Section> = Vec::new();
    let mut open: Option<Open> = None;
    for (index, raw) in text.lines().enumerate() {
        let line_no = index + 1;
        let line = raw.trim_end_matches('\u{c}');
        if page_line(line) {
            continue;
        }
        if open.is_some()
            && line.starts_with("         ")
            && let Some(bytes) = hex_line(line)
        {
            if let Some(block) = open.as_mut() {
                block.bytes.extend(bytes);
            }
            continue;
        }
        close(&mut open, &mut sections);
        let first = line.chars().next().unwrap_or(' ');
        if first.is_ascii_digit() {
            if let Some((number, _)) = line.split_once(".  ")
                && let Ok(number) = number.parse()
            {
                sections.push(Section {
                    number,
                    steps: Vec::new(),
                    skipped: 0,
                });
            }
            continue;
        }
        let Some(section) = sections.last_mut() else {
            continue;
        };
        let trimmed = line.trim();
        if line.starts_with("   {client}") || line.starts_with("   {server}") {
            section.steps.push(Step {
                line: line_no,
                heading: trimmed.trim_end_matches(':').to_string(),
                blocks: Vec::new(),
            });
            continue;
        }
        let label_line = line.starts_with("      ") && !line.starts_with("       ");
        if !label_line || section.steps.is_empty() {
            continue;
        }
        let Some((label, value)) = trimmed.split_once(":  ") else {
            continue;
        };
        let Some((name, count)) = label.split_once(" (") else {
            section.skipped += 1;
            continue;
        };
        let Some(count) = count
            .strip_suffix(" octets)")
            .and_then(|c| c.parse::<usize>().ok())
        else {
            section.skipped += 1;
            continue;
        };
        let bytes = if value.trim() == "(empty)" {
            Vec::new()
        } else {
            hex_line(value).unwrap_or_else(|| panic!("line {line_no}: not hex: {value}"))
        };
        open = Some(Open {
            line: line_no,
            label: name.to_string(),
            count,
            bytes,
        });
    }
    close(&mut open, &mut sections);
    sections
}

fn trace() -> Vec<Section> {
    let text = fs::read_to_string(rfc8448_dir().join("rfc8448.txt")).expect("read rfc8448.txt");
    parse(&text)
}

fn section(number: u32) -> Section {
    trace()
        .into_iter()
        .find(|s| s.number == number)
        .unwrap_or_else(|| panic!("no section {number}"))
}

// ---------------------------------------------------------------------------------------------
// Each step's own inputs and outputs.

/// The label of a heading's quoted `"tls13 <label>"`.
fn quoted_label(heading: &str) -> Option<&str> {
    let start = heading.find("\"tls13 ")? + "\"tls13 ".len();
    let rest = &heading[start..];
    Some(&rest[..rest.find('"')?])
}

fn expand(prk: &[u8], label: &str, context: &[u8], len: usize) -> Vec<u8> {
    let mut out = vec![0u8; len];
    key_schedule::hkdf_expand_label(
        HashAlgorithm::Sha256,
        prk,
        label.as_bytes(),
        context,
        &mut out,
    )
    .expect("expand");
    out
}

fn label_info(len: usize, label: &str, context: &[u8]) -> Vec<u8> {
    key_schedule::hkdf_label(len as u16, label.as_bytes(), context).expect("label")
}

fn group_of(heading: &str) -> Group {
    if heading.contains("x25519") {
        Group::X25519
    } else if heading.contains("P-256") {
        Group::Secp256r1
    } else {
        panic!("no group in {heading}")
    }
}

/// Checks every step whose inputs and outputs the trace prints in the step itself: the
/// extractions, the Derive-Secret and HKDF-Expand-Label steps with their `info`, the finished
/// keys, the traffic keys and IVs, and the public key of each ephemeral key pair made from its
/// private key. The number of blocks compared.
fn check_steps(section: &Section) -> usize {
    let mut compared = 0;
    for step in &section.steps {
        let h = step.heading.as_str();
        let at = |what: &str| format!("section {} line {}: {what}", section.number, step.line);
        if h.contains("extract secret") && step.block("IKM").is_some() {
            let salt = step.block("salt").unwrap_or(&[]);
            let secret = key_schedule::hkdf_extract(HashAlgorithm::Sha256, salt, step.need("IKM"));
            same(&at("secret"), secret.as_ref(), step.need("secret"));
            compared += 1;
        } else if (h.contains("derive secret") || h.contains("generate resumption secret"))
            && step.block("info").is_some()
        {
            let label = quoted_label(h).expect("a label");
            let hash = step.need("hash");
            let expanded = step.need("expanded");
            same(
                &at("info"),
                &label_info(expanded.len(), label, hash),
                step.need("info"),
            );
            let out = expand(step.need("PRK"), label, hash, expanded.len());
            same(&at("expanded"), &out, expanded);
            compared += 2;
        } else if h.contains("calculate finished") && step.block("info").is_some() {
            let expanded = step.need("expanded");
            same(
                &at("info"),
                &label_info(32, "finished", &[]),
                step.need("info"),
            );
            let out = expand(step.need("PRK"), "finished", &[], 32);
            same(&at("finished key"), &out, expanded);
            compared += 2;
        } else if h.contains("traffic keys") && step.block("key info").is_some() {
            let prk = step.need("PRK");
            let key = step.need("key expanded");
            let iv = step.need("iv expanded");
            same(
                &at("key info"),
                &label_info(key.len(), "key", &[]),
                step.need("key info"),
            );
            same(
                &at("iv info"),
                &label_info(iv.len(), "iv", &[]),
                step.need("iv info"),
            );
            same(&at("key"), &expand(prk, "key", &[], key.len()), key);
            same(&at("iv"), &expand(prk, "iv", &[], iv.len()), iv);
            compared += 4;
        } else if h.contains("create an ephemeral") {
            let private = step.need("private key");
            let random = Random::fixed_slice(private);
            let key = EphemeralKey::generate(group_of(h), &random).expect("a key");
            same(&at("public key"), key.public_key(), step.need("public key"));
            compared += 1;
        }
    }
    compared
}

// ---------------------------------------------------------------------------------------------
// The replay.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Side {
    Client,
    Server,
}

/// The random of a HelloRetryRequest (RFC 8446 section 4.1.3), SHA-256 of
/// "HelloRetryRequest".
fn is_hello_retry_request(server_hello: &[u8]) -> bool {
    let special = crypto::hash(HashAlgorithm::Sha256, b"HelloRetryRequest");
    server_hello.get(6..38) == Some(special.as_ref())
}

/// The suite a ServerHello selects: after the version, the random and the session id echo.
fn selected_suite(server_hello: &[u8]) -> CipherSuite {
    let id_len = usize::from(server_hello[38]);
    let at = 39 + id_len;
    let code = u16::from_be_bytes([server_hello[at], server_hello[at + 1]]);
    CipherSuite::from_code(code).expect("a TLS 1.3 suite")
}

/// The handshake messages of a payload, each with its header.
fn messages(payload: &[u8]) -> Vec<&[u8]> {
    let mut out = Vec::new();
    let mut rest = payload;
    while rest.len() >= 4 {
        let len = usize::from(rest[1]) << 16 | usize::from(rest[2]) << 8 | usize::from(rest[3]);
        out.push(&rest[..4 + len]);
        rest = &rest[4 + len..];
    }
    assert!(rest.is_empty(), "a payload of whole messages");
    out
}

struct KeyPair {
    group: Group,
    private: Vec<u8>,
    public: Vec<u8>,
}

struct Replay<'a> {
    section: &'a Section,
    side: Side,
    layer: RecordLayer<Pipe>,
    transcript: Transcript,
    suite: Option<CipherSuite>,
    client_key: Option<KeyPair>,
    server_key: Option<KeyPair>,
    master: Option<Secret>,
    handshake: Option<TrafficSecrets>,
    application: Option<TrafficSecrets>,
    /// Records written or read and compared with the trace.
    records: usize,
    /// `change_cipher_spec` records written or handed up.
    change_cipher_specs: usize,
    /// Secrets, keys, IVs, hashes and `verify_data` compared with the trace.
    secrets: usize,
    /// Records whose `legacy_record_version` differed from the trace: the trace's first
    /// ClientHello carries 0x0301, surf writes 0x0303 on every record.
    version_differs: usize,
    /// The trace's "construct a(n) X handshake message" blocks of each sender, client first,
    /// in the trace's order, with their lines; each message written or handed up is compared
    /// with the next one of its sender.
    constructed: [Vec<(usize, &'a [u8])>; 2],
    /// Messages compared with their "construct" block.
    messages_compared: usize,
}

/// The index of `side` in `Replay::constructed`.
fn side_index(side: Side) -> usize {
    match side {
        Side::Client => 0,
        Side::Server => 1,
    }
}

/// The "construct a(n) X handshake message" blocks of `section`, of each sender in order.
fn constructed(section: &Section) -> [Vec<(usize, &[u8])>; 2] {
    let mut out = [Vec::new(), Vec::new()];
    for step in &section.steps {
        let h = step.heading.as_str();
        if h.contains("  construct a") && h.ends_with(" handshake message") {
            assert_eq!(step.blocks.len(), 1, "line {}: {h}", step.line);
            let sender = if step.client() {
                Side::Client
            } else {
                Side::Server
            };
            out[side_index(sender)].push((step.line, step.blocks[0].bytes.as_slice()));
        }
    }
    out
}

impl<'a> Replay<'a> {
    fn new(section: &'a Section, side: Side) -> Self {
        Replay {
            section,
            side,
            layer: RecordLayer::new(Pipe::default()),
            transcript: Transcript::new(),
            suite: None,
            client_key: None,
            server_key: None,
            master: None,
            handshake: None,
            application: None,
            records: 0,
            change_cipher_specs: 0,
            secrets: 0,
            version_differs: 0,
            constructed: constructed(section),
            messages_compared: 0,
        }
    }

    fn expect(&mut self, what: &str, actual: &[u8], heading: &str, label: &str) {
        let expected = self.section.value(heading, label);
        same(
            &format!("section {} {:?}: {what}", self.section.number, self.side),
            actual,
            expected,
        );
        self.secrets += 1;
    }

    fn expect_traffic_key(&mut self, secret: &Secret, heading: &str) {
        let suite = self.suite.expect("a suite");
        let keys = key_schedule::traffic_key(suite, secret).expect("traffic key");
        self.expect("key", keys.key(), heading, "key expanded");
        self.expect("iv", keys.iv(), heading, "iv expanded");
    }

    fn run(mut self) -> Self {
        let section = self.section;
        for step in &section.steps {
            let h = step.heading.as_str();
            if h.contains("create an ephemeral") {
                let pair = KeyPair {
                    group: group_of(h),
                    private: step.need("private key").to_vec(),
                    public: step.need("public key").to_vec(),
                };
                if step.client() {
                    self.client_key = Some(pair);
                } else {
                    self.server_key = Some(pair);
                }
            } else if h.ends_with(" record") && h.contains("send ") {
                let sender = if step.client() {
                    Side::Client
                } else {
                    Side::Server
                };
                if sender == self.side {
                    self.write(step, sender);
                } else {
                    self.read(step, sender);
                }
            }
        }
        assert_eq!(self.layer.get_ref().unread(), 0, "every record read");
        for (side, left) in [Side::Client, Side::Server]
            .into_iter()
            .zip(&self.constructed)
        {
            assert!(
                left.is_empty(),
                "section {}: {} constructed messages of the {side:?} never sent, the first at \
                 line {}",
                self.section.number,
                left.len(),
                left.first().map_or(0, |c| c.0)
            );
        }
        self
    }

    fn write(&mut self, step: &Step, sender: Side) {
        let payload = step.need("payload");
        let h = step.heading.as_str();
        let result = if h.contains("send handshake record") {
            self.layer.write_handshake(payload)
        } else if h.contains("send application_data record") {
            self.layer.write_application_data(payload)
        } else if h.contains("send alert record") {
            let level = match payload[0] {
                1 => AlertLevel::Warning,
                _ => AlertLevel::Fatal,
            };
            self.layer
                .write_alert(level, surf_tls::error::AlertDescription(payload[1]))
        } else if h.contains("send change_cipher_spec record") {
            self.change_cipher_specs += 1;
            self.layer.write_change_cipher_spec()
        } else {
            panic!("line {}: no such record: {h}", step.line)
        };
        result.unwrap_or_else(|e| panic!("line {}: {h}: {e:?}", step.line));
        let written = std::mem::take(&mut self.layer.get_mut().output);
        let mut expected = step.need("complete record").to_vec();
        assert_eq!(
            &written[1..3],
            &[3, 3],
            "line {}: every record 0x0303",
            step.line
        );
        if expected[1..3] == [3, 1] && written[1..3] == [3, 3] {
            expected[1..3].copy_from_slice(&[3, 3]);
            self.version_differs += 1;
        }
        same(&format!("line {}: {h}", step.line), &written, &expected);
        self.records += 1;
        if h.contains("send handshake record") {
            for message in messages(payload) {
                self.message(sender, message);
            }
        }
    }

    fn read(&mut self, step: &Step, sender: Side) {
        let payload = step.need("payload");
        let record = step.need("complete record");
        let h = step.heading.as_str();
        let at = format!("section {} line {}: {h}", self.section.number, step.line);
        self.layer.get_mut().input.extend_from_slice(record);
        if h.contains("send handshake record") {
            let mut handed = Vec::new();
            while handed.len() < payload.len() {
                match self.layer.read_message() {
                    Ok(Message::Handshake(message)) => {
                        handed.extend_from_slice(&message);
                        self.message(sender, &message);
                    }
                    other => panic!("{at}: {other:?}"),
                }
            }
            same(&at, &handed, payload);
        } else if h.contains("send application_data record") {
            match self.layer.read_message() {
                Ok(Message::ApplicationData(data)) => same(&at, &data, payload),
                other => panic!("{at}: {other:?}"),
            }
        } else if h.contains("send alert record") {
            match self.layer.read_message() {
                Ok(Message::Alert { level, description }) => {
                    same(&at, &[level, description.0], payload)
                }
                other => panic!("{at}: {other:?}"),
            }
        } else if h.contains("send change_cipher_spec record") {
            match self.layer.read_message() {
                Ok(Message::ChangeCipherSpec(content)) => same(&at, &content, payload),
                other => panic!("{at}: {other:?}"),
            }
            self.change_cipher_specs += 1;
        } else {
            panic!("{at}: no such record");
        }
        assert_eq!(
            self.layer.get_ref().unread(),
            0,
            "{at}: the record read whole"
        );
        self.records += 1;
    }

    /// One handshake message, written or read: the transcript, and the keys it brings.
    fn message(&mut self, sender: Side, message: &[u8]) {
        let queue = &mut self.constructed[side_index(sender)];
        assert!(
            !queue.is_empty(),
            "section {}: a message of the {sender:?} the trace does not construct",
            self.section.number
        );
        let (line, expected) = queue.remove(0);
        same(
            &format!(
                "section {} {:?}: the message constructed at line {line}",
                self.section.number, self.side
            ),
            message,
            expected,
        );
        self.messages_compared += 1;
        match message[0] {
            2 if is_hello_retry_request(message) => {
                let suite = selected_suite(message);
                let retried = self.transcript.hello_retry_request(suite.hash(), message);
                assert_eq!(retried, Ok(()), "the HelloRetryRequest of the trace");
            }
            2 => self.server_hello(message),
            20 => self.finished(sender, message),
            _ => self.transcript.add(message),
        }
    }

    fn server_hello(&mut self, message: &[u8]) {
        let suite = selected_suite(message);
        self.suite = Some(suite);
        assert!(self.transcript.select(suite.hash()));
        self.transcript.add(message);
        let shared = {
            let (own, peer) = match self.side {
                Side::Client => (&self.client_key, &self.server_key),
                Side::Server => (&self.server_key, &self.client_key),
            };
            let own = own.as_ref().expect("our key pair");
            let peer = peer.as_ref().expect("the peer's share");
            assert_eq!(own.group, peer.group);
            let random = Random::fixed_slice(&own.private);
            let key = EphemeralKey::generate(own.group, &random).expect("our key");
            key.agree(&peer.public).expect("the shared secret")
        };
        self.expect(
            "shared secret",
            shared.as_ref(),
            "{server}  extract secret \"handshake\"",
            "IKM",
        );
        let early = key_schedule::early_secret(suite.hash(), None);
        self.expect(
            "early secret",
            early.as_ref(),
            "{server}  extract secret \"early\"",
            "secret",
        );
        let handshake = key_schedule::handshake_secret(&early, shared.as_ref()).expect("hs");
        self.expect(
            "handshake secret",
            handshake.as_ref(),
            "{server}  extract secret \"handshake\"",
            "secret",
        );
        let hash = self.transcript.current().expect("a transcript hash");
        self.expect(
            "transcript ClientHello..ServerHello",
            hash.as_ref(),
            "{server}  derive secret \"tls13 c hs traffic\"",
            "hash",
        );
        let secrets =
            key_schedule::handshake_traffic_secrets(&handshake, hash.as_ref()).expect("traffic");
        self.expect(
            "c hs traffic",
            secrets.client.as_ref(),
            "{server}  derive secret \"tls13 c hs traffic\"",
            "expanded",
        );
        self.expect(
            "s hs traffic",
            secrets.server.as_ref(),
            "{server}  derive secret \"tls13 s hs traffic\"",
            "expanded",
        );
        self.expect_traffic_key(
            &secrets.server,
            "{server}  derive write traffic keys for handshake data",
        );
        self.expect_traffic_key(
            &secrets.client,
            "{server}  derive read traffic keys for handshake data",
        );
        let master = key_schedule::master_secret(&handshake).expect("master");
        self.expect(
            "master secret",
            master.as_ref(),
            "{server}  extract secret \"master\"",
            "secret",
        );
        let (write, read) = match self.side {
            Side::Client => (&secrets.client, &secrets.server),
            Side::Server => (&secrets.server, &secrets.client),
        };
        self.layer.set_write_keys(suite, write).expect("write keys");
        self.layer.set_read_keys(suite, read).expect("read keys");
        assert_eq!(self.layer.read_sequence(), Some(0));
        assert_eq!(self.layer.write_sequence(), Some(0));
        self.master = Some(master);
        self.handshake = Some(secrets);
    }

    fn finished(&mut self, sender: Side, message: &[u8]) {
        let secrets = self.handshake.clone().expect("handshake secrets");
        let master = self.master.clone().expect("the master secret");
        let suite = self.suite.expect("a suite");
        let base = match sender {
            Side::Client => &secrets.client,
            Side::Server => &secrets.server,
        };
        let hash = self.transcript.current().expect("a transcript hash");
        let verify = key_schedule::verify_data(base, hash.as_ref()).expect("verify_data");
        let heading = match sender {
            Side::Client => "{client}  calculate finished \"tls13 finished\"",
            Side::Server => "{server}  calculate finished \"tls13 finished\"",
        };
        self.expect("verify_data", verify.as_ref(), heading, "finished");
        let key = key_schedule::finished_key(base).expect("finished key");
        self.expect("finished key", key.as_ref(), heading, "expanded");
        same("the Finished message", &message[4..], verify.as_ref());
        assert!(key_schedule::check_verify_data(
            base,
            hash.as_ref(),
            &message[4..]
        ));
        let mut flipped = message[4..].to_vec();
        flipped[0] ^= 1;
        assert!(!key_schedule::check_verify_data(
            base,
            hash.as_ref(),
            &flipped
        ));
        self.transcript.add(message);
        let hash = self.transcript.current().expect("a transcript hash");
        match sender {
            Side::Server => {
                self.expect(
                    "transcript ClientHello..server Finished",
                    hash.as_ref(),
                    "{server}  derive secret \"tls13 c ap traffic\"",
                    "hash",
                );
                let app = key_schedule::application_traffic_secrets(&master, hash.as_ref())
                    .expect("application secrets");
                self.expect(
                    "c ap traffic",
                    app.client.as_ref(),
                    "{server}  derive secret \"tls13 c ap traffic\"",
                    "expanded",
                );
                self.expect(
                    "s ap traffic",
                    app.server.as_ref(),
                    "{server}  derive secret \"tls13 s ap traffic\"",
                    "expanded",
                );
                let exporter =
                    key_schedule::exporter_master_secret(&master, hash.as_ref()).expect("exp");
                self.expect(
                    "exp master",
                    exporter.as_ref(),
                    "{server}  derive secret \"tls13 exp master\"",
                    "expanded",
                );
                self.expect_traffic_key(
                    &app.server,
                    "{server}  derive write traffic keys for application data",
                );
                self.expect_traffic_key(
                    &app.client,
                    "{client}  derive write traffic keys for application data",
                );
                match self.side {
                    Side::Server => self.layer.set_write_keys(suite, &app.server),
                    Side::Client => self.layer.set_read_keys(suite, &app.server),
                }
                .expect("server application keys");
                self.application = Some(app);
            }
            Side::Client => {
                self.expect(
                    "transcript ClientHello..client Finished",
                    hash.as_ref(),
                    "{client}  derive secret \"tls13 res master\"",
                    "hash",
                );
                let resumption =
                    key_schedule::resumption_master_secret(&master, hash.as_ref()).expect("res");
                self.expect(
                    "res master",
                    resumption.as_ref(),
                    "{client}  derive secret \"tls13 res master\"",
                    "expanded",
                );
                let ticket = "{server}  generate resumption secret \"tls13 resumption\"";
                if self.section.steps.iter().any(|s| s.heading == ticket) {
                    let nonce = self.section.value(ticket, "hash").to_vec();
                    let psk = key_schedule::resumption_psk(&resumption, &nonce).expect("psk");
                    self.expect("resumption psk", psk.as_ref(), ticket, "expanded");
                }
                let app = self.application.clone().expect("application secrets");
                match self.side {
                    Side::Client => self.layer.set_write_keys(suite, &app.client),
                    Side::Server => self.layer.set_read_keys(suite, &app.client),
                }
                .expect("client application keys");
            }
        }
    }
}

fn replay(number: u32, side: Side) -> (Section, usize, usize, usize, usize, usize) {
    let section = section(number);
    let steps_checked = check_steps(&section);
    let done = Replay::new(&section, side).run();
    let expected_messages = match number {
        3 => 8,
        5 => 9,
        _ => 7,
    };
    assert_eq!(
        done.messages_compared, expected_messages,
        "section {number}: messages compared with their construct block"
    );
    let counts = (
        steps_checked,
        done.records,
        done.secrets,
        done.change_cipher_specs,
        done.version_differs,
    );
    println!(
        "rfc8448 section {number} as the {side:?}: {} steps, {} blocks read, {} without an octet \
         count skipped; {} blocks of steps checked, {} records, {} secrets, {} \
         change_cipher_spec, {} records of version 0x0301 in the trace, {} messages equal to \
         their construct block",
        section.steps.len(),
        section.blocks(),
        section.skipped,
        counts.0,
        counts.1,
        counts.2,
        counts.3,
        counts.4,
        done.messages_compared
    );
    (section, counts.0, counts.1, counts.2, counts.3, counts.4)
}

// ---------------------------------------------------------------------------------------------
// The tests.

/// Section 3, the client: the X25519 key from the trace's private key, the shared secret,
/// every secret, traffic key and IV; the server's records open into its messages; our
/// ClientHello, Finished, application data and alert records are the trace's bytes (the
/// ClientHello's `legacy_record_version` aside: the trace writes 0x0301, surf 0x0303). Our
/// records carry no padding: each record's length is the trace's. In every replay each
/// handshake message written or handed up equals, byte for byte, the trace's "construct a(n) X
/// handshake message" block of its sender, in order, and every such block is sent.
#[test]
fn section_3() {
    let (_, steps, records, secrets, ccs, version) = replay(3, Side::Client);
    assert!(steps >= 40, "{steps} blocks of steps checked");
    assert_eq!(
        records, 9,
        "ClientHello, ServerHello, the server's flight, Finished, the ticket, two application data records, two alerts"
    );
    assert!(secrets >= 25, "{secrets} secrets compared");
    assert_eq!(ccs, 0);
    assert_eq!(version, 1, "the first ClientHello");
}

/// Section 5: the transcript after the HelloRetryRequest, with `message_hash`, and every
/// secret after it, over a P-256 share.
#[test]
fn section_5() {
    let (_, steps, records, secrets, ccs, version) = replay(5, Side::Client);
    assert!(steps >= 40, "{steps} blocks of steps checked");
    assert_eq!(
        records, 8,
        "two ClientHellos, the HelloRetryRequest, the ServerHello, the server's flight, Finished, two alerts"
    );
    assert!(secrets >= 20, "{secrets} secrets compared");
    assert_eq!(ccs, 0);
    assert_eq!(
        version, 1,
        "the first ClientHello; the second carries 0x0303"
    );
}

/// Section 7, compatibility mode: the two `change_cipher_spec` records are handed up and
/// written, and the rest equals the trace.
#[test]
fn section_7() {
    let (_, steps, records, secrets, ccs, version) = replay(7, Side::Client);
    assert!(steps >= 40, "{steps} blocks of steps checked");
    assert_eq!(records, 8, "{records} records");
    assert!(secrets >= 20, "{secrets} secrets compared");
    assert_eq!(ccs, 2, "the server's and the client's");
    assert_eq!(version, 1);
}

/// The server direction, which `story:tls-test-server` relies on: with the server's keys of
/// the trace's own schedule, our protection of each server-to-client record (the flight of
/// EncryptedExtensions, Certificate, CertificateVerify and Finished in one record, the
/// NewSessionTicket, the application data, the alert) from its plaintext and its sequence
/// number is the trace's bytes; and the client's records open. Sections 3, 5 and 7.
#[test]
fn server_direction() {
    let (_, _, records, secrets, _, version) = replay(3, Side::Server);
    assert_eq!(records, 9);
    assert!(secrets >= 25, "{secrets} secrets compared");
    assert_eq!(version, 0, "the server writes no record of 0x0301");
    let (_, _, records, _, _, _) = replay(5, Side::Server);
    assert_eq!(records, 8);
    let (_, _, records, _, ccs, _) = replay(7, Side::Server);
    assert_eq!(records, 8);
    assert_eq!(ccs, 2);
}

/// The parser reads the trace whole: the section headings, every step of sections 3, 5 and 7
/// with a block, and each block's byte count equal to the count its label states (checked as
/// it is read).
#[test]
fn trace_reads_whole() {
    let sections = trace();
    let numbers: Vec<u32> = sections.iter().map(|s| s.number).collect();
    assert_eq!(numbers, (1..=10).collect::<Vec<u32>>());
    for number in [3, 5, 7] {
        let s = sections
            .iter()
            .find(|s| s.number == number)
            .expect("section");
        assert!(
            s.steps.len() > 40,
            "section {number}: {} steps",
            s.steps.len()
        );
        assert!(s.blocks() > 80, "section {number}: {} blocks", s.blocks());
    }
}

/// Every file of `dir` but `SOURCE` against the sha256 its `SOURCE` lists, hashed through the
/// seam: the number of files compared, or one line per file that differs, is missing or is not
/// listed.
fn compare_digests(dir: &Path) -> Result<usize, Vec<String>> {
    let source = fs::read_to_string(dir.join("SOURCE")).expect("read SOURCE");
    let mut listed = Vec::new();
    for line in source.lines() {
        let Some((digest, name)) = line.split_once("  ") else {
            continue;
        };
        if digest.len() == 64 && digest.bytes().all(|b| b.is_ascii_hexdigit()) {
            listed.push((digest.to_string(), name.to_string()));
        }
    }
    let mut problems = Vec::new();
    for (digest, name) in &listed {
        match fs::read(dir.join(name)) {
            Ok(content) => {
                let actual = to_hex(crypto::hash(HashAlgorithm::Sha256, &content).as_ref());
                if actual != *digest {
                    problems.push(format!("{name}: sha256 {actual}, SOURCE lists {digest}"));
                }
            }
            Err(e) => problems.push(format!("{name}: {e}")),
        }
    }
    for entry in fs::read_dir(dir).expect("read the directory") {
        let name = entry
            .expect("entry")
            .file_name()
            .to_string_lossy()
            .into_owned();
        if name != "SOURCE" && !listed.iter().any(|(_, n)| *n == name) {
            problems.push(format!("{name}: not listed in SOURCE"));
        }
    }
    if problems.is_empty() {
        Ok(listed.len())
    } else {
        Err(problems)
    }
}

/// `rfc8448.txt` against the sha256 of its `SOURCE`; a copy with one changed byte fails
/// naming the file.
#[test]
fn sources_match_their_digests() {
    let dir = rfc8448_dir();
    assert_eq!(compare_digests(&dir), Ok(1), "rfc8448.txt");
    let copy = Path::new(env!("CARGO_TARGET_TMPDIR")).join("rfc8448-digests");
    let _ = fs::remove_dir_all(&copy);
    fs::create_dir_all(&copy).expect("create the copy");
    for entry in fs::read_dir(&dir).expect("read the directory") {
        let entry = entry.expect("entry");
        fs::copy(entry.path(), copy.join(entry.file_name())).expect("copy a file");
    }
    let changed = copy.join("rfc8448.txt");
    let mut content = fs::read(&changed).expect("read the copy");
    content[100] ^= 0x01;
    fs::write(&changed, content).expect("write the copy");
    let problems = compare_digests(&copy).expect_err("a changed byte fails");
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert!(
        problems[0].starts_with("rfc8448.txt: sha256 "),
        "{problems:?}"
    );
    let _ = fs::remove_dir_all(&copy);
}
