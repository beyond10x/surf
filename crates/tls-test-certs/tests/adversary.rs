//! Adversary cases of story `tls-test-certs`: each committed case holds the one defect its name
//! says and no other (read from `certs/` only), and `taskfiles/tls-certs-check.sh` refuses a
//! change it does not see today. No test here runs `openssl`, generates a key or reads the clock.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use surf_tls::crypto::{self, PublicKey, PublicKeyKind, ShippedVerifier, SignatureError};
use surf_tls_test_certs::{der, inspect, pem};

/// The validation time of `INDEX`, as `YYYYMMDDHHMMSS`.
const AT: &str = "20261122000000";

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn certs_dir() -> PathBuf {
    crate_dir().join("certs")
}

fn pem_file(case: &str, file: &str) -> Vec<Vec<u8>> {
    let path = certs_dir().join(case).join(file);
    let text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    pem::decode_certificates(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// notBefore and notAfter of `der` as `YYYYMMDDHHMMSS`.
fn validity(der_bytes: &[u8]) -> (String, String) {
    let cert = inspect::certificate(der_bytes).unwrap();
    let tbs = der::expect(cert.tbs, 0x30).unwrap();
    let fields = der::elements(tbs.content).unwrap();
    let skip = usize::from(fields[0].tag == 0xa0);
    let times = der::elements(fields[skip + 3].content).unwrap();
    let text = |e: &der::Element<'_>| {
        let s = std::str::from_utf8(e.content)
            .unwrap()
            .trim_end_matches('Z');
        if e.tag == 0x17 {
            let yy: u32 = s[..2].parse().unwrap();
            format!("{}{s}", if yy >= 50 { "19" } else { "20" })
        } else {
            s.to_string()
        }
    };
    (text(&times[0]), text(&times[1]))
}

/// The extensions of `cert`: OID (DER), critical, value.
fn extensions<'a>(cert: &inspect::Certificate<'a>) -> Vec<(Vec<u8>, bool, &'a [u8])> {
    der::elements(cert.extensions.unwrap_or(&[]))
        .unwrap()
        .into_iter()
        .map(|e| {
            let parts = der::elements(e.content).unwrap();
            let critical = parts.len() == 3 && parts[1].content == [0xff];
            (
                parts[0].whole.to_vec(),
                critical,
                parts.last().unwrap().content,
            )
        })
        .collect()
}

fn ext<'a>(cert: &inspect::Certificate<'a>, oid: &str) -> Option<&'a [u8]> {
    inspect::extension(cert, oid).unwrap()
}

/// basicConstraints: (cA, pathLenConstraint), None without the extension.
fn basic_constraints(cert: &inspect::Certificate<'_>) -> Option<(bool, Option<u8>)> {
    let value = ext(cert, "2.5.29.19")?;
    let seq = der::expect(value, 0x30).unwrap();
    let mut ca = false;
    let mut len = None;
    for e in der::elements(seq.content).unwrap() {
        match e.tag {
            0x01 => ca = e.content == [0xff],
            0x02 => len = Some(*e.content.last().unwrap()),
            _ => {}
        }
    }
    Some((ca, len))
}

/// The first byte of the keyUsage bits, None without the extension.
fn key_usage(cert: &inspect::Certificate<'_>) -> Option<u8> {
    let value = ext(cert, "2.5.29.15")?;
    let bits = der::expect(value, 0x03).unwrap();
    Some(*bits.content.get(1).unwrap_or(&0))
}

/// Whether the leaf's subjectAltName names `host` (RFC 9525: `*` the whole left-most label,
/// one label; an IP host only by iPAddress).
fn names_host(leaf: &inspect::Certificate<'_>, host: &str) -> bool {
    let Some(value) = ext(leaf, "2.5.29.17") else {
        return false;
    };
    let seq = der::expect(value, 0x30).unwrap();
    let ip: Option<Vec<u8>> = host.split('.').map(|p| p.parse::<u8>().ok()).collect();
    der::elements(seq.content)
        .unwrap()
        .iter()
        .any(|e| match (&ip, e.tag) {
            (Some(octets), 0x87) => e.content == &octets[..],
            (None, 0x82) => {
                let name = std::str::from_utf8(e.content).unwrap().to_ascii_lowercase();
                match name.strip_prefix("*.") {
                    Some(rest) => host
                        .split_once('.')
                        .is_some_and(|(label, after)| !label.is_empty() && after == rest),
                    None => name == host,
                }
            }
            _ => false,
        })
}

const KNOWN: [&str; 7] = [
    "2.5.29.19",
    "2.5.29.15",
    "2.5.29.37",
    "2.5.29.17",
    "2.5.29.30",
    "2.5.29.14",
    "2.5.29.35",
];

/// What is wrong with a linear case: the chain sent is the leaf and then each issuer, the root
/// is the one anchor. Each defect is one word.
fn defects(case: &str, host: &str) -> BTreeSet<&'static str> {
    let chain = pem_file(case, "chain.pem");
    let root = pem_file(case, "root.pem").remove(0);
    let certs: Vec<inspect::Certificate<'_>> = chain
        .iter()
        .map(|d| inspect::certificate(d).unwrap())
        .collect();
    let anchor = inspect::certificate(&root).unwrap();
    let leaf = &certs[0];
    let mut out = BTreeSet::new();

    for (n, der_bytes) in chain.iter().enumerate() {
        let (from, to) = validity(der_bytes);
        if to.as_str() < AT {
            out.insert(if n == 0 { "expired" } else { "issuer-expired" });
        }
        if from.as_str() > AT {
            out.insert(if n == 0 {
                "not-yet-valid"
            } else {
                "issuer-not-yet-valid"
            });
        }
    }
    if !names_host(leaf, host) {
        out.insert("name");
    }
    if leaf.version.is_none() {
        out.insert("v1");
    }
    if key_usage(leaf).is_some_and(|b| b & 0x80 == 0) {
        out.insert("leaf-key-usage");
    }
    if let Some(eku) = ext(leaf, "2.5.29.37") {
        let server = der::oid("1.3.6.1.5.5.7.3.1");
        if !eku.windows(server.len()).any(|w| w == &server[..]) {
            out.insert("eku");
        }
    }
    if basic_constraints(leaf).is_some_and(|(ca, _)| ca) {
        out.insert("leaf-is-ca");
    }
    // The path: each certificate sent is issued by the next; the last by the anchor, or by
    // itself when it is a root of its own.
    let mut path_ends_at_anchor = false;
    let mut intermediates = 0;
    for (n, cert) in certs.iter().enumerate() {
        let self_signed = n > 0 && cert.subject == cert.issuer;
        let issuer = if let Some(next) = certs.get(n + 1) {
            next
        } else if cert.issuer == anchor.subject {
            path_ends_at_anchor = true;
            &anchor
        } else if self_signed {
            cert
        } else {
            out.insert("no-issuer");
            continue;
        };
        if n > 0 && !self_signed {
            intermediates += 1;
        }
        if issuer.subject != cert.issuer {
            out.insert("issuer-name");
        }
        let (kind, bytes) = inspect::public_key(issuer.spki).unwrap();
        if let PublicKeyKind::Rsa { bits } = inspect::public_key(cert.spki).unwrap().0
            && bits < 2048
        {
            out.insert("weak-key");
        }
        let algorithm = inspect::signature_algorithm(cert.signature_algorithm).unwrap();
        match crypto::check_signature(
            &ShippedVerifier,
            algorithm,
            &PublicKey { kind, bytes },
            cert.tbs,
            cert.signature,
        ) {
            Ok(_) => {}
            Err(SignatureError::Refused(r)) if r.check() == "WeakAlgorithm" => {
                out.insert("weak-algorithm");
            }
            Err(e) => {
                out.insert("bad-signature");
                let _ = e;
            }
        }
        if !std::ptr::eq(issuer, cert) {
            match basic_constraints(issuer) {
                Some((true, len)) => {
                    // pathLenConstraint counts the intermediates below the issuer.
                    if let Some(len) = len
                        && n > usize::from(len)
                    {
                        out.insert("pathlen");
                    }
                }
                _ => {
                    out.insert("issuer-not-ca");
                }
            }
            if key_usage(issuer).is_some_and(|b| b & 0x04 == 0) {
                out.insert("issuer-key-usage");
            }
            if ext(issuer, "2.5.29.30").is_some() {
                out.insert("name-constraint");
            }
        }
    }
    if !path_ends_at_anchor {
        out.insert("no-anchor");
    }
    if intermediates > 6 {
        out.insert("too-long");
    }
    for cert in certs.iter().chain([&anchor]) {
        for (oid, critical, _) in extensions(cert) {
            let known = KNOWN.iter().any(|k| der::oid(k) == oid);
            if !known && critical {
                out.insert("unknown-critical");
            }
        }
    }
    out
}

/// The two cases of the limits of item 9.4, whose structure
/// [`candidate_limit_cases_hold_their_structure`] checks instead of [`defects`].
const CANDIDATE_CASES: [&str; 2] = ["candidate-limit", "candidate-limit-no-anchor"];

/// Every case of `INDEX`, written here by hand and not read from `src/cases.rs`: its host, its
/// outcome as `INDEX` writes it, and the one defect it is named for (the positive cases have
/// none). A case of `INDEX` missing here, or a row missing from `INDEX`, fails.
const TABLE: [(&str, &str, &str, &[&str]); 33] = [
    ("ok-rsa2048", "localhost", "ok", &[]),
    ("ok-ecdsa-p256", "localhost", "ok", &[]),
    ("ok-ecdsa-p384", "localhost", "ok", &[]),
    ("ok-ed25519", "localhost", "ok", &[]),
    ("ok-wildcard", "a.example.test", "ok", &[]),
    ("expired", "localhost", "CertificateExpired", &["expired"]),
    (
        "not-yet-valid",
        "localhost",
        "CertificateExpired",
        &["not-yet-valid"],
    ),
    (
        "wrong-host",
        "localhost",
        "CertificateNameMismatch",
        &["name"],
    ),
    (
        "untrusted-root",
        "localhost",
        "CertificateNotTrusted NoPathToAnchor",
        &["no-anchor"],
    ),
    (
        "bad-signature",
        "localhost",
        "CertificateNotTrusted BadSignature",
        &["bad-signature"],
    ),
    (
        "intermediate-not-ca",
        "localhost",
        "CertificateNotTrusted IssuerNotCa",
        &["issuer-not-ca"],
    ),
    (
        "path-too-long",
        "localhost",
        "CertificateNotTrusted PathTooLong",
        &["too-long"],
    ),
    (
        "pathlen-exceeded",
        "localhost",
        "CertificateNotTrusted PathTooLong",
        &["pathlen"],
    ),
    (
        "leaf-no-digital-signature",
        "localhost",
        "CertificateNotTrusted KeyUsage",
        &["leaf-key-usage"],
    ),
    (
        "issuer-no-key-cert-sign",
        "localhost",
        "CertificateNotTrusted KeyUsage",
        &["issuer-key-usage"],
    ),
    (
        "candidate-limit",
        "localhost",
        "CertificateNotTrusted CandidateLimit",
        &[],
    ),
    (
        "candidate-limit-no-anchor",
        "localhost",
        "CertificateNotTrusted CandidateLimit",
        &[],
    ),
    (
        "unknown-critical-extension",
        "localhost",
        "CertificateNotTrusted UnknownCriticalExtension",
        &["unknown-critical"],
    ),
    (
        "wildcard-two-labels",
        "a.b.example.test",
        "CertificateNameMismatch",
        &["name"],
    ),
    (
        "common-name-only",
        "localhost",
        "CertificateNameMismatch",
        &["name"],
    ),
    (
        "ip-mismatch",
        "127.0.0.1",
        "CertificateNameMismatch",
        &["name"],
    ),
    (
        "name-constraints-excluded",
        "localhost",
        "CertificateNotTrusted NameConstraint",
        &["name-constraint"],
    ),
    (
        "eku-client-only",
        "localhost",
        "CertificateNotTrusted NoServerAuth",
        &["eku"],
    ),
    ("1000-sans", "localhost", "ok", &[]),
    ("ok-rsa-pss", "localhost", "ok", &[]),
    ("ok-rsa-pss-sha384", "localhost", "ok", &[]),
    ("ok-rsa-pss-sha512", "localhost", "ok", &[]),
    ("ok-rsa-pkcs1-sha384", "localhost", "ok", &[]),
    ("ok-rsa-pkcs1-sha512", "localhost", "ok", &[]),
    // A version 1 certificate has no extensions, so no subjectAltName: the name check fails
    // too. The INDEX check Unreadable comes first (the reader refuses it).
    (
        "leaf-v1",
        "localhost",
        "CertificateNotTrusted Unreadable",
        &["v1", "name"],
    ),
    (
        "sha1",
        "localhost",
        "CertificateNotTrusted WeakAlgorithm",
        &["weak-algorithm"],
    ),
    (
        "md5",
        "localhost",
        "CertificateNotTrusted WeakAlgorithm",
        &["weak-algorithm"],
    ),
    // The 1024-bit key is the leaf's own and verifies no signature on the path: a validator
    // applies the weak-key rule to the leaf's key, not only to keys that verify a signature.
    (
        "rsa-1024",
        "localhost",
        "CertificateNotTrusted WeakKey",
        &["weak-key"],
    ),
];

/// The row of [`TABLE`] for `case`.
fn row(case: &str) -> (&'static str, &'static str, &'static [&'static str]) {
    let (_, host, outcome, defects) = TABLE
        .iter()
        .find(|(name, ..)| *name == case)
        .unwrap_or_else(|| panic!("{case}: a case of INDEX this test does not know"));
    (host, outcome, defects)
}

/// The names of `INDEX`, in order.
fn index_names() -> Vec<String> {
    fs::read_to_string(certs_dir().join("INDEX"))
        .unwrap()
        .lines()
        .map(|l| l.split(' ').next().unwrap().to_string())
        .collect()
}

#[test]
fn each_case_holds_the_defect_its_name_says_and_no_other() {
    let index = fs::read_to_string(certs_dir().join("INDEX")).unwrap();
    let mut wrong = Vec::new();
    let mut seen = 0;
    let mut structural = 0;
    for line in index.lines() {
        let fields: Vec<&str> = line.split(' ').collect();
        let (case, host) = (fields[0], fields[1]);
        let (want_host, want_outcome, want) = row(case);
        let outcome = fields[3..].join(" ");
        if host != want_host || outcome != want_outcome {
            wrong.push(format!(
                "{case}: INDEX has {host} {outcome}, the table {want_host} {want_outcome}"
            ));
        }
        if CANDIDATE_CASES.contains(&case) {
            structural += 1;
            continue;
        }
        seen += 1;
        let found = defects(case, host);
        let want: BTreeSet<&str> = want.iter().copied().collect();
        if found != want {
            wrong.push(format!("{case}: has {found:?}, its name says {want:?}"));
        }
    }
    let named: BTreeSet<String> = index_names().into_iter().collect();
    for (case, ..) in TABLE {
        if !named.contains(case) {
            wrong.push(format!("{case}: in the table, not in INDEX"));
        }
    }
    assert_eq!((seen, structural), (31, 2), "cases in INDEX");
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// The two cases of the limits of item 9.4: every certificate but the leaf and the bridge has
/// one subject; `candidate-limit` holds a path of three intermediates to the anchor (leaf, I0,
/// I1, the bridge), `candidate-limit-no-anchor` none.
#[test]
fn candidate_limit_cases_hold_their_structure() {
    let check = |case: &str, want: &[u8], len: usize| {
        let chain = pem_file(case, "chain.pem");
        let root = pem_file(case, "root.pem").remove(0);
        let anchor = inspect::certificate(&root).unwrap();
        let certs: Vec<inspect::Certificate<'_>> = chain
            .iter()
            .map(|d| inspect::certificate(d).unwrap())
            .collect();
        assert_eq!(certs.len(), len, "{case}");
        let name = certs[1].subject;
        assert!(
            certs[1..49]
                .iter()
                .all(|c| c.subject == name && c.issuer == name),
            "{case}"
        );
        assert_eq!(certs[0].issuer, name, "{case}");
        let verifies = |child: &inspect::Certificate<'_>, by: &inspect::Certificate<'_>| {
            let (kind, bytes) = inspect::public_key(by.spki).unwrap();
            let alg = inspect::signature_algorithm(child.signature_algorithm).unwrap();
            crypto::check_signature(
                &ShippedVerifier,
                alg,
                &PublicKey { kind, bytes },
                child.tbs,
                child.signature,
            )
            .is_ok()
        };
        let path: Vec<usize> = want.iter().map(|i| usize::from(*i)).collect();
        if path.is_empty() {
            assert!(certs.iter().all(|c| c.issuer != anchor.subject), "{case}");
        } else {
            let mut at = 0;
            for next in &path {
                assert!(
                    verifies(&certs[at], &certs[*next]),
                    "{case}: {at} by {next}"
                );
                at = *next;
            }
            assert_eq!(certs[at].issuer, anchor.subject, "{case}");
            assert!(
                verifies(&certs[at], &anchor),
                "{case}: the bridge by the anchor"
            );
        }
    };
    let names = index_names();
    for case in CANDIDATE_CASES {
        assert_eq!(
            names.iter().filter(|n| *n == case).count(),
            1,
            "{case} in INDEX"
        );
    }
    // Sent: the leaf, I47 down to I0 (I0 at 48, I1 at 47), the bridge at 49.
    check("candidate-limit", &[48, 47, 49], 50);
    check("candidate-limit-no-anchor", &[], 49);
}

// The check script, on a copy of the crate under CARGO_TARGET_TMPDIR.

const FAILED: &str =
    "tls-certs: the sources changed since certs/ was generated; run task tls-certs-slow";

fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), &target).unwrap();
        }
    }
}

fn copy_crate(label: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "tls-certs-adversary-{label}-{}",
        std::process::id()
    ));
    if dir.exists() {
        fs::remove_dir_all(&dir).unwrap();
    }
    for part in ["src", "keys", "openssl", "certs"] {
        copy_tree(&crate_dir().join(part), &dir.join(part));
    }
    fs::copy(crate_dir().join("Cargo.toml"), dir.join("Cargo.toml")).unwrap();
    dir
}

fn run_check(dir: &Path) -> (Option<i32>, String) {
    let out = Command::new("sh")
        .arg(crate_dir().join("../../taskfiles/tls-certs-check.sh"))
        .arg(dir)
        .output()
        .unwrap();
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    (out.status.code(), text)
}

/// A key file whose name holds a space is a source like any other: adding it changes what
/// certs/ may be made from, so the check fails.
#[test]
fn check_fails_on_an_added_source_whose_name_holds_a_space() {
    let dir = copy_crate("space");
    fs::copy(dir.join("keys/p256-00.pk8"), dir.join("keys/p256 99.pk8")).unwrap();
    let (code, text) = run_check(&dir);
    fs::remove_dir_all(&dir).unwrap();
    assert_eq!(code, Some(1), "{text}");
    assert!(text.lines().any(|l| l == FAILED), "{text}");
}

/// Without certs/INDEX there are no cases: the check does not print that certs/ matches.
#[test]
fn check_fails_without_index() {
    let dir = copy_crate("index");
    fs::remove_file(dir.join("certs/INDEX")).unwrap();
    let (code, text) = run_check(&dir);
    fs::remove_dir_all(&dir).unwrap();
    assert_ne!(code, Some(0), "{text}");
    assert!(!text.contains("certs/ matches its sources"), "{text}");
}
