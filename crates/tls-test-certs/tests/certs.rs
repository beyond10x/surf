//! The committed certificates of `certs/` (story `tls-test-certs`, acceptance 1 to 5).

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use surf_tls::crypto::{
    self, PublicKey, PublicKeyKind, Scheme, ShippedVerifier, SignatureAlgorithm, SignatureError,
    SignatureHash,
};
use surf_tls_test_certs::{OPENSSL_CASES, VALIDATION_TIME, cases, inspect, keys, output, pem};

/// The twelve checks of `NotTrusted(check)`, copied from docs/design/tls.md item 7, in its order.
const CHECKS: [&str; 12] = [
    "Unreadable",
    "NoPathToAnchor",
    "BadSignature",
    "IssuerNotCa",
    "PathTooLong",
    "CandidateLimit",
    "KeyUsage",
    "UnknownCriticalExtension",
    "NameConstraint",
    "NoServerAuth",
    "WeakAlgorithm",
    "WeakKey",
];

/// The files of every case.
const CASE_FILES: [&str; 3] = ["chain.pem", "root.pem", "leaf.pk8"];

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn certs_dir() -> PathBuf {
    crate_dir().join("certs")
}

fn read_text(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// The certificates of `<case>/<file>`, DER, in file order.
fn pem_file(case: &str, file: &str) -> Vec<Vec<u8>> {
    let path = certs_dir().join(case).join(file);
    pem::decode_certificates(&read_text(&path))
        .unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// The names of the cases `src/cases.rs` makes, then the three of `openssl/`.
fn all_names() -> Vec<&'static str> {
    let mut names: Vec<&'static str> = cases::specs().iter().map(|s| s.name).collect();
    names.extend(OPENSSL_CASES);
    names
}

/// The check of INDEX: each line `<case> <host> <time> <outcome>`, the outcome `ok`,
/// `CertificateExpired`, `CertificateNameMismatch` or `CertificateNotTrusted <check>`; every name
/// once; every check one of [`CHECKS`] and each of them the check of at least one case. An
/// error names the line as `INDEX:<n>: <line>`.
fn check_index(text: &str, names: &[&str]) -> Result<usize, String> {
    let mut seen = BTreeSet::new();
    let mut checks = BTreeSet::new();
    for (i, line) in text.lines().enumerate() {
        let at = format!("INDEX:{}: {line}", i + 1);
        let fields: Vec<&str> = line.split(' ').collect();
        if fields.len() < 4 {
            return Err(format!("{at}: fewer than four fields"));
        }
        let (name, host, time) = (fields[0], fields[1], fields[2]);
        if !names.contains(&name) {
            return Err(format!("{at}: {name} is no case"));
        }
        if !seen.insert(name) {
            return Err(format!("{at}: {name} named twice"));
        }
        if host.is_empty() {
            return Err(format!("{at}: no host"));
        }
        if time != VALIDATION_TIME {
            return Err(format!("{at}: the time is not {VALIDATION_TIME}"));
        }
        match (fields[3], fields.len()) {
            ("ok" | "CertificateExpired" | "CertificateNameMismatch", 4) => {}
            ("CertificateNotTrusted", 5) => {
                let check = fields[4];
                if !CHECKS.contains(&check) {
                    return Err(format!("{at}: {check} is no check of item 7"));
                }
                checks.insert(check);
            }
            _ => return Err(format!("{at}: no outcome of item 7")),
        }
    }
    for name in names {
        if !seen.contains(name) {
            return Err(format!("INDEX: {name} is not named"));
        }
    }
    for check in CHECKS {
        if !checks.contains(check) {
            return Err(format!("INDEX: no case has the check {check}"));
        }
    }
    Ok(seen.len())
}

#[test]
fn index() {
    let text = read_text(&certs_dir().join("INDEX"));
    let names = all_names();
    let count = check_index(&text, &names).unwrap();
    assert_eq!(count, names.len());
    for name in &names {
        for file in CASE_FILES {
            let path = certs_dir().join(name).join(file);
            assert!(path.is_file(), "{} is missing", path.display());
        }
    }
    // The INDEX line of each case of src/cases.rs is the one its spec gives.
    for spec in cases::specs() {
        let line = format!(
            "{} {} {VALIDATION_TIME} {}",
            spec.name,
            spec.host,
            spec.outcome.index_text()
        );
        assert!(text.lines().any(|l| l == line), "INDEX lacks {line}");
    }
    println!("INDEX: {count} cases, {} checks", CHECKS.len());
}

#[test]
fn index_names_the_line_of_a_check_outside_item_7() {
    let text = read_text(&certs_dir().join("INDEX"));
    let number = text
        .lines()
        .position(|l| l.ends_with(" IssuerNotCa"))
        .expect("a line with IssuerNotCa")
        + 1;
    let copy = text.replace("IssuerNotCa", "NotACa");
    let error = check_index(&copy, &all_names()).unwrap_err();
    assert!(
        error.starts_with(&format!("INDEX:{number}: ")) && error.contains("NotACa"),
        "{error}"
    );
}

/// The public key and the algorithm a certificate is checked with.
fn key_of(spki_cert: &inspect::Certificate<'_>) -> (PublicKeyKind, Vec<u8>) {
    let (kind, bytes) = inspect::public_key(spki_cert.spki).unwrap();
    (kind, bytes.to_vec())
}

#[test]
fn signatures() {
    let mut verified = 0;
    let mut refused = 0;
    for spec in cases::specs() {
        let mut ders = pem_file(spec.name, "chain.pem");
        let chain_len = ders.len();
        ders.extend(pem_file(spec.name, "root.pem"));
        let parsed: Vec<inspect::Certificate<'_>> = ders
            .iter()
            .map(|d| inspect::certificate(d).unwrap())
            .collect();
        for (n, cert) in parsed.iter().enumerate() {
            let algorithm = inspect::signature_algorithm(cert.signature_algorithm).unwrap();
            let issuers: Vec<&inspect::Certificate<'_>> =
                parsed.iter().filter(|c| c.subject == cert.issuer).collect();
            assert!(
                !issuers.is_empty(),
                "{} certificate {n}: no certificate of the case has its issuer's name",
                spec.name
            );
            let ok = issuers.iter().any(|issuer| {
                let (kind, bytes) = key_of(issuer);
                crypto::check_signature(
                    &ShippedVerifier,
                    algorithm,
                    &PublicKey {
                        kind,
                        bytes: &bytes,
                    },
                    cert.tbs,
                    cert.signature,
                )
                .is_ok()
            });
            let flipped = spec.name == "bad-signature" && n == 0;
            if flipped {
                assert!(!ok, "the leaf of bad-signature verifies");
                refused += 1;
            } else {
                assert!(
                    ok,
                    "{} certificate {n} (of {chain_len} sent) does not verify under its issuer",
                    spec.name
                );
                verified += 1;
            }
        }
    }
    assert_eq!(refused, 1);
    println!("signatures: {verified} verified, {refused} refused");
}

#[test]
fn reproducible() {
    let keys = keys::Keys::read(&crate_dir().join("keys")).unwrap();
    let first = output::files(&cases::build_all(&keys).unwrap());
    let second = output::files(&cases::build_all(&keys).unwrap());
    assert!(first == second, "two builds differ");
    let mut compared = 0;
    for (path, bytes) in &first {
        let committed =
            fs::read(certs_dir().join(path)).unwrap_or_else(|e| panic!("certs/{path}: {e}"));
        if path == "INDEX" {
            assert!(
                committed.starts_with(bytes),
                "certs/INDEX does not start with the lines of src/cases.rs"
            );
        } else {
            assert!(committed == *bytes, "certs/{path} differs from the build");
        }
        compared += 1;
    }
    // Nothing else lies in the directories of the built cases.
    for spec in cases::specs() {
        for entry in fs::read_dir(certs_dir().join(spec.name)).unwrap() {
            let name = entry.unwrap().file_name().into_string().unwrap();
            assert!(
                CASE_FILES.contains(&name.as_str()),
                "certs/{}/{name}",
                spec.name
            );
        }
    }
    assert_eq!(compared, cases::specs().len() * CASE_FILES.len() + 1);
    println!("reproducible: {compared} files equal");
}

/// The DER of an AlgorithmIdentifier of `oid` (DER of the OBJECT IDENTIFIER's content) with
/// NULL parameters.
fn with_null(oid: &[u8]) -> Vec<u8> {
    let mut out = vec![0x30, oid.len() as u8 + 4, 0x06, oid.len() as u8];
    out.extend_from_slice(oid);
    out.extend_from_slice(&[0x05, 0x00]);
    out
}

const RSA_PKCS1: &[u8] = &[0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01];
const SHA2: &[u8] = &[0x60, 0x86, 0x48, 0x01, 0x65, 0x03, 0x04, 0x02];
/// The BIT STRING header of a signature of 256 bytes, which follows the outer algorithm.
const SIGNATURE_2048: &[u8] = &[0x03, 0x82, 0x01, 0x01, 0x00];

fn oid(prefix: &[u8], last: u8) -> Vec<u8> {
    let mut out = prefix.to_vec();
    out.push(last);
    out
}

/// The outer signature algorithm of a leaf signed by the RSA 2048 intermediate: `algorithm`
/// followed by the BIT STRING header of a 256-byte signature, at the end of the certificate.
fn signed_with(der: &[u8], algorithm: &[u8]) -> bool {
    let mut tail = algorithm.to_vec();
    tail.extend_from_slice(SIGNATURE_2048);
    der.len() >= tail.len() + 256 && der[der.len() - 256 - tail.len()..der.len() - 256] == tail[..]
}

/// RSASSA-PSS with `hash` (the last byte of the SHA-2 OID), MGF1 of it and `salt` (RFC 4055).
fn pss(hash: u8, salt: u8) -> Vec<u8> {
    let hash_alg = with_null(&oid(SHA2, hash));
    let mut mgf = vec![0x06, 0x09];
    mgf.extend(oid(RSA_PKCS1, 0x08));
    mgf.extend(&hash_alg);
    let mut params = vec![0xa0, hash_alg.len() as u8];
    params.extend(&hash_alg);
    params.extend([0xa1, mgf.len() as u8 + 2, 0x30, mgf.len() as u8]);
    params.extend(&mgf);
    params.extend([0xa2, 0x03, 0x02, 0x01, salt]);
    let mut out = vec![0x06, 0x09];
    out.extend(oid(RSA_PKCS1, 0x0a));
    out.extend([0x30, params.len() as u8]);
    out.extend(&params);
    let mut algorithm = vec![0x30, out.len() as u8];
    algorithm.extend(out);
    algorithm
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}

#[test]
fn openssl_cases() {
    let intermediate_key = |case: &str| {
        let chain = pem_file(case, "chain.pem");
        let cert = inspect::certificate(&chain[1]).unwrap();
        key_of(&cert)
    };

    // sha1 and md5: the OID found by a byte search; the seam refuses them by name.
    for (case, last, hash) in [
        ("sha1", 0x05, SignatureHash::Sha1),
        ("md5", 0x04, SignatureHash::Md5),
    ] {
        let leaf = &pem_file(case, "chain.pem")[0];
        assert!(
            signed_with(leaf, &with_null(&oid(RSA_PKCS1, last))),
            "{case}: algorithm"
        );
        let algorithm = SignatureAlgorithm::RsaPkcs1(hash);
        let (kind, bytes) = intermediate_key(case);
        let cert = inspect::certificate(leaf).unwrap();
        let checked = crypto::check_signature(
            &ShippedVerifier,
            algorithm,
            &PublicKey {
                kind,
                bytes: &bytes,
            },
            cert.tbs,
            cert.signature,
        );
        match checked {
            Err(SignatureError::Refused(refusal)) => assert_eq!(refusal.check(), "WeakAlgorithm"),
            other => panic!("{case}: {other:?}"),
        }
    }

    // rsa-1024: an RSAPublicKey whose modulus is 129 bytes with its sign byte, 128 without.
    let leaf = &pem_file("rsa-1024", "chain.pem")[0];
    let spki_1024 = [
        0x03, 0x81, 0x8d, 0x00, 0x30, 0x81, 0x89, 0x02, 0x81, 0x81, 0x00,
    ];
    assert!(
        contains(leaf, &spki_1024),
        "rsa-1024: no modulus of 128 bytes"
    );
    let refusal = crypto::refusal(
        SignatureAlgorithm::RsaPkcs1(SignatureHash::Sha256),
        PublicKeyKind::Rsa { bits: 128 * 8 },
    )
    .unwrap();
    assert_eq!(refusal.check(), "WeakKey");

    // The five other RSA schemes of signature_algorithms_cert, each verified under the
    // intermediate key.
    let five = [
        (
            "ok-rsa-pss",
            pss(0x01, 32),
            Scheme::RsaPssRsaeSha256,
            SignatureAlgorithm::RsaPss(SignatureHash::Sha256),
        ),
        (
            "ok-rsa-pss-sha384",
            pss(0x02, 48),
            Scheme::RsaPssRsaeSha384,
            SignatureAlgorithm::RsaPss(SignatureHash::Sha384),
        ),
        (
            "ok-rsa-pss-sha512",
            pss(0x03, 64),
            Scheme::RsaPssRsaeSha512,
            SignatureAlgorithm::RsaPss(SignatureHash::Sha512),
        ),
        (
            "ok-rsa-pkcs1-sha384",
            with_null(&oid(RSA_PKCS1, 0x0c)),
            Scheme::RsaPkcs1Sha384,
            SignatureAlgorithm::RsaPkcs1(SignatureHash::Sha384),
        ),
        (
            "ok-rsa-pkcs1-sha512",
            with_null(&oid(RSA_PKCS1, 0x0d)),
            Scheme::RsaPkcs1Sha512,
            SignatureAlgorithm::RsaPkcs1(SignatureHash::Sha512),
        ),
    ];
    for (case, algorithm_der, scheme, algorithm) in five {
        let leaf = &pem_file(case, "chain.pem")[0];
        assert!(signed_with(leaf, &algorithm_der), "{case}: algorithm");
        let (kind, bytes) = intermediate_key(case);
        let cert = inspect::certificate(leaf).unwrap();
        let checked = crypto::check_signature(
            &ShippedVerifier,
            algorithm,
            &PublicKey {
                kind,
                bytes: &bytes,
            },
            cert.tbs,
            cert.signature,
        );
        assert_eq!(checked, Ok(scheme), "{case}");
    }

    // leaf-v1: the TBSCertificate starts with the serial number, no [0] version; a v3 leaf
    // starts with [0] { INTEGER 2 }.
    let tbs_start = |case: &str| {
        let leaf = &pem_file(case, "chain.pem")[0];
        let cert = inspect::certificate(leaf).unwrap();
        let header = if cert.tbs[1] & 0x80 == 0 {
            2
        } else {
            2 + (cert.tbs[1] & 0x7f) as usize
        };
        cert.tbs[header..header + 5].to_vec()
    };
    assert_eq!(tbs_start("leaf-v1")[0], 0x02, "leaf-v1 has a version field");
    assert_eq!(tbs_start("ok-ecdsa-p256"), [0xa0, 0x03, 0x02, 0x01, 0x02]);

    // The output of openssl/make.sh is the one openssl/SOURCE records.
    let source = read_text(&crate_dir().join("openssl").join("SOURCE"));
    let mut recorded = 0;
    for case in OPENSSL_CASES {
        for file in CASE_FILES {
            let path = format!("{case}/{file}");
            let bytes = fs::read(certs_dir().join(&path)).unwrap();
            let digest = crypto::hash(crypto::HashAlgorithm::Sha256, &bytes);
            let hex: String = digest.as_ref().iter().map(|b| format!("{b:02x}")).collect();
            let line = format!("{hex}  {path}");
            assert!(
                source.lines().any(|l| l == line),
                "openssl/SOURCE lacks {line}"
            );
            recorded += 1;
        }
    }
    assert_eq!(recorded, 9);
}

#[test]
fn rsa_1024_and_1000_sans() {
    let leaf = &pem_file("rsa-1024", "chain.pem")[0];
    let cert = inspect::certificate(leaf).unwrap();
    assert_eq!(
        inspect::public_key(cert.spki).unwrap().0,
        PublicKeyKind::Rsa { bits: 1024 }
    );

    let leaf = &pem_file("1000-sans", "chain.pem")[0];
    let cert = inspect::certificate(leaf).unwrap();
    let names = inspect::dns_names(&cert).unwrap();
    assert_eq!(names.len(), 1000);
    assert_eq!(names.last().copied(), Some(&b"localhost"[..]));
}
