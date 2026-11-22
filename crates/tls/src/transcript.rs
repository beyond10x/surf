//! The transcript hash of a handshake (RFC 8446 section 4.4.1): the hash of the handshake
//! messages so far, each with its 4-byte header, with the `message_hash` replacement of the
//! first ClientHello after a HelloRetryRequest.
//!
//! The hash is the suite's, which the client learns only from the server's answer, so a
//! transcript holds the messages until its hash is selected ([`Transcript::select`] or
//! [`Transcript::hello_retry_request`]) and hashes as it goes from then on. Before that it
//! holds the ClientHello alone, whose size the handshake bounds.

use std::fmt;

use crate::crypto::{self, Digest, HashAlgorithm, RunningHash};
use crate::key_schedule::MESSAGE_HASH;

enum State {
    Holding(Vec<u8>),
    Hashing(Box<RunningHash>),
}

/// The running transcript of one handshake.
pub struct Transcript {
    state: State,
}

impl Default for Transcript {
    fn default() -> Self {
        Transcript::new()
    }
}

impl Transcript {
    /// A transcript of no message whose hash is not selected yet.
    pub fn new() -> Self {
        Transcript {
            state: State::Holding(Vec::new()),
        }
    }

    /// A transcript of no message that hashes with `algorithm`.
    pub fn with_hash(algorithm: HashAlgorithm) -> Self {
        Transcript {
            state: State::Hashing(Box::new(RunningHash::new(algorithm))),
        }
    }

    /// Adds a whole handshake message, its 4-byte header included.
    pub fn add(&mut self, message: &[u8]) {
        match &mut self.state {
            State::Holding(held) => held.extend_from_slice(message),
            State::Hashing(hash) => hash.update(message),
        }
    }

    /// Selects the hash: the messages held so far are hashed with `algorithm`, and every later
    /// one as it comes. False, changing nothing, when the transcript already hashes with
    /// another algorithm.
    pub fn select(&mut self, algorithm: HashAlgorithm) -> bool {
        match &mut self.state {
            State::Hashing(hash) => hash.algorithm() == algorithm,
            State::Holding(held) => {
                let mut hash = RunningHash::new(algorithm);
                hash.update(held);
                self.state = State::Hashing(Box::new(hash));
                true
            }
        }
    }

    /// The HelloRetryRequest `hrr` (RFC 8446 section 4.4.1): the transcript so far, the first
    /// ClientHello, is replaced by the message `message_hash` that holds its hash under
    /// `algorithm`, and `hrr` is added after it. False, changing nothing, when the transcript
    /// already hashes with another algorithm.
    pub fn hello_retry_request(&mut self, algorithm: HashAlgorithm, hrr: &[u8]) -> bool {
        let first = match &self.state {
            State::Holding(held) => crypto::hash(algorithm, held),
            State::Hashing(hash) if hash.algorithm() == algorithm => hash.current(),
            State::Hashing(_) => return false,
        };
        let mut hash = RunningHash::new(algorithm);
        // The length of a digest is 32 or 48, so it fits the one byte of the uint24's last
        // octet.
        let len = u8::try_from(first.as_ref().len()).unwrap_or(u8::MAX);
        hash.update(&[MESSAGE_HASH, 0, 0, len]);
        hash.update(first.as_ref());
        hash.update(hrr);
        self.state = State::Hashing(Box::new(hash));
        true
    }

    /// The selected hash, if any.
    pub fn algorithm(&self) -> Option<HashAlgorithm> {
        match &self.state {
            State::Holding(_) => None,
            State::Hashing(hash) => Some(hash.algorithm()),
        }
    }

    /// The transcript hash of the messages so far; none before the hash is selected.
    pub fn current(&self) -> Option<Digest> {
        match &self.state {
            State::Holding(_) => None,
            State::Hashing(hash) => Some(hash.current()),
        }
    }
}

impl fmt::Debug for Transcript {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.state {
            State::Holding(held) => write!(f, "Transcript(holding {} bytes)", held.len()),
            State::Hashing(hash) => write!(f, "Transcript({:?})", hash.algorithm()),
        }
    }
}
