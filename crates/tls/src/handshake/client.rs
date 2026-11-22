//! The client's handshake of RFC 8446 section 4, from its first ClientHello to its Finished,
//! over a [`RecordLayer`]. The order of the server's messages is the order of this code: each
//! step asks for the one message that may come next and refuses any other with
//! `unexpected_message`, `UnexpectedMessage { expected, got }`.

use std::io::{Read, Write};

use crate::crypto::{self, EphemeralKey, Random, SignatureError, SignatureVerifier};
use crate::der::Time;
use crate::error::{AlertDescription, Error, Violation};
use crate::key_schedule::{self, CipherSuite, Secret};
use crate::record::{AlertLevel, Message, RecordLayer};
use crate::transcript::Transcript;
use crate::verify::{CertificateRejection, Check, LeafKey, ServerCertVerifier};

use super::messages::{
    self, Abort, HelloParts, Offered, Place, ServerHello, TLS12, TLS13, ext, kind,
};

/// Why the handshake ended, before the alert is written.
#[derive(Debug)]
pub(crate) enum Fail {
    /// A violation: the alert is sent and `AlertSent` returned.
    Abort(Abort),
    /// The verifier rejected the certificates: its alert is sent, `Certificate` returned.
    Certificate(CertificateRejection),
    /// The server selected TLS 1.2 or below (kind (c) of item 7): `protocol_version` is sent,
    /// `VersionUnsupported` returned.
    Version(String),
    /// Another end, already handled by the record layer or without an alert.
    Other(Error),
}

impl From<Abort> for Fail {
    fn from(abort: Abort) -> Self {
        Fail::Abort(abort)
    }
}

impl From<Error> for Fail {
    fn from(error: Error) -> Self {
        Fail::Other(error)
    }
}

fn unexpected(expected: &'static str, got: u8) -> Fail {
    Fail::Abort(Abort::new(
        AlertDescription::UNEXPECTED_MESSAGE,
        Violation::UnexpectedMessage { expected, got },
    ))
}

/// The most `change_cipher_spec` records the client drops between its first ClientHello and
/// the server's Finished: one, the one a server in compatibility mode sends (RFC 8446 section 5
/// and appendix D.4; `story:tls-handshake`).
pub(crate) const MAX_CHANGE_CIPHER_SPECS: usize = 1;

/// Which ClientHellos the client sends.
pub(crate) enum Hellos<'a> {
    /// Its own, built from the random source.
    Own,
    /// The given ones, in order (the test mode of `trace.rs`): the first, and the second after
    /// a HelloRetryRequest.
    #[cfg_attr(not(feature = "test-tooling"), allow(dead_code))]
    Given(&'a [Vec<u8>]),
}

/// How the CertificateVerify signature is checked.
pub(crate) enum SignatureCheck<'a> {
    /// Through the seam: `REFUSED`, then the scheme's verification.
    Seam,
    /// By the given verifier alone, as a trace answers it (`trace.rs`).
    #[cfg_attr(not(feature = "test-tooling"), allow(dead_code))]
    Given(&'a dyn SignatureVerifier),
}

/// What the handshake is given.
pub(crate) struct Settings<'a> {
    /// The host the connection is for.
    pub host: &'a str,
    /// Validates the server's certificates.
    pub verifier: &'a dyn ServerCertVerifier,
    /// The time validity is checked at.
    pub now: Time,
    /// The source of the random, the session id and the key shares.
    pub random: &'a Random<'a>,
    /// Own or given hellos.
    pub hellos: Hellos<'a>,
    /// How CertificateVerify is checked.
    pub signature_check: SignatureCheck<'a>,
}

/// A finished handshake: the record layer with the application keys set.
pub(crate) struct Established<S> {
    /// The layer.
    pub layer: RecordLayer<S>,
    /// The suite.
    pub suite: CipherSuite,
}

struct Client<'a, S> {
    settings: &'a Settings<'a>,
    layer: RecordLayer<S>,
    transcript: Transcript,
    /// What the hello now in force offered.
    offered: Offered,
    /// The key of each share of that hello, in its order.
    keys: Vec<EphemeralKey>,
    /// A ServerHello or HelloRetryRequest has come.
    server_hello_seen: bool,
    /// `change_cipher_spec` records dropped so far.
    change_cipher_specs: usize,
}

/// Runs the handshake over `layer` (no keys set yet). On failure the alert of the failure is
/// written, if one is due, and the error returned.
pub(crate) fn handshake<S: Read + Write>(
    layer: RecordLayer<S>,
    settings: &Settings<'_>,
) -> Result<Established<S>, Error> {
    let mut client = Client {
        settings,
        layer,
        transcript: Transcript::new(),
        offered: Offered::default(),
        keys: Vec::new(),
        server_hello_seen: false,
        change_cipher_specs: 0,
    };
    match client.run() {
        Ok(suite) => Ok(Established {
            layer: client.layer,
            suite,
        }),
        Err(fail) => Err(end(&mut client.layer, fail)),
    }
}

/// Writes the alert of `fail`, if one is due, and returns its error.
pub(crate) fn end<S: Read + Write>(layer: &mut RecordLayer<S>, fail: Fail) -> Error {
    match fail {
        Fail::Abort(abort) => layer.send_alert(abort.alert, abort.violation),
        Fail::Certificate(rejection) => {
            let _ = layer.write_alert(AlertLevel::Fatal, rejection.alert());
            Error::Certificate(rejection)
        }
        Fail::Version(detail) => {
            let _ = layer.write_alert(AlertLevel::Fatal, AlertDescription::PROTOCOL_VERSION);
            Error::VersionUnsupported { detail }
        }
        Fail::Other(error) => error,
    }
}

fn random_bytes(random: &Random<'_>) -> Result<[u8; 32], Fail> {
    let mut out = [0u8; 32];
    random
        .fill(&mut out)
        .map_err(|_| Fail::Other(Error::Io(std::io::ErrorKind::Other)))?;
    Ok(out)
}

fn key(group: crypto::Group, random: &Random<'_>) -> Result<EphemeralKey, Fail> {
    EphemeralKey::generate(group, random)
        .map_err(|_| Fail::Other(Error::Io(std::io::ErrorKind::Other)))
}

fn invalid_input() -> Fail {
    Fail::Other(Error::Io(std::io::ErrorKind::InvalidInput))
}

/// The first hello, and for an own hello its random and session id.
type FirstHello = (Vec<u8>, Option<([u8; 32], [u8; 32])>);

/// What a HelloRetryRequest asks for.
struct Retry {
    suite: CipherSuite,
    group: Option<u16>,
    cookie: Option<Vec<u8>>,
}

impl<S: Read + Write> Client<'_, S> {
    fn run(&mut self) -> Result<CipherSuite, Fail> {
        let (first, own) = self.first_hello()?;
        self.send_hello(&first)?;

        let mut retry: Option<Retry> = None;
        let (hello, suite) = loop {
            let message = self.next("ServerHello")?;
            if message[0] != kind::SERVER_HELLO {
                return Err(unexpected("ServerHello", message[0]));
            }
            let is_retry = message.get(6..38) == Some(&messages::hello_retry_request_random()[..]);
            if is_retry && retry.is_some() {
                return Err(Abort::new(
                    AlertDescription::UNEXPECTED_MESSAGE,
                    Violation::SecondHelloRetryRequest,
                )
                .into());
            }
            self.server_hello_seen = true;
            if is_retry {
                let asked = self.hello_retry_request(&message)?;
                self.transcript
                    .hello_retry_request(asked.suite.hash(), &message)
                    .map_err(|v| Abort::new(AlertDescription::UNEXPECTED_MESSAGE, v))?;
                let second = self.second_hello(&own, &asked)?;
                self.send_hello(&second)?;
                retry = Some(asked);
                continue;
            }
            let suite = self.server_hello_checks(&message, retry.as_ref())?;
            break (message, suite);
        };
        let secrets = self.key_exchange(&hello, suite, retry.as_ref())?;
        let (master, handshake) = secrets;
        self.layer.set_read_keys(suite, &handshake.server)?;
        self.layer.set_write_keys(suite, &handshake.client)?;

        self.encrypted_extensions()?;
        let (leaf, certificate_requested) = self.certificate()?;
        self.certificate_verify(&leaf)?;
        self.server_finished(suite, &handshake.server)?;

        let hash = self.transcript_hash()?;
        let application = key_schedule::application_traffic_secrets(&master, hash.as_ref())
            .map_err(|_| invalid_input())?;
        self.layer.set_read_keys(suite, &application.server)?;

        // The change_cipher_spec, the empty Certificate and the Finished are one flight: one
        // write and one flush (`RecordLayer::cork`), so that no part of it waits for the
        // server's delayed ACK on a socket without TCP_NODELAY.
        self.layer.cork();
        if !self.offered.session_id.is_empty() {
            self.layer.write_change_cipher_spec()?;
        }
        let mut flight = Vec::new();
        if certificate_requested {
            let empty = super::codec::handshake_message(kind::CERTIFICATE, &[0, 0, 0, 0])
                .map_err(|_| invalid_input())?;
            self.transcript.add(&empty);
            flight.extend_from_slice(&empty);
        }
        let hash = self.transcript_hash()?;
        let verify_data = key_schedule::verify_data(&handshake.client, hash.as_ref())
            .map_err(|_| invalid_input())?;
        let finished = super::codec::handshake_message(kind::FINISHED, verify_data.as_ref())
            .map_err(|_| invalid_input())?;
        self.transcript.add(&finished);
        flight.extend_from_slice(&finished);
        self.layer.write_handshake(&flight)?;
        self.layer.uncork()?;
        self.layer.set_write_keys(suite, &application.client)?;
        Ok(suite)
    }

    /// The first hello: built, or the first given one; its keys made from the random source.
    /// Also the random and session id of an own hello, for the second.
    fn first_hello(&mut self) -> Result<FirstHello, Fail> {
        match self.settings.hellos {
            Hellos::Own => {
                let random = random_bytes(self.settings.random)?;
                let session_id = random_bytes(self.settings.random)?;
                let share = key(crypto::Group::X25519, self.settings.random)?;
                let name = messages::server_name(self.settings.host);
                if name.is_some_and(|n| n.len() > messages::MAX_SERVER_NAME) {
                    return Err(invalid_input());
                }
                let hello = messages::client_hello(&HelloParts {
                    random: &random,
                    session_id: &session_id,
                    server_name: name,
                    shares: &[(share.group(), share.public_key())],
                    cookie: None,
                })
                .map_err(|_| invalid_input())?;
                self.keys = vec![share];
                Ok((hello, Some((random, session_id))))
            }
            Hellos::Given(hellos) => {
                let first = hellos.first().ok_or_else(invalid_input)?.clone();
                Ok((first, None))
            }
        }
    }

    /// Writes `hello`, reads what it offered, and for a given hello makes the keys of its
    /// shares from the random source, checking each public key against the share.
    fn send_hello(&mut self, hello: &[u8]) -> Result<(), Fail> {
        self.offered = messages::offered(hello).map_err(|_| invalid_input())?;
        if let Hellos::Given(_) = self.settings.hellos {
            let mut keys = Vec::new();
            for code in &self.offered.shares {
                let group = messages::group_of(*code).ok_or_else(invalid_input)?;
                keys.push(key(group, self.settings.random)?);
            }
            for k in &keys {
                if !contains(hello, k.public_key()) {
                    return Err(invalid_input());
                }
            }
            self.keys = keys;
        }
        self.transcript.add(hello);
        self.layer.write_handshake(hello)?;
        Ok(())
    }

    /// The second hello: the first with one share of the group asked for (or the first's share
    /// when none was asked for) and the cookie; or the second given one.
    fn second_hello(
        &mut self,
        own: &Option<([u8; 32], [u8; 32])>,
        retry: &Retry,
    ) -> Result<Vec<u8>, Fail> {
        match (self.settings.hellos.given(), own) {
            (Some(hellos), _) => Ok(hellos.get(1).ok_or_else(invalid_input)?.clone()),
            (None, Some((random, session_id))) => {
                if let Some(code) = retry.group {
                    let group = messages::group_of(code).ok_or_else(invalid_input)?;
                    self.keys = vec![key(group, self.settings.random)?];
                }
                let shares: Vec<(crypto::Group, &[u8])> = self
                    .keys
                    .iter()
                    .map(|k| (k.group(), k.public_key()))
                    .collect();
                messages::client_hello(&HelloParts {
                    random,
                    session_id,
                    server_name: messages::server_name(self.settings.host),
                    shares: &shares,
                    cookie: retry.cookie.as_deref(),
                })
                .map_err(|_| invalid_input())
            }
            (None, None) => Err(invalid_input()),
        }
    }

    /// The next handshake message, `expected` naming it for a refusal. A `change_cipher_spec`
    /// of the one byte 0x01 is dropped, at most [`MAX_CHANGE_CIPHER_SPECS`]; an alert ends the
    /// handshake (`close_notify` as a closed connection, `user_canceled` read and ignored).
    fn next(&mut self, expected: &'static str) -> Result<Vec<u8>, Fail> {
        loop {
            match self.layer.read_message()? {
                Message::Handshake(message) => return Ok(message),
                Message::ChangeCipherSpec(content) => {
                    if content != [1] || self.change_cipher_specs >= MAX_CHANGE_CIPHER_SPECS {
                        return Err(Abort::new(
                            AlertDescription::UNEXPECTED_MESSAGE,
                            Violation::UnexpectedChangeCipherSpec,
                        )
                        .into());
                    }
                    self.change_cipher_specs += 1;
                }
                Message::Alert { description, .. } => match description {
                    AlertDescription::CLOSE_NOTIFY => {
                        return Err(Fail::Other(Error::ConnectionClosed));
                    }
                    AlertDescription::USER_CANCELED => {}
                    _ => {
                        return Err(Fail::Other(Error::AlertReceived {
                            description,
                            before_server_hello: !self.server_hello_seen,
                        }));
                    }
                },
                Message::ApplicationData(_) => return Err(unexpected(expected, 23)),
            }
        }
    }

    fn transcript_hash(&self) -> Result<crypto::Digest, Fail> {
        self.transcript.current().ok_or_else(invalid_input)
    }

    /// The version rules of a ServerHello or HelloRetryRequest (RFC 8446 sections 4.1.3,
    /// 4.1.4 and 4.2.1), in the story's order.
    fn version(&self, hello: &ServerHello<'_>, message: &'static str) -> Result<(), Fail> {
        match hello.extension(ext::SUPPORTED_VERSIONS) {
            Some(data) => {
                if messages::selected_version(data, message)? != TLS13 {
                    return Err(Abort::illegal(Violation::UnofferedVersion).into());
                }
                Ok(())
            }
            None if message == "HelloRetryRequest" => Err(Abort::new(
                AlertDescription::MISSING_EXTENSION,
                Violation::MissingExtension,
            )
            .into()),
            None if hello.legacy_version <= TLS12 => {
                let tail = &hello.random[24..];
                if tail == messages::DOWNGRADE_TLS12 || tail == messages::DOWNGRADE_TLS11 {
                    return Err(Abort::illegal(Violation::Downgrade).into());
                }
                Err(Fail::Version(format!(
                    "the server selected {}",
                    messages::version_name(hello.legacy_version)
                )))
            }
            None => Err(Abort::illegal(Violation::UnofferedVersion).into()),
        }
    }

    /// The suite, the session id echo and the compression method.
    fn common(&self, hello: &ServerHello<'_>) -> Result<CipherSuite, Fail> {
        let suite = CipherSuite::from_code(hello.suite)
            .filter(|_| self.offered.suites.contains(&hello.suite))
            .ok_or_else(|| Abort::illegal(Violation::UnofferedSuite))?;
        if hello.session_id != self.offered.session_id.as_slice() {
            return Err(Abort::illegal(Violation::SessionIdMismatch).into());
        }
        if hello.compression != 0 {
            return Err(Abort::illegal(Violation::CompressionNotNull).into());
        }
        Ok(suite)
    }

    /// The checks of a HelloRetryRequest (RFC 8446 section 4.1.4).
    fn hello_retry_request(&self, message: &[u8]) -> Result<Retry, Fail> {
        let name = "HelloRetryRequest";
        let hello = messages::server_hello(&message[4..], name)?;
        self.version(&hello, name)?;
        let suite = self.common(&hello)?;
        let offered = |t: u16| t == ext::COOKIE || self.offered.offers(t);
        messages::check_extensions(hello.extensions, Place::HelloRetryRequest, &offered)?;
        let group = match hello.extension(ext::KEY_SHARE) {
            Some(data) => {
                let group = messages::selected_group(data)?;
                if !self.offered.groups.contains(&group) || self.offered.shares.contains(&group) {
                    return Err(Abort::illegal(Violation::UnofferedGroup).into());
                }
                Some(group)
            }
            None => None,
        };
        let cookie = match hello.extension(ext::COOKIE) {
            Some(data) => {
                let cookie = messages::cookie(data)?;
                if cookie.len() > messages::MAX_COOKIE {
                    return Err(Abort::illegal(Violation::MessageTooLarge {
                        declared: cookie.len(),
                        bound: messages::MAX_COOKIE,
                    })
                    .into());
                }
                Some(cookie.to_vec())
            }
            None => None,
        };
        if group.is_none() && cookie.is_none() {
            return Err(Abort::illegal(Violation::HelloRetryRequestNoChange).into());
        }
        Ok(Retry {
            suite,
            group,
            cookie,
        })
    }

    /// The checks of a ServerHello; its suite.
    fn server_hello_checks(
        &self,
        message: &[u8],
        retry: Option<&Retry>,
    ) -> Result<CipherSuite, Fail> {
        let name = "ServerHello";
        let hello = messages::server_hello(&message[4..], name)?;
        self.version(&hello, name)?;
        let suite = self.common(&hello)?;
        if let Some(retry) = retry
            && retry.suite != suite
        {
            return Err(Abort::illegal(Violation::SuiteChangedAfterRetry).into());
        }
        messages::check_extensions(hello.extensions, Place::ServerHello, &|t| {
            self.offered.offers(t)
        })?;
        Ok(suite)
    }

    /// The key share of the ServerHello, the agreement and the handshake secrets; the
    /// ServerHello joins the transcript. The master secret and the handshake traffic secrets.
    fn key_exchange(
        &mut self,
        message: &[u8],
        suite: CipherSuite,
        retry: Option<&Retry>,
    ) -> Result<(Secret, key_schedule::TrafficSecrets), Fail> {
        let hello = messages::server_hello(&message[4..], "ServerHello")?;
        let data = hello.extension(ext::KEY_SHARE).ok_or_else(|| {
            Abort::new(
                AlertDescription::MISSING_EXTENSION,
                Violation::MissingExtension,
            )
        })?;
        let (group, public) = messages::server_share(data)?;
        match retry {
            Some(retry) => {
                let expected = retry.group.or_else(|| self.offered.shares.first().copied());
                if Some(group) != expected {
                    return Err(Abort::illegal(Violation::KeyShareGroupChanged).into());
                }
            }
            None => {
                if !self.offered.shares.contains(&group) {
                    return Err(Abort::illegal(Violation::UnofferedKeyShare).into());
                }
            }
        }
        let at = self
            .keys
            .iter()
            .position(|k| messages::group_code(k.group()) == group)
            .ok_or_else(|| Abort::illegal(Violation::UnofferedKeyShare))?;
        let own = self.keys.swap_remove(at);
        self.keys.clear();
        let shared = own
            .agree(public)
            .map_err(|_| Abort::illegal(Violation::BadKeyShare))?;
        if !self.transcript.select(suite.hash()) {
            return Err(Abort::illegal(Violation::SuiteChangedAfterRetry).into());
        }
        self.transcript.add(message);
        let early = key_schedule::early_secret(suite.hash(), None);
        let handshake =
            key_schedule::handshake_secret(&early, shared.as_ref()).map_err(|_| invalid_input())?;
        let hash = self.transcript_hash()?;
        let secrets = key_schedule::handshake_traffic_secrets(&handshake, hash.as_ref())
            .map_err(|_| invalid_input())?;
        let master = key_schedule::master_secret(&handshake).map_err(|_| invalid_input())?;
        Ok((master, secrets))
    }

    /// EncryptedExtensions: only what the hello offered, `server_name` empty, ALPN one offered
    /// protocol, `supported_groups` read and ignored (RFC 8446 section 4.2.7).
    fn encrypted_extensions(&mut self) -> Result<(), Fail> {
        let message = self.next("EncryptedExtensions")?;
        if message[0] != kind::ENCRYPTED_EXTENSIONS {
            return Err(unexpected("EncryptedExtensions", message[0]));
        }
        let name = "EncryptedExtensions";
        let mut reader = super::codec::Reader::new(&message[4..]);
        let block = reader
            .vec16(0, 0xffff)
            .map_err(|_| Abort::malformed(name))?;
        reader.finish().map_err(|_| Abort::malformed(name))?;
        let extensions = messages::extensions_of(block, name)?;
        messages::check_extensions(extensions, Place::EncryptedExtensions, &|t| {
            self.offered.offers(t)
        })?;
        for (extension_type, data) in extensions.iter() {
            match extension_type {
                ext::SERVER_NAME if !data.is_empty() => {
                    return Err(Abort::malformed(name).into());
                }
                ext::ALPN => {
                    let protocol = messages::alpn_protocol(data)?;
                    if !self.offered.alpn.iter().any(|p| p == protocol) {
                        return Err(Abort::illegal(Violation::UnofferedAlpn).into());
                    }
                }
                _ => {}
            }
        }
        self.transcript.add(&message);
        Ok(())
    }

    /// The Certificate (after a CertificateRequest, if one comes first), checked by the
    /// verifier: the leaf's key.
    fn certificate(&mut self) -> Result<(LeafKey, bool), Fail> {
        let mut requested = false;
        let mut message = self.next("Certificate")?;
        if message[0] == kind::CERTIFICATE_REQUEST {
            messages::certificate_request(&message[4..])?;
            self.transcript.add(&message);
            requested = true;
            message = self.next("Certificate")?;
        }
        if message[0] != kind::CERTIFICATE {
            return Err(unexpected("Certificate", message[0]));
        }
        let offered = &self.offered;
        let entry = |extensions: messages::Extensions<'_>| {
            messages::check_extensions(extensions, Place::Certificate, &|t| offered.offers(t))
        };
        let certificates = messages::certificate_list(&message[4..], &entry)?;
        let leaf = self
            .settings
            .verifier
            .verify(&certificates, self.settings.host, self.settings.now)
            .map_err(Fail::Certificate)?;
        self.transcript.add(&message);
        Ok((leaf, requested))
    }

    /// CertificateVerify: an offered scheme that fits the leaf's key, RSA only as PSS, and a
    /// signature over the content of section 4.4.3.
    fn certificate_verify(&mut self, leaf: &LeafKey) -> Result<(), Fail> {
        let message = self.next("CertificateVerify")?;
        if message[0] != kind::CERTIFICATE_VERIFY {
            return Err(unexpected("CertificateVerify", message[0]));
        }
        let (code, signature) = messages::certificate_verify(&message[4..])?;
        let unoffered = || Abort::illegal(Violation::UnofferedSignatureScheme);
        let scheme = crypto::Scheme::from_code(code)
            .filter(|_| self.offered.signature_algorithms.contains(&code))
            .filter(|s| {
                !matches!(
                    s,
                    crypto::Scheme::RsaPkcs1Sha256
                        | crypto::Scheme::RsaPkcs1Sha384
                        | crypto::Scheme::RsaPkcs1Sha512
                )
            })
            .filter(|s| crypto::scheme(s.algorithm(), leaf.kind) == Some(*s))
            .ok_or_else(unoffered)?;
        let hash = self.transcript_hash()?;
        let content = messages::certificate_verify_content(hash.as_ref());
        let checked = match self.settings.signature_check {
            SignatureCheck::Seam => crypto::check_signature(
                &crypto::ShippedVerifier,
                scheme.algorithm(),
                &leaf.public_key(),
                &content,
                signature,
            )
            .map(|_| ()),
            SignatureCheck::Given(verifier) => verifier
                .verify(scheme, &leaf.bytes, &content, signature)
                .map_err(|_| SignatureError::BadSignature),
        };
        match checked {
            Ok(()) => {}
            Err(SignatureError::Refused(refusal)) => {
                let check = match refusal {
                    crypto::Refusal::WeakAlgorithm { algorithm } => Check::WeakAlgorithm {
                        certificate: 0,
                        algorithm,
                    },
                    crypto::Refusal::WeakKey { bits } => Check::WeakKey {
                        certificate: 0,
                        bits,
                    },
                };
                return Err(Fail::Certificate(CertificateRejection::NotTrusted(check)));
            }
            Err(SignatureError::Unsupported) => return Err(unoffered().into()),
            Err(SignatureError::BadSignature) => {
                return Err(Abort::new(
                    AlertDescription::DECRYPT_ERROR,
                    Violation::BadCertificateVerify,
                )
                .into());
            }
        }
        self.transcript.add(&message);
        Ok(())
    }

    /// The server's Finished: `verify_data` of the hash's length, equal to the value of
    /// section 4.4.4, compared in constant time.
    fn server_finished(&mut self, suite: CipherSuite, server: &Secret) -> Result<(), Fail> {
        let message = self.next("Finished")?;
        if message[0] != kind::FINISHED {
            return Err(unexpected("Finished", message[0]));
        }
        let received = &message[4..];
        if received.len() != suite.hash().output_len() {
            return Err(Abort::malformed("Finished").into());
        }
        let hash = self.transcript_hash()?;
        let expected =
            key_schedule::verify_data(server, hash.as_ref()).map_err(|_| invalid_input())?;
        if !crypto::constant_time_eq(expected.as_ref(), received) {
            return Err(Abort::new(AlertDescription::DECRYPT_ERROR, Violation::BadFinished).into());
        }
        self.transcript.add(&message);
        Ok(())
    }
}

impl<'a> Hellos<'a> {
    fn given(&self) -> Option<&'a [Vec<u8>]> {
        match self {
            Hellos::Own => None,
            Hellos::Given(hellos) => Some(hellos),
        }
    }
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty() && haystack.windows(needle.len()).any(|w| w == needle)
}
