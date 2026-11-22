//! The adversary's cases against the record layer, the key schedule and the transcript of
//! `story:tls-record`: costs measured by the counting allocator, key updates and alerts checked
//! against records sealed here through the seam, the SHA-384 schedule built from the formulas
//! of RFC 8446 section 7.1, the transcript after a HelloRetryRequest, and seeded sequences of
//! records read through a stream that returns a few bytes at a time.

#![forbid(unsafe_code)]

use std::io::{self, ErrorKind, Read, Write};

use surf_tls::crypto::{self, AeadKey, HashAlgorithm};
use surf_tls::error::{AlertDescription, Error, Violation};
use surf_tls::key_schedule::{self, CipherSuite, Secret};
use surf_tls::record::{MAX_CIPHERTEXT, MAX_PLAINTEXT, Message, RecordLayer};
use surf_tls::transcript::Transcript;

use crate::alloc::{allocated_while, peak_while};
use crate::rfc8448::Pipe;

const SUITE: CipherSuite = CipherSuite::Aes128GcmSha256;
const KIB: usize = 1024;

fn secret(suite: CipherSuite, name: &str) -> Secret {
    key_schedule::hkdf_extract(suite.hash(), &[], name.as_bytes())
}

/// A protected record of the inner plaintext `inner`, sealed here through the seam with the key
/// and IV of `secret` at `sequence` (RFC 8446 section 5.3).
fn sealed(suite: CipherSuite, secret: &Secret, sequence: u64, inner: &[u8]) -> Vec<u8> {
    let keys = key_schedule::traffic_key(suite, secret).expect("keys");
    let key = AeadKey::new(suite.aead(), keys.key()).expect("key");
    let mut nonce = *keys.iv();
    for (i, b) in sequence.to_be_bytes().iter().enumerate() {
        nonce[4 + i] ^= b;
    }
    let len = (inner.len() + 16) as u16;
    let mut record = vec![23, 3, 3];
    record.extend_from_slice(&len.to_be_bytes());
    let mut body = inner.to_vec();
    key.seal(&nonce, &record, &mut body).expect("seal");
    record.extend_from_slice(&body);
    record
}

fn with_type(content: &[u8], content_type: u8) -> Vec<u8> {
    let mut inner = content.to_vec();
    inner.push(content_type);
    inner
}

fn application_data(layer: &mut RecordLayer<Pipe>) -> Vec<u8> {
    match layer.read_message() {
        Ok(Message::ApplicationData(data)) => data,
        other => panic!("application data, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------------------------
// Cost of coalesced handshake messages.

/// The smallest NewSessionTicket (RFC 8446 section 4.6.1), header included: 18 bytes, a message
/// a server may send any number of after the handshake.
fn smallest_ticket() -> Vec<u8> {
    let body = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0xaa, 0, 0];
    let mut m = vec![4, 0, 0, body.len() as u8];
    m.extend_from_slice(&body);
    m
}

/// One protected record of 910 tickets, 16,380 bytes of content.
fn record_of_tickets() -> (Vec<u8>, usize, Vec<u8>) {
    let ticket = smallest_ticket();
    let count = MAX_PLAINTEXT / ticket.len();
    let content = ticket.repeat(count);
    let peer = secret(SUITE, "the server's application traffic secret");
    (
        sealed(SUITE, &peer, 0, &with_type(&content, 22)),
        count,
        ticket,
    )
}

fn reader_of(input: Vec<u8>) -> RecordLayer<Pipe> {
    let mut layer = RecordLayer::new(Pipe::with_input(input));
    let peer = secret(SUITE, "the server's application traffic secret");
    layer.set_read_keys(SUITE, &peer).expect("read keys");
    layer
}

/// Reading the 910 messages of one record of 16 KiB allocates bytes linear in the input: a
/// layer that keeps the record's content and hands up each message allocates about twice the
/// input. At most 16 times the input is asserted.
#[test]
fn adversary_coalesced_messages_allocate_linearly() {
    let (input, count, ticket) = record_of_tickets();
    let input_len = input.len();
    let mut layer = reader_of(input);
    let (handed, allocated) = allocated_while(|| {
        let mut handed = 0;
        for _ in 0..count {
            match layer.read_message() {
                Ok(Message::Handshake(m)) => assert_eq!(m, ticket),
                other => panic!("a ticket, got {other:?}"),
            }
            handed += 1;
        }
        handed
    });
    assert_eq!(handed, count);
    assert!(
        allocated <= 16 * input_len,
        "allocated {allocated} bytes to hand up {count} messages of {} bytes from one record \
         of {input_len} bytes",
        ticket.len()
    );
}

/// Each message handed up holds memory of about its own length: the capacities of the 910
/// messages of one record, summed, stay within four times the record's content.
#[test]
fn adversary_handed_up_messages_hold_their_own_length() {
    let (input, count, ticket) = record_of_tickets();
    let mut layer = reader_of(input);
    let mut held = Vec::with_capacity(count);
    for _ in 0..count {
        match layer.read_message() {
            Ok(Message::Handshake(m)) => held.push(m),
            other => panic!("a ticket, got {other:?}"),
        }
    }
    let content = count * ticket.len();
    let capacity: usize = held.iter().map(Vec::capacity).sum();
    let largest = held.iter().map(Vec::capacity).max().unwrap_or(0);
    assert!(
        capacity <= 4 * content,
        "{count} messages of {} bytes hold {capacity} bytes of capacity, the largest {largest}, \
         for {content} bytes of content",
        ticket.len()
    );
}

// ---------------------------------------------------------------------------------------------
// Key update.

/// After `update_write_keys` the layer seals with the next application traffic secret ("traffic
/// upd", RFC 8446 section 7.2) from sequence number 0, and after `update_read_keys` it opens
/// with it; a record under the old key after the update does not open. Each suite; the other
/// direction's sequence number does not move.
#[test]
fn adversary_key_update_uses_the_next_secret() {
    for suite in CipherSuite::ALL {
        let current = secret(suite, "application traffic secret 0");
        let next = key_schedule::next_traffic_secret(&current).expect("next");

        let mut writer = RecordLayer::new(Pipe::default());
        writer.set_write_keys(suite, &current).expect("keys");
        writer.set_read_keys(suite, &current).expect("keys");
        writer.write_application_data(b"before").expect("before");
        writer.update_write_keys().expect("update");
        assert_eq!(writer.write_sequence(), Some(0));
        assert_eq!(writer.read_sequence(), Some(0));
        writer.write_application_data(b"after").expect("after");
        let expected = [
            sealed(suite, &current, 0, &with_type(b"before", 23)),
            sealed(suite, &next, 0, &with_type(b"after", 23)),
        ]
        .concat();
        assert_eq!(writer.get_ref().output, expected, "{suite:?}: written");

        let mut reader = RecordLayer::new(Pipe::with_input(expected));
        reader.set_read_keys(suite, &current).expect("keys");
        assert_eq!(application_data(&mut reader), b"before");
        reader.update_read_keys().expect("update");
        assert_eq!(application_data(&mut reader), b"after", "{suite:?}: read");

        let stale = [
            sealed(suite, &current, 0, &with_type(b"before", 23)),
            sealed(suite, &current, 1, &with_type(b"stale", 23)),
        ]
        .concat();
        let mut reader = RecordLayer::new(Pipe::with_input(stale));
        reader.set_read_keys(suite, &current).expect("keys");
        assert_eq!(application_data(&mut reader), b"before");
        reader.update_read_keys().expect("update");
        assert_eq!(
            reader.read_message().expect_err("the old key"),
            Error::AlertSent {
                description: AlertDescription::BAD_RECORD_MAC,
                violation: Violation::BadRecordMac,
            },
            "{suite:?}: a record under the old key"
        );
    }
}

// ---------------------------------------------------------------------------------------------
// The alert after a failed open.

/// A record that does not open, read while write keys are set: the `bad_record_mac` alert is
/// written protected under the write key at the next write sequence number, and nothing is
/// written or handed up after it.
#[test]
fn adversary_alert_after_a_failed_open_is_protected() {
    let ours = secret(SUITE, "our write secret");
    let theirs = secret(SUITE, "the peer's write secret");
    let mut layer = RecordLayer::new(Pipe::default());
    layer.set_read_keys(SUITE, &theirs).expect("read keys");
    layer.set_write_keys(SUITE, &ours).expect("write keys");
    layer.write_application_data(b"one").expect("one");
    layer.write_application_data(b"two").expect("two");
    layer.get_mut().output.clear();
    let mut record = sealed(SUITE, &theirs, 0, &with_type(b"the page", 23));
    let last = record.len() - 1;
    record[last] ^= 1;
    record.extend(sealed(SUITE, &theirs, 0, &with_type(b"the page", 23)));
    layer.get_mut().input = record;
    let error = layer.read_message().expect_err("refused");
    assert_eq!(
        error,
        Error::AlertSent {
            description: AlertDescription::BAD_RECORD_MAC,
            violation: Violation::BadRecordMac,
        }
    );
    let alert = sealed(SUITE, &ours, 2, &[2, 20, 21]);
    assert_eq!(
        layer.get_ref().output,
        alert,
        "the alert, sealed at number 2"
    );
    assert_eq!(layer.read_message().expect_err("closed"), error);
    assert_eq!(layer.write_application_data(b"x"), Err(error.clone()));
    assert_eq!(layer.get_ref().output, alert, "nothing after the alert");
}

// ---------------------------------------------------------------------------------------------
// The SHA-384 schedule.

/// The `HkdfLabel` of RFC 8446 section 7.1, written out here.
fn label(length: usize, label: &str, context: &[u8]) -> Vec<u8> {
    let mut info = vec![(length >> 8) as u8, length as u8, 6 + label.len() as u8];
    info.extend_from_slice(b"tls13 ");
    info.extend_from_slice(label.as_bytes());
    info.push(context.len() as u8);
    info.extend_from_slice(context);
    info
}

fn expand(algorithm: HashAlgorithm, prk: &[u8], name: &str, context: &[u8], len: usize) -> Vec<u8> {
    let mut out = vec![0u8; len];
    crypto::hkdf_expand(algorithm, prk, &[&label(len, name, context)], &mut out).expect("expand");
    out
}

fn same(what: &str, actual: &[u8], expected: &[u8]) {
    assert!(
        crypto::constant_time_eq(actual, expected),
        "{what}: the key schedule differs from the formula of RFC 8446 section 7.1"
    );
}

/// The schedule of TLS_AES_256_GCM_SHA384, which no trace of RFC 8448 covers, against the
/// formulas of RFC 8446 sections 7.1 to 7.3 built here from the seam's HKDF: the early secret
/// HKDF-Extract(0, 0) with 0 the string of 48 zero bytes, the handshake and master secrets, the
/// traffic secrets, the exporter and resumption secrets, the finished key and `verify_data`, the
/// next secret, and the traffic key and IV of 32 and 12 bytes.
#[test]
fn adversary_sha384_schedule_from_the_rfc_formulas() {
    let alg = HashAlgorithm::Sha384;
    let n = 48;
    let zeros = [0u8; 48];
    let empty = crypto::hash(alg, &[]);
    let shared = [7u8; 32];
    let transcript = [9u8; 48];

    let early = crypto::hkdf_extract(alg, &zeros, &zeros);
    let ks_early = key_schedule::early_secret(alg, None);
    assert_eq!(ks_early.len(), n);
    same("early secret", ks_early.as_ref(), early.as_ref());
    let psk = [5u8; 48];
    same(
        "early secret of a PSK",
        key_schedule::early_secret(alg, Some(&psk)).as_ref(),
        crypto::hkdf_extract(alg, &zeros, &psk).as_ref(),
    );

    let salt = expand(alg, early.as_ref(), "derived", empty.as_ref(), n);
    let handshake = crypto::hkdf_extract(alg, &salt, &shared);
    let ks_handshake = key_schedule::handshake_secret(&ks_early, &shared).expect("handshake");
    same(
        "handshake secret",
        ks_handshake.as_ref(),
        handshake.as_ref(),
    );

    let salt = expand(alg, handshake.as_ref(), "derived", empty.as_ref(), n);
    let master = crypto::hkdf_extract(alg, &salt, &zeros);
    let ks_master = key_schedule::master_secret(&ks_handshake).expect("master");
    same("master secret", ks_master.as_ref(), master.as_ref());

    let hs = key_schedule::handshake_traffic_secrets(&ks_handshake, &transcript).expect("hs");
    let c_hs = expand(alg, handshake.as_ref(), "c hs traffic", &transcript, n);
    same("c hs traffic", hs.client.as_ref(), &c_hs);
    same(
        "s hs traffic",
        hs.server.as_ref(),
        &expand(alg, handshake.as_ref(), "s hs traffic", &transcript, n),
    );
    let ap = key_schedule::application_traffic_secrets(&ks_master, &transcript).expect("ap");
    same(
        "c ap traffic",
        ap.client.as_ref(),
        &expand(alg, master.as_ref(), "c ap traffic", &transcript, n),
    );
    same(
        "s ap traffic",
        ap.server.as_ref(),
        &expand(alg, master.as_ref(), "s ap traffic", &transcript, n),
    );
    same(
        "exp master",
        key_schedule::exporter_master_secret(&ks_master, &transcript)
            .expect("exp")
            .as_ref(),
        &expand(alg, master.as_ref(), "exp master", &transcript, n),
    );
    same(
        "res master",
        key_schedule::resumption_master_secret(&ks_master, &transcript)
            .expect("res")
            .as_ref(),
        &expand(alg, master.as_ref(), "res master", &transcript, n),
    );
    let finished = expand(alg, &c_hs, "finished", &[], n);
    same(
        "finished key",
        key_schedule::finished_key(&hs.client)
            .expect("finished")
            .as_ref(),
        &finished,
    );
    same(
        "verify_data",
        key_schedule::verify_data(&hs.client, &transcript)
            .expect("verify")
            .as_ref(),
        crypto::hmac_sign(alg, &finished, &transcript).as_ref(),
    );
    same(
        "traffic upd",
        key_schedule::next_traffic_secret(&hs.client)
            .expect("next")
            .as_ref(),
        &expand(alg, &c_hs, "traffic upd", &[], n),
    );
    let keys = key_schedule::traffic_key(CipherSuite::Aes256GcmSha384, &hs.client).expect("keys");
    same("key", keys.key(), &expand(alg, &c_hs, "key", &[], 32));
    same("iv", keys.iv(), &expand(alg, &c_hs, "iv", &[], 12));
}

/// `HkdfLabel.label` is `opaque label<7..255>` (RFC 8446 section 7.1): "tls13 " with a label of
/// 249 bytes and a context of 255 bytes are encoded; an empty label, 6 bytes, is below the
/// lower bound and refused.
#[test]
fn adversary_hkdf_label_lengths_follow_7_to_255() {
    let longest = key_schedule::hkdf_label(32, &[b'a'; 249], &[1; 255]).expect("the longest");
    assert_eq!(longest[2], 255);
    assert_eq!(longest[3 + 255], 255);
    assert!(
        key_schedule::hkdf_label(32, b"", &[]).is_err(),
        "an HkdfLabel of label length 6 was encoded: {:?}",
        key_schedule::hkdf_label(32, b"", &[])
    );
}

// ---------------------------------------------------------------------------------------------
// The transcript.

fn message(msg_type: u8, len: usize, fill: u8) -> Vec<u8> {
    let mut m = vec![msg_type, (len >> 16) as u8, (len >> 8) as u8, len as u8];
    m.extend(std::iter::repeat_n(fill, len));
    m
}

fn after_retry(algorithm: HashAlgorithm, first: &[u8], rest: &[&[u8]]) -> crypto::Digest {
    let hash = crypto::hash(algorithm, first);
    let mut all = vec![254, 0, 0, hash.as_ref().len() as u8];
    all.extend_from_slice(hash.as_ref());
    for m in rest {
        all.extend_from_slice(m);
    }
    crypto::hash(algorithm, &all)
}

/// A transcript whose hash was selected before the HelloRetryRequest (`select`, or
/// `with_hash`) replaces the first ClientHello with `message_hash` as one that held it does.
#[test]
fn adversary_hello_retry_request_after_select() {
    for algorithm in [HashAlgorithm::Sha256, HashAlgorithm::Sha384] {
        let hello = message(1, 120, 0x11);
        let retry = message(2, 60, 0x22);
        let mut selected = Transcript::new();
        assert!(selected.select(algorithm));
        selected.add(&hello);
        let mut with_hash = Transcript::with_hash(algorithm);
        with_hash.add(&hello);
        let expected = after_retry(algorithm, &hello, &[&retry]);
        for (name, transcript) in [("select", &mut selected), ("with_hash", &mut with_hash)] {
            assert_eq!(transcript.hello_retry_request(algorithm, &retry), Ok(()));
            let current = transcript.current().expect("hashing");
            same(
                &format!("{algorithm:?} after {name}"),
                current.as_ref(),
                expected.as_ref(),
            );
        }
    }
}

/// `message_hash` replaces the first ClientHello once (RFC 8446 section 4.4.1): a second
/// `hello_retry_request`, after the second ClientHello, is refused and leaves the transcript
/// as it was.
#[test]
fn adversary_second_hello_retry_request_is_refused() {
    let algorithm = HashAlgorithm::Sha256;
    let first = message(1, 120, 0x11);
    let retry = message(2, 60, 0x22);
    let second = message(1, 130, 0x33);
    let mut transcript = Transcript::new();
    transcript.add(&first);
    assert_eq!(transcript.hello_retry_request(algorithm, &retry), Ok(()));
    transcript.add(&second);
    let expected = after_retry(algorithm, &first, &[&retry, &second]);
    let refused = transcript.hello_retry_request(algorithm, &retry);
    let current = transcript.current().expect("hashing");
    assert!(
        refused == Err(Violation::SecondHelloRetryRequest)
            && crypto::constant_time_eq(current.as_ref(), expected.as_ref()),
        "a second HelloRetryRequest was not refused ({refused:?}) and the transcript of ClientHello1, \
         HelloRetryRequest and ClientHello2 was replaced by one message_hash"
    );
}

// ---------------------------------------------------------------------------------------------
// A write after a failed write.

/// A stream whose writes fail after `room` bytes, part of a record written.
struct Failing {
    room: usize,
    output: Vec<u8>,
}

impl Read for Failing {
    fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
        Ok(0)
    }
}

impl Write for Failing {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if self.room == 0 {
            return Err(io::Error::from(ErrorKind::BrokenPipe));
        }
        let n = buf.len().min(self.room);
        self.room -= n;
        self.output.extend_from_slice(&buf[..n]);
        Ok(n)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// A write that fails half way ends the layer: every later write is refused with the same
/// error and writes nothing, and the sequence number used by the failed record is not used
/// again.
#[test]
fn adversary_no_write_after_a_failed_write() {
    let ours = secret(SUITE, "our write secret");
    let mut layer = RecordLayer::new(Failing {
        room: 30,
        output: Vec::new(),
    });
    layer.set_write_keys(SUITE, &ours).expect("keys");
    let error = layer
        .write_application_data(&[7u8; 100])
        .expect_err("broken pipe");
    assert_eq!(error, Error::Io(ErrorKind::BrokenPipe));
    layer.get_mut().room = usize::MAX;
    let before = layer.get_ref().output.len();
    assert_eq!(layer.write_application_data(b"again"), Err(error.clone()));
    assert_eq!(layer.read_message().expect_err("ended"), error);
    assert_eq!(layer.get_ref().output.len(), before, "nothing written");
    assert_ne!(layer.write_sequence(), Some(0), "number 0 is spent");
}

// ---------------------------------------------------------------------------------------------
// Seeded sequences of records.

struct XorShift(u64);

impl XorShift {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, n: u64) -> usize {
        (self.next() % n) as usize
    }

    fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.next() as u8).collect()
    }

    fn pick<T: Copy>(&mut self, from: &[T]) -> T {
        from[self.below(from.len() as u64)]
    }
}

/// A stream that hands out its input a few bytes at a time: one to seven per read.
struct Trickle {
    input: Vec<u8>,
    at: usize,
    random: XorShift,
    output: Vec<u8>,
}

impl Read for Trickle {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let step = 1 + self.random.below(7);
        let n = buf.len().min(step).min(self.input.len() - self.at);
        buf[..n].copy_from_slice(&self.input[self.at..self.at + n]);
        self.at += n;
        Ok(n)
    }
}

impl Write for Trickle {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.output.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Content for a record: random bytes, or a handshake fragment whose header declares a length
/// around the bounds.
fn content(random: &mut XorShift) -> Vec<u8> {
    let len = random.pick(&[0usize, 1, 2, 3, 4, 5, 17, 64, 300]);
    if random.below(3) == 0 {
        let declared = random.pick(&[0usize, 1, 3, 40, 65_535, 65_536, 16_777_215]);
        let mut m = vec![
            random.next() as u8,
            (declared >> 16) as u8,
            (declared >> 8) as u8,
            declared as u8,
        ];
        m.extend(random.bytes(len));
        m.truncate(random.below(m.len() as u64 + 1).max(1));
        m
    } else {
        random.bytes(len)
    }
}

/// One input: one to six records, each sealed under the peer's key at the next number, sealed
/// at a wrong number, without protection, a header alone with a length at a bound, or garbage.
fn records(random: &mut XorShift, peer: &Secret) -> Vec<u8> {
    let mut input = Vec::new();
    let mut sequence = 0;
    for _ in 0..1 + random.below(6) {
        match random.below(6) {
            0..=2 => {
                let mut inner = content(random);
                inner.push(random.pick(&[0u8, 20, 21, 22, 22, 23, 23, 24, 255]));
                inner.extend(vec![0u8; random.pick(&[0usize, 0, 1, 7, 300])]);
                let number = if random.below(10) == 0 {
                    sequence + 1
                } else {
                    sequence
                };
                input.extend(sealed(SUITE, peer, number, &inner));
                sequence += 1;
            }
            3 => {
                let body = content(random);
                input.push(random.pick(&[20u8, 21, 22, 23, 0, 24, 255]));
                input.extend([3, random.next() as u8]);
                input.extend((body.len() as u16).to_be_bytes());
                input.extend(body);
            }
            4 => {
                let len = random.pick(&[
                    0usize,
                    15,
                    16,
                    MAX_PLAINTEXT,
                    MAX_PLAINTEXT + 1,
                    MAX_CIPHERTEXT,
                    MAX_CIPHERTEXT + 1,
                    65_535,
                ]);
                input.push(random.pick(&[20u8, 21, 22, 23]));
                input.extend([3, 3]);
                input.extend((len as u16).to_be_bytes());
                let body = random.below(40);
                input.extend(random.bytes(body));
            }
            _ => {
                let len = random.below(40);
                input.extend(random.bytes(len));
            }
        }
    }
    input
}

/// 3,000 seeded inputs of one to six records, read through a stream that returns one to seven
/// bytes per read, with read keys or without, with a key update or a new key asked between two
/// reads: the layer hands up messages or an error, never panics, an error stays, and the layer
/// holds under 192 KiB at any time.
#[test]
fn adversary_record_sequences_never_panic() {
    let mut random = XorShift(0x0dd5_eed5_7a1e_c0de);
    let peer = secret(SUITE, "the peer's traffic secret");
    let mut handed = 0usize;
    let mut alerts = 0usize;
    for _ in 0..3_000 {
        let input = records(&mut random, &peer);
        let stream = Trickle {
            input,
            at: 0,
            random: XorShift(random.next() | 1),
            output: Vec::new(),
        };
        let mut layer = RecordLayer::new(stream);
        if random.below(4) != 0 {
            layer.set_read_keys(SUITE, &peer).expect("keys");
        }
        if random.below(2) == 0 {
            layer.set_write_keys(SUITE, &peer).expect("keys");
        }
        let ((), peak) = peak_while(|| {
            for _ in 0..12 {
                match random.below(12) {
                    0 => {
                        let _ = layer.update_read_keys();
                    }
                    1 => {
                        let _ = layer.set_read_keys(SUITE, &peer);
                    }
                    _ => {}
                }
                match layer.read_message() {
                    Ok(_) => handed += 1,
                    Err(first) => {
                        if matches!(first, Error::AlertSent { .. }) {
                            alerts += 1;
                        }
                        assert_eq!(layer.read_message().expect_err("stays"), first);
                        break;
                    }
                }
            }
        });
        assert!(peak < 192 * KIB, "peak {peak} bytes");
    }
    println!("record sequences: {handed} messages handed up, {alerts} inputs ended by an alert");
    assert!(
        handed > 300 && alerts > 1_000,
        "{handed} handed, {alerts} alerts"
    );
}
