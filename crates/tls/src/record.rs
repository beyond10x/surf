//! The record layer of TLS 1.3 (RFC 8446 section 5) over any `Read + Write`: records read and
//! written, protected by the AEAD of the suite through the seam once keys are set, handshake
//! messages reassembled across records, alerts and `change_cipher_spec` records handed up.
//!
//! It serves either side of a connection: the caller gives the traffic secret of the records
//! it reads ([`RecordLayer::set_read_keys`]) and of those it writes
//! ([`RecordLayer::set_write_keys`]), its own and its peer's, from
//! [`crate::key_schedule`]. Each direction counts its own sequence number from 0 at each key;
//! the nonce of a record is the IV XOR that number (section 5.3), and a number that would
//! wrap protects no record ([`Error::SequenceExhausted`]).
//!
//! What it refuses on reading, each with the alert it writes before it returns
//! [`Error::AlertSent`] (the table "RFC 8446 record obligations" of `story:tls-record`):
//!
//! - an outer or inner content type RFC 8446 does not define: `unexpected_message`,
//!   `UnknownContentType`; a protected `change_cipher_spec`: `unexpected_message`,
//!   `UnexpectedChangeCipherSpec`;
//! - a record without protection longer than 2^14: `record_overflow`, `RecordOverflow`; a
//!   protected one longer than 2^14 + 256, at its header, and an inner plaintext longer than
//!   2^14 + 1 after opening: the same;
//! - application data without protection, before the read keys are set; a handshake or alert
//!   record without protection after: `unexpected_message`, `UnprotectedRecord`;
//! - a protected record that does not open: `bad_record_mac`, `BadRecordMac`; its bytes are
//!   overwritten and nothing of it is handed up;
//! - an inner plaintext of zero padding only: `unexpected_message`, `EmptyInnerPlaintext`;
//! - a handshake or alert record with no content: `unexpected_message`, `EmptyContent`; an
//!   alert record of other than two bytes: `decode_error`, `BadAlertRecord`;
//! - a record of another type between two fragments of a handshake message:
//!   `unexpected_message`, `InterleavedHandshake`; a key change while handshake bytes are
//!   buffered: `unexpected_message`, `HandshakeSpansKeyChange`;
//! - a handshake message that declares more than [`MAX_HANDSHAKE_MESSAGE`] bytes, at its
//!   4-byte header (docs/design/tls.md, item 9.1): `illegal_parameter`, `MessageTooLarge`.
//!
//! The end of the stream, inside a record or between records, is [`Error::ConnectionClosed`];
//! the connection layer reads it as `Truncated` after the handshake. An error of the stream is
//! [`Error::Io`]. After any error the layer refuses every later call with the same error.
//!
//! `legacy_record_version` is read and ignored; every record written carries 0x0303, no
//! padding and no empty handshake fragment. A record is at most 2^14 bytes of content plus
//! the AEAD's 17 bytes, so the layer holds at most one record and one handshake message:
//! about 2^14 + 256 and 65,535 + 2^14 bytes.

use std::fmt;
use std::io::{ErrorKind, Read, Write};

use crate::crypto::{AeadAlgorithm, AeadKey};
use crate::error::{AlertDescription, Error, Violation};
use crate::key_schedule::{self, CipherSuite, Secret};

/// The most content a record carries: 2^14 bytes (section 5.1).
pub const MAX_PLAINTEXT: usize = 1 << 14;
/// The most an inner plaintext holds, content, type and padding: 2^14 + 1 bytes (5.4).
pub const MAX_INNER_PLAINTEXT: usize = MAX_PLAINTEXT + 1;
/// The longest protected record: 2^14 + 256 bytes (5.2).
pub const MAX_CIPHERTEXT: usize = MAX_PLAINTEXT + 256;
/// The longest handshake message surf takes, its header aside: 65,535 bytes
/// (docs/design/tls.md, item 9.1).
pub const MAX_HANDSHAKE_MESSAGE: usize = 65_535;
/// The `legacy_record_version` of every record written.
pub const RECORD_VERSION: [u8; 2] = [3, 3];

const HEADER_LEN: usize = 5;
const TAG_LEN: usize = AeadAlgorithm::TAG_LEN;

/// The content type of a record (section 5.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ContentType {
    /// `change_cipher_spec` (20).
    ChangeCipherSpec,
    /// `alert` (21).
    Alert,
    /// `handshake` (22).
    Handshake,
    /// `application_data` (23).
    ApplicationData,
}

impl ContentType {
    /// The byte of the type.
    pub fn code(self) -> u8 {
        match self {
            ContentType::ChangeCipherSpec => 20,
            ContentType::Alert => 21,
            ContentType::Handshake => 22,
            ContentType::ApplicationData => 23,
        }
    }

    /// The type of a byte, if RFC 8446 defines it.
    pub fn from_code(code: u8) -> Option<ContentType> {
        match code {
            20 => Some(ContentType::ChangeCipherSpec),
            21 => Some(ContentType::Alert),
            22 => Some(ContentType::Handshake),
            23 => Some(ContentType::ApplicationData),
            _ => None,
        }
    }
}

/// The level of an alert written (section 6): `close_notify` and `user_canceled` are
/// written as warnings, every other alert as fatal.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AlertLevel {
    /// `warning` (1).
    Warning,
    /// `fatal` (2).
    Fatal,
}

impl AlertLevel {
    /// The byte of the level.
    pub fn code(self) -> u8 {
        match self {
            AlertLevel::Warning => 1,
            AlertLevel::Fatal => 2,
        }
    }
}

/// What a read hands up: one whole handshake message, one record of another type.
pub enum Message {
    /// One handshake message, its 4-byte header included.
    Handshake(Vec<u8>),
    /// The content of a `change_cipher_spec` record, for the handshake to judge.
    ChangeCipherSpec(Vec<u8>),
    /// One alert; the level as received, which TLS 1.3 lets the reader ignore.
    Alert {
        /// The level byte.
        level: u8,
        /// The description.
        description: AlertDescription,
    },
    /// The content of an application data record, possibly empty.
    ApplicationData(Vec<u8>),
}

impl fmt::Debug for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Message::Handshake(m) => write!(
                f,
                "Handshake(type {}, {} bytes)",
                m.first().copied().unwrap_or(0),
                m.len()
            ),
            Message::ChangeCipherSpec(c) => write!(f, "ChangeCipherSpec({c:?})"),
            Message::Alert { level, description } => {
                write!(f, "Alert {{ level: {level}, description: {description} }}")
            }
            Message::ApplicationData(d) => write!(f, "ApplicationData({} bytes)", d.len()),
        }
    }
}

/// The protection of one direction: the AEAD key and IV of a traffic secret, the secret for
/// its key update, and the sequence number.
struct Protection {
    suite: CipherSuite,
    secret: Secret,
    key: AeadKey,
    iv: [u8; AeadAlgorithm::NONCE_LEN],
    sequence: u64,
}

impl Protection {
    fn new(suite: CipherSuite, secret: &Secret) -> Result<Protection, Error> {
        let keys = key_schedule::traffic_key(suite, secret).map_err(|_| primitives())?;
        let key = AeadKey::new(suite.aead(), keys.key()).map_err(|_| primitives())?;
        Ok(Protection {
            suite,
            secret: secret.clone(),
            key,
            iv: *keys.iv(),
            sequence: 0,
        })
    }

    /// The nonce of the current sequence number: the IV XOR the number, left-padded.
    fn nonce(&self) -> [u8; AeadAlgorithm::NONCE_LEN] {
        let mut nonce = self.iv;
        let number = self.sequence.to_be_bytes();
        let start = nonce.len() - number.len();
        for (n, s) in nonce[start..].iter_mut().zip(number) {
            *n ^= s;
        }
        nonce
    }

    /// The numbers left before the number would wrap; the last, 2^64 - 1, is never used.
    fn left(&self) -> u64 {
        u64::MAX - self.sequence
    }

    fn next(&self) -> Result<Protection, Error> {
        let secret = key_schedule::next_traffic_secret(&self.secret).map_err(|_| primitives())?;
        Protection::new(self.suite, &secret)
    }
}

impl Drop for Protection {
    fn drop(&mut self) {
        self.iv.fill(0);
    }
}

impl fmt::Debug for Protection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Protection({:?}, sequence {})",
            self.suite, self.sequence
        )
    }
}

/// A failure of the primitives that the lengths used here rule out (a key of the wrong
/// length, a nonce of another size).
fn primitives() -> Error {
    Error::Io(ErrorKind::InvalidInput)
}

fn io(error: std::io::Error) -> Error {
    match error.kind() {
        ErrorKind::UnexpectedEof => Error::ConnectionClosed,
        kind => Error::Io(kind),
    }
}

/// Why a read failed: a violation, for which an alert is written, or another error.
enum Failure {
    Alert(AlertDescription, Violation),
    Other(Error),
}

impl From<Error> for Failure {
    fn from(error: Error) -> Self {
        Failure::Other(error)
    }
}

fn violation(description: AlertDescription, violation: Violation) -> Failure {
    Failure::Alert(description, violation)
}

/// The records of one connection over `stream`, read and written.
pub struct RecordLayer<S> {
    stream: S,
    read: Option<Protection>,
    write: Option<Protection>,
    /// Handshake bytes received and not yet handed up.
    handshake: Vec<u8>,
    failed: Option<Error>,
}

impl<S> RecordLayer<S> {
    /// A layer over `stream`, without keys.
    pub fn new(stream: S) -> Self {
        RecordLayer {
            stream,
            read: None,
            write: None,
            handshake: Vec::new(),
            failed: None,
        }
    }

    /// The stream.
    pub fn get_ref(&self) -> &S {
        &self.stream
    }

    /// The stream, to change.
    pub fn get_mut(&mut self) -> &mut S {
        &mut self.stream
    }

    /// The stream, the layer given up.
    pub fn into_inner(self) -> S {
        self.stream
    }

    /// The sequence number of the next record read, once read keys are set.
    pub fn read_sequence(&self) -> Option<u64> {
        self.read.as_ref().map(|p| p.sequence)
    }

    /// The sequence number of the next record written, once write keys are set.
    pub fn write_sequence(&self) -> Option<u64> {
        self.write.as_ref().map(|p| p.sequence)
    }

    /// Whether handshake bytes are received and not yet handed up: part of a message, or
    /// whole messages after the last one handed up.
    pub fn has_buffered_handshake(&self) -> bool {
        !self.handshake.is_empty()
    }

    /// Sets the sequence number of the next record read (test tooling only), to reach the
    /// end of the numbers without 2^64 records.
    #[cfg(feature = "test-tooling")]
    pub fn set_read_sequence(&mut self, sequence: u64) {
        if let Some(p) = self.read.as_mut() {
            p.sequence = sequence;
        }
    }

    /// Sets the sequence number of the next record written (test tooling only).
    #[cfg(feature = "test-tooling")]
    pub fn set_write_sequence(&mut self, sequence: u64) {
        if let Some(p) = self.write.as_mut() {
            p.sequence = sequence;
        }
    }

    fn check(&self) -> Result<(), Error> {
        match &self.failed {
            Some(error) => Err(error.clone()),
            None => Ok(()),
        }
    }
}

impl<S: Read + Write> RecordLayer<S> {
    /// Protects the records read from now on with `secret` under `suite`, from sequence
    /// number 0. Refused with `HandshakeSpansKeyChange` while handshake bytes are buffered.
    pub fn set_read_keys(&mut self, suite: CipherSuite, secret: &Secret) -> Result<(), Error> {
        self.check()?;
        self.before_key_change()?;
        let protection = Protection::new(suite, secret).map_err(|e| self.fail(e))?;
        self.read = Some(protection);
        Ok(())
    }

    /// Protects the records written from now on with `secret` under `suite`, from sequence
    /// number 0.
    pub fn set_write_keys(&mut self, suite: CipherSuite, secret: &Secret) -> Result<(), Error> {
        self.check()?;
        let protection = Protection::new(suite, secret).map_err(|e| self.fail(e))?;
        self.write = Some(protection);
        Ok(())
    }

    /// After a `KeyUpdate` received: the records read from now on are protected with the next
    /// application traffic secret (RFC 8446 section 7.2), from sequence number 0.
    pub fn update_read_keys(&mut self) -> Result<(), Error> {
        self.check()?;
        self.before_key_change()?;
        let next = match &self.read {
            Some(p) => p.next(),
            None => {
                return Err(self.alert(
                    AlertDescription::UNEXPECTED_MESSAGE,
                    Violation::UnexpectedMessage {
                        expected: "a key to update",
                        got: 24,
                    },
                ));
            }
        };
        self.read = Some(next.map_err(|e| self.fail(e))?);
        Ok(())
    }

    /// After a `KeyUpdate` written: the records written from now on are protected with the
    /// next application traffic secret, from sequence number 0.
    pub fn update_write_keys(&mut self) -> Result<(), Error> {
        self.check()?;
        let next = match &self.write {
            Some(p) => p.next(),
            None => return Err(self.fail(primitives())),
        };
        self.write = Some(next.map_err(|e| self.fail(e))?);
        Ok(())
    }

    fn before_key_change(&mut self) -> Result<(), Error> {
        if self.handshake.is_empty() {
            Ok(())
        } else {
            Err(self.alert(
                AlertDescription::UNEXPECTED_MESSAGE,
                Violation::HandshakeSpansKeyChange,
            ))
        }
    }

    /// Ends the connection on a violation the caller found: writes the fatal alert
    /// `description`, if it can, and returns `AlertSent`; every later call returns it too.
    pub fn send_alert(&mut self, description: AlertDescription, violation: Violation) -> Error {
        if let Err(error) = self.check() {
            return error;
        }
        self.alert(description, violation)
    }

    fn alert(&mut self, description: AlertDescription, violation: Violation) -> Error {
        let _ = self.write_record(
            ContentType::Alert,
            &[AlertLevel::Fatal.code(), description.0],
        );
        let _ = self.stream.flush();
        self.fail(Error::AlertSent {
            description,
            violation,
        })
    }

    fn fail(&mut self, error: Error) -> Error {
        self.failed = Some(error.clone());
        error
    }

    /// The next message: a whole handshake message, an alert, the content of a
    /// `change_cipher_spec` or of an application data record.
    pub fn read_message(&mut self) -> Result<Message, Error> {
        self.check()?;
        match self.next_message() {
            Ok(message) => Ok(message),
            Err(Failure::Alert(description, v)) => Err(self.alert(description, v)),
            Err(Failure::Other(error)) => Err(self.fail(error)),
        }
    }

    fn next_message(&mut self) -> Result<Message, Failure> {
        loop {
            if let Some(message) = self.take_handshake_message()? {
                return Ok(Message::Handshake(message));
            }
            let (content_type, content) = self.read_record()?;
            match content_type {
                ContentType::Handshake => {
                    if content.is_empty() {
                        return Err(violation(
                            AlertDescription::UNEXPECTED_MESSAGE,
                            Violation::EmptyContent,
                        ));
                    }
                    self.handshake.extend_from_slice(&content);
                    continue;
                }
                _ if !self.handshake.is_empty() => {
                    return Err(violation(
                        AlertDescription::UNEXPECTED_MESSAGE,
                        Violation::InterleavedHandshake,
                    ));
                }
                ContentType::Alert => {
                    return match content.as_slice() {
                        [] => Err(violation(
                            AlertDescription::UNEXPECTED_MESSAGE,
                            Violation::EmptyContent,
                        )),
                        [level, description] => Ok(Message::Alert {
                            level: *level,
                            description: AlertDescription(*description),
                        }),
                        _ => Err(violation(
                            AlertDescription::DECODE_ERROR,
                            Violation::BadAlertRecord,
                        )),
                    };
                }
                ContentType::ChangeCipherSpec => return Ok(Message::ChangeCipherSpec(content)),
                ContentType::ApplicationData => return Ok(Message::ApplicationData(content)),
            }
        }
    }

    /// The first buffered handshake message, if it is whole; refused once its header
    /// declares more than the bound.
    fn take_handshake_message(&mut self) -> Result<Option<Vec<u8>>, Failure> {
        let Some(header) = self.handshake.get(..4) else {
            return Ok(None);
        };
        let declared =
            usize::from(header[1]) << 16 | usize::from(header[2]) << 8 | usize::from(header[3]);
        if declared > MAX_HANDSHAKE_MESSAGE {
            return Err(violation(
                AlertDescription::ILLEGAL_PARAMETER,
                Violation::MessageTooLarge {
                    declared,
                    bound: MAX_HANDSHAKE_MESSAGE,
                },
            ));
        }
        let len = 4 + declared;
        if self.handshake.len() < len {
            return Ok(None);
        }
        let rest = self.handshake.split_off(len);
        Ok(Some(std::mem::replace(&mut self.handshake, rest)))
    }

    /// One record: its content type and content, opened when it is protected.
    fn read_record(&mut self) -> Result<(ContentType, Vec<u8>), Failure> {
        let mut header = [0u8; HEADER_LEN];
        self.stream.read_exact(&mut header).map_err(io)?;
        let len = usize::from(u16::from_be_bytes([header[3], header[4]]));
        let Some(outer) = ContentType::from_code(header[0]) else {
            return Err(violation(
                AlertDescription::UNEXPECTED_MESSAGE,
                Violation::UnknownContentType,
            ));
        };
        let protected = self.read.is_some() && outer == ContentType::ApplicationData;
        if !protected {
            let unprotected = match outer {
                ContentType::ApplicationData => true,
                ContentType::Handshake | ContentType::Alert => self.read.is_some(),
                ContentType::ChangeCipherSpec => false,
            };
            if unprotected {
                return Err(violation(
                    AlertDescription::UNEXPECTED_MESSAGE,
                    Violation::UnprotectedRecord,
                ));
            }
            if len > MAX_PLAINTEXT {
                return Err(violation(
                    AlertDescription::RECORD_OVERFLOW,
                    Violation::RecordOverflow,
                ));
            }
            let mut content = vec![0u8; len];
            self.stream.read_exact(&mut content).map_err(io)?;
            return Ok((outer, content));
        }
        if len > MAX_CIPHERTEXT {
            return Err(violation(
                AlertDescription::RECORD_OVERFLOW,
                Violation::RecordOverflow,
            ));
        }
        let mut body = vec![0u8; len];
        self.stream.read_exact(&mut body).map_err(io)?;
        let opened = self.open(&header, &mut body);
        if opened.is_err() {
            body.fill(0);
        }
        opened.map(|content_type| (content_type, body))
    }

    /// Opens `body` in place: on success it holds the content and the type is returned.
    fn open(
        &mut self,
        header: &[u8; HEADER_LEN],
        body: &mut Vec<u8>,
    ) -> Result<ContentType, Failure> {
        let Some(protection) = self.read.as_mut() else {
            return Err(Failure::Other(primitives()));
        };
        if protection.left() == 0 {
            return Err(Failure::Other(Error::SequenceExhausted));
        }
        let nonce = protection.nonce();
        let plaintext_len = match protection.key.open(&nonce, header, body) {
            Ok(plaintext) => plaintext.len(),
            Err(_) => {
                return Err(violation(
                    AlertDescription::BAD_RECORD_MAC,
                    Violation::BadRecordMac,
                ));
            }
        };
        protection.sequence += 1;
        body.truncate(plaintext_len);
        if body.len() > MAX_INNER_PLAINTEXT {
            return Err(violation(
                AlertDescription::RECORD_OVERFLOW,
                Violation::RecordOverflow,
            ));
        }
        let Some(at) = body.iter().rposition(|&b| b != 0) else {
            return Err(violation(
                AlertDescription::UNEXPECTED_MESSAGE,
                Violation::EmptyInnerPlaintext,
            ));
        };
        let inner = body[at];
        body.truncate(at);
        match ContentType::from_code(inner) {
            None => Err(violation(
                AlertDescription::UNEXPECTED_MESSAGE,
                Violation::UnknownContentType,
            )),
            Some(ContentType::ChangeCipherSpec) => Err(violation(
                AlertDescription::UNEXPECTED_MESSAGE,
                Violation::UnexpectedChangeCipherSpec,
            )),
            Some(content_type) => Ok(content_type),
        }
    }

    /// Writes handshake messages, in records of at most 2^14 bytes, none empty; protected
    /// once write keys are set.
    pub fn write_handshake(&mut self, messages: &[u8]) -> Result<(), Error> {
        self.write_fragments(ContentType::Handshake, messages)
    }

    /// Writes application data, in protected records of at most 2^14 bytes. Refused with
    /// `UnprotectedRecord`, writing nothing but an `internal_error` alert, before write keys
    /// are set.
    pub fn write_application_data(&mut self, data: &[u8]) -> Result<(), Error> {
        self.check()?;
        if self.write.is_none() {
            return Err(self.alert(
                AlertDescription::INTERNAL_ERROR,
                Violation::UnprotectedRecord,
            ));
        }
        self.write_fragments(ContentType::ApplicationData, data)
    }

    /// Writes one alert.
    pub fn write_alert(
        &mut self,
        level: AlertLevel,
        description: AlertDescription,
    ) -> Result<(), Error> {
        self.check()?;
        self.write_record(ContentType::Alert, &[level.code(), description.0])
            .and_then(|()| self.stream.flush().map_err(io))
            .map_err(|e| self.fail(e))
    }

    /// Writes the `change_cipher_spec` record of compatibility mode, never protected:
    /// `14 03 03 00 01 01`.
    pub fn write_change_cipher_spec(&mut self) -> Result<(), Error> {
        self.check()?;
        let mut record = [ContentType::ChangeCipherSpec.code(), 0, 0, 0, 1, 1];
        record[1..3].copy_from_slice(&RECORD_VERSION);
        self.stream
            .write_all(&record)
            .and_then(|()| self.stream.flush())
            .map_err(io)
            .map_err(|e| self.fail(e))
    }

    fn write_fragments(&mut self, content_type: ContentType, data: &[u8]) -> Result<(), Error> {
        self.check()?;
        let records = data.len().div_ceil(MAX_PLAINTEXT) as u64;
        if let Some(p) = &self.write
            && p.left() < records
        {
            return Err(self.fail(Error::SequenceExhausted));
        }
        for fragment in data.chunks(MAX_PLAINTEXT) {
            self.write_record(content_type, fragment)
                .map_err(|e| self.fail(e))?;
        }
        self.stream.flush().map_err(io).map_err(|e| self.fail(e))
    }

    /// Writes one record of at most 2^14 bytes of `content`.
    fn write_record(&mut self, content_type: ContentType, content: &[u8]) -> Result<(), Error> {
        let mut record = Vec::with_capacity(HEADER_LEN + content.len() + 1 + TAG_LEN);
        match self.write.as_mut() {
            None => {
                let len = u16::try_from(content.len()).map_err(|_| primitives())?;
                record.push(content_type.code());
                record.extend_from_slice(&RECORD_VERSION);
                record.extend_from_slice(&len.to_be_bytes());
                record.extend_from_slice(content);
            }
            Some(protection) => {
                if protection.left() == 0 {
                    return Err(Error::SequenceExhausted);
                }
                let len = u16::try_from(content.len() + 1 + TAG_LEN).map_err(|_| primitives())?;
                let mut header = [ContentType::ApplicationData.code(), 0, 0, 0, 0];
                header[1..3].copy_from_slice(&RECORD_VERSION);
                header[3..].copy_from_slice(&len.to_be_bytes());
                let mut inner = Vec::with_capacity(content.len() + 1 + TAG_LEN);
                inner.extend_from_slice(content);
                inner.push(content_type.code());
                let nonce = protection.nonce();
                protection
                    .key
                    .seal(&nonce, &header, &mut inner)
                    .map_err(|_| primitives())?;
                protection.sequence += 1;
                record.extend_from_slice(&header);
                record.extend_from_slice(&inner);
            }
        }
        self.stream.write_all(&record).map_err(io)
    }
}

impl<S> fmt::Debug for RecordLayer<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RecordLayer")
            .field("read", &self.read)
            .field("write", &self.write)
            .field("buffered_handshake", &self.handshake.len())
            .field("failed", &self.failed)
            .finish()
    }
}
