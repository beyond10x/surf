//! Probe of `story:tls-decisions`: ring 0.17.14 builds for a release target and runs there.
//! It verifies one fixed ECDSA P-256 signature (made by `openssl dgst -sha256 -sign`), refuses
//! the same signature over another message, runs one X25519 agreement between two keys drawn
//! from the operating system's source, and prints `probe ok`.

use ring::{agreement, rand, signature};

const MESSAGE: &[u8] = b"surf probe/tls-ring: ring 0.17.14 on the release targets";
const PUBLIC_KEY: &str = "049e795a11978b59f4dc548d37a0ba87b3f21d702c93ebf3ea5daf457f8e2d314be18d0cc323afe461541efcb912966abe91c9cd0ccc2a77a76e88980f89825602";
const SIGNATURE: &str = "30450221008b8e0e89867b2a4af1bb3b09cad0a1582edc10fa749629940cb41fad769a28b9022060db33da2520469a6a42db09e0d05de439d12c89364cca3b12277edfc339905f";

fn unhex(text: &str) -> Vec<u8> {
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).expect("hex"))
        .collect()
}

fn fail(what: &str) -> ! {
    eprintln!("probe failed: {what}");
    std::process::exit(1);
}

fn main() {
    let public = unhex(PUBLIC_KEY);
    let sig = unhex(SIGNATURE);
    let key = signature::UnparsedPublicKey::new(&signature::ECDSA_P256_SHA256_ASN1, &public);
    if key.verify(MESSAGE, &sig).is_err() {
        fail("the fixed ECDSA P-256 signature does not verify");
    }
    if key.verify(b"another message", &sig).is_ok() {
        fail("the ECDSA P-256 signature verifies over another message");
    }
    println!("ecdsa p256 sha256: verified");

    let rng = rand::SystemRandom::new();
    let alg = &agreement::X25519;
    let ours = agreement::EphemeralPrivateKey::generate(alg, &rng)
        .unwrap_or_else(|_| fail("x25519 key generation"));
    let theirs = agreement::EphemeralPrivateKey::generate(alg, &rng)
        .unwrap_or_else(|_| fail("x25519 key generation"));
    let our_public = ours
        .compute_public_key()
        .unwrap_or_else(|_| fail("x25519 public key"));
    let their_public = theirs
        .compute_public_key()
        .unwrap_or_else(|_| fail("x25519 public key"));
    let a = agreement::agree_ephemeral(
        ours,
        &agreement::UnparsedPublicKey::new(alg, their_public.as_ref()),
        |s| s.to_vec(),
    )
    .unwrap_or_else(|_| fail("x25519 agreement"));
    let b = agreement::agree_ephemeral(
        theirs,
        &agreement::UnparsedPublicKey::new(alg, our_public.as_ref()),
        |s| s.to_vec(),
    )
    .unwrap_or_else(|_| fail("x25519 agreement"));
    if a != b || a.len() != 32 || a.iter().all(|&x| x == 0) {
        fail("the two sides of the x25519 agreement differ");
    }
    println!("x25519: agreed");

    println!(
        "target: {} {} {}",
        std::env::consts::ARCH,
        std::env::consts::OS,
        if cfg!(target_env = "musl") {
            "musl"
        } else if cfg!(target_env = "gnu") {
            "gnu"
        } else {
            "-"
        }
    );
    println!("probe ok");
}
