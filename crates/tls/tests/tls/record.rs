//! The record layer of `surf-tls` against hostile records, each built in memory
//! (`story:tls-record`): every record rule of RFC 8446 sections 5.1 to 5.4 that binds a
//! client, each refusal with the alert it writes and the `Error` it returns, the bounds of
//! RFC 8446 and of docs/design/tls.md item 9, the sequence numbers, the key update, the three
//! suites, and the table `rfc8446-record-musts.txt`, whose every row names its tests.

#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use surf_tls::crypto::{self, AeadKey, HashAlgorithm};
use surf_tls::error::{AlertDescription, Error, Violation};
use surf_tls::key_schedule::{self, CipherSuite, Secret};
use surf_tls::record::{
    AES_GCM_RECORD_LIMIT, AlertLevel, ContentType, MAX_CIPHERTEXT, MAX_HANDSHAKE_MESSAGE,
    MAX_PLAINTEXT, Message, RecordLayer, write_limit,
};
use surf_tls::transcript::Transcript;

use crate::alloc::{allocated_while, peak_while};
use crate::rfc8448::Pipe;

// ---------------------------------------------------------------------------------------------
// Helpers.

const SUITE: CipherSuite = CipherSuite::Aes128GcmSha256;
/// The suite without a record limit of its own: only the sequence number bounds its keys.
const CHACHA: CipherSuite = CipherSuite::ChaCha20Poly1305Sha256;
const KIB: usize = 1024;

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn secret(suite: CipherSuite, name: &str) -> Secret {
    key_schedule::hkdf_extract(suite.hash(), &[], name.as_bytes())
}

fn peer_secret() -> Secret {
    secret(SUITE, "the peer's traffic secret")
}

fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// A record of `content_type` without protection.
fn plain(content_type: u8, content: &[u8]) -> Vec<u8> {
    let len = content.len() as u16;
    let mut record = vec![content_type, 3, 3];
    record.extend_from_slice(&len.to_be_bytes());
    record.extend_from_slice(content);
    record
}

/// A protected record of the inner plaintext `inner` (content, type and padding as given),
/// sealed here through the seam with the key and IV of `secret` at `sequence`.
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

/// A protected record of the peer at `sequence`, of `content` and its type, no padding.
fn peer_record(sequence: u64, content: &[u8], content_type: u8) -> Vec<u8> {
    let mut inner = content.to_vec();
    inner.push(content_type);
    sealed(SUITE, &peer_secret(), sequence, &inner)
}

/// A record layer that reads `input`, with the peer's keys for reading when `protected`.
fn reader(input: Vec<u8>, protected: bool) -> RecordLayer<Pipe> {
    let mut layer = RecordLayer::new(Pipe::with_input(input));
    if protected {
        layer
            .set_read_keys(SUITE, &peer_secret())
            .expect("read keys");
    }
    layer
}

/// The record of a fatal alert without protection.
fn alert_written(description: AlertDescription) -> Vec<u8> {
    vec![21, 3, 3, 0, 2, 2, description.0]
}

/// The next read is refused with the alert `description` and `violation`: the alert is
/// written, the same error comes again, and nothing more is written or handed up.
fn refused(layer: &mut RecordLayer<Pipe>, description: AlertDescription, violation: Violation) {
    let expected = Error::AlertSent {
        description,
        violation,
    };
    let first = layer.read_message().expect_err("refused");
    assert_eq!(first, expected);
    assert_eq!(layer.get_ref().output, alert_written(description));
    let again = layer.read_message().expect_err("still refused");
    assert_eq!(again, expected);
    assert_eq!(layer.get_ref().output, alert_written(description));
}

fn handshake(layer: &mut RecordLayer<Pipe>) -> Vec<u8> {
    match layer.read_message() {
        Ok(Message::Handshake(message)) => message,
        other => panic!("a handshake message, got {other:?}"),
    }
}

/// A handshake message of `msg_type` with `len` bytes of body.
fn message(msg_type: u8, len: usize) -> Vec<u8> {
    let mut m = vec![msg_type, (len >> 16) as u8, (len >> 8) as u8, len as u8];
    m.extend((0..len).map(|i| (i % 251) as u8));
    m
}

// ---------------------------------------------------------------------------------------------
// RFC 8446 section 5: the content types.

/// An inner content type RFC 8446 does not define, 0x19 among them, is `unexpected_message`,
/// `UnknownContentType`; every byte but the padding 0 and the four types is tried.
#[test]
fn unknown_content_type() {
    for content_type in (1..=255u8).filter(|t| !(20..=23).contains(t)) {
        let mut layer = reader(peer_record(0, b"abc", content_type), true);
        refused(
            &mut layer,
            AlertDescription::UNEXPECTED_MESSAGE,
            Violation::UnknownContentType,
        );
    }
}

/// An outer record type 0x18, or any other the four do not hold, is `unexpected_message`,
/// `UnknownContentType`, refused at its header, with keys and without.
#[test]
fn unknown_outer_type() {
    for outer in (0..=255u8).filter(|t| !(20..=23).contains(t)) {
        for protected in [false, true] {
            let mut layer = reader(plain(outer, &[1, 2]), protected);
            refused(
                &mut layer,
                AlertDescription::UNEXPECTED_MESSAGE,
                Violation::UnknownContentType,
            );
        }
    }
}

/// A protected `change_cipher_spec` is `unexpected_message`, `UnexpectedChangeCipherSpec`:
/// RFC 8446 section 5 lets it come only without protection.
#[test]
fn protected_change_cipher_spec() {
    let mut layer = reader(peer_record(0, &[1], 20), true);
    refused(
        &mut layer,
        AlertDescription::UNEXPECTED_MESSAGE,
        Violation::UnexpectedChangeCipherSpec,
    );
}

// ---------------------------------------------------------------------------------------------
// RFC 8446 section 5.1.

/// A record of another type between two fragments of one handshake message is
/// `unexpected_message`, `InterleavedHandshake`: an alert, a `change_cipher_spec` and, under
/// protection, application data.
#[test]
fn interleaved_handshake() {
    let half = &message(20, 32)[..10];
    for other in [plain(21, &[1, 0]), plain(20, &[1])] {
        let mut input = plain(22, half);
        input.extend(other);
        let mut layer = reader(input, false);
        refused(
            &mut layer,
            AlertDescription::UNEXPECTED_MESSAGE,
            Violation::InterleavedHandshake,
        );
    }
    let mut input = peer_record(0, half, 22);
    input.extend(peer_record(1, b"data", 23));
    let mut layer = reader(input, true);
    refused(
        &mut layer,
        AlertDescription::UNEXPECTED_MESSAGE,
        Violation::InterleavedHandshake,
    );
}

/// A key change asked while half a Finished is buffered is `unexpected_message`,
/// `HandshakeSpansKeyChange`: the record held a whole message and half of the next. The same
/// for a key update.
#[test]
fn handshake_spans_key_change() {
    let whole = message(8, 2);
    let half = &message(20, 32)[..14];
    let mut content = whole.clone();
    content.extend_from_slice(half);
    let mut layer = reader(plain(22, &content), false);
    assert_eq!(handshake(&mut layer), whole);
    let error = layer
        .set_read_keys(SUITE, &peer_secret())
        .expect_err("refused");
    assert_eq!(
        error,
        Error::AlertSent {
            description: AlertDescription::UNEXPECTED_MESSAGE,
            violation: Violation::HandshakeSpansKeyChange,
        }
    );
    assert_eq!(
        layer.get_ref().output,
        alert_written(AlertDescription::UNEXPECTED_MESSAGE)
    );
    assert_eq!(layer.read_message().expect_err("ended"), error);

    let mut layer = reader(peer_record(0, &content, 22), true);
    assert_eq!(handshake(&mut layer), whole);
    let error = layer.update_read_keys().expect_err("refused");
    assert_eq!(
        error,
        Error::AlertSent {
            description: AlertDescription::UNEXPECTED_MESSAGE,
            violation: Violation::HandshakeSpansKeyChange,
        }
    );
}

/// A handshake record of length 0 is `unexpected_message`, `EmptyContent`; surf writes no
/// empty handshake fragment: nothing for no bytes, and a message one byte over a record in
/// two records, neither empty.
#[test]
fn empty_handshake_fragment() {
    let mut layer = reader(plain(22, &[]), false);
    refused(
        &mut layer,
        AlertDescription::UNEXPECTED_MESSAGE,
        Violation::EmptyContent,
    );

    let mut writer = RecordLayer::new(Pipe::default());
    writer.write_handshake(&[]).expect("nothing");
    assert!(writer.get_ref().output.is_empty());
    let big = message(11, MAX_PLAINTEXT - 3);
    assert_eq!(big.len(), MAX_PLAINTEXT + 1);
    writer.write_handshake(&big).expect("two records");
    let out = &writer.get_ref().output;
    assert_eq!(&out[..5], &[22, 3, 3, 0x40, 0x00]);
    assert_eq!(
        &out[5 + MAX_PLAINTEXT..5 + MAX_PLAINTEXT + 5],
        &[22, 3, 3, 0, 1]
    );
    assert_eq!(out.len(), 2 * 5 + MAX_PLAINTEXT + 1);
    let mut layer = reader(out.clone(), false);
    assert_eq!(handshake(&mut layer), big);
}

/// An alert record that is not one alert of two bytes is `decode_error`, `BadAlertRecord`:
/// 1 and 4 bytes without protection, 3 with.
#[test]
fn alert_record_length() {
    for content in [&[2u8][..], &[2, 40, 2, 40][..]] {
        let mut layer = reader(plain(21, content), false);
        refused(
            &mut layer,
            AlertDescription::DECODE_ERROR,
            Violation::BadAlertRecord,
        );
    }
    let mut layer = reader(peer_record(0, &[2, 40, 0], 21), true);
    refused(
        &mut layer,
        AlertDescription::DECODE_ERROR,
        Violation::BadAlertRecord,
    );
    let mut layer = reader(plain(21, &[2, 40]), false);
    assert!(matches!(
        layer.read_message(),
        Ok(Message::Alert {
            level: 2,
            description: AlertDescription::HANDSHAKE_FAILURE
        })
    ));
}

/// `legacy_record_version` is read and ignored: a ServerHello record of version 0x0301 (and
/// 0x0000, 0xffff) is read; every record surf writes carries 0x0303, with keys and without.
#[test]
fn record_version_ignored() {
    let hello = message(2, 40);
    for version in [[3, 1], [0, 0], [0xff, 0xff]] {
        let mut record = plain(22, &hello);
        record[1..3].copy_from_slice(&version);
        let mut layer = reader(record, false);
        assert_eq!(handshake(&mut layer), hello);
    }
    let mut writer = RecordLayer::new(Pipe::default());
    writer.write_handshake(&hello).expect("hello");
    writer.write_change_cipher_spec().expect("ccs");
    writer
        .write_alert(AlertLevel::Warning, AlertDescription::CLOSE_NOTIFY)
        .expect("alert");
    writer.set_write_keys(SUITE, &peer_secret()).expect("keys");
    writer.write_handshake(&message(20, 32)).expect("finished");
    writer.write_application_data(b"data").expect("data");
    writer
        .write_alert(AlertLevel::Warning, AlertDescription::CLOSE_NOTIFY)
        .expect("alert");
    let out = writer.get_ref().output.clone();
    let mut at = 0;
    let mut records = 0;
    while at < out.len() {
        assert_eq!(&out[at + 1..at + 3], &[3, 3], "record {records}");
        at += 5 + usize::from(u16::from_be_bytes([out[at + 3], out[at + 4]]));
        records += 1;
    }
    assert_eq!(records, 6);
}

/// A record before protection longer than 2^14 is `record_overflow`, `RecordOverflow`, at
/// its header; one of 2^14 is read.
#[test]
fn oversized_plaintext_record() {
    let content = vec![0u8; MAX_PLAINTEXT + 1];
    let mut layer = reader(plain(22, &content), false);
    refused(
        &mut layer,
        AlertDescription::RECORD_OVERFLOW,
        Violation::RecordOverflow,
    );
    let exact = message(11, MAX_PLAINTEXT - 4);
    let mut layer = reader(plain(22, &exact), false);
    assert_eq!(handshake(&mut layer), exact);
}

/// Application data is never unprotected: an application data record before any key is
/// `unexpected_message`, `UnprotectedRecord`; so is a handshake or an alert record without
/// protection after the keys are set.
#[test]
fn unprotected_application_data() {
    let mut layer = reader(plain(23, b"hello"), false);
    refused(
        &mut layer,
        AlertDescription::UNEXPECTED_MESSAGE,
        Violation::UnprotectedRecord,
    );
    for record in [plain(22, &message(20, 32)), plain(21, &[2, 40])] {
        let mut layer = reader(record, true);
        refused(
            &mut layer,
            AlertDescription::UNEXPECTED_MESSAGE,
            Violation::UnprotectedRecord,
        );
    }
    let mut writer = RecordLayer::new(Pipe::default());
    assert!(writer.write_application_data(b"data").is_err());
    assert!(
        writer.get_ref().output.iter().all(|&b| b != b'd'),
        "no application data written unprotected"
    );
}

// ---------------------------------------------------------------------------------------------
// RFC 8446 section 5.2.

/// A protected record of 2^14 + 257 bytes is `record_overflow`, `RecordOverflow`, refused at
/// its header: the layer holds under 64 KiB while it refuses. One of 2^14 + 256 is read and
/// fails to open.
#[test]
fn oversized_ciphertext() {
    let mut input = vec![23, 3, 3];
    input.extend_from_slice(&((MAX_CIPHERTEXT + 1) as u16).to_be_bytes());
    input.extend(vec![0u8; MAX_CIPHERTEXT + 1]);
    let mut layer = reader(input, true);
    let (result, peak) = peak_while(|| layer.read_message());
    assert_eq!(
        result.expect_err("refused"),
        Error::AlertSent {
            description: AlertDescription::RECORD_OVERFLOW,
            violation: Violation::RecordOverflow,
        }
    );
    assert!(peak < 64 * KIB, "peak {peak} bytes");
    assert_eq!(layer.get_ref().read, 5, "refused at its header");
    assert_eq!(
        layer.get_ref().output,
        alert_written(AlertDescription::RECORD_OVERFLOW)
    );

    let mut input = vec![23, 3, 3];
    input.extend_from_slice(&(MAX_CIPHERTEXT as u16).to_be_bytes());
    input.extend(vec![0u8; MAX_CIPHERTEXT]);
    let mut layer = reader(input, true);
    refused(
        &mut layer,
        AlertDescription::BAD_RECORD_MAC,
        Violation::BadRecordMac,
    );
}

/// One flipped bit of a tag, of the ciphertext or of the header is `bad_record_mac`,
/// `BadRecordMac`; no plaintext is handed up, and the valid record after it is not read.
#[test]
fn flipped_tag() {
    let good = peer_record(0, b"the plaintext", 23);
    for (at, bit) in [
        (good.len() - 1, 0),
        (good.len() - 16, 7),
        (6, 0),
        (4, 0),
        (2, 0),
    ] {
        let mut record = good.clone();
        record[at] ^= 1 << bit;
        record.extend(peer_record(0, b"the plaintext", 23));
        let mut layer = reader(record, true);
        if at == 4 {
            // A changed length reads past the record or stops short of it: either way no
            // plaintext.
            assert!(layer.read_message().is_err());
            continue;
        }
        refused(
            &mut layer,
            AlertDescription::BAD_RECORD_MAC,
            Violation::BadRecordMac,
        );
        assert_eq!(layer.read_sequence(), Some(0));
    }
    let mut layer = reader(good, true);
    match layer.read_message() {
        Ok(Message::ApplicationData(data)) => assert_eq!(data, b"the plaintext"),
        other => panic!("{other:?}"),
    }
    assert_eq!(layer.read_sequence(), Some(1));
}

// ---------------------------------------------------------------------------------------------
// RFC 8446 section 5.3.

/// A sequence number at 2^64 - 1 refuses to protect another record: `SequenceExhausted`, no
/// record written; at 2^64 - 2 one record is written and then none. A write of two records
/// with one number left writes neither. A record to read at 2^64 - 1 is refused unopened.
#[test]
fn sequence_exhausted() {
    let chacha_secret = secret(CHACHA, "the peer's traffic secret");
    let mut writer = RecordLayer::new(Pipe::default());
    writer.set_write_keys(CHACHA, &chacha_secret).expect("keys");
    writer.set_write_sequence(u64::MAX);
    assert_eq!(
        writer.write_application_data(b"data"),
        Err(Error::SequenceExhausted)
    );
    assert!(writer.get_ref().output.is_empty(), "no record written");
    assert_eq!(
        writer.write_application_data(b"data"),
        Err(Error::SequenceExhausted)
    );

    let mut writer = RecordLayer::new(Pipe::default());
    writer.set_write_keys(CHACHA, &chacha_secret).expect("keys");
    writer.set_write_sequence(u64::MAX - 1);
    writer
        .write_application_data(&vec![7u8; MAX_PLAINTEXT + 1])
        .expect_err("two records, one number");
    assert!(writer.get_ref().output.is_empty(), "neither written");

    let mut writer = RecordLayer::new(Pipe::default());
    writer.set_write_keys(CHACHA, &chacha_secret).expect("keys");
    writer.set_write_sequence(u64::MAX - 1);
    writer.write_application_data(b"last").expect("the last");
    assert_eq!(writer.write_sequence(), Some(u64::MAX));
    assert_eq!(
        writer.get_ref().output,
        sealed(CHACHA, &chacha_secret, u64::MAX - 1, b"last\x17")
    );
    assert_eq!(
        writer.write_handshake(&message(24, 1)),
        Err(Error::SequenceExhausted)
    );

    let mut layer = reader(peer_record(u64::MAX, b"x", 23), true);
    layer.set_read_sequence(u64::MAX);
    assert_eq!(
        layer.read_message().expect_err("refused"),
        Error::SequenceExhausted
    );
}

/// An AES-GCM write key protects at most `AES_GCM_RECORD_LIMIT` records (RFC 8446 section 5.5,
/// 2^24.5 rounded down): the record past it is refused with `KeyLimitReached`, nothing of it
/// is written, and the layer stays closed. ChaCha20-Poly1305 writes on at the same number; the
/// read side has no such limit.
#[test]
fn aes_gcm_write_limit() {
    assert_eq!(AES_GCM_RECORD_LIMIT, 23_726_566);
    for suite in [CipherSuite::Aes128GcmSha256, CipherSuite::Aes256GcmSha384] {
        assert_eq!(write_limit(suite), AES_GCM_RECORD_LIMIT, "{suite:?}");
        let key = secret(suite, "the peer's traffic secret");
        let mut writer = RecordLayer::new(Pipe::default());
        writer.set_write_keys(suite, &key).expect("keys");
        writer.set_write_sequence(AES_GCM_RECORD_LIMIT - 1);
        writer.write_application_data(b"last").expect("the last");
        assert_eq!(
            writer.get_ref().output,
            sealed(suite, &key, AES_GCM_RECORD_LIMIT - 1, b"last\x17")
        );
        let written = writer.get_ref().output.len();
        assert_eq!(
            writer.write_application_data(b"one more"),
            Err(Error::KeyLimitReached)
        );
        assert_eq!(writer.get_ref().output.len(), written, "nothing written");
        assert_eq!(
            writer.write_handshake(&message(24, 1)),
            Err(Error::KeyLimitReached)
        );
        assert_eq!(
            writer.read_message().expect_err("closed"),
            Error::KeyLimitReached
        );
        assert_eq!(writer.get_ref().output.len(), written, "nothing written");

        let mut writer = RecordLayer::new(Pipe::default());
        writer.set_write_keys(suite, &key).expect("keys");
        writer.set_write_sequence(AES_GCM_RECORD_LIMIT - 1);
        assert_eq!(
            writer.write_application_data(&vec![7u8; MAX_PLAINTEXT + 1]),
            Err(Error::KeyLimitReached),
            "two records, one left"
        );
        assert!(writer.get_ref().output.is_empty(), "neither written");
    }

    assert_eq!(write_limit(CHACHA), u64::MAX);
    let key = secret(CHACHA, "the peer's traffic secret");
    let mut writer = RecordLayer::new(Pipe::default());
    writer.set_write_keys(CHACHA, &key).expect("keys");
    writer.set_write_sequence(AES_GCM_RECORD_LIMIT);
    writer.write_application_data(b"past").expect("no limit");
    assert_eq!(
        writer.get_ref().output,
        sealed(CHACHA, &key, AES_GCM_RECORD_LIMIT, b"past\x17")
    );

    let mut layer = reader(peer_record(AES_GCM_RECORD_LIMIT, b"x", 23), true);
    layer.set_read_sequence(AES_GCM_RECORD_LIMIT);
    match layer.read_message() {
        Ok(Message::ApplicationData(data)) => assert_eq!(data, b"x"),
        other => panic!("read past the write limit: {other:?}"),
    }
}

/// Each record has its own nonce: the sequence number counts each record from 0 at each
/// key, and the same plaintext gives other bytes at each number.
#[test]
fn sequence_counts_from_zero_at_each_key() {
    let mut writer = RecordLayer::new(Pipe::default());
    writer.set_write_keys(SUITE, &peer_secret()).expect("keys");
    for n in 0..3 {
        assert_eq!(writer.write_sequence(), Some(n));
        writer.write_application_data(b"same").expect("write");
    }
    let out = writer.get_ref().output.clone();
    let len = out.len() / 3;
    assert_ne!(out[..len], out[len..2 * len]);
    assert_eq!(out[..len], peer_record(0, b"same", 23));
    assert_eq!(out[2 * len..], peer_record(2, b"same", 23));
    writer
        .set_write_keys(SUITE, &peer_secret())
        .expect("keys again");
    assert_eq!(writer.write_sequence(), Some(0));
}

// ---------------------------------------------------------------------------------------------
// RFC 8446 section 5.4.

/// A protected alert or handshake record whose content is empty is `unexpected_message`,
/// `EmptyContent`; so is an alert record of length 0 without protection.
#[test]
fn empty_alert_content() {
    for content_type in [21u8, 22] {
        let mut layer = reader(peer_record(0, &[], content_type), true);
        refused(
            &mut layer,
            AlertDescription::UNEXPECTED_MESSAGE,
            Violation::EmptyContent,
        );
    }
    let mut layer = reader(plain(21, &[]), false);
    refused(
        &mut layer,
        AlertDescription::UNEXPECTED_MESSAGE,
        Violation::EmptyContent,
    );
}

/// A record of zero padding only, or of no inner plaintext at all, is `unexpected_message`,
/// `EmptyInnerPlaintext`; only the cleartext is scanned, not the tag. Padding after a type is
/// taken away.
#[test]
fn padding_only() {
    for inner in [
        vec![0u8; 5],
        vec![0u8; 1],
        Vec::new(),
        vec![0u8; MAX_PLAINTEXT + 1],
    ] {
        let record = sealed(SUITE, &peer_secret(), 0, &inner);
        let mut layer = reader(record, true);
        refused(
            &mut layer,
            AlertDescription::UNEXPECTED_MESSAGE,
            Violation::EmptyInnerPlaintext,
        );
    }
    let mut inner = b"data".to_vec();
    inner.push(23);
    inner.extend([0u8; 100]);
    let mut layer = reader(sealed(SUITE, &peer_secret(), 0, &inner), true);
    match layer.read_message() {
        Ok(Message::ApplicationData(data)) => assert_eq!(data, b"data"),
        other => panic!("{other:?}"),
    }
}

/// An inner plaintext over 2^14 + 1 bytes is `record_overflow`, `RecordOverflow`, after
/// opening: content of 2^14 + 1 bytes, and one byte of content padded to 2^14 + 2. Content of
/// 2^14 is read.
#[test]
fn oversized_inner_plaintext() {
    let mut inner = vec![7u8; MAX_PLAINTEXT + 1];
    inner.push(23);
    let mut padded = vec![7u8, 23];
    padded.extend(vec![0u8; MAX_PLAINTEXT]);
    for inner in [inner, padded] {
        assert_eq!(inner.len(), MAX_PLAINTEXT + 2);
        let mut layer = reader(sealed(SUITE, &peer_secret(), 0, &inner), true);
        refused(
            &mut layer,
            AlertDescription::RECORD_OVERFLOW,
            Violation::RecordOverflow,
        );
    }
    let mut layer = reader(peer_record(0, &vec![7u8; MAX_PLAINTEXT], 23), true);
    match layer.read_message() {
        Ok(Message::ApplicationData(data)) => assert_eq!(data.len(), MAX_PLAINTEXT),
        other => panic!("{other:?}"),
    }
}

// ---------------------------------------------------------------------------------------------
// The end of the stream and the bound of a handshake message.

/// A record cut after 3 of its 5 header bytes, then the end of the stream, is
/// `ConnectionClosed`, and so is one cut inside its body; no alert is written.
#[test]
fn truncated_record() {
    let mut layer = reader(vec![22, 3, 3], false);
    assert_eq!(
        layer.read_message().expect_err("closed"),
        Error::ConnectionClosed
    );
    assert!(layer.get_ref().output.is_empty());
    let mut record = plain(22, &message(8, 10));
    record.truncate(9);
    let mut layer = reader(record, false);
    assert_eq!(
        layer.read_message().expect_err("closed"),
        Error::ConnectionClosed
    );
    let mut layer = reader(Vec::new(), true);
    assert_eq!(
        layer.read_message().expect_err("closed"),
        Error::ConnectionClosed
    );
}

/// A handshake header declaring 2^24 - 1 bytes is `illegal_parameter`, `MessageTooLarge`
/// after its 4 bytes, the layer holding under 64 KiB; 65,536 bytes is refused too, and a
/// message of 65,535 bytes over five records is read.
#[test]
fn handshake_message_too_large() {
    let mut layer = reader(plain(22, &[1, 0xff, 0xff, 0xff]), false);
    let (result, peak) = peak_while(|| layer.read_message());
    let expected = Error::AlertSent {
        description: AlertDescription::ILLEGAL_PARAMETER,
        violation: Violation::MessageTooLarge {
            declared: 16_777_215,
            bound: MAX_HANDSHAKE_MESSAGE,
        },
    };
    assert_eq!(result.expect_err("refused"), expected);
    assert!(peak < 64 * KIB, "peak {peak} bytes");
    assert_eq!(
        layer.get_ref().output,
        alert_written(AlertDescription::ILLEGAL_PARAMETER)
    );

    let mut layer = reader(plain(22, &[11, 1, 0, 0]), false);
    refused(
        &mut layer,
        AlertDescription::ILLEGAL_PARAMETER,
        Violation::MessageTooLarge {
            declared: 65_536,
            bound: 65_535,
        },
    );

    let largest = message(11, MAX_HANDSHAKE_MESSAGE);
    let mut input = Vec::new();
    for chunk in largest.chunks(MAX_PLAINTEXT) {
        input.extend(plain(22, chunk));
    }
    let mut layer = reader(input, false);
    assert_eq!(handshake(&mut layer), largest);
}

/// The bytes allocated to read one record of `count` handshake messages of 8 bytes each and
/// hand them up one at a time.
fn allocated_for_messages(count: usize) -> usize {
    let one = message(4, 4);
    let content: Vec<u8> = one
        .iter()
        .copied()
        .cycle()
        .take(count * one.len())
        .collect();
    let mut layer = reader(plain(22, &content), false);
    let (handed, allocated) = allocated_while(|| {
        let mut handed = 0;
        while layer.has_buffered_handshake() || handed == 0 {
            match layer.read_message() {
                Ok(Message::Handshake(m)) => assert_eq!(m, one),
                other => panic!("a handshake message, got {other:?}"),
            }
            handed += 1;
        }
        handed
    });
    assert_eq!(handed, count);
    allocated
}

/// Handing up the messages of one record allocates bytes linear in their number: twice the
/// messages, in a record twice as long, allocate at most 2.5 times the bytes (a copy of what
/// is left after each message would allocate about four times).
#[test]
fn coalesced_messages_allocate_linearly() {
    let n = 1000;
    let at_n = allocated_for_messages(n);
    let at_2n = allocated_for_messages(2 * n);
    eprintln!(
        "allocated: {n} messages {at_n} bytes, {} messages {at_2n} bytes",
        2 * n
    );
    assert!(
        at_2n * 2 <= at_n * 5,
        "{n} messages allocated {at_n} bytes, {} allocated {at_2n}",
        2 * n
    );
}

/// Handshake messages are reassembled across records, a header split one byte per record
/// included, and several in one record are handed up one at a time.
#[test]
fn handshake_messages_reassembled() {
    let first = message(8, 3);
    let second = message(11, 300);
    let mut input = Vec::new();
    for byte in &first[..4] {
        input.extend(plain(22, &[*byte]));
    }
    let mut rest = first[4..].to_vec();
    rest.extend_from_slice(&second);
    input.extend(plain(22, &rest));
    let mut layer = reader(input, false);
    assert_eq!(handshake(&mut layer), first);
    assert!(layer.has_buffered_handshake());
    assert_eq!(handshake(&mut layer), second);
    assert!(!layer.has_buffered_handshake());
}

// ---------------------------------------------------------------------------------------------
// Any bytes, no panic.

struct XorShift(u64);

impl XorShift {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.next() as u8).collect()
    }
}

/// Reads until an error; an error stays.
fn drain(layer: &mut RecordLayer<Pipe>) {
    for _ in 0..8 {
        if let Err(first) = layer.read_message() {
            assert_eq!(layer.read_message().expect_err("stays"), first);
            return;
        }
    }
}

/// Any record length, outer type, inner type and padding, with keys and without, and
/// random bytes: the layer hands up a message or an error and never panics.
#[test]
fn any_bytes_never_panic() {
    let mut random = XorShift(0x5eed_1234_abcd_ef01);
    let mut cases = 0;
    for outer in 0..=255u8 {
        for len in [0usize, 1, 2, 4, 5, 17, 300] {
            for protected in [false, true] {
                let mut record = vec![outer, 3, 3];
                record.extend_from_slice(&(len as u16).to_be_bytes());
                record.extend(random.bytes(len));
                drain(&mut reader(record, protected));
                cases += 1;
            }
        }
    }
    for content_type in 0..=255u8 {
        for padding in 0..3 {
            for len in [0usize, 1, 5] {
                let mut inner = random.bytes(len);
                inner.push(content_type);
                inner.extend(vec![0u8; padding]);
                drain(&mut reader(sealed(SUITE, &peer_secret(), 0, &inner), true));
                cases += 1;
            }
        }
    }
    for _ in 0..1_000 {
        let len = (random.next() % 64) as usize;
        let mut input = random.bytes(len);
        if let Some(first) = input.first_mut() {
            *first = 20 + (*first % 5);
        }
        drain(&mut reader(input, random.next().is_multiple_of(2)));
        cases += 1;
    }
    println!("any bytes: {cases} inputs read");
}

// ---------------------------------------------------------------------------------------------
// Keys, suites and secrets.

/// `from` writes handshake messages, application data and a `close_notify`; `to` reads them.
fn exchange(
    suite: CipherSuite,
    from: &mut RecordLayer<Pipe>,
    to: &mut RecordLayer<Pipe>,
    messages: &[u8],
    data: &[u8],
) {
    from.write_handshake(messages).expect("handshake");
    from.write_application_data(data).expect("data");
    from.write_alert(AlertLevel::Warning, AlertDescription::CLOSE_NOTIFY)
        .expect("alert");
    let bytes = std::mem::take(&mut from.get_mut().output);
    to.get_mut().input.extend(bytes);
    let mut handed = Vec::new();
    while handed.len() < messages.len() {
        handed.extend(handshake(to));
    }
    assert_eq!(handed, messages);
    let mut received = Vec::new();
    while received.len() < data.len() {
        match to.read_message() {
            Ok(Message::ApplicationData(d)) => received.extend(d),
            other => panic!("{suite:?}: {other:?}"),
        }
    }
    assert_eq!(received, data);
    assert!(matches!(
        to.read_message(),
        Ok(Message::Alert {
            level: 1,
            description: AlertDescription::CLOSE_NOTIFY
        })
    ));
}

/// Each of the three suites carries handshake messages, application data over several
/// records and an alert both ways, before and after a key update in each direction; the keys
/// have the lengths of the suite.
#[test]
fn every_suite_round_trips() {
    for suite in CipherSuite::ALL {
        let client = secret(suite, "client");
        let server = secret(suite, "server");
        assert_eq!(client.len(), suite.hash().output_len());
        let keys = key_schedule::traffic_key(suite, &client).expect("keys");
        assert_eq!(keys.key().len(), suite.aead().key_len());
        let mut a = RecordLayer::new(Pipe::default());
        let mut b = RecordLayer::new(Pipe::default());
        a.set_write_keys(suite, &client).expect("a write");
        a.set_read_keys(suite, &server).expect("a read");
        b.set_write_keys(suite, &server).expect("b write");
        b.set_read_keys(suite, &client).expect("b read");
        for round in 0..2 {
            let messages = [message(8, 10), message(11, 20_000), message(15, 70)].concat();
            let data = vec![round as u8; 2 * MAX_PLAINTEXT + 5];
            exchange(suite, &mut a, &mut b, &messages, &data);
            exchange(suite, &mut b, &mut a, &messages, &data);
            assert!(a.write_sequence().expect("keys") > 0);
            a.update_write_keys().expect("a update");
            b.update_read_keys().expect("b update");
            b.update_write_keys().expect("b update");
            a.update_read_keys().expect("a update");
            assert_eq!(a.write_sequence(), Some(0));
            assert_eq!(b.read_sequence(), Some(0));
        }
    }
}

/// The next application traffic secret is HKDF-Expand-Label of the current one under
/// "traffic upd" with an empty context, built here from the seam with its `HkdfLabel`
/// written out; the same for SHA-384 and for the `HkdfLabel` of any label.
#[test]
fn key_update_next_secret() {
    for suite in [CipherSuite::Aes128GcmSha256, CipherSuite::Aes256GcmSha384] {
        let current = secret(suite, "current");
        let next = key_schedule::next_traffic_secret(&current).expect("next");
        let len = suite.hash().output_len();
        let mut expected = vec![0u8; len];
        crypto::hkdf_expand(
            suite.hash(),
            current.as_ref(),
            &[&[0, len as u8], &[17], b"tls13 traffic upd", &[0]],
            &mut expected,
        )
        .expect("expand");
        assert!(crypto::constant_time_eq(next.as_ref(), &expected));
        assert_eq!(next.algorithm(), suite.hash());
    }
    let info = key_schedule::hkdf_label(48, b"c hs traffic", &[9; 48]).expect("label");
    assert_eq!(&info[..3], &[0, 48, 18]);
    assert_eq!(&info[3..21], b"tls13 c hs traffic");
    assert_eq!(info[21], 48);
    assert_eq!(info.len(), 22 + 48);
    assert!(key_schedule::hkdf_label(32, &[b'a'; 250], &[]).is_err());
    assert!(key_schedule::hkdf_label(32, b"x", &[0; 256]).is_err());
    let wrong = secret(CipherSuite::Aes256GcmSha384, "sha384");
    assert!(key_schedule::traffic_key(CipherSuite::Aes128GcmSha256, &wrong).is_err());
}

/// After a HelloRetryRequest the transcript is the hash of `message_hash`, the hash of the
/// first ClientHello, and what follows, built here by hand for SHA-384 too.
#[test]
fn transcript_message_hash() {
    for algorithm in [HashAlgorithm::Sha256, HashAlgorithm::Sha384] {
        let hello = message(1, 100);
        let retry = message(2, 50);
        let mut transcript = Transcript::new();
        transcript.add(&hello);
        assert!(transcript.current().is_none());
        assert_eq!(transcript.hello_retry_request(algorithm, &retry), Ok(()));
        let first = crypto::hash(algorithm, &hello);
        let mut manual = vec![254, 0, 0, first.as_ref().len() as u8];
        manual.extend_from_slice(first.as_ref());
        manual.extend_from_slice(&retry);
        let expected = crypto::hash(algorithm, &manual);
        let current = transcript.current().expect("hashing");
        assert!(crypto::constant_time_eq(
            current.as_ref(),
            expected.as_ref()
        ));
        let other = match algorithm {
            HashAlgorithm::Sha256 => HashAlgorithm::Sha384,
            HashAlgorithm::Sha384 => HashAlgorithm::Sha256,
        };
        assert!(!transcript.select(other));
        assert_eq!(
            transcript.hello_retry_request(other, &retry),
            Err(Violation::SecondHelloRetryRequest)
        );
        assert!(transcript.select(algorithm));
        let mut selected = Transcript::with_hash(other);
        selected.add(&hello);
        let before = selected.current().expect("hashing");
        assert_eq!(
            selected.hello_retry_request(algorithm, &retry),
            Err(Violation::UnofferedSuite)
        );
        let after = selected.current().expect("hashing");
        assert!(crypto::constant_time_eq(before.as_ref(), after.as_ref()));
    }
}

/// No `Debug` or `Display` form holds a byte of a secret, a traffic key or an IV, nor the
/// plaintext of application data.
#[test]
fn debug_forms_hold_no_secret_bytes() {
    let secret = peer_secret();
    let keys = key_schedule::traffic_key(SUITE, &secret).expect("keys");
    let secrets = key_schedule::handshake_traffic_secrets(&secret, &[1; 32]).expect("secrets");
    let mut layer = reader(peer_record(0, b"PLAINTEXT-OF-THE-PAGE", 23), true);
    layer.set_write_keys(SUITE, &secret).expect("keys");
    let message = layer.read_message().expect("a record");
    let refusal = reader(plain(23, b"x"), false)
        .read_message()
        .expect_err("refused");
    let forms = [
        format!("{secret:?}"),
        format!("{keys:?}"),
        format!("{secrets:?}"),
        format!("{layer:?}"),
        format!("{message:?}"),
        format!("{refusal:?} {refusal}"),
        format!("{:?}", Transcript::new()),
        format!("{:?}", ContentType::ApplicationData),
    ];
    let hidden = [
        to_hex(secret.as_ref()),
        to_hex(&secret.as_ref()[..4]),
        to_hex(keys.key()),
        to_hex(keys.iv()),
        format!("{:?}", &secret.as_ref()[..4]),
        format!("{:?}", &keys.key()[..4]),
        format!("{:?}", &keys.iv()[..4]),
        to_hex(secrets.client.as_ref()),
        "PLAINTEXT".to_string(),
    ];
    for form in &forms {
        for value in &hidden {
            assert!(!form.contains(value.as_str()), "{form} holds {value}");
        }
    }
}

// ---------------------------------------------------------------------------------------------
// The table of RFC 8446 record obligations.

/// Whether `source` holds `fn <function>(` with `#[test]` among the attribute lines right
/// above it (doc comments may stand among them).
fn has_test(source: &str, function: &str) -> bool {
    let lines: Vec<&str> = source.lines().collect();
    let needle = format!("fn {function}(");
    let public = format!("pub {needle}");
    for (i, line) in lines.iter().enumerate() {
        let line = line.trim_start();
        if !(line.starts_with(&needle) || line.starts_with(&public)) {
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

/// Checks the table: four columns on every line, and every entry of the last one either
/// `tests/tls/<file>.rs::<function>`, where that file of the crate (read by `read`) holds the
/// test, or `not reachable: <why>`. The number of rows, or one problem per line.
fn check_musts(text: &str, read: &dyn Fn(&str) -> Option<String>) -> Result<usize, Vec<String>> {
    let mut problems = Vec::new();
    let mut rows = 0;
    let mut sources: HashMap<String, Option<String>> = HashMap::new();
    for (index, line) in text.lines().enumerate() {
        let n = index + 1;
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

fn read_crate_file(path: &str) -> Option<String> {
    fs::read_to_string(crate_dir().join(path)).ok()
}

/// Every line of `rfc8446-record-musts.txt` has four columns and names its tests, each a
/// `#[test]` of this crate; the file has the 16 rows of the story's table. A copy with one
/// function name changed, and a copy of the tests with one `#[test]` removed, each fail
/// naming the line.
#[test]
fn musts_rows_name_tests() {
    let text = fs::read_to_string(crate_dir().join("tests/tls/rfc8446-record-musts.txt"))
        .expect("read the table");
    assert_eq!(check_musts(&text, &read_crate_file), Ok(16));

    let renamed = text.replacen("record.rs::flipped_tag", "record.rs::flipped_tags", 1);
    let line = text
        .lines()
        .position(|l| l.contains("record.rs::flipped_tag"))
        .expect("the row")
        + 1;
    let problems = check_musts(&renamed, &read_crate_file).expect_err("a renamed test fails");
    assert_eq!(
        problems,
        vec![format!(
            "line {line}: tests/tls/record.rs has no #[test] fn flipped_tags("
        )]
    );

    let without = |path: &str| {
        read_crate_file(path).map(|source| {
            if path.ends_with("record.rs") {
                source.replacen("#[test]\nfn flipped_tag()", "fn flipped_tag()", 1)
            } else {
                source
            }
        })
    };
    assert_ne!(
        without("tests/tls/record.rs"),
        read_crate_file("tests/tls/record.rs")
    );
    let problems = check_musts(&text, &without).expect_err("a removed #[test] fails");
    assert_eq!(
        problems,
        vec![format!(
            "line {line}: tests/tls/record.rs has no #[test] fn flipped_tag("
        )]
    );

    let wildcard = text.replacen("record.rs::flipped_tag", "record.rs::*", 1);
    assert!(check_musts(&wildcard, &read_crate_file).is_err());
    let three = text.replacen(" | ", " ", 1);
    assert!(check_musts(&three, &read_crate_file).is_err());
    let unreachable = text.replacen("tests/tls/record.rs::flipped_tag", "not reachable: ", 1);
    assert!(check_musts(&unreachable, &read_crate_file).is_err());
}
