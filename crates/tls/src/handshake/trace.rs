//! The test mode `GivenClientHello` (feature `test-tooling` only; `story:tls-handshake`): the
//! client's state machine run over given bytes, in memory, with the ClientHellos of a trace
//! given as bytes instead of built. RFC 8448 has an empty legacy session id and offers
//! `record_size_limit`, which surf's own hello neither does nor offers; with its hellos given
//! the trace runs as written. The connection layer has no path here, and no stream but
//! [`Memory`] reaches it.

use std::io::{self, Read, Write};

use crate::connection::Connection;
use crate::crypto::{Random, SignatureVerifier};
use crate::der::Time;
use crate::error::Error;
use crate::record::RecordLayer;
use crate::verify::ServerCertVerifier;

use super::client::{self, Hellos, Settings, SignatureCheck};

/// The bytes a trace's server sent, read from the front, and the bytes the client wrote.
#[derive(Debug, Default)]
pub struct Memory {
    input: Vec<u8>,
    read: usize,
    written: Vec<u8>,
}

impl Memory {
    /// A memory whose reads return `input`, then the end of the stream.
    pub fn new(input: Vec<u8>) -> Self {
        Memory {
            input,
            read: 0,
            written: Vec::new(),
        }
    }

    /// Every byte the client wrote.
    pub fn written(&self) -> &[u8] {
        &self.written
    }

    /// The bytes of the input not read yet.
    pub fn unread(&self) -> usize {
        self.input.len() - self.read
    }
}

impl Read for Memory {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = buf.len().min(self.input.len() - self.read);
        buf[..n].copy_from_slice(&self.input[self.read..self.read + n]);
        self.read += n;
        Ok(n)
    }
}

impl Write for Memory {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.written.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// A trace's client: its ClientHellos as bytes, the private key of each key share, and the
/// server's records as bytes.
pub struct GivenClientHello<'a> {
    /// The ClientHello messages, each with its 4-byte header: the first, and the second after
    /// a HelloRetryRequest.
    pub hellos: Vec<Vec<u8>>,
    /// The private key of each key share of the hellos, in their order (32 bytes each): the
    /// shares are made from these bytes, and each must give the public key its hello holds.
    pub private_keys: Vec<Vec<u8>>,
    /// The server's records, as the trace has them.
    pub server_flight: Vec<u8>,
    /// The host.
    pub host: &'a str,
    /// The verifier of the server's certificates.
    pub verifier: &'a dyn ServerCertVerifier,
    /// The answer of the trace for the CertificateVerify signature (RFC 8448's server key is
    /// RSA-1024, which the seam refuses).
    pub signatures: &'a dyn SignatureVerifier,
    /// The time.
    pub now: Time,
}

impl GivenClientHello<'_> {
    /// Runs the handshake over `memory`, whose input becomes the server's records: the
    /// connection after the client's Finished, or the error, with what the client wrote kept
    /// in `memory`.
    pub fn run<'m>(&self, memory: &'m mut Memory) -> Result<Connection<&'m mut Memory>, Error> {
        *memory = Memory::new(self.server_flight.clone());
        let slices: Vec<&[u8]> = self.private_keys.iter().map(Vec::as_slice).collect();
        let random = Random::fixed_sequence(&slices);
        let settings = Settings {
            host: self.host,
            verifier: self.verifier,
            now: self.now,
            random: &random,
            hellos: Hellos::Given(&self.hellos),
            signature_check: SignatureCheck::Given(self.signatures),
        };
        let established = client::handshake(RecordLayer::new(memory), &settings)?;
        Ok(Connection::established(established, self.host.to_string()))
    }
}
