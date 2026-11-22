//! The key schedule of TLS 1.3 (RFC 8446 section 7): HKDF-Extract, HKDF-Expand-Label and
//! Derive-Secret over the seam; the early, handshake and master secrets; the client and server
//! traffic secrets; the finished keys and `verify_data`; the traffic key and IV of a record
//! protection; and the next application traffic secret of a `KeyUpdate`. For SHA-256 and
//! SHA-384.
//!
//! It serves either side of a connection: every stage returns the client's and the server's
//! secret alike, and the caller says which one protects the records it writes and which the
//! records it reads (docs/design/tls.md, item 6).
//!
//! A [`Secret`] and a [`TrafficKey`] have no `==` and print no byte in their `Debug` form; a
//! `verify_data` is checked with [`check_verify_data`], which compares in constant time.

use std::fmt;

use crate::crypto::{self, AeadAlgorithm, CryptoError, Digest, HashAlgorithm};

/// The longest digest of a suite: 48 bytes, SHA-384.
pub const MAX_HASH_LEN: usize = 48;

/// The type of a `message_hash` handshake message (RFC 8446 section 4.4.1).
pub const MESSAGE_HASH: u8 = 254;

/// A TLS 1.3 cipher suite (RFC 8446 section B.4): its AEAD and its hash.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CipherSuite {
    /// TLS_AES_128_GCM_SHA256 (0x1301).
    Aes128GcmSha256,
    /// TLS_AES_256_GCM_SHA384 (0x1302).
    Aes256GcmSha384,
    /// TLS_CHACHA20_POLY1305_SHA256 (0x1303).
    ChaCha20Poly1305Sha256,
}

impl CipherSuite {
    /// The three suites, in the order of their code points.
    pub const ALL: [CipherSuite; 3] = [
        CipherSuite::Aes128GcmSha256,
        CipherSuite::Aes256GcmSha384,
        CipherSuite::ChaCha20Poly1305Sha256,
    ];

    /// The code point of RFC 8446 section B.4.
    pub fn code(self) -> u16 {
        match self {
            CipherSuite::Aes128GcmSha256 => 0x1301,
            CipherSuite::Aes256GcmSha384 => 0x1302,
            CipherSuite::ChaCha20Poly1305Sha256 => 0x1303,
        }
    }

    /// The suite of a code point, if it is one of the three.
    pub fn from_code(code: u16) -> Option<CipherSuite> {
        CipherSuite::ALL.into_iter().find(|s| s.code() == code)
    }

    /// The hash of the key schedule and the transcript.
    pub fn hash(self) -> HashAlgorithm {
        match self {
            CipherSuite::Aes256GcmSha384 => HashAlgorithm::Sha384,
            CipherSuite::Aes128GcmSha256 | CipherSuite::ChaCha20Poly1305Sha256 => {
                HashAlgorithm::Sha256
            }
        }
    }

    /// The AEAD that protects the records.
    pub fn aead(self) -> AeadAlgorithm {
        match self {
            CipherSuite::Aes128GcmSha256 => AeadAlgorithm::Aes128Gcm,
            CipherSuite::Aes256GcmSha384 => AeadAlgorithm::Aes256Gcm,
            CipherSuite::ChaCha20Poly1305Sha256 => AeadAlgorithm::ChaCha20Poly1305,
        }
    }
}

/// A secret of the key schedule, of the length of its hash. It has no `==` and its `Debug`
/// form names its length only; its bytes are overwritten with zeros when it is dropped.
#[derive(Clone)]
pub struct Secret {
    bytes: [u8; MAX_HASH_LEN],
    len: usize,
    algorithm: HashAlgorithm,
}

impl Secret {
    fn new(algorithm: HashAlgorithm, value: &[u8]) -> Secret {
        let mut bytes = [0u8; MAX_HASH_LEN];
        let len = value.len().min(MAX_HASH_LEN);
        bytes[..len].copy_from_slice(&value[..len]);
        Secret {
            bytes,
            len,
            algorithm,
        }
    }

    /// The hash the secret belongs to.
    pub fn algorithm(&self) -> HashAlgorithm {
        self.algorithm
    }

    /// The length in bytes, the hash's length.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Whether the secret has no byte; never true for a secret of the schedule.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

impl AsRef<[u8]> for Secret {
    fn as_ref(&self) -> &[u8] {
        &self.bytes[..self.len]
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Secret({:?}, {} bytes)", self.algorithm, self.len)
    }
}

impl Drop for Secret {
    fn drop(&mut self) {
        self.bytes.fill(0);
    }
}

/// HKDF-Extract (RFC 5869 section 2.2) of `ikm` under `salt`. An empty salt is a salt of
/// zeros of the hash's length, as RFC 8446 section 7.1 writes "0".
pub fn hkdf_extract(algorithm: HashAlgorithm, salt: &[u8], ikm: &[u8]) -> Secret {
    let prk = crypto::hkdf_extract(algorithm, salt, ikm);
    Secret::new(algorithm, prk.as_ref())
}

/// The `HkdfLabel` of RFC 8446 section 7.1: the length, `"tls13 "` and `label`, and
/// `context`, as HKDF-Expand-Label passes it as `info`. Refused with `OutputTooLong` when the
/// label or the context is longer than the structure holds (249 and 255 bytes).
pub fn hkdf_label(length: u16, label: &[u8], context: &[u8]) -> Result<Vec<u8>, CryptoError> {
    let (label_len, context_len) = label_lengths(label, context)?;
    let mut info = Vec::with_capacity(2 + 1 + 6 + label.len() + 1 + context.len());
    info.extend_from_slice(&length.to_be_bytes());
    info.push(label_len);
    info.extend_from_slice(b"tls13 ");
    info.extend_from_slice(label);
    info.push(context_len);
    info.extend_from_slice(context);
    Ok(info)
}

fn label_lengths(label: &[u8], context: &[u8]) -> Result<(u8, u8), CryptoError> {
    let label_len = u8::try_from(6 + label.len()).map_err(|_| CryptoError::OutputTooLong)?;
    let context_len = u8::try_from(context.len()).map_err(|_| CryptoError::OutputTooLong)?;
    Ok((label_len, context_len))
}

/// HKDF-Expand-Label (RFC 8446 section 7.1): fills `out` from `secret` under `label` (without
/// its prefix `"tls13 "`) and `context`.
pub fn hkdf_expand_label(
    algorithm: HashAlgorithm,
    secret: &[u8],
    label: &[u8],
    context: &[u8],
    out: &mut [u8],
) -> Result<(), CryptoError> {
    let length = u16::try_from(out.len()).map_err(|_| CryptoError::OutputTooLong)?;
    let (label_len, context_len) = label_lengths(label, context)?;
    crypto::hkdf_expand(
        algorithm,
        secret,
        &[
            &length.to_be_bytes(),
            &[label_len],
            b"tls13 ",
            label,
            &[context_len],
            context,
        ],
        out,
    )
}

fn expand_secret(secret: &Secret, label: &[u8], context: &[u8]) -> Result<Secret, CryptoError> {
    let algorithm = secret.algorithm;
    let mut out = [0u8; MAX_HASH_LEN];
    let len = algorithm.output_len();
    let result = hkdf_expand_label(algorithm, secret.as_ref(), label, context, &mut out[..len]);
    let derived = result.map(|()| Secret::new(algorithm, &out[..len]));
    out.fill(0);
    derived
}

/// Derive-Secret (RFC 8446 section 7.1): HKDF-Expand-Label of `secret` under `label` with the
/// transcript hash `transcript_hash` as the context, of the hash's length.
pub fn derive_secret(
    secret: &Secret,
    label: &[u8],
    transcript_hash: &[u8],
) -> Result<Secret, CryptoError> {
    expand_secret(secret, label, transcript_hash)
}

/// The hash of no messages, the context of `"derived"`.
fn empty_hash(algorithm: HashAlgorithm) -> Digest {
    crypto::hash(algorithm, &[])
}

/// The early secret: HKDF-Extract of `psk`, or of zeros when there is none, under a salt of
/// zeros.
pub fn early_secret(algorithm: HashAlgorithm, psk: Option<&[u8]>) -> Secret {
    let zeros = [0u8; MAX_HASH_LEN];
    let ikm = psk.unwrap_or(&zeros[..algorithm.output_len()]);
    hkdf_extract(algorithm, &[], ikm)
}

/// The handshake secret: HKDF-Extract of the (EC)DHE shared secret under
/// Derive-Secret(early, "derived", "").
pub fn handshake_secret(early: &Secret, shared_secret: &[u8]) -> Result<Secret, CryptoError> {
    let salt = derive_secret(early, b"derived", empty_hash(early.algorithm).as_ref())?;
    Ok(hkdf_extract(early.algorithm, salt.as_ref(), shared_secret))
}

/// The master secret: HKDF-Extract of zeros under Derive-Secret(handshake, "derived", "").
pub fn master_secret(handshake: &Secret) -> Result<Secret, CryptoError> {
    let algorithm = handshake.algorithm;
    let salt = derive_secret(handshake, b"derived", empty_hash(algorithm).as_ref())?;
    let zeros = [0u8; MAX_HASH_LEN];
    Ok(hkdf_extract(
        algorithm,
        salt.as_ref(),
        &zeros[..algorithm.output_len()],
    ))
}

/// The traffic secrets of one stage, the client's and the server's.
#[derive(Clone, Debug)]
pub struct TrafficSecrets {
    /// The secret of the records the client writes.
    pub client: Secret,
    /// The secret of the records the server writes.
    pub server: Secret,
}

/// The handshake traffic secrets, `"c hs traffic"` and `"s hs traffic"`, over the transcript
/// hash of ClientHello to ServerHello.
pub fn handshake_traffic_secrets(
    handshake: &Secret,
    transcript_hash: &[u8],
) -> Result<TrafficSecrets, CryptoError> {
    Ok(TrafficSecrets {
        client: derive_secret(handshake, b"c hs traffic", transcript_hash)?,
        server: derive_secret(handshake, b"s hs traffic", transcript_hash)?,
    })
}

/// The first application traffic secrets, `"c ap traffic"` and `"s ap traffic"`, over the
/// transcript hash of ClientHello to the server's Finished.
pub fn application_traffic_secrets(
    master: &Secret,
    transcript_hash: &[u8],
) -> Result<TrafficSecrets, CryptoError> {
    Ok(TrafficSecrets {
        client: derive_secret(master, b"c ap traffic", transcript_hash)?,
        server: derive_secret(master, b"s ap traffic", transcript_hash)?,
    })
}

/// The exporter master secret, `"exp master"`, over the transcript hash of ClientHello to the
/// server's Finished.
pub fn exporter_master_secret(
    master: &Secret,
    transcript_hash: &[u8],
) -> Result<Secret, CryptoError> {
    derive_secret(master, b"exp master", transcript_hash)
}

/// The resumption master secret, `"res master"`, over the transcript hash of ClientHello to
/// the client's Finished.
pub fn resumption_master_secret(
    master: &Secret,
    transcript_hash: &[u8],
) -> Result<Secret, CryptoError> {
    derive_secret(master, b"res master", transcript_hash)
}

/// The PSK of a ticket (RFC 8446 section 4.6.1): HKDF-Expand-Label of the resumption master
/// secret under `"resumption"` with the ticket's nonce.
pub fn resumption_psk(
    resumption_master: &Secret,
    ticket_nonce: &[u8],
) -> Result<Secret, CryptoError> {
    expand_secret(resumption_master, b"resumption", ticket_nonce)
}

/// The next application traffic secret after a `KeyUpdate` (RFC 8446 section 7.2):
/// HKDF-Expand-Label of the current one under `"traffic upd"` with an empty context.
pub fn next_traffic_secret(current: &Secret) -> Result<Secret, CryptoError> {
    expand_secret(current, b"traffic upd", &[])
}

/// The finished key of a handshake traffic secret (RFC 8446 section 4.4.4):
/// HKDF-Expand-Label under `"finished"` with an empty context.
pub fn finished_key(base: &Secret) -> Result<Secret, CryptoError> {
    expand_secret(base, b"finished", &[])
}

/// The `verify_data` of a Finished message: the HMAC, under the finished key of `base`, of
/// the transcript hash up to the message before the Finished.
pub fn verify_data(base: &Secret, transcript_hash: &[u8]) -> Result<Digest, CryptoError> {
    let key = finished_key(base)?;
    Ok(crypto::hmac_sign(
        base.algorithm,
        key.as_ref(),
        transcript_hash,
    ))
}

/// Whether `received` is the `verify_data` of `base` over `transcript_hash`, checked in
/// constant time by the seam.
pub fn check_verify_data(base: &Secret, transcript_hash: &[u8], received: &[u8]) -> bool {
    match finished_key(base) {
        Ok(key) => {
            crypto::hmac_verify(base.algorithm, key.as_ref(), transcript_hash, received).is_ok()
        }
        Err(_) => false,
    }
}

/// The traffic key and IV of a record protection (RFC 8446 section 7.3). It has no `==`, its
/// `Debug` form names the key's length only, and its bytes are overwritten with zeros when it
/// is dropped.
pub struct TrafficKey {
    key: [u8; 32],
    key_len: usize,
    iv: [u8; AeadAlgorithm::NONCE_LEN],
}

impl TrafficKey {
    /// The key, of the length the suite's AEAD takes.
    pub fn key(&self) -> &[u8] {
        &self.key[..self.key_len]
    }

    /// The IV, 12 bytes.
    pub fn iv(&self) -> &[u8; AeadAlgorithm::NONCE_LEN] {
        &self.iv
    }
}

impl fmt::Debug for TrafficKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TrafficKey({}-byte key)", self.key_len)
    }
}

impl Drop for TrafficKey {
    fn drop(&mut self) {
        self.key.fill(0);
        self.iv.fill(0);
    }
}

/// The traffic key (`"key"`) and IV (`"iv"`) of `secret` for `suite`. Refused with
/// `KeyLength` when the secret belongs to another hash than the suite's.
pub fn traffic_key(suite: CipherSuite, secret: &Secret) -> Result<TrafficKey, CryptoError> {
    if secret.algorithm != suite.hash() {
        return Err(CryptoError::KeyLength);
    }
    let key_len = suite.aead().key_len();
    let mut out = TrafficKey {
        key: [0u8; 32],
        key_len,
        iv: [0u8; AeadAlgorithm::NONCE_LEN],
    };
    hkdf_expand_label(
        secret.algorithm,
        secret.as_ref(),
        b"key",
        &[],
        &mut out.key[..key_len],
    )?;
    hkdf_expand_label(secret.algorithm, secret.as_ref(), b"iv", &[], &mut out.iv)?;
    Ok(out)
}
