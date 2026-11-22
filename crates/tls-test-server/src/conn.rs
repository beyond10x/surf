//! One connection of the server: the server side of RFC 8446 section 4 as far as the tests
//! need it, over the record layer, key schedule and transcript of `surf-tls`, then one HTTP/1.1
//! response; or the misbehaviour of the configured [`Mode`].

use std::io::{self, ErrorKind, Read, Write};
use std::net::{Shutdown, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use surf_tls::crypto::{EphemeralKey, Group, Random, SigningKey};
use surf_tls::error::{AlertDescription, Error};
use surf_tls::key_schedule::{self, CipherSuite};
use surf_tls::record::{AlertLevel, Message, RecordLayer};
use surf_tls::transcript::Transcript;

use crate::wire::{self, ClientHello};
use crate::{IDLE, Mode, Received, Response, Seen, Setup};

/// How often a blocked read looks at the stop flag and the idle bound.
const POLL: Duration = Duration::from_millis(20);

/// The longest request head the server reads.
const MAX_REQUEST: usize = 64 * 1024;

/// The socket of one connection: a read gives up when the server stops or after [`IDLE`]
/// without a byte from the client.
pub(crate) struct Watched<'a> {
    stream: TcpStream,
    stop: &'a AtomicBool,
    last: Instant,
}

impl Read for Watched<'_> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        loop {
            if self.stop.load(Ordering::SeqCst) {
                return Err(io::Error::new(ErrorKind::TimedOut, "the server stops"));
            }
            match self.stream.read(buf) {
                Ok(n) => {
                    self.last = Instant::now();
                    return Ok(n);
                }
                Err(e) if matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {
                    if self.last.elapsed() >= IDLE {
                        return Err(io::Error::new(ErrorKind::TimedOut, "the client is idle"));
                    }
                }
                Err(e) if e.kind() == ErrorKind::Interrupted => {}
                Err(e) => return Err(e),
            }
        }
    }
}

impl Write for Watched<'_> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.stream.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.stream.flush()
    }
}

/// Why the server ended a connection, in words.
type End = String;

fn ended(error: Error) -> End {
    format!("{error}")
}

fn failed<E: std::fmt::Debug>(what: &'static str) -> impl Fn(E) -> End {
    move |e| format!("{what}: {e:?}")
}

/// Serves one accepted connection to its end and returns what it saw.
pub(crate) fn serve(stream: TcpStream, setup: &Setup, stop: &AtomicBool) -> Seen {
    let mut seen = Seen::default();
    let ready = stream
        .set_read_timeout(Some(POLL))
        .and_then(|()| stream.set_write_timeout(Some(IDLE)))
        .and_then(|()| stream.set_nodelay(true));
    if let Err(e) = ready {
        seen.end = format!("socket: {e}");
        return seen;
    }
    let watched = Watched {
        stream,
        stop,
        last: Instant::now(),
    };
    let mut server = Server {
        layer: RecordLayer::new(watched),
        setup,
        seen,
        finished: false,
        random: Random::system(),
    };
    let end = match server.run() {
        Ok(()) => "done".to_string(),
        Err(end) => end,
    };
    let _ = server.layer.get_ref().stream.shutdown(Shutdown::Both);
    server.seen.end = end;
    server.seen
}

struct Server<'a> {
    layer: RecordLayer<Watched<'a>>,
    setup: &'a Setup,
    seen: Seen,
    /// The client's Finished was read: every later record is noted in `after_handshake`.
    finished: bool,
    random: Random<'static>,
}

impl Server<'_> {
    fn mode(&self) -> Mode {
        self.setup.config.mode
    }

    /// The next message, noted: an alert in `alerts`, and after the handshake every message in
    /// `after_handshake`.
    fn next(&mut self) -> Result<Message, End> {
        let message = self.layer.read_message().map_err(ended)?;
        if let Message::Alert { description, .. } = &message {
            self.seen.alerts.push(*description);
        }
        if self.finished {
            self.seen.after_handshake.push(match &message {
                Message::Handshake(m) => Received::Handshake(m[0]),
                Message::ChangeCipherSpec(_) => Received::ChangeCipherSpec,
                Message::Alert { description, .. } => Received::Alert(*description),
                Message::ApplicationData(d) => Received::ApplicationData(d.len()),
            });
        }
        Ok(message)
    }

    /// The next handshake message; a `change_cipher_spec` of compatibility mode is dropped,
    /// an alert ends the connection.
    fn handshake_message(&mut self) -> Result<Vec<u8>, End> {
        loop {
            match self.next()? {
                Message::Handshake(m) => return Ok(m),
                Message::ChangeCipherSpec(_) => {}
                Message::Alert { description, .. } => {
                    return Err(format!("the client sent {description}"));
                }
                Message::ApplicationData(_) => {
                    return Err(self.abort(AlertDescription::UNEXPECTED_MESSAGE));
                }
            }
        }
    }

    /// Sends a fatal alert and ends.
    fn abort(&mut self, description: AlertDescription) -> End {
        let _ = self.layer.write_alert(AlertLevel::Fatal, description);
        format!("the server sent {description}")
    }

    /// Reads, noting every alert, until the client closes, goes idle or the server stops; then
    /// ends with `why`.
    fn drain(&mut self, why: &str) -> Result<(), End> {
        loop {
            match self.next() {
                Ok(Message::Alert { description, .. })
                    if description != AlertDescription::USER_CANCELED =>
                {
                    return Err(format!("{why}; the client sent {description}"));
                }
                Ok(_) => {}
                Err(end) => return Err(format!("{why}; {end}")),
            }
        }
    }

    fn random_bytes(&self) -> Result<[u8; 32], End> {
        let mut out = [0u8; 32];
        self.random.fill(&mut out).map_err(failed("random"))?;
        Ok(out)
    }

    fn run(&mut self) -> Result<(), End> {
        let first = self.handshake_message()?;
        let hello = match wire::client_hello(&first) {
            Ok(hello) => hello,
            Err(_) => return Err(self.abort(AlertDescription::DECODE_ERROR)),
        };
        self.seen.server_name = hello.server_name.clone();
        self.seen.alpn = hello.alpn.clone();
        match self.mode() {
            Mode::NeverAnswer => return self.drain("never answers"),
            Mode::Tls12ReadsSupportedVersions => {
                if !hello.versions.is_empty() && !hello.versions.contains(&wire::TLS12) {
                    self.abort(AlertDescription::PROTOCOL_VERSION);
                    return self.drain("TLS 1.2 only, protocol_version sent");
                }
                return self.tls12(&hello);
            }
            Mode::Tls12IgnoresSupportedVersions => return self.tls12(&hello),
            Mode::SelectsTls12 => {
                self.tls12_server_hello(&hello, wire::TLS12_ECDHE_RSA_AES128_GCM)?;
                return self.drain("selected TLS 1.2");
            }
            _ => {}
        }
        self.tls13(first, hello)
    }

    /// A server limited to TLS 1.2 that chose by `legacy_version`: `protocol_version` for a
    /// hello below TLS 1.2; a TLS 1.2 ServerHello for a TLS 1.2 suite of the hello; else
    /// `handshake_failure`, both alerts before any ServerHello.
    fn tls12(&mut self, hello: &ClientHello) -> Result<(), End> {
        if hello.legacy_version < wire::TLS12 {
            self.abort(AlertDescription::PROTOCOL_VERSION);
            return self.drain("TLS 1.2 only, the hello is older");
        }
        let suite = hello
            .suites
            .iter()
            .copied()
            .find(|s| wire::TLS12_SUITES.contains(s));
        match suite {
            Some(suite) => {
                self.tls12_server_hello(hello, suite)?;
                self.drain("selected TLS 1.2")
            }
            None => {
                self.abort(AlertDescription::HANDSHAKE_FAILURE);
                self.drain("TLS 1.2 only, no suite in common")
            }
        }
    }

    /// A ServerHello of TLS 1.2 selecting `suite`, without `supported_versions` and without a
    /// downgrade value in its random (its last 8 bytes never start with `D`).
    fn tls12_server_hello(&mut self, hello: &ClientHello, suite: u16) -> Result<(), End> {
        let mut random = self.random_bytes()?;
        random[24] = 0;
        let message = wire::server_hello(&random, &hello.session_id, suite, None);
        self.layer.write_handshake(&message).map_err(ended)?;
        self.seen.server_hello_sent = true;
        Ok(())
    }

    fn tls13(&mut self, first: Vec<u8>, hello: ClientHello) -> Result<(), End> {
        let setup = self.setup;
        let config = &setup.config;
        if !hello.versions.contains(&wire::TLS13) {
            return Err(self.abort(AlertDescription::PROTOCOL_VERSION));
        }
        let suite = config
            .suites
            .iter()
            .copied()
            .find(|s| hello.suites.contains(&s.code()));
        let Some(suite) = suite else {
            return Err(self.abort(AlertDescription::HANDSHAKE_FAILURE));
        };
        if !hello.signature_algorithms.contains(&config.scheme.code()) {
            return Err(self.abort(AlertDescription::HANDSHAKE_FAILURE));
        }
        self.seen.suite = Some(suite);
        let compatibility = !hello.session_id.is_empty();
        let mut transcript = Transcript::new();
        transcript.add(&first);

        let share = if config.hello_retry_request {
            None
        } else {
            config
                .groups
                .iter()
                .copied()
                .find_map(|g| share_of(&hello, g))
        };
        let (group, client_share) = match share {
            Some(found) => {
                transcript.select(suite.hash());
                found
            }
            None => self.hello_retry_request(&hello, suite, &mut transcript)?,
        };
        self.seen.group = Some(group);

        let key = EphemeralKey::generate(group, &self.random).map_err(failed("key share"))?;
        let public = key.public_key().to_vec();
        let shared = match key.agree(&client_share) {
            Ok(shared) => shared,
            Err(_) => return Err(self.abort(AlertDescription::ILLEGAL_PARAMETER)),
        };
        let random = self.random_bytes()?;
        let extensions = wire::server_hello_extensions(group_code(group), &public);
        let server_hello =
            wire::server_hello(&random, &hello.session_id, suite.code(), Some(&extensions));
        transcript.add(&server_hello);
        self.layer.write_handshake(&server_hello).map_err(ended)?;
        self.seen.server_hello_sent = true;
        if compatibility && !self.seen.hello_retry_request_sent {
            self.layer.write_change_cipher_spec().map_err(ended)?;
        }
        if self.mode() == Mode::StallAfterServerHello {
            return self.drain("stalled after the ServerHello");
        }

        let early = key_schedule::early_secret(suite.hash(), None);
        let handshake = key_schedule::handshake_secret(&early, shared.as_ref())
            .map_err(failed("handshake secret"))?;
        let hash = current(&transcript)?;
        let secrets = key_schedule::handshake_traffic_secrets(&handshake, &hash)
            .map_err(failed("handshake traffic secrets"))?;
        let master = key_schedule::master_secret(&handshake).map_err(failed("master secret"))?;
        self.layer
            .set_write_keys(suite, &secrets.server)
            .map_err(ended)?;
        self.layer
            .set_read_keys(suite, &secrets.client)
            .map_err(ended)?;

        let alpn = hello.alpn.iter().find(|p| p.as_str() == "http/1.1");
        let encrypted_extensions =
            wire::encrypted_extensions(hello.server_name.is_some(), alpn.map(String::as_str));
        transcript.add(&encrypted_extensions);
        let mut flight = encrypted_extensions;
        if self.mode() == Mode::OversizedCertificate {
            // A Certificate that declares 2^24 - 1 bytes, the most a uint24 holds (16 MiB less
            // one byte); its header alone is sent.
            flight.extend_from_slice(&[wire::kind::CERTIFICATE, 0xff, 0xff, 0xff]);
            self.layer.write_handshake(&flight).map_err(ended)?;
            return self.drain("sent an oversized Certificate");
        }
        let certificate = wire::certificate(&self.setup.chain);
        transcript.add(&certificate);
        flight.extend_from_slice(&certificate);

        let content = wire::certificate_verify_content(&current(&transcript)?);
        let signature = self
            .setup
            .key
            .sign(&content, &self.random)
            .map_err(failed("sign"))?;
        let scheme = self.setup.key.scheme();
        let certificate_verify = wire::certificate_verify(scheme.code(), &signature);
        transcript.add(&certificate_verify);
        flight.extend_from_slice(&certificate_verify);
        self.seen.scheme = Some(scheme);

        let verify_data = key_schedule::verify_data(&secrets.server, &current(&transcript)?)
            .map_err(failed("verify_data"))?;
        let finished = wire::message(wire::kind::FINISHED, verify_data.as_ref());
        transcript.add(&finished);
        if self.mode() == Mode::BadFinished {
            let mut bad = verify_data.as_ref().to_vec();
            bad[0] ^= 0x01;
            flight.extend_from_slice(&wire::message(wire::kind::FINISHED, &bad));
            self.layer.write_handshake(&flight).map_err(ended)?;
            return self.drain("sent a bad Finished");
        }
        flight.extend_from_slice(&finished);
        self.layer.write_handshake(&flight).map_err(ended)?;

        let hash = current(&transcript)?;
        let application = key_schedule::application_traffic_secrets(&master, &hash)
            .map_err(failed("application traffic secrets"))?;
        self.layer
            .set_write_keys(suite, &application.server)
            .map_err(ended)?;
        let client_finished = self.handshake_message()?;
        let good = client_finished[0] == wire::kind::FINISHED
            && key_schedule::check_verify_data(&secrets.client, &hash, &client_finished[4..]);
        if !good {
            return Err(self.abort(AlertDescription::DECRYPT_ERROR));
        }
        self.layer
            .set_read_keys(suite, &application.client)
            .map_err(ended)?;
        self.finished = true;

        if self.mode() == Mode::Normal {
            let ticket = self.random_bytes()?;
            let age_add = u32::from_be_bytes([ticket[0], ticket[1], ticket[2], ticket[3]]);
            let message = wire::new_session_ticket(age_add, &ticket);
            self.layer.write_handshake(&message).map_err(ended)?;
            self.seen.ticket_sent = true;
        }
        self.http()
    }

    /// The HelloRetryRequest asking for secp256r1 (or, when the hello holds no share of a group
    /// the server accepts, the first such group it names), and the share of the second hello.
    fn hello_retry_request(
        &mut self,
        hello: &ClientHello,
        suite: CipherSuite,
        transcript: &mut Transcript,
    ) -> Result<(Group, Vec<u8>), End> {
        let setup = self.setup;
        let config = &setup.config;
        let wanted = if config.hello_retry_request {
            Some(Group::Secp256r1)
        } else {
            config.groups.first().copied()
        };
        let group = wanted
            .filter(|g| config.groups.contains(g))
            .filter(|g| hello.groups.contains(&group_code(*g)));
        let Some(group) = group else {
            return Err(self.abort(AlertDescription::HANDSHAKE_FAILURE));
        };
        let extensions = wire::hello_retry_request_extensions(group_code(group));
        let retry = wire::server_hello(
            &wire::hello_retry_request_random(),
            &hello.session_id,
            suite.code(),
            Some(&extensions),
        );
        transcript
            .hello_retry_request(suite.hash(), &retry)
            .map_err(failed("transcript"))?;
        self.layer.write_handshake(&retry).map_err(ended)?;
        self.seen.hello_retry_request_sent = true;
        if !hello.session_id.is_empty() {
            self.layer.write_change_cipher_spec().map_err(ended)?;
        }
        let second = self.handshake_message()?;
        transcript.add(&second);
        let share = wire::client_hello(&second)
            .ok()
            .and_then(|h| share_of(&h, group));
        match share {
            Some(found) => Ok(found),
            None => Err(self.abort(AlertDescription::ILLEGAL_PARAMETER)),
        }
    }

    /// Reads one request head, answers it from the map and ends as the mode says.
    fn http(&mut self) -> Result<(), End> {
        let mut head = Vec::new();
        while !head.windows(4).any(|w| w == b"\r\n\r\n") {
            match self.next()? {
                Message::ApplicationData(data) => head.extend_from_slice(&data),
                Message::Alert { description, .. }
                    if description != AlertDescription::USER_CANCELED =>
                {
                    return Err(format!("the client sent {description}"));
                }
                _ => {}
            }
            if head.len() > MAX_REQUEST {
                return Err(self.abort(AlertDescription::INTERNAL_ERROR));
            }
        }
        let line_end = head.windows(2).position(|w| w == b"\r\n").unwrap_or(0);
        let line = String::from_utf8_lossy(&head[..line_end]).into_owned();
        let path = line.split(' ').nth(1).unwrap_or("").to_string();
        self.seen.request = Some(line);
        let response = self
            .setup
            .config
            .responses
            .get(&path)
            .cloned()
            .unwrap_or_else(Response::not_found);
        let delimited_by_end = self.mode() == Mode::NoCloseNotify;
        let bytes = response.to_bytes(!delimited_by_end);
        self.layer.write_application_data(&bytes).map_err(ended)?;
        if delimited_by_end {
            let _ = self.layer.get_ref().stream.shutdown(Shutdown::Write);
            return self.drain("closed without close_notify");
        }
        self.layer
            .write_alert(AlertLevel::Warning, AlertDescription::CLOSE_NOTIFY)
            .map_err(ended)?;
        self.drain("answered")
    }
}

/// The transcript hash so far.
fn current(transcript: &Transcript) -> Result<Vec<u8>, End> {
    transcript
        .current()
        .map(|d| d.as_ref().to_vec())
        .ok_or_else(|| "the transcript has no hash".to_string())
}

/// The code point of a group (RFC 8446 section 4.2.7).
pub(crate) fn group_code(group: Group) -> u16 {
    match group {
        Group::X25519 => 0x001d,
        Group::Secp256r1 => 0x0017,
    }
}

/// The share of `group` in `hello`, if it holds one.
fn share_of(hello: &ClientHello, group: Group) -> Option<(Group, Vec<u8>)> {
    hello
        .shares
        .iter()
        .find(|(code, _)| *code == group_code(group))
        .map(|(_, key)| (group, key.clone()))
}

/// The signing key of a setup, its scheme checked by `SigningKey` itself.
pub(crate) fn signing_key(
    scheme: surf_tls::crypto::Scheme,
    pkcs8: &[u8],
) -> Result<SigningKey, String> {
    SigningKey::from_pkcs8(scheme, pkcs8, &Random::system()).map_err(|e| format!("{e:?}"))
}
