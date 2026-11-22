//! The one connection layer of surf's TLS client (`story:tls-handshake`; docs/design/tls.md
//! item 6): [`Connection`], a client over any `Read + Write`, given a host, a
//! [`ServerCertVerifier`] and the time. [`Connection::new`] runs the handshake to the client's
//! Finished; then [`Connection::read_data`] and [`Connection::write_data`] carry application
//! data, and the `std::io` traits do the same.
//!
//! After the handshake the server may send application data, NewSessionTickets (read whole and
//! dropped: no resumption), KeyUpdates (an `update_requested` is answered before the next
//! data), `user_canceled` (ignored) and `close_notify` (the end of the stream; data after it
//! is never read). Every other handshake message is `unexpected_message`, a
//! `change_cipher_spec` is `UnexpectedChangeCipherSpec`, any other alert ends the connection
//! with `AlertReceived`. The end of the stream without `close_notify` is `Truncated`; a read
//! that times out is `TimedOut`. After an error the connection reads and writes nothing more
//! and returns that error.
//!
//! Before a write would pass the AES-GCM record limit of a key (`record::AES_GCM_RECORD_LIMIT`,
//! RFC 8446 section 5.5) the connection sends a KeyUpdate and writes on under the next key.
//!
//! [`Connection::close`], and the drop of a connection whose handshake ended and that has not
//! failed, write one `close_notify` and shut the connection's write side: every later write is
//! refused. The stream itself is the caller's: [`Connection::into_inner`] hands it back, so a
//! socket's own write side is shut there (`TcpStream::shutdown`).

use std::io::{self, ErrorKind, Read, Write};

use crate::crypto::Random;
use crate::der::Time;
use crate::error::{AlertDescription, Error, Violation};
use crate::handshake::client::{self, Established, Fail, Hellos, Settings, SignatureCheck};
use crate::handshake::messages::{self, kind};
use crate::key_schedule::CipherSuite;
use crate::record::{self, AlertLevel, MAX_PLAINTEXT, Message, RecordLayer};
use crate::verify::ServerCertVerifier;

/// A TLS 1.3 client connection over `S`, its handshake done.
pub struct Connection<S: Read + Write> {
    layer: Option<RecordLayer<S>>,
    suite: CipherSuite,
    failed: Option<Error>,
    /// `close_notify` came: reads return the end of the stream.
    received_close: bool,
    /// `close_notify` was written: writes are refused.
    sent_close: bool,
    /// Application data received and not yet returned, from `pending_at` on.
    pending: Vec<u8>,
    pending_at: usize,
}

/// The error of the stream as the connection reports it: a read that timed out is `TimedOut`.
fn reported(error: Error) -> Error {
    match error {
        Error::Io(ErrorKind::TimedOut | ErrorKind::WouldBlock) => Error::TimedOut,
        other => other,
    }
}

impl<S: Read + Write> Connection<S> {
    /// Runs the handshake for `host` over `stream`, the certificates checked by `verifier` at
    /// `now`, with the operating system's random source.
    pub fn new(
        stream: S,
        host: &str,
        verifier: &dyn ServerCertVerifier,
        now: Time,
    ) -> Result<Self, Error> {
        Connection::with_random(stream, host, verifier, now, &Random::system())
    }

    /// As [`Connection::new`], with `random` as the source of the hello's random, its session
    /// id and its key shares (a fixed source exists only under the feature `test-tooling`).
    pub fn with_random(
        stream: S,
        host: &str,
        verifier: &dyn ServerCertVerifier,
        now: Time,
        random: &Random<'_>,
    ) -> Result<Self, Error> {
        #[cfg(feature = "test-tooling")]
        std::hint::black_box(crate::TEST_TOOLING_MARKER);
        let settings = Settings {
            host,
            verifier,
            now,
            random,
            hellos: Hellos::Own,
            signature_check: SignatureCheck::Seam,
        };
        let established =
            client::handshake(RecordLayer::new(stream), &settings).map_err(reported)?;
        Ok(Connection::established(established))
    }

    pub(crate) fn established(established: Established<S>) -> Self {
        Connection {
            layer: Some(established.layer),
            suite: established.suite,
            failed: None,
            received_close: false,
            sent_close: false,
            pending: Vec::new(),
            pending_at: 0,
        }
    }

    /// The suite the server selected.
    pub fn suite(&self) -> CipherSuite {
        self.suite
    }

    /// The stream.
    pub fn get_ref(&self) -> &S {
        self.layer().get_ref()
    }

    /// The stream, given back without a `close_notify`; the connection is gone.
    pub fn into_inner(mut self) -> S {
        self.failed = Some(Error::ConnectionClosed);
        let layer = self.layer.take();
        match layer {
            Some(layer) => layer.into_inner(),
            None => unreachable!("the layer is taken only here"),
        }
    }

    fn layer(&self) -> &RecordLayer<S> {
        match &self.layer {
            Some(layer) => layer,
            None => unreachable!("the layer is taken only by into_inner, which consumes"),
        }
    }

    fn layer_mut(&mut self) -> &mut RecordLayer<S> {
        match &mut self.layer {
            Some(layer) => layer,
            None => unreachable!("the layer is taken only by into_inner, which consumes"),
        }
    }

    /// Sets the sequence number of the next record written (test tooling only), to reach the
    /// record limit of a key without writing millions of records.
    #[cfg(feature = "test-tooling")]
    pub fn set_write_sequence(&mut self, sequence: u64) {
        self.layer_mut().set_write_sequence(sequence);
    }

    fn check(&self) -> Result<(), Error> {
        match &self.failed {
            Some(error) => Err(error.clone()),
            None => Ok(()),
        }
    }

    fn fail(&mut self, error: Error) -> Error {
        let error = reported(error);
        self.failed = Some(error.clone());
        error
    }

    /// Reads application data into `buf`: the bytes read, 0 at the end of the stream
    /// (`close_notify` received) or when `buf` is empty.
    pub fn read_data(&mut self, buf: &mut [u8]) -> Result<usize, Error> {
        self.check()?;
        if buf.is_empty() {
            return Ok(0);
        }
        loop {
            if self.pending_at < self.pending.len() {
                let n = buf.len().min(self.pending.len() - self.pending_at);
                buf[..n].copy_from_slice(&self.pending[self.pending_at..self.pending_at + n]);
                self.pending_at += n;
                return Ok(n);
            }
            if self.received_close {
                return Ok(0);
            }
            let message = self.layer_mut().read_message();
            match message {
                Ok(Message::ApplicationData(data)) => {
                    self.pending = data;
                    self.pending_at = 0;
                }
                Ok(Message::Handshake(message)) => self.post_handshake(&message)?,
                Ok(Message::Alert { description, .. }) => match description {
                    AlertDescription::CLOSE_NOTIFY => self.received_close = true,
                    AlertDescription::USER_CANCELED => {}
                    _ => {
                        return Err(self.fail(Error::AlertReceived {
                            description,
                            before_server_hello: false,
                        }));
                    }
                },
                Ok(Message::ChangeCipherSpec(_)) => {
                    let error = self.layer_mut().send_alert(
                        AlertDescription::UNEXPECTED_MESSAGE,
                        Violation::UnexpectedChangeCipherSpec,
                    );
                    return Err(self.fail(error));
                }
                Err(Error::ConnectionClosed) => return Err(self.fail(Error::Truncated)),
                Err(error) => return Err(self.fail(error)),
            }
        }
    }

    /// A handshake message after the handshake: a NewSessionTicket read and dropped, a
    /// KeyUpdate taken (and answered when it asks for it), any other refused.
    fn post_handshake(&mut self, message: &[u8]) -> Result<(), Error> {
        let body = &message[4..];
        let outcome = match message[0] {
            kind::NEW_SESSION_TICKET => messages::new_session_ticket(body).map(|()| None),
            kind::KEY_UPDATE => messages::key_update(body).map(Some),
            other => {
                return Err(self.abort(
                    AlertDescription::UNEXPECTED_MESSAGE,
                    Violation::UnexpectedMessage {
                        expected: "application data",
                        got: other,
                    },
                ));
            }
        };
        let update = match outcome {
            Ok(update) => update,
            Err(abort) => return Err(self.abort(abort.alert, abort.violation)),
        };
        if let Some(requested) = update {
            if let Err(error) = self.layer_mut().update_read_keys() {
                return Err(self.fail(error));
            }
            if requested && !self.sent_close {
                self.send_key_update()?;
            }
        }
        Ok(())
    }

    fn abort(&mut self, alert: AlertDescription, violation: Violation) -> Error {
        let error = client::end(
            self.layer_mut(),
            Fail::Abort(messages::Abort::new(alert, violation)),
        );
        self.fail(error)
    }

    /// Writes a KeyUpdate of `update_not_requested` under the current key and takes the next
    /// one for every later record.
    fn send_key_update(&mut self) -> Result<(), Error> {
        let message = messages::key_update_message(false);
        let layer = self.layer_mut();
        let sent = layer
            .write_handshake(&message)
            .and_then(|()| layer.update_write_keys());
        sent.map_err(|e| self.fail(e))
    }

    /// Writes `data` as application data, in records of at most 2^14 bytes. Before the record
    /// that would leave no record of an AES-GCM key for a KeyUpdate, a KeyUpdate is sent.
    pub fn write_data(&mut self, data: &[u8]) -> Result<(), Error> {
        self.check()?;
        if self.sent_close {
            return Err(Error::Io(ErrorKind::BrokenPipe));
        }
        for chunk in data.chunks(MAX_PLAINTEXT) {
            let limit = record::write_limit(self.suite);
            let sequence = self.layer().write_sequence().unwrap_or(0);
            if limit < u64::MAX && limit.saturating_sub(sequence) < 2 {
                self.send_key_update()?;
            }
            if let Err(error) = self.layer_mut().write_application_data(chunk) {
                return Err(self.fail(error));
            }
        }
        Ok(())
    }

    /// Writes one `close_notify` under the application key and shuts the write side: every
    /// later write is refused. Reads go on until the server's `close_notify`. A second call
    /// writes nothing.
    pub fn close(&mut self) -> Result<(), Error> {
        self.check()?;
        if self.sent_close {
            return Ok(());
        }
        self.sent_close = true;
        let written = self
            .layer_mut()
            .write_alert(AlertLevel::Warning, AlertDescription::CLOSE_NOTIFY);
        written.map_err(|e| self.fail(e))
    }
}

impl<S: Read + Write> Drop for Connection<S> {
    fn drop(&mut self) {
        if self.layer.is_some() && self.failed.is_none() && !self.sent_close {
            let _ = self.close();
        }
    }
}

/// The `std::io` form of an error: the kind of the stream's own error, `TimedOut`,
/// `UnexpectedEof` for a stream that ended early, `InvalidData` for every other; the `Error`
/// travels inside (`io::Error::get_ref`).
fn io_error(error: Error) -> io::Error {
    let kind = match &error {
        Error::Io(kind) => *kind,
        Error::TimedOut => ErrorKind::TimedOut,
        Error::ConnectionClosed | Error::Truncated => ErrorKind::UnexpectedEof,
        _ => ErrorKind::InvalidData,
    };
    io::Error::new(kind, error)
}

impl<S: Read + Write> Read for Connection<S> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.read_data(buf).map_err(io_error)
    }
}

impl<S: Read + Write> Write for Connection<S> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.write_data(buf).map_err(io_error)?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.check().map_err(io_error)
    }
}

impl<S: Read + Write> std::fmt::Debug for Connection<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Connection")
            .field("suite", &self.suite)
            .field("failed", &self.failed)
            .field("received_close", &self.received_close)
            .field("sent_close", &self.sent_close)
            .finish()
    }
}
