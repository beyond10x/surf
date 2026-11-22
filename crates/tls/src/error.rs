//! The public error of a TLS connection: a closed list (docs/design/tls.md, item 6). Each kind
//! maps to one failure reason of item 7; the alert a kind sends travels with it.
//!
//! `story:tls-x509` adds the variant `Certificate`, and no other story changes this file.

use std::fmt;
use std::io::ErrorKind;

/// An alert description of RFC 8446 section 6, as sent or received. A received alert may carry
/// a value RFC 8446 does not name; [`AlertDescription::name`] says `unknown` for it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AlertDescription(pub u8);

impl AlertDescription {
    /// `close_notify` (0).
    pub const CLOSE_NOTIFY: Self = Self(0);
    /// `unexpected_message` (10).
    pub const UNEXPECTED_MESSAGE: Self = Self(10);
    /// `bad_record_mac` (20).
    pub const BAD_RECORD_MAC: Self = Self(20);
    /// `record_overflow` (22).
    pub const RECORD_OVERFLOW: Self = Self(22);
    /// `handshake_failure` (40).
    pub const HANDSHAKE_FAILURE: Self = Self(40);
    /// `bad_certificate` (42).
    pub const BAD_CERTIFICATE: Self = Self(42);
    /// `unsupported_certificate` (43).
    pub const UNSUPPORTED_CERTIFICATE: Self = Self(43);
    /// `certificate_revoked` (44).
    pub const CERTIFICATE_REVOKED: Self = Self(44);
    /// `certificate_expired` (45).
    pub const CERTIFICATE_EXPIRED: Self = Self(45);
    /// `certificate_unknown` (46).
    pub const CERTIFICATE_UNKNOWN: Self = Self(46);
    /// `illegal_parameter` (47).
    pub const ILLEGAL_PARAMETER: Self = Self(47);
    /// `unknown_ca` (48).
    pub const UNKNOWN_CA: Self = Self(48);
    /// `access_denied` (49).
    pub const ACCESS_DENIED: Self = Self(49);
    /// `decode_error` (50).
    pub const DECODE_ERROR: Self = Self(50);
    /// `decrypt_error` (51).
    pub const DECRYPT_ERROR: Self = Self(51);
    /// `protocol_version` (70).
    pub const PROTOCOL_VERSION: Self = Self(70);
    /// `insufficient_security` (71).
    pub const INSUFFICIENT_SECURITY: Self = Self(71);
    /// `internal_error` (80).
    pub const INTERNAL_ERROR: Self = Self(80);
    /// `inappropriate_fallback` (86).
    pub const INAPPROPRIATE_FALLBACK: Self = Self(86);
    /// `user_canceled` (90).
    pub const USER_CANCELED: Self = Self(90);
    /// `missing_extension` (109).
    pub const MISSING_EXTENSION: Self = Self(109);
    /// `unsupported_extension` (110).
    pub const UNSUPPORTED_EXTENSION: Self = Self(110);
    /// `unrecognized_name` (112).
    pub const UNRECOGNIZED_NAME: Self = Self(112);
    /// `bad_certificate_status_response` (113).
    pub const BAD_CERTIFICATE_STATUS_RESPONSE: Self = Self(113);
    /// `unknown_psk_identity` (115).
    pub const UNKNOWN_PSK_IDENTITY: Self = Self(115);
    /// `certificate_required` (116).
    pub const CERTIFICATE_REQUIRED: Self = Self(116);
    /// `no_application_protocol` (120).
    pub const NO_APPLICATION_PROTOCOL: Self = Self(120);

    /// The name RFC 8446 section 6 gives the value, or `unknown`.
    pub fn name(self) -> &'static str {
        match self.0 {
            0 => "close_notify",
            10 => "unexpected_message",
            20 => "bad_record_mac",
            22 => "record_overflow",
            40 => "handshake_failure",
            42 => "bad_certificate",
            43 => "unsupported_certificate",
            44 => "certificate_revoked",
            45 => "certificate_expired",
            46 => "certificate_unknown",
            47 => "illegal_parameter",
            48 => "unknown_ca",
            49 => "access_denied",
            50 => "decode_error",
            51 => "decrypt_error",
            70 => "protocol_version",
            71 => "insufficient_security",
            80 => "internal_error",
            86 => "inappropriate_fallback",
            90 => "user_canceled",
            109 => "missing_extension",
            110 => "unsupported_extension",
            112 => "unrecognized_name",
            113 => "bad_certificate_status_response",
            115 => "unknown_psk_identity",
            116 => "certificate_required",
            120 => "no_application_protocol",
            _ => "unknown",
        }
    }
}

impl fmt::Display for AlertDescription {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.name() {
            "unknown" => write!(f, "unknown alert {}", self.0),
            name => f.write_str(name),
        }
    }
}

/// Why surf sent an alert: the protocol violation it found (item 6).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Violation {
    /// A record longer than RFC 8446 section 5 allows.
    RecordOverflow,
    /// A protected record that does not open.
    BadRecordMac,
    /// An inner content type RFC 8446 does not define.
    UnknownContentType,
    /// An inner plaintext of padding only, with no content type.
    EmptyInnerPlaintext,
    /// A handshake message that declares more bytes than item 9 allows.
    MessageTooLarge {
        /// The length the message's header declares.
        declared: usize,
        /// The most surf takes.
        bound: usize,
    },
    /// A handshake message where another was expected.
    UnexpectedMessage {
        /// What the handshake expected there.
        expected: &'static str,
        /// The handshake type that came.
        got: u8,
    },
    /// A Certificate message with no certificate.
    EmptyCertificate,
    /// A Certificate message with more certificates than item 9 allows.
    TooManyCertificates,
    /// A suite surf did not offer, a TLS 1.2 suite among them.
    UnofferedSuite,
    /// A `supported_versions` that holds anything but 0x0304.
    UnofferedVersion,
    /// An extension surf did not offer.
    UnofferedExtension,
    /// A group surf did not offer.
    UnofferedGroup,
    /// An ALPN protocol surf did not offer.
    UnofferedAlpn,
    /// A second HelloRetryRequest.
    SecondHelloRetryRequest,
    /// A ServerHello after a HelloRetryRequest that selects another suite.
    SuiteChangedAfterRetry,
    /// A HelloRetryRequest that would not change the hello.
    HelloRetryRequestNoChange,
    /// An extension a message must carry is missing.
    MissingExtension,
    /// A session id echo other than the one sent.
    SessionIdMismatch,
    /// A compression method other than null.
    CompressionNotNull,
    /// A ServerHello share in a group the client sent no share for.
    UnofferedKeyShare,
    /// After a HelloRetryRequest, a ServerHello share in another group than it selected.
    KeyShareGroupChanged,
    /// A share the agreement refuses.
    BadKeyShare,
    /// An extension in a message the table of RFC 8446 section 4.2 does not allow it in.
    ExtensionNotAllowedHere,
    /// Two extensions of one type in one block.
    RepeatedExtension,
    /// A `certificate_request_context` that is not empty in a message of the handshake.
    NonEmptyRequestContext,
    /// A CertificateVerify scheme not offered, not fitting the key, RSA PKCS#1 or SHA-1.
    UnofferedSignatureScheme,
    /// A `request_update` other than 0 or 1.
    BadKeyUpdate,
    /// A message that does not parse (alert `decode_error`).
    Malformed {
        /// The message that did not parse.
        message: &'static str,
    },
    /// A handshake message interleaved with records of another type.
    InterleavedHandshake,
    /// A handshake message that spans a change of keys.
    HandshakeSpansKeyChange,
    /// A handshake or alert record with no content.
    EmptyContent,
    /// An alert record that is not one alert of two bytes.
    BadAlertRecord,
    /// Application data without protection.
    UnprotectedRecord,
    /// A downgrade value of RFC 8446 section 4.1.3 in the ServerHello random.
    Downgrade,
    /// A `change_cipher_spec` record where none may come.
    UnexpectedChangeCipherSpec,
    /// A CertificateVerify whose signature does not verify.
    BadCertificateVerify,
    /// A Finished whose `verify_data` is not the expected one.
    BadFinished,
}

/// Why a TLS connection ended (item 6).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    /// The server did not finish the handshake or the response in time.
    TimedOut,
    /// The connection closed before the handshake ended.
    ConnectionClosed,
    /// The connection closed after the handshake without `close_notify`.
    Truncated,
    /// An error of the socket.
    Io(ErrorKind),
    /// The server selected a version below TLS 1.3.
    VersionUnsupported {
        /// What the server selected, in words.
        detail: String,
    },
    /// The server sent an alert other than `close_notify` and `user_canceled`.
    AlertReceived {
        /// The alert.
        description: AlertDescription,
        /// Whether it came before any ServerHello.
        before_server_hello: bool,
    },
    /// surf found a violation and sent an alert.
    AlertSent {
        /// The alert surf sent.
        description: AlertDescription,
        /// What surf found.
        violation: Violation,
    },
    /// A sequence number would wrap.
    SequenceExhausted,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::TimedOut => f.write_str("timed out"),
            Error::ConnectionClosed => f.write_str("the connection closed during the handshake"),
            Error::Truncated => f.write_str("the connection closed without close_notify"),
            Error::Io(kind) => write!(f, "{kind}"),
            Error::VersionUnsupported { detail } => f.write_str(detail),
            Error::AlertReceived { description, .. } => {
                write!(f, "the server sent {description}")
            }
            Error::AlertSent {
                description,
                violation,
            } => write!(f, "{description}: {violation:?}"),
            Error::SequenceExhausted => f.write_str("the sequence number would wrap"),
        }
    }
}

impl std::error::Error for Error {}
