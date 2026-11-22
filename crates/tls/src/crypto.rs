//! The seam: the one module of surf that names ring (docs/design/tls.md, items 5 and 6).
//!
//! It offers what TLS 1.3 needs and nothing else: AEAD seal and open for AES-128-GCM,
//! AES-256-GCM and ChaCha20-Poly1305; SHA-256 and SHA-384, one-shot and running; HMAC, HKDF
//! extract and expand; X25519 and P-256 agreement with a key drawn from a [`Random`] source;
//! signature checks for the nine schemes of item 14 behind the trait [`SignatureVerifier`]; and
//! [`REFUSED`], the signature algorithms it refuses by name before any verification.
//!
//! The shipped random source is the operating system's ([`Random::system`]). Behind the cargo
//! feature `test-tooling`, never on in a release: signing from PKCS#8 ([`SigningKey`]) and the
//! fixed random sources of ring's test module, which makes signatures and keys reproducible.

use std::fmt;
use std::marker::PhantomData;

use ring::{aead, agreement, digest, hkdf, hmac, rand, signature};

/// What an operation of the seam refused, or why it failed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CryptoError {
    /// A key of another length than the algorithm takes.
    KeyLength,
    /// A nonce of another length than 96 bits.
    NonceLength,
    /// A protected record that does not open: a wrong tag, key, nonce or additional data.
    OpenFailed,
    /// More output than HKDF-Expand can give (255 times the hash length).
    OutputTooLong,
    /// A peer's public key the agreement refuses: off the curve, of the wrong length, or one
    /// that gives an X25519 result of all zeros.
    BadPublicKey,
    /// A signature that does not verify.
    BadSignature,
    /// A private key the primitives refuse.
    KeyRejected,
    /// The random source failed, or another failure ring does not name.
    Unspecified,
}

impl fmt::Display for CryptoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

impl std::error::Error for CryptoError {}

// ---------------------------------------------------------------------------------------------
// Random sources.

enum Source<'a> {
    System(rand::SystemRandom, PhantomData<&'a [u8]>),
    #[cfg(feature = "test-tooling")]
    #[allow(deprecated)]
    FixedByte(ring::test::rand::FixedByteRandom),
    #[cfg(feature = "test-tooling")]
    #[allow(deprecated)]
    FixedSlice(ring::test::rand::FixedSliceRandom<'a>),
    #[cfg(feature = "test-tooling")]
    #[allow(deprecated)]
    FixedSequence(ring::test::rand::FixedSliceSequenceRandom<'a>),
}

/// A source of random bytes for keys, nonces and the hello's random.
pub struct Random<'a> {
    source: Source<'a>,
}

impl Random<'static> {
    /// The operating system's source (getrandom), the one a release uses.
    pub fn system() -> Self {
        Random {
            source: Source::System(rand::SystemRandom::new(), PhantomData),
        }
    }
}

impl<'a> Random<'a> {
    /// Fills `out` with random bytes.
    pub fn fill(&self, out: &mut [u8]) -> Result<(), CryptoError> {
        self.ring()
            .fill(out)
            .map_err(|ring::error::Unspecified| CryptoError::Unspecified)
    }

    #[allow(deprecated)]
    fn ring(&self) -> &dyn rand::SecureRandom {
        match &self.source {
            Source::System(random, _) => random,
            #[cfg(feature = "test-tooling")]
            Source::FixedByte(random) => random,
            #[cfg(feature = "test-tooling")]
            Source::FixedSlice(random) => random,
            #[cfg(feature = "test-tooling")]
            Source::FixedSequence(random) => random,
        }
    }

    /// A source that fills every draw with `byte` (test tooling only).
    #[cfg(feature = "test-tooling")]
    #[allow(deprecated)]
    pub fn fixed_byte(byte: u8) -> Random<'static> {
        Random {
            source: Source::FixedByte(ring::test::rand::FixedByteRandom { byte }),
        }
    }

    /// A source that answers every draw with `bytes` (test tooling only). A draw of another
    /// length panics, so it is given a buffer of the exact length of the draws it serves: 32
    /// bytes for an X25519 or P-256 key, and for a P-256 key pair and its signatures.
    #[cfg(feature = "test-tooling")]
    #[allow(deprecated)]
    pub fn fixed_slice(bytes: &'a [u8]) -> Random<'a> {
        Random {
            source: Source::FixedSlice(ring::test::rand::FixedSliceRandom { bytes }),
        }
    }

    /// A source that answers each draw with the next of `slices`, each of the exact length of
    /// its draw (test tooling only).
    #[cfg(feature = "test-tooling")]
    #[allow(deprecated)]
    pub fn fixed_sequence(slices: &'a [&'a [u8]]) -> Random<'a> {
        Random {
            source: Source::FixedSequence(ring::test::rand::FixedSliceSequenceRandom {
                bytes: slices,
                current: core::cell::UnsafeCell::new(0),
            }),
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Hashes, HMAC and HKDF.

/// The hash of a TLS 1.3 suite.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum HashAlgorithm {
    /// SHA-256.
    Sha256,
    /// SHA-384.
    Sha384,
}

impl HashAlgorithm {
    /// The length of a digest, in bytes.
    pub fn output_len(self) -> usize {
        match self {
            HashAlgorithm::Sha256 => 32,
            HashAlgorithm::Sha384 => 48,
        }
    }

    fn digest(self) -> &'static digest::Algorithm {
        match self {
            HashAlgorithm::Sha256 => &digest::SHA256,
            HashAlgorithm::Sha384 => &digest::SHA384,
        }
    }

    fn hmac(self) -> hmac::Algorithm {
        match self {
            HashAlgorithm::Sha256 => hmac::HMAC_SHA256,
            HashAlgorithm::Sha384 => hmac::HMAC_SHA384,
        }
    }

    fn hkdf(self) -> hkdf::Algorithm {
        match self {
            HashAlgorithm::Sha256 => hkdf::HKDF_SHA256,
            HashAlgorithm::Sha384 => hkdf::HKDF_SHA384,
        }
    }
}

/// A digest or an HMAC tag: at most 48 bytes.
#[derive(Clone, PartialEq, Eq)]
pub struct Digest {
    bytes: [u8; 48],
    len: usize,
}

impl Digest {
    fn new(value: &[u8]) -> Self {
        let mut bytes = [0u8; 48];
        bytes[..value.len()].copy_from_slice(value);
        Digest {
            bytes,
            len: value.len(),
        }
    }
}

impl AsRef<[u8]> for Digest {
    fn as_ref(&self) -> &[u8] {
        &self.bytes[..self.len]
    }
}

impl fmt::Debug for Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Digest({} bytes)", self.len)
    }
}

/// The hash of `data`.
pub fn hash(algorithm: HashAlgorithm, data: &[u8]) -> Digest {
    Digest::new(digest::digest(algorithm.digest(), data).as_ref())
}

/// A hash fed in parts, such as the transcript of a handshake.
#[derive(Clone)]
pub struct RunningHash {
    context: digest::Context,
    algorithm: HashAlgorithm,
}

impl RunningHash {
    /// A hash of nothing yet.
    pub fn new(algorithm: HashAlgorithm) -> Self {
        RunningHash {
            context: digest::Context::new(algorithm.digest()),
            algorithm,
        }
    }

    /// The algorithm.
    pub fn algorithm(&self) -> HashAlgorithm {
        self.algorithm
    }

    /// Feeds `data`.
    pub fn update(&mut self, data: &[u8]) {
        self.context.update(data);
    }

    /// The hash of what was fed so far; feeding may go on.
    pub fn current(&self) -> Digest {
        Digest::new(self.context.clone().finish().as_ref())
    }

    /// The hash of everything fed.
    pub fn finish(self) -> Digest {
        Digest::new(self.context.finish().as_ref())
    }
}

impl fmt::Debug for RunningHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "RunningHash({:?})", self.algorithm)
    }
}

/// HMAC of `data` under `key` (RFC 2104), as the Finished message needs it.
pub fn hmac_sign(algorithm: HashAlgorithm, key: &[u8], data: &[u8]) -> Digest {
    Digest::new(hmac::sign(&hmac::Key::new(algorithm.hmac(), key), data).as_ref())
}

/// Checks in constant time that `tag` is the HMAC of `data` under `key`.
pub fn hmac_verify(
    algorithm: HashAlgorithm,
    key: &[u8],
    data: &[u8],
    tag: &[u8],
) -> Result<(), CryptoError> {
    hmac::verify(&hmac::Key::new(algorithm.hmac(), key), data, tag)
        .map_err(|ring::error::Unspecified| CryptoError::BadSignature)
}

/// HKDF-Extract (RFC 5869 section 2.2): the pseudorandom key of `ikm` under `salt`, which is
/// HMAC with `salt` as the key. An empty salt is a salt of zeros.
pub fn hkdf_extract(algorithm: HashAlgorithm, salt: &[u8], ikm: &[u8]) -> Digest {
    hmac_sign(algorithm, salt, ikm)
}

struct OutputLen(usize);

impl hkdf::KeyType for OutputLen {
    fn len(&self) -> usize {
        self.0
    }
}

/// HKDF-Expand (RFC 5869 section 2.3): fills `out` from the pseudorandom key `prk` and the
/// parts of `info`, concatenated.
pub fn hkdf_expand(
    algorithm: HashAlgorithm,
    prk: &[u8],
    info: &[&[u8]],
    out: &mut [u8],
) -> Result<(), CryptoError> {
    let prk = hkdf::Prk::new_less_safe(algorithm.hkdf(), prk);
    prk.expand(info, OutputLen(out.len()))
        .and_then(|okm| okm.fill(out))
        .map_err(|ring::error::Unspecified| CryptoError::OutputTooLong)
}

// ---------------------------------------------------------------------------------------------
// AEAD.

/// The AEAD of a TLS 1.3 suite.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AeadAlgorithm {
    /// AES-128-GCM (TLS_AES_128_GCM_SHA256).
    Aes128Gcm,
    /// AES-256-GCM (TLS_AES_256_GCM_SHA384).
    Aes256Gcm,
    /// ChaCha20-Poly1305 (TLS_CHACHA20_POLY1305_SHA256).
    ChaCha20Poly1305,
}

impl AeadAlgorithm {
    /// The length of a nonce: 96 bits for each.
    pub const NONCE_LEN: usize = 12;
    /// The length of a tag: 128 bits for each.
    pub const TAG_LEN: usize = 16;

    /// The length of a key, in bytes.
    pub fn key_len(self) -> usize {
        match self {
            AeadAlgorithm::Aes128Gcm => 16,
            AeadAlgorithm::Aes256Gcm | AeadAlgorithm::ChaCha20Poly1305 => 32,
        }
    }

    fn ring(self) -> &'static aead::Algorithm {
        match self {
            AeadAlgorithm::Aes128Gcm => &aead::AES_128_GCM,
            AeadAlgorithm::Aes256Gcm => &aead::AES_256_GCM,
            AeadAlgorithm::ChaCha20Poly1305 => &aead::CHACHA20_POLY1305,
        }
    }
}

/// An AEAD key. The caller gives each record its own nonce (RFC 8446 section 5.3).
pub struct AeadKey {
    key: aead::LessSafeKey,
    algorithm: AeadAlgorithm,
}

impl AeadKey {
    /// The key `key` of `algorithm`.
    pub fn new(algorithm: AeadAlgorithm, key: &[u8]) -> Result<Self, CryptoError> {
        let unbound = aead::UnboundKey::new(algorithm.ring(), key)
            .map_err(|ring::error::Unspecified| CryptoError::KeyLength)?;
        Ok(AeadKey {
            key: aead::LessSafeKey::new(unbound),
            algorithm,
        })
    }

    /// The algorithm.
    pub fn algorithm(&self) -> AeadAlgorithm {
        self.algorithm
    }

    /// Seals `in_out` in place under `nonce` and `aad`, and appends the tag.
    pub fn seal(&self, nonce: &[u8], aad: &[u8], in_out: &mut Vec<u8>) -> Result<(), CryptoError> {
        let nonce = aead::Nonce::try_assume_unique_for_key(nonce)
            .map_err(|ring::error::Unspecified| CryptoError::NonceLength)?;
        self.key
            .seal_in_place_append_tag(nonce, aead::Aad::from(aad), in_out)
            .map_err(|ring::error::Unspecified| CryptoError::Unspecified)
    }

    /// Opens `in_out`, ciphertext and tag, in place under `nonce` and `aad`: the plaintext, at
    /// the start of `in_out`.
    pub fn open<'b>(
        &self,
        nonce: &[u8],
        aad: &[u8],
        in_out: &'b mut [u8],
    ) -> Result<&'b mut [u8], CryptoError> {
        let nonce = aead::Nonce::try_assume_unique_for_key(nonce)
            .map_err(|ring::error::Unspecified| CryptoError::NonceLength)?;
        self.key
            .open_in_place(nonce, aead::Aad::from(aad), in_out)
            .map_err(|ring::error::Unspecified| CryptoError::OpenFailed)
    }
}

impl fmt::Debug for AeadKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "AeadKey({:?})", self.algorithm)
    }
}

// ---------------------------------------------------------------------------------------------
// Key agreement.

/// A group of the key exchange.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Group {
    /// X25519 (RFC 7748).
    X25519,
    /// secp256r1, P-256; a public key is the uncompressed point of 65 bytes.
    Secp256r1,
}

impl Group {
    fn ring(self) -> &'static agreement::Algorithm {
        match self {
            Group::X25519 => &agreement::X25519,
            Group::Secp256r1 => &agreement::ECDH_P256,
        }
    }
}

/// A shared secret of an agreement, or another secret the caller derives keys from.
#[derive(Clone, PartialEq, Eq)]
pub struct Secret(Vec<u8>);

impl AsRef<[u8]> for Secret {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Secret({} bytes)", self.0.len())
    }
}

/// An ephemeral private key and its public key, used for one agreement.
pub struct EphemeralKey {
    private: agreement::EphemeralPrivateKey,
    public: agreement::PublicKey,
    group: Group,
}

impl EphemeralKey {
    /// A key of `group` drawn from `random`: 32 bytes for either group.
    pub fn generate(group: Group, random: &Random<'_>) -> Result<Self, CryptoError> {
        let private = agreement::EphemeralPrivateKey::generate(group.ring(), random.ring())
            .map_err(|ring::error::Unspecified| CryptoError::KeyRejected)?;
        let public = private
            .compute_public_key()
            .map_err(|ring::error::Unspecified| CryptoError::KeyRejected)?;
        Ok(EphemeralKey {
            private,
            public,
            group,
        })
    }

    /// The group.
    pub fn group(&self) -> Group {
        self.group
    }

    /// The public key, as a key share carries it.
    pub fn public_key(&self) -> &[u8] {
        self.public.as_ref()
    }

    /// The shared secret with the peer's public key `peer`; refused when the key is not a valid
    /// key of the group or the X25519 result is all zeros.
    pub fn agree(self, peer: &[u8]) -> Result<Secret, CryptoError> {
        let peer = agreement::UnparsedPublicKey::new(self.group.ring(), peer);
        agreement::agree_ephemeral(self.private, &peer, |shared| Secret(shared.to_vec()))
            .map_err(|ring::error::Unspecified| CryptoError::BadPublicKey)
    }
}

impl fmt::Debug for EphemeralKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "EphemeralKey({:?})", self.group)
    }
}

// ---------------------------------------------------------------------------------------------
// Signatures.

/// The hash of a signature algorithm, the refused ones included.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SignatureHash {
    /// MD5, refused.
    Md5,
    /// SHA-1, refused.
    Sha1,
    /// SHA-256.
    Sha256,
    /// SHA-384.
    Sha384,
    /// SHA-512.
    Sha512,
}

/// A signature algorithm as a certificate or a handshake names it: the kind and the hash.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SignatureAlgorithm {
    /// RSASSA-PKCS1-v1_5.
    RsaPkcs1(SignatureHash),
    /// RSASSA-PSS with MGF1 of the same hash and a salt of the digest's length.
    RsaPss(SignatureHash),
    /// ECDSA.
    Ecdsa(SignatureHash),
    /// Ed25519.
    Ed25519,
}

impl SignatureAlgorithm {
    /// The name the algorithm has in a certificate (RFC 3279, 4055, 5758, 8410).
    pub fn name(self) -> &'static str {
        use SignatureHash::{Md5, Sha1, Sha256, Sha384, Sha512};
        match self {
            SignatureAlgorithm::RsaPkcs1(Md5) => "md5WithRSAEncryption",
            SignatureAlgorithm::RsaPkcs1(Sha1) => "sha1WithRSAEncryption",
            SignatureAlgorithm::RsaPkcs1(Sha256) => "sha256WithRSAEncryption",
            SignatureAlgorithm::RsaPkcs1(Sha384) => "sha384WithRSAEncryption",
            SignatureAlgorithm::RsaPkcs1(Sha512) => "sha512WithRSAEncryption",
            SignatureAlgorithm::RsaPss(Md5) => "RSASSA-PSS with MD5",
            SignatureAlgorithm::RsaPss(Sha1) => "RSASSA-PSS with SHA-1",
            SignatureAlgorithm::RsaPss(Sha256) => "RSASSA-PSS with SHA-256",
            SignatureAlgorithm::RsaPss(Sha384) => "RSASSA-PSS with SHA-384",
            SignatureAlgorithm::RsaPss(Sha512) => "RSASSA-PSS with SHA-512",
            SignatureAlgorithm::Ecdsa(Md5) => "ecdsa-with-MD5",
            SignatureAlgorithm::Ecdsa(Sha1) => "ecdsa-with-SHA1",
            SignatureAlgorithm::Ecdsa(Sha256) => "ecdsa-with-SHA256",
            SignatureAlgorithm::Ecdsa(Sha384) => "ecdsa-with-SHA384",
            SignatureAlgorithm::Ecdsa(Sha512) => "ecdsa-with-SHA512",
            SignatureAlgorithm::Ed25519 => "Ed25519",
        }
    }

    fn hash(self) -> Option<SignatureHash> {
        match self {
            SignatureAlgorithm::RsaPkcs1(h)
            | SignatureAlgorithm::RsaPss(h)
            | SignatureAlgorithm::Ecdsa(h) => Some(h),
            SignatureAlgorithm::Ed25519 => None,
        }
    }
}

/// The kind of a public key, with what [`REFUSED`] reads of it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PublicKeyKind {
    /// RSA; the key bytes are an RSAPublicKey (RFC 8017, DER).
    Rsa {
        /// The length of the modulus in bits.
        bits: usize,
    },
    /// ECDSA on P-256; the key bytes are the uncompressed point.
    EcP256,
    /// ECDSA on P-384; the key bytes are the uncompressed point.
    EcP384,
    /// Ed25519; the key bytes are the 32 bytes of RFC 8032.
    Ed25519,
}

/// A public key: its kind and its bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PublicKey<'a> {
    /// The kind.
    pub kind: PublicKeyKind,
    /// The bytes, in the form [`PublicKeyKind`] names.
    pub bytes: &'a [u8],
}

/// A signature scheme the seam verifies (RFC 8446 section 4.2.3; docs/design/tls.md, item 14).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Scheme {
    /// rsa_pkcs1_sha256 (0x0401), certificates only.
    RsaPkcs1Sha256,
    /// rsa_pkcs1_sha384 (0x0501), certificates only.
    RsaPkcs1Sha384,
    /// rsa_pkcs1_sha512 (0x0601), certificates only.
    RsaPkcs1Sha512,
    /// rsa_pss_rsae_sha256 (0x0804).
    RsaPssRsaeSha256,
    /// rsa_pss_rsae_sha384 (0x0805).
    RsaPssRsaeSha384,
    /// rsa_pss_rsae_sha512 (0x0806).
    RsaPssRsaeSha512,
    /// ecdsa_secp256r1_sha256 (0x0403).
    EcdsaSecp256r1Sha256,
    /// ecdsa_secp384r1_sha384 (0x0503).
    EcdsaSecp384r1Sha384,
    /// ed25519 (0x0807).
    Ed25519,
}

impl Scheme {
    /// The nine schemes, in the order of item 14's `signature_algorithms_cert`.
    pub const ALL: [Scheme; 9] = [
        Scheme::EcdsaSecp256r1Sha256,
        Scheme::EcdsaSecp384r1Sha384,
        Scheme::Ed25519,
        Scheme::RsaPssRsaeSha256,
        Scheme::RsaPssRsaeSha384,
        Scheme::RsaPssRsaeSha512,
        Scheme::RsaPkcs1Sha256,
        Scheme::RsaPkcs1Sha384,
        Scheme::RsaPkcs1Sha512,
    ];

    /// The code point of RFC 8446 section 4.2.3.
    pub fn code(self) -> u16 {
        match self {
            Scheme::RsaPkcs1Sha256 => 0x0401,
            Scheme::RsaPkcs1Sha384 => 0x0501,
            Scheme::RsaPkcs1Sha512 => 0x0601,
            Scheme::RsaPssRsaeSha256 => 0x0804,
            Scheme::RsaPssRsaeSha384 => 0x0805,
            Scheme::RsaPssRsaeSha512 => 0x0806,
            Scheme::EcdsaSecp256r1Sha256 => 0x0403,
            Scheme::EcdsaSecp384r1Sha384 => 0x0503,
            Scheme::Ed25519 => 0x0807,
        }
    }

    /// The scheme of a code point, if the seam verifies it.
    pub fn from_code(code: u16) -> Option<Scheme> {
        Scheme::ALL.into_iter().find(|s| s.code() == code)
    }

    /// The name of RFC 8446 section 4.2.3.
    pub fn name(self) -> &'static str {
        match self {
            Scheme::RsaPkcs1Sha256 => "rsa_pkcs1_sha256",
            Scheme::RsaPkcs1Sha384 => "rsa_pkcs1_sha384",
            Scheme::RsaPkcs1Sha512 => "rsa_pkcs1_sha512",
            Scheme::RsaPssRsaeSha256 => "rsa_pss_rsae_sha256",
            Scheme::RsaPssRsaeSha384 => "rsa_pss_rsae_sha384",
            Scheme::RsaPssRsaeSha512 => "rsa_pss_rsae_sha512",
            Scheme::EcdsaSecp256r1Sha256 => "ecdsa_secp256r1_sha256",
            Scheme::EcdsaSecp384r1Sha384 => "ecdsa_secp384r1_sha384",
            Scheme::Ed25519 => "ed25519",
        }
    }

    /// The signature algorithm the scheme is.
    pub fn algorithm(self) -> SignatureAlgorithm {
        use SignatureHash::{Sha256, Sha384, Sha512};
        match self {
            Scheme::RsaPkcs1Sha256 => SignatureAlgorithm::RsaPkcs1(Sha256),
            Scheme::RsaPkcs1Sha384 => SignatureAlgorithm::RsaPkcs1(Sha384),
            Scheme::RsaPkcs1Sha512 => SignatureAlgorithm::RsaPkcs1(Sha512),
            Scheme::RsaPssRsaeSha256 => SignatureAlgorithm::RsaPss(Sha256),
            Scheme::RsaPssRsaeSha384 => SignatureAlgorithm::RsaPss(Sha384),
            Scheme::RsaPssRsaeSha512 => SignatureAlgorithm::RsaPss(Sha512),
            Scheme::EcdsaSecp256r1Sha256 => SignatureAlgorithm::Ecdsa(Sha256),
            Scheme::EcdsaSecp384r1Sha384 => SignatureAlgorithm::Ecdsa(Sha384),
            Scheme::Ed25519 => SignatureAlgorithm::Ed25519,
        }
    }

    fn verification(self) -> &'static dyn signature::VerificationAlgorithm {
        match self {
            Scheme::RsaPkcs1Sha256 => &signature::RSA_PKCS1_2048_8192_SHA256,
            Scheme::RsaPkcs1Sha384 => &signature::RSA_PKCS1_2048_8192_SHA384,
            Scheme::RsaPkcs1Sha512 => &signature::RSA_PKCS1_2048_8192_SHA512,
            Scheme::RsaPssRsaeSha256 => &signature::RSA_PSS_2048_8192_SHA256,
            Scheme::RsaPssRsaeSha384 => &signature::RSA_PSS_2048_8192_SHA384,
            Scheme::RsaPssRsaeSha512 => &signature::RSA_PSS_2048_8192_SHA512,
            Scheme::EcdsaSecp256r1Sha256 => &signature::ECDSA_P256_SHA256_ASN1,
            Scheme::EcdsaSecp384r1Sha384 => &signature::ECDSA_P384_SHA384_ASN1,
            Scheme::Ed25519 => &signature::ED25519,
        }
    }
}

/// What the seam refuses by name, before any verification.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Refused {
    /// Every signature with one of these hashes: MD5 and SHA-1.
    pub hashes: &'static [SignatureHash],
    /// The fewest bits an RSA key may have: 2048.
    pub rsa_bits_min: usize,
}

/// The algorithms the seam refuses by name: every MD5 or SHA-1 signature, and RSA keys under
/// 2048 bits.
pub const REFUSED: Refused = Refused {
    hashes: &[SignatureHash::Md5, SignatureHash::Sha1],
    rsa_bits_min: 2048,
};

/// A refusal of [`REFUSED`], named as the check of docs/design/tls.md item 7 names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// An MD5 or SHA-1 signature.
    WeakAlgorithm {
        /// The algorithm's name ([`SignatureAlgorithm::name`]).
        algorithm: &'static str,
    },
    /// An RSA key under 2048 bits.
    WeakKey {
        /// The length of its modulus.
        bits: usize,
    },
}

impl Refusal {
    /// The name of the check: `WeakAlgorithm` or `WeakKey`.
    pub fn check(self) -> &'static str {
        match self {
            Refusal::WeakAlgorithm { .. } => "WeakAlgorithm",
            Refusal::WeakKey { .. } => "WeakKey",
        }
    }
}

/// The refusal of [`REFUSED`] for a signature by `algorithm` under a key of `key`, if any;
/// it reads no signature and verifies nothing. A weak algorithm is named before a weak key.
pub fn refusal(algorithm: SignatureAlgorithm, key: PublicKeyKind) -> Option<Refusal> {
    if let Some(hash) = algorithm.hash()
        && REFUSED.hashes.contains(&hash)
    {
        return Some(Refusal::WeakAlgorithm {
            algorithm: algorithm.name(),
        });
    }
    match key {
        PublicKeyKind::Rsa { bits } if bits < REFUSED.rsa_bits_min => {
            Some(Refusal::WeakKey { bits })
        }
        _ => None,
    }
}

/// The scheme that checks a signature by `algorithm` under a key of `key`, if the seam has one.
pub fn scheme(algorithm: SignatureAlgorithm, key: PublicKeyKind) -> Option<Scheme> {
    use SignatureHash::{Sha256, Sha384, Sha512};
    let scheme = match (algorithm, key) {
        (SignatureAlgorithm::RsaPkcs1(Sha256), PublicKeyKind::Rsa { .. }) => Scheme::RsaPkcs1Sha256,
        (SignatureAlgorithm::RsaPkcs1(Sha384), PublicKeyKind::Rsa { .. }) => Scheme::RsaPkcs1Sha384,
        (SignatureAlgorithm::RsaPkcs1(Sha512), PublicKeyKind::Rsa { .. }) => Scheme::RsaPkcs1Sha512,
        (SignatureAlgorithm::RsaPss(Sha256), PublicKeyKind::Rsa { .. }) => Scheme::RsaPssRsaeSha256,
        (SignatureAlgorithm::RsaPss(Sha384), PublicKeyKind::Rsa { .. }) => Scheme::RsaPssRsaeSha384,
        (SignatureAlgorithm::RsaPss(Sha512), PublicKeyKind::Rsa { .. }) => Scheme::RsaPssRsaeSha512,
        (SignatureAlgorithm::Ecdsa(Sha256), PublicKeyKind::EcP256) => Scheme::EcdsaSecp256r1Sha256,
        (SignatureAlgorithm::Ecdsa(Sha384), PublicKeyKind::EcP384) => Scheme::EcdsaSecp384r1Sha384,
        (SignatureAlgorithm::Ed25519, PublicKeyKind::Ed25519) => Scheme::Ed25519,
        _ => return None,
    };
    Some(scheme)
}

/// Verifies a signature of one scheme. The shipped one is [`ShippedVerifier`]; a test may give
/// its own to the checks that take one.
pub trait SignatureVerifier {
    /// Whether `signature` is the signature of `message` under `public_key` in `scheme`.
    fn verify(
        &self,
        scheme: Scheme,
        public_key: &[u8],
        message: &[u8],
        signature: &[u8],
    ) -> Result<(), CryptoError>;
}

/// The verifier of the primitives.
#[derive(Clone, Copy, Debug, Default)]
pub struct ShippedVerifier;

impl SignatureVerifier for ShippedVerifier {
    fn verify(
        &self,
        scheme: Scheme,
        public_key: &[u8],
        message: &[u8],
        signature: &[u8],
    ) -> Result<(), CryptoError> {
        signature::UnparsedPublicKey::new(scheme.verification(), public_key)
            .verify(message, signature)
            .map_err(|ring::error::Unspecified| CryptoError::BadSignature)
    }
}

/// Why [`check_signature`] did not accept a signature.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SignatureError {
    /// Refused by name, before any verification.
    Refused(Refusal),
    /// An algorithm and key the seam has no scheme for.
    Unsupported,
    /// The signature does not verify.
    BadSignature,
}

/// Checks a signature: first [`refusal`], which verifies nothing; then the scheme of the
/// algorithm and key; then `verifier`. The scheme that verified it.
pub fn check_signature(
    verifier: &dyn SignatureVerifier,
    algorithm: SignatureAlgorithm,
    key: &PublicKey<'_>,
    message: &[u8],
    signature: &[u8],
) -> Result<Scheme, SignatureError> {
    if let Some(refused) = refusal(algorithm, key.kind) {
        return Err(SignatureError::Refused(refused));
    }
    let scheme = scheme(algorithm, key.kind).ok_or(SignatureError::Unsupported)?;
    verifier
        .verify(scheme, key.bytes, message, signature)
        .map_err(|_| SignatureError::BadSignature)?;
    Ok(scheme)
}

// ---------------------------------------------------------------------------------------------
// Signing, test tooling only.

#[cfg(feature = "test-tooling")]
enum Signer {
    Ecdsa(signature::EcdsaKeyPair),
    Ed25519(signature::Ed25519KeyPair),
    Rsa(signature::RsaKeyPair),
}

/// A private key read from PKCS#8 that signs in one scheme (test tooling only): ECDSA P-256
/// and P-384, Ed25519 (PKCS#8 v1 as `openssl genpkey` writes it, or v2), and RSA PKCS#1 and
/// PSS from 2048 bits.
#[cfg(feature = "test-tooling")]
pub struct SigningKey {
    signer: Signer,
    scheme: Scheme,
}

#[cfg(feature = "test-tooling")]
impl SigningKey {
    /// The key of `pkcs8` for `scheme`. An ECDSA key pair draws its nonce key from `random`
    /// here: 32 bytes for P-256, 48 for P-384.
    pub fn from_pkcs8(
        scheme: Scheme,
        pkcs8: &[u8],
        random: &Random<'_>,
    ) -> Result<Self, CryptoError> {
        let rejected = |_| CryptoError::KeyRejected;
        let signer = match scheme {
            Scheme::EcdsaSecp256r1Sha256 => Signer::Ecdsa(
                signature::EcdsaKeyPair::from_pkcs8(
                    &signature::ECDSA_P256_SHA256_ASN1_SIGNING,
                    pkcs8,
                    random.ring(),
                )
                .map_err(rejected)?,
            ),
            Scheme::EcdsaSecp384r1Sha384 => Signer::Ecdsa(
                signature::EcdsaKeyPair::from_pkcs8(
                    &signature::ECDSA_P384_SHA384_ASN1_SIGNING,
                    pkcs8,
                    random.ring(),
                )
                .map_err(rejected)?,
            ),
            Scheme::Ed25519 => Signer::Ed25519(
                signature::Ed25519KeyPair::from_pkcs8_maybe_unchecked(pkcs8).map_err(rejected)?,
            ),
            _ => Signer::Rsa(signature::RsaKeyPair::from_pkcs8(pkcs8).map_err(rejected)?),
        };
        Ok(SigningKey { signer, scheme })
    }

    /// The scheme it signs in.
    pub fn scheme(&self) -> Scheme {
        self.scheme
    }

    /// The public key, in the form [`check_signature`] takes.
    pub fn public_key(&self) -> PublicKey<'_> {
        use signature::KeyPair;
        match &self.signer {
            Signer::Ecdsa(pair) => PublicKey {
                kind: if self.scheme == Scheme::EcdsaSecp256r1Sha256 {
                    PublicKeyKind::EcP256
                } else {
                    PublicKeyKind::EcP384
                },
                bytes: pair.public_key().as_ref(),
            },
            Signer::Ed25519(pair) => PublicKey {
                kind: PublicKeyKind::Ed25519,
                bytes: pair.public_key().as_ref(),
            },
            Signer::Rsa(pair) => PublicKey {
                kind: PublicKeyKind::Rsa {
                    bits: pair.public().modulus_len() * 8,
                },
                bytes: pair.public_key().as_ref(),
            },
        }
    }

    /// The signature of `message`. ECDSA draws the block length of the digest less the nonce
    /// key (32 bytes for P-256, 80 for P-384), RSA-PSS a salt of the digest's length; PKCS#1
    /// and Ed25519 draw nothing.
    pub fn sign(&self, message: &[u8], random: &Random<'_>) -> Result<Vec<u8>, CryptoError> {
        let failed = |ring::error::Unspecified| CryptoError::Unspecified;
        match &self.signer {
            Signer::Ecdsa(pair) => Ok(pair
                .sign(random.ring(), message)
                .map_err(failed)?
                .as_ref()
                .to_vec()),
            Signer::Ed25519(pair) => Ok(pair.sign(message).as_ref().to_vec()),
            Signer::Rsa(pair) => {
                let padding: &'static dyn signature::RsaEncoding = match self.scheme {
                    Scheme::RsaPkcs1Sha256 => &signature::RSA_PKCS1_SHA256,
                    Scheme::RsaPkcs1Sha384 => &signature::RSA_PKCS1_SHA384,
                    Scheme::RsaPkcs1Sha512 => &signature::RSA_PKCS1_SHA512,
                    Scheme::RsaPssRsaeSha256 => &signature::RSA_PSS_SHA256,
                    Scheme::RsaPssRsaeSha384 => &signature::RSA_PSS_SHA384,
                    _ => &signature::RSA_PSS_SHA512,
                };
                let mut out = vec![0u8; pair.public().modulus_len()];
                pair.sign(padding, random.ring(), message, &mut out)
                    .map_err(failed)?;
                Ok(out)
            }
        }
    }
}

#[cfg(feature = "test-tooling")]
impl fmt::Debug for SigningKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SigningKey({:?})", self.scheme)
    }
}
