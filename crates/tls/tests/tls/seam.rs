//! The seam, `surf_tls::crypto`: the Wycheproof vectors of `tests/data/wycheproof/` (one test
//! per file), the hashes against FIPS 180-4, signing under the feature `test-tooling`, the
//! refusals by name, the digests of the vendored files, and that no file of the crate but
//! `crypto.rs` names ring (docs/design/tls.md, items 5, 6 and 10).

#![forbid(unsafe_code)]

use std::cell::Cell;
use std::fs;
use std::path::{Path, PathBuf};

use surf_html_conformance::json::{self, Value};
use surf_tls::crypto::{
    self, AeadAlgorithm, AeadKey, CryptoError, EphemeralKey, Group, HashAlgorithm, PublicKey,
    PublicKeyKind, REFUSED, Random, Refusal, RunningHash, Scheme, ShippedVerifier,
    SignatureAlgorithm, SignatureError, SignatureHash, SignatureVerifier, SigningKey,
};

// ---------------------------------------------------------------------------------------------
// Helpers.

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn wycheproof_dir() -> PathBuf {
    crate_dir().join("tests/data/wycheproof")
}

fn hex(s: &str) -> Vec<u8> {
    assert!(s.len().is_multiple_of(2), "odd hex {s:?}");
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap_or_else(|_| panic!("hex {s:?}")))
        .collect()
}

fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// The string member `key` of `v`.
fn text(v: &Value, key: &str) -> String {
    match v.get(key) {
        Some(Value::String(points)) => points
            .iter()
            .map(|&p| char::from_u32(p).expect("a scalar value"))
            .collect(),
        other => panic!("member {key}: {other:?}"),
    }
}

/// The number member `key` of `v`.
fn number(v: &Value, key: &str) -> usize {
    match v.get(key) {
        Some(Value::Number(n)) => n.parse().unwrap_or_else(|_| panic!("number {n}")),
        other => panic!("member {key}: {other:?}"),
    }
}

/// The hex string member `key` of `v`, as bytes.
fn bytes(v: &Value, key: &str) -> Vec<u8> {
    hex(&text(v, key))
}

fn array<'a>(v: &'a Value, key: &str) -> &'a [Value] {
    match v.get(key) {
        Some(Value::Array(items)) => items,
        other => panic!("member {key}: {other:?}"),
    }
}

/// What the seam did with one vector.
enum Verdict {
    /// The operation succeeded and gave the expected output.
    Accepted,
    /// The operation was refused.
    Refused,
    /// The operation succeeded with another output than the vector's.
    Wrong(String),
    /// The vector is not run, for the named reason.
    Skip(&'static str),
}

/// The counts of one file, as the test prints them.
struct Tally {
    vectors: usize,
    valid: usize,
    invalid: usize,
    acceptable: usize,
    skipped: Vec<(&'static str, usize)>,
}

impl Tally {
    fn skipped(&self, reason: &str) -> usize {
        self.skipped
            .iter()
            .find(|(r, _)| *r == reason)
            .map_or(0, |(_, n)| *n)
    }
}

/// Runs `check` on every vector of `file`: every `valid` one must be accepted, every `invalid`
/// one refused; `acceptable` ones are counted whatever the seam did, unless it gave a wrong
/// output. Prints the line of the file and the skips by name, and fails naming each vector
/// that went wrong.
fn run(file: &str, check: impl Fn(&Value, &Value) -> Verdict) -> Tally {
    let data = fs::read_to_string(wycheproof_dir().join(file)).expect("read the vectors");
    let root = json::parse(&data).expect("the vectors are JSON");
    let mut tally = Tally {
        vectors: 0,
        valid: 0,
        invalid: 0,
        acceptable: 0,
        skipped: Vec::new(),
    };
    let mut failures = Vec::new();
    for group in array(&root, "testGroups") {
        for test in array(group, "tests") {
            tally.vectors += 1;
            let id = number(test, "tcId");
            let result = text(test, "result");
            match (check(group, test), result.as_str()) {
                (Verdict::Skip(reason), _) => {
                    match tally.skipped.iter_mut().find(|(r, _)| *r == reason) {
                        Some((_, n)) => *n += 1,
                        None => tally.skipped.push((reason, 1)),
                    }
                }
                (Verdict::Wrong(why), _) => failures.push(format!("tcId {id} ({result}): {why}")),
                (Verdict::Accepted, "valid") => tally.valid += 1,
                (Verdict::Refused, "invalid") => tally.invalid += 1,
                (_, "acceptable") => tally.acceptable += 1,
                (Verdict::Refused, "valid") => failures.push(format!("tcId {id}: valid, refused")),
                (Verdict::Accepted, "invalid") => {
                    failures.push(format!("tcId {id}: invalid, accepted"))
                }
                (_, other) => failures.push(format!("tcId {id}: result {other:?}")),
            }
        }
    }
    let skipped: usize = tally.skipped.iter().map(|(_, n)| n).sum();
    println!(
        "wycheproof {file}: {} vectors, {} valid passed, {} invalid refused, {} acceptable, {skipped} skipped",
        tally.vectors, tally.valid, tally.invalid, tally.acceptable
    );
    for (reason, n) in &tally.skipped {
        println!("  skipped {n}: {reason}");
    }
    assert!(
        failures.is_empty(),
        "wycheproof {file}: {} vector(s) went wrong:\n{}",
        failures.len(),
        failures.join("\n")
    );
    assert_eq!(
        tally.vectors,
        tally.valid + tally.invalid + tally.acceptable + skipped,
        "{file}: every vector counted once"
    );
    tally
}

/// A Wycheproof AEAD vector through the seam: open the ciphertext and tag, and seal the message.
fn aead(algorithm: AeadAlgorithm, test: &Value) -> Verdict {
    let (key, nonce, aad) = (bytes(test, "key"), bytes(test, "iv"), bytes(test, "aad"));
    let (msg, ct, tag) = (bytes(test, "msg"), bytes(test, "ct"), bytes(test, "tag"));
    let Ok(key) = AeadKey::new(algorithm, &key) else {
        return Verdict::Refused;
    };
    let mut sealed_input = ct.clone();
    sealed_input.extend_from_slice(&tag);
    let mut opened = sealed_input.clone();
    let Ok(plain) = key.open(&nonce, &aad, &mut opened) else {
        return Verdict::Refused;
    };
    if plain != msg.as_slice() {
        return Verdict::Wrong(format!("opened to {}", to_hex(plain)));
    }
    let mut sealed = msg.clone();
    match key.seal(&nonce, &aad, &mut sealed) {
        Ok(()) if sealed == sealed_input => Verdict::Accepted,
        Ok(()) => Verdict::Wrong(format!("sealed to {}", to_hex(&sealed))),
        Err(e) => Verdict::Wrong(format!("opened but did not seal: {e:?}")),
    }
}

/// An agreement with the vector's private key, drawn through a fixed source, and the peer's
/// public key; the shared secret compared.
fn agree(group: Group, private: &[u8], public: &[u8], shared: &[u8]) -> Verdict {
    let random = Random::fixed_slice(private);
    let Ok(key) = EphemeralKey::generate(group, &random) else {
        return Verdict::Refused;
    };
    match key.agree(public) {
        Err(_) => Verdict::Refused,
        Ok(secret) if secret.as_ref() == shared => Verdict::Accepted,
        Ok(secret) => Verdict::Wrong(format!("shared secret {}", to_hex(secret.as_ref()))),
    }
}

/// The one TLV of `tag` at the start of `input`: its content and what follows it.
fn tlv(input: &[u8], tag: u8) -> Option<(&[u8], &[u8])> {
    let (&first, rest) = input.split_first()?;
    if first != tag {
        return None;
    }
    let (&len, rest) = rest.split_first()?;
    let (len, rest) = match len {
        0..=0x7f => (usize::from(len), rest),
        0x81 => (usize::from(*rest.first()?), &rest[1..]),
        0x82 => (
            usize::from(*rest.first()?) << 8 | usize::from(*rest.get(1)?),
            rest.get(2..)?,
        ),
        _ => return None,
    };
    (rest.len() >= len).then(|| rest.split_at(len))
}

/// The AlgorithmIdentifier of a P-256 key by name (RFC 5480 section 2.1.1): `id-ecPublicKey`
/// and `secp256r1`.
const P256_ALGORITHM: &[u8] = &[
    0x06, 0x07, 0x2a, 0x86, 0x48, 0xce, 0x3d, 0x02, 0x01, 0x06, 0x08, 0x2a, 0x86, 0x48, 0xce, 0x3d,
    0x03, 0x01, 0x07,
];

/// The point of a DER SubjectPublicKeyInfo of a P-256 key named by its OID, as RFC 5480
/// requires; `None` for any other: another curve, explicit curve parameters, or not DER of
/// that shape. In TLS a key share is the point itself; the ECDH file wraps each in a
/// SubjectPublicKeyInfo, and this test reads it as a reader of such keys must.
fn spki_p256_point(der: &[u8]) -> Option<&[u8]> {
    let (outer, rest) = tlv(der, 0x30)?;
    if !rest.is_empty() {
        return None;
    }
    let (algorithm, rest) = tlv(outer, 0x30)?;
    if algorithm != P256_ALGORITHM {
        return None;
    }
    let (bits, rest) = tlv(rest, 0x03)?;
    let (&unused, point) = bits.split_first()?;
    (rest.is_empty() && unused == 0).then_some(point)
}

/// The scalar of a Wycheproof private key as the 32 bytes a P-256 key is drawn as.
fn p256_scalar(private: &[u8]) -> Vec<u8> {
    let start = private
        .iter()
        .position(|&b| b != 0)
        .unwrap_or(private.len());
    let digits = &private[start..];
    assert!(
        digits.len() <= 32,
        "a P-256 scalar of {} bytes",
        digits.len()
    );
    let mut out = vec![0u8; 32 - digits.len()];
    out.extend_from_slice(digits);
    out
}

fn signature_hash(name: &str) -> SignatureHash {
    match name {
        "SHA-256" => SignatureHash::Sha256,
        "SHA-384" => SignatureHash::Sha384,
        "SHA-512" => SignatureHash::Sha512,
        other => panic!("hash {other}"),
    }
}

/// A signature vector through the seam's check, with the shipped verifier.
fn signature(algorithm: SignatureAlgorithm, key: PublicKey<'_>, test: &Value) -> Verdict {
    let (msg, sig) = (bytes(test, "msg"), bytes(test, "sig"));
    match crypto::check_signature(&ShippedVerifier, algorithm, &key, &msg, &sig) {
        Ok(_) => Verdict::Accepted,
        Err(_) => Verdict::Refused,
    }
}

fn hkdf(algorithm: HashAlgorithm, test: &Value) -> Verdict {
    let (ikm, salt, info) = (bytes(test, "ikm"), bytes(test, "salt"), bytes(test, "info"));
    let okm = bytes(test, "okm");
    let prk = crypto::hkdf_extract(algorithm, &salt, &ikm);
    let mut out = vec![0u8; number(test, "size")];
    match crypto::hkdf_expand(algorithm, prk.as_ref(), &[&info], &mut out) {
        Err(_) => Verdict::Refused,
        Ok(()) if out == okm => Verdict::Accepted,
        Ok(()) => Verdict::Wrong(format!("okm {}", to_hex(&out))),
    }
}

/// The value of the line `<name>: <value>` of the Wycheproof `SOURCE`.
fn source_field(name: &str) -> String {
    let source = fs::read_to_string(wycheproof_dir().join("SOURCE")).expect("read SOURCE");
    let prefix = format!("{name}: ");
    source
        .lines()
        .find_map(|l| l.strip_prefix(&prefix))
        .unwrap_or_else(|| panic!("no line {name:?} in SOURCE"))
        .to_string()
}

// ---------------------------------------------------------------------------------------------
// 9. The Wycheproof files, one test each.

#[test]
fn wycheproof_aes_gcm() {
    let tally = run("aes_gcm_test.json", |group, test| {
        let algorithm = match number(group, "keySize") {
            128 => AeadAlgorithm::Aes128Gcm,
            256 => AeadAlgorithm::Aes256Gcm,
            192 => return Verdict::Skip("AES-192"),
            other => panic!("AES key of {other} bits"),
        };
        if number(group, "ivSize") != 96 {
            return Verdict::Skip("AES-GCM nonce other than 96 bits");
        }
        aead(algorithm, test)
    });
    let aes192: usize = source_field("aes-192 vectors")
        .split(',')
        .next()
        .and_then(|n| n.parse().ok())
        .expect("a count in SOURCE");
    assert_eq!(
        tally.skipped("AES-192"),
        aes192,
        "the AES-192 count of SOURCE"
    );
}

#[test]
fn wycheproof_chacha20_poly1305() {
    run("chacha20_poly1305_test.json", |_, test| {
        aead(AeadAlgorithm::ChaCha20Poly1305, test)
    });
}

#[test]
fn wycheproof_x25519() {
    run("x25519_test.json", |_, test| {
        agree(
            Group::X25519,
            &bytes(test, "private"),
            &bytes(test, "public"),
            &bytes(test, "shared"),
        )
    });
}

#[test]
fn wycheproof_ecdh_secp256r1() {
    let (by_reader, invalid_by_reader) = (Cell::new(0usize), Cell::new(0usize));
    let tally = run("ecdh_secp256r1_test.json", |_, test| {
        let public = bytes(test, "public");
        let Some(point) = spki_p256_point(&public) else {
            by_reader.set(by_reader.get() + 1);
            if text(test, "result") == "invalid" {
                invalid_by_reader.set(invalid_by_reader.get() + 1);
            }
            return Verdict::Refused;
        };
        agree(
            Group::Secp256r1,
            &p256_scalar(&bytes(test, "private")),
            point,
            &bytes(test, "shared"),
        )
    });
    // The refusals of the test's reader are counted apart, so that the line above does not
    // credit the seam with them.
    println!(
        "  refused by the SubjectPublicKeyInfo reader of the test, not the seam: {} ({} of them invalid; no P-256 by name, or not DER)",
        by_reader.get(),
        invalid_by_reader.get()
    );
    assert!(by_reader.get() <= tally.invalid + tally.acceptable);
}

#[test]
fn wycheproof_ecdsa_secp256r1_sha256() {
    run("ecdsa_secp256r1_sha256_test.json", |group, test| {
        let key = bytes(group.get("publicKey").expect("publicKey"), "uncompressed");
        let key = PublicKey {
            kind: PublicKeyKind::EcP256,
            bytes: &key,
        };
        signature(SignatureAlgorithm::Ecdsa(SignatureHash::Sha256), key, test)
    });
}

#[test]
fn wycheproof_ecdsa_secp384r1_sha384() {
    run("ecdsa_secp384r1_sha384_test.json", |group, test| {
        let key = bytes(group.get("publicKey").expect("publicKey"), "uncompressed");
        let key = PublicKey {
            kind: PublicKeyKind::EcP384,
            bytes: &key,
        };
        signature(SignatureAlgorithm::Ecdsa(SignatureHash::Sha384), key, test)
    });
}

#[test]
fn wycheproof_ed25519() {
    run("ed25519_test.json", |group, test| {
        let key = bytes(group.get("publicKey").expect("publicKey"), "pk");
        let key = PublicKey {
            kind: PublicKeyKind::Ed25519,
            bytes: &key,
        };
        signature(SignatureAlgorithm::Ed25519, key, test)
    });
}

#[test]
fn wycheproof_hkdf_sha256() {
    run("hkdf_sha256_test.json", |_, test| {
        hkdf(HashAlgorithm::Sha256, test)
    });
}

#[test]
fn wycheproof_hkdf_sha384() {
    run("hkdf_sha384_test.json", |_, test| {
        hkdf(HashAlgorithm::Sha384, test)
    });
}

fn rsa_pkcs1(file: &str) {
    run(file, |group, test| {
        let key = bytes(group, "publicKeyAsn");
        let key = PublicKey {
            kind: PublicKeyKind::Rsa {
                bits: number(group, "keySize"),
            },
            bytes: &key,
        };
        let hash = signature_hash(&text(group, "sha"));
        signature(SignatureAlgorithm::RsaPkcs1(hash), key, test)
    });
}

fn rsa_pss(file: &str) {
    run(file, |group, test| {
        let hash = signature_hash(&text(group, "sha"));
        assert_eq!(
            text(group, "mgfSha"),
            text(group, "sha"),
            "{file}: MGF1 hash"
        );
        let digest_len = match hash {
            SignatureHash::Sha256 => 32,
            SignatureHash::Sha384 => 48,
            _ => 64,
        };
        assert_eq!(number(group, "sLen"), digest_len, "{file}: salt length");
        let key = bytes(group, "publicKeyAsn");
        let key = PublicKey {
            kind: PublicKeyKind::Rsa {
                bits: number(group, "keySize"),
            },
            bytes: &key,
        };
        signature(SignatureAlgorithm::RsaPss(hash), key, test)
    });
}

#[test]
fn wycheproof_rsa_signature_2048_sha256() {
    rsa_pkcs1("rsa_signature_2048_sha256_test.json");
}

#[test]
fn wycheproof_rsa_signature_2048_sha384() {
    rsa_pkcs1("rsa_signature_2048_sha384_test.json");
}

#[test]
fn wycheproof_rsa_signature_2048_sha512() {
    rsa_pkcs1("rsa_signature_2048_sha512_test.json");
}

#[test]
fn wycheproof_rsa_pss_2048_sha256_mgf1_32() {
    rsa_pss("rsa_pss_2048_sha256_mgf1_32_test.json");
}

#[test]
fn wycheproof_rsa_pss_2048_sha384_mgf1_48() {
    rsa_pss("rsa_pss_2048_sha384_mgf1_48_test.json");
}

// ---------------------------------------------------------------------------------------------
// 10. The hashes.

/// SHA-384 of "abc", the example of FIPS 180-4 (its appendix of examples, SHA384.pdf).
const SHA384_ABC: &str = concat!(
    "cb00753f45a35e8bb5a03d699ac65007272c32ab0eded163",
    "1a8b605a43ff5bed8086072ba1e7cc2358baeca134c825a7",
);

#[test]
fn sha384_of_abc_is_the_fips_example() {
    let digest = crypto::hash(HashAlgorithm::Sha384, b"abc");
    assert_eq!(to_hex(digest.as_ref()), SHA384_ABC);
}

#[test]
fn a_running_hash_in_uneven_parts_equals_the_one_shot_hash() {
    let message: Vec<u8> = (0..1000u32).map(|i| (i * 7 % 251) as u8).collect();
    for algorithm in [HashAlgorithm::Sha256, HashAlgorithm::Sha384] {
        let mut running = RunningHash::new(algorithm);
        running.update(&message[..1]);
        running.update(&message[1..130]);
        let partial = running.current();
        assert_eq!(
            partial.as_ref(),
            crypto::hash(algorithm, &message[..130]).as_ref(),
            "{algorithm:?}: the hash so far"
        );
        running.update(&message[130..]);
        assert_eq!(
            running.finish().as_ref(),
            crypto::hash(algorithm, &message).as_ref(),
            "{algorithm:?}"
        );
    }
}

// ---------------------------------------------------------------------------------------------
// 11. Signing under `test-tooling`. The keys are PKCS#8 made once by `openssl genpkey` for these
// tests (their commands in `tests/data/wycheproof/SOURCE`).

/// Ed25519, PKCS#8 v1 (48 bytes) as `openssl genpkey -algorithm ed25519` writes it.
const ED25519_PKCS8: &str = concat!(
    "302e020100300506032b657004220420c4b5596f5e84df865c44a306ad3eb1d0ab77e01601ca26884442bbbb",
    "bf7dfcc9",
);

/// ECDSA P-256, PKCS#8.
const P256_PKCS8: &str = concat!(
    "308187020100301306072a8648ce3d020106082a8648ce3d030107046d306b020101042015b4f791d6fb8abc",
    "0cbccc969aac43b8c2cbf60f8717e8b1dcfef0ccbcc5aab1a14403420004cd5018ad8390137842dc06f155ef",
    "27a982d5ebc9cd4023916b1b3cb0120a07e2f5cb9b68532d0a1ddbffaa87f9d4fa32db2f41e0d86a25a11d9c",
    "ba362e5169f0",
);

/// ECDSA P-384, PKCS#8.
const P384_PKCS8: &str = concat!(
    "3081b6020100301006072a8648ce3d020106052b8104002204819e30819b02010104305a021ce3194cf6666f",
    "a6981268edc92fdbb37859d255c1a92a23d9ef8a31478b41b0ac8b279f0fef15389d8e769fb5cca164036200",
    "046c29352475607f6d5f41a03e2ad4a675f5983fafdc0dfc5859e81fb45e33e853e8ab7a0a48224030db46e8",
    "ac3eb9cd3568fda2b1bf6b09c3eefd8eb1399a1475f241961d133be1fad28e4f034d21df0928f3cee42436bc",
    "768bee56feededd58c",
);

/// The message every signing test signs.
const MESSAGE: &[u8] = b"surf-tls signs this message for the seam tests";

/// `scheme` signs [`MESSAGE`] with the key `pkcs8`; the check of the seam accepts the signature
/// and refuses it with one byte flipped. Returns the signature.
fn sign_and_check(scheme: Scheme, pkcs8: &[u8], random: &Random<'_>) -> Vec<u8> {
    let key = SigningKey::from_pkcs8(scheme, pkcs8, random)
        .unwrap_or_else(|e| panic!("{scheme:?}: key from PKCS#8: {e:?}"));
    let signature = key
        .sign(MESSAGE, random)
        .unwrap_or_else(|e| panic!("{scheme:?}: sign: {e:?}"));
    let public = key.public_key();
    let algorithm = scheme.algorithm();
    assert_eq!(
        crypto::check_signature(&ShippedVerifier, algorithm, &public, MESSAGE, &signature),
        Ok(scheme),
        "{scheme:?}: the signature verifies"
    );
    let mut flipped = signature.clone();
    let middle = flipped.len() / 2;
    flipped[middle] ^= 0x01;
    assert_eq!(
        crypto::check_signature(&ShippedVerifier, algorithm, &public, MESSAGE, &flipped),
        Err(SignatureError::BadSignature),
        "{scheme:?}: one byte flipped"
    );
    signature
}

#[test]
fn ecdsa_p256_signs_and_the_seam_checks_it() {
    sign_and_check(
        Scheme::EcdsaSecp256r1Sha256,
        &hex(P256_PKCS8),
        &Random::system(),
    );
}

#[test]
fn ecdsa_p384_signs_and_the_seam_checks_it() {
    sign_and_check(
        Scheme::EcdsaSecp384r1Sha384,
        &hex(P384_PKCS8),
        &Random::system(),
    );
}

#[test]
fn ed25519_from_pkcs8_v1_signs_and_the_seam_checks_it() {
    let pkcs8 = hex(ED25519_PKCS8);
    assert_eq!(pkcs8.len(), 48, "PKCS#8 v1 as openssl genpkey writes it");
    assert_eq!(&pkcs8[2..5], &[0x02, 0x01, 0x00], "version 0, v1");
    sign_and_check(Scheme::Ed25519, &pkcs8, &Random::system());
}

#[test]
fn rsa_pkcs1_sha256_signs_and_the_seam_checks_it() {
    sign_and_check(Scheme::RsaPkcs1Sha256, &rsa2048_pkcs8(), &Random::system());
}

#[test]
fn rsa_pss_sha256_signs_and_the_seam_checks_it() {
    sign_and_check(
        Scheme::RsaPssRsaeSha256,
        &rsa2048_pkcs8(),
        &Random::system(),
    );
}

#[test]
fn ecdsa_under_one_fixed_source_signs_one_message_twice_to_equal_bytes() {
    let seed = [0x5au8; 32];
    let random = Random::fixed_slice(&seed);
    let pkcs8 = hex(P256_PKCS8);
    let key = SigningKey::from_pkcs8(Scheme::EcdsaSecp256r1Sha256, &pkcs8, &random)
        .expect("key from PKCS#8");
    let first = key.sign(MESSAGE, &random).expect("sign once");
    let second = key.sign(MESSAGE, &random).expect("sign twice");
    assert_eq!(to_hex(&first), to_hex(&second));
    let other = SigningKey::from_pkcs8(Scheme::EcdsaSecp256r1Sha256, &pkcs8, &random)
        .expect("the key again");
    assert_eq!(
        other
            .sign(MESSAGE, &random)
            .expect("sign with the key again"),
        first
    );
    let p384 = Random::fixed_byte(0x5a);
    let first = sign_and_check(Scheme::EcdsaSecp384r1Sha384, &hex(P384_PKCS8), &p384);
    let second = sign_and_check(Scheme::EcdsaSecp384r1Sha384, &hex(P384_PKCS8), &p384);
    assert_eq!(first, second, "P-384 under a fixed byte");
}

// ---------------------------------------------------------------------------------------------
// 12. Refused by name.

/// A verifier that counts its calls and accepts every signature.
struct Counting(Cell<usize>);

impl SignatureVerifier for Counting {
    fn verify(&self, _: Scheme, _: &[u8], _: &[u8], _: &[u8]) -> Result<(), CryptoError> {
        self.0.set(self.0.get() + 1);
        Ok(())
    }
}

#[test]
fn weak_keys_and_algorithms_are_refused_by_name_before_any_verification() {
    assert_eq!(REFUSED.rsa_bits_min, 2048);
    assert!(REFUSED.hashes.contains(&SignatureHash::Md5));
    assert!(REFUSED.hashes.contains(&SignatureHash::Sha1));
    let counting = Counting(Cell::new(0));
    let rsa1024 = PublicKey {
        kind: PublicKeyKind::Rsa { bits: 1024 },
        bytes: b"never read",
    };
    let rsa2048 = PublicKey {
        kind: PublicKeyKind::Rsa { bits: 2048 },
        bytes: b"never read",
    };
    let cases = [
        (
            SignatureAlgorithm::RsaPkcs1(SignatureHash::Sha256),
            &rsa1024,
            Refusal::WeakKey { bits: 1024 },
            "WeakKey",
        ),
        (
            SignatureAlgorithm::RsaPkcs1(SignatureHash::Sha1),
            &rsa2048,
            Refusal::WeakAlgorithm {
                algorithm: "sha1WithRSAEncryption",
            },
            "WeakAlgorithm",
        ),
        (
            SignatureAlgorithm::RsaPkcs1(SignatureHash::Md5),
            &rsa2048,
            Refusal::WeakAlgorithm {
                algorithm: "md5WithRSAEncryption",
            },
            "WeakAlgorithm",
        ),
    ];
    for (algorithm, key, refusal, check) in cases {
        assert_eq!(crypto::refusal(algorithm, key.kind), Some(refusal));
        assert_eq!(refusal.check(), check);
        assert_eq!(
            crypto::check_signature(&counting, algorithm, key, MESSAGE, b"never read"),
            Err(SignatureError::Refused(refusal)),
        );
    }
    assert_eq!(counting.0.get(), 0, "no verification ran");
    assert_eq!(
        crypto::check_signature(
            &counting,
            SignatureAlgorithm::RsaPkcs1(SignatureHash::Sha256),
            &rsa2048,
            MESSAGE,
            b"accepted by the counting verifier",
        ),
        Ok(Scheme::RsaPkcs1Sha256),
    );
    assert_eq!(
        counting.0.get(),
        1,
        "the verifier is the one the caller gave"
    );
}

#[test]
fn an_aes_128_gcm_record_with_one_flipped_tag_bit_does_not_open() {
    let key = AeadKey::new(AeadAlgorithm::Aes128Gcm, &[7u8; 16]).expect("key");
    let nonce = [1u8; 12];
    let mut record = b"a record of application data".to_vec();
    key.seal(&nonce, b"\x17\x03\x03\x00\x2c", &mut record)
        .expect("seal");
    let mut good = record.clone();
    assert!(key.open(&nonce, b"\x17\x03\x03\x00\x2c", &mut good).is_ok());
    let last = record.len() - 1;
    record[last] ^= 0x01;
    assert_eq!(
        key.open(&nonce, b"\x17\x03\x03\x00\x2c", &mut record),
        Err(CryptoError::OpenFailed)
    );
}

#[test]
fn a_p256_public_key_off_the_curve_is_refused_by_agreement() {
    let key = EphemeralKey::generate(Group::Secp256r1, &Random::system()).expect("key");
    let mut point = key.public_key().to_vec();
    let last = point.len() - 1;
    point[last] ^= 0x01;
    assert!(matches!(key.agree(&point), Err(CryptoError::BadPublicKey)));
}

// ---------------------------------------------------------------------------------------------
// 13. The digests of the vendored files.

/// Every file of `dir` but `SOURCE` against the sha256 its `SOURCE` lists, hashed through the
/// seam: the number of files compared, or one line per file that differs, is missing or is not
/// listed.
fn compare_digests(dir: &Path) -> Result<usize, Vec<String>> {
    let source = fs::read_to_string(dir.join("SOURCE")).expect("read SOURCE");
    let mut listed = Vec::new();
    for line in source.lines() {
        let Some((digest, name)) = line.split_once("  ") else {
            continue;
        };
        if digest.len() == 64 && digest.bytes().all(|b| b.is_ascii_hexdigit()) {
            listed.push((digest.to_string(), name.to_string()));
        }
    }
    let mut problems = Vec::new();
    for (digest, name) in &listed {
        match fs::read(dir.join(name)) {
            Ok(content) => {
                let actual = to_hex(crypto::hash(HashAlgorithm::Sha256, &content).as_ref());
                if actual != *digest {
                    problems.push(format!("{name}: sha256 {actual}, SOURCE lists {digest}"));
                }
            }
            Err(e) => problems.push(format!("{name}: {e}")),
        }
    }
    for entry in fs::read_dir(dir).expect("read the directory") {
        let name = entry
            .expect("entry")
            .file_name()
            .to_string_lossy()
            .into_owned();
        if name != "SOURCE" && !listed.iter().any(|(_, n)| *n == name) {
            problems.push(format!("{name}: not listed in SOURCE"));
        }
    }
    if problems.is_empty() {
        Ok(listed.len())
    } else {
        Err(problems)
    }
}

#[test]
fn sources_match_their_digests() {
    let dir = wycheproof_dir();
    assert_eq!(compare_digests(&dir), Ok(15), "14 vector files and LICENSE");
    let copy = Path::new(env!("CARGO_TARGET_TMPDIR")).join("wycheproof-digests");
    let _ = fs::remove_dir_all(&copy);
    fs::create_dir_all(&copy).expect("create the copy");
    for entry in fs::read_dir(&dir).expect("read the directory") {
        let entry = entry.expect("entry");
        fs::copy(entry.path(), copy.join(entry.file_name())).expect("copy a file");
    }
    let changed = copy.join("ed25519_test.json");
    let mut content = fs::read(&changed).expect("read the copy");
    content[100] ^= 0x01;
    fs::write(&changed, content).expect("write the copy");
    let problems = compare_digests(&copy).expect_err("a changed byte fails");
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert!(
        problems[0].starts_with("ed25519_test.json: sha256 "),
        "{problems:?}"
    );
    let _ = fs::remove_dir_all(&copy);
}

// ---------------------------------------------------------------------------------------------
// 14. Only `crypto.rs` names ring.

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("read a directory") {
        let path = entry.expect("entry").path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

#[test]
fn only_crypto_names_ring() {
    let src = crate_dir().join("src");
    let mut files = Vec::new();
    rust_files(&src, &mut files);
    assert!(files.contains(&src.join("crypto.rs")), "{files:?}");
    let mut naming = Vec::new();
    for file in files.iter().filter(|f| **f != src.join("crypto.rs")) {
        let text = fs::read_to_string(file).expect("read a source file");
        for (n, line) in text.lines().enumerate() {
            if line.contains("ring::")
                || line.contains("extern crate ring")
                || line.contains("use ring ")
            {
                naming.push(format!(
                    "{}:{}: {}",
                    file.strip_prefix(crate_dir()).unwrap_or(file).display(),
                    n + 1,
                    line.trim()
                ));
            }
        }
    }
    assert!(
        naming.is_empty(),
        "only src/crypto.rs may name ring:\n{}",
        naming.join("\n")
    );
}

// ---------------------------------------------------------------------------------------------

/// RSA 2048, PKCS#8.
fn rsa2048_pkcs8() -> Vec<u8> {
    hex(RSA2048_PKCS8)
}

const RSA2048_PKCS8: &str = concat!(
    "308204bc020100300d06092a864886f70d0101010500048204a6308204a202010002820101009d40afa748e6",
    "8c5eb02b8486f7ef56c68573eccdd559a1cc7035fd68c688572cad4c4d33c4db251b1be291adbd416fbc9415",
    "062fd77e41b483b39c873a71e4f08057bcae20067a7c8e12696643b83db2d19b4c5092b27e32744888bbf8e7",
    "9f240fad5e6eb0415cca2e46a25d26d73b04e1eb2d5dfaa147c80e97bed2824f6fc429c70bb691dede1ccbf3",
    "7563e7ec271a4e67b031819764157182c2d9eaa532cc327efb2fd9cb2b9e880a6df4d519e86206a02d36ab73",
    "40e35973d67f637ba77ab7f1837fec158176bc9c0e974b49773dd016e3647d37bb2cbf2865ff533c6fa9df7a",
    "1c27304a58d4d15913a3d6d84425a8be0a0ffac36413fec8cf0ce78a7a390203010001028201000afbc52910",
    "88f5ddf93d18e51a752fa0ef1a513c713df0f4a813c54ee7ab0737ba24fdb1737b0104fb362089bb91d1fda0",
    "743579eaf9c91a1426dbf4800b0b7c16e2c9ed79942a42ffe35e5443bf9f2399cb5891ba2558762ab41444d8",
    "d648c1ca8c87ef8045187790b2e34d5154b02d1fa07f0b34e1c5f33dbdb098e07c2ee8af0abbf78fbdb195d6",
    "aa7a59d660f6be098034dc53049e1ad04026080477665531ff9d7a9e4a1d4f8647b1fdccf16c8e94b5abed12",
    "fc209fd16e7f6c77cf6a30f1c819089fbcce30da0c1ff1f731573b17c79990b71bf46d2376e81d398e3f45bb",
    "e1eeb9dfff6eef96fdb82e8859d0cf6af4dc1a6cb88778704f6882ecb0588102818100d91caeb866bdd14c5c",
    "d5d6205a403531197f258646fac2b4d121fc8076015f8105d0a4314f0507b816ca4765ce583f30add374ac8c",
    "bf9eace3d9b8d4a8d69db86e873a54fd25b2dd120f54af03d79ff012568d40715b764684950cd178bf44b4f9",
    "7946b18ae8a78b7fe7b33f57bd39db70d370edacd6e56cece3657b4fc0521902818100b96b4025874ef26332",
    "725218c1e3b23625995bcd2799affa58aef175797b6e258bc3edac50f633ea44aa5dc91be243feb0f3d553d9",
    "57af86ae8448f490dfed86395cb4c8cbe739f8070d5b14895d2e459a7ab7b6ac7f5fe0d68cc5ceb6dda637ce",
    "0ec7fe022528196bfd8448645a88ebb064919fc170e3606b869284050dad210281806cfc2af0c21252f82439",
    "eb6cfe20f1129e9be6b84af521bc26a70adb69613311cf5b76b091dc128de452d21856c691a1dc0023b76247",
    "e65d1f5b719ebeca4d0e01ddbbf861c304d0587a7da60525be5182c313d0df966bfee61af3fe4ac0a8bab7e8",
    "9b061096cd540776c5d1ec373ecddb0b28cb06523184c63d37f9f884b0310281800657d6eff1bb0fabb91414",
    "1ee8b3236b9ac9ae4c4c68838eaaee0130a5a145595f78aa6ed4ae37847228e9e07165c231ba57fd1a4b3b26",
    "b412d04fb69e262af8068a575cef2ccfdcde50e4a6c5343cdb00b7e60507cdabb8035ef97d62e01e49967107",
    "02778c51dcdcc6e5799c85445ddbc0f5026419ba8d97e018af0f85324102818055f64881f6af59bd7b4a5530",
    "cc297b844d5d28b2e230ed3324563696e15b61f154323e50a37673e1bc38d46a53ce920e387753f0656199de",
    "e84682ae395ad6ff5e36f00d41f5d86acc59202bc7c01ee61aac1ecf27d15783ee0e1a01663c0125d3f2ea2b",
    "9f15f357a55428a2ec14010a30d0740f4bbd4cc24b5a854d65a09ba5",
);
