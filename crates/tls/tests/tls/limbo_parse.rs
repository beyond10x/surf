//! x509-limbo (`tests/data/x509-limbo/`): every distinct certificate of the vendored cases read
//! by `surf_tls::x509` or refused with a named error, in the counts `parse-counts.txt` holds
//! (`story:tls-x509`, acceptance 5), and the vendored files against the digests of their
//! `SOURCE` (acceptance 6).
//!
//! The cases lie in five files by case group (`SOURCE` says how): one test per file, each
//! reading its file once, so the tests run side by side and none reads the 20 MiB whole. A
//! certificate is distinct within its file; one that two files hold is counted in each.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use surf_html_conformance::json::{self, Value};
use surf_tls::crypto::{self, HashAlgorithm};
use surf_tls::pem;
use surf_tls::x509::Certificate;

use crate::x509_parse::{LIMBO_CASE, LIMBO_LEAF};

/// The data files, each with the case group it holds.
const FILES: [&str; 5] = [
    "limbo-other.json",
    "limbo-bettertls-1.json",
    "limbo-bettertls-2.json",
    "limbo-bettertls-3.json",
    "limbo-bettertls-4.json",
];

/// The cases limbo expects to succeed that hold a certificate the reader refuses as designed:
/// case id, the reader's error, and why. Each is written to `parse-counts.txt` as an
/// `expected-refusal` line; any other such case is a `finding` line.
const EXPECTED_REFUSALS: [(&str, &str, &str); 3] = [
    ("rfc9881::ml-dsa-44", "UnsupportedAlgorithm", ML_DSA),
    ("rfc9881::ml-dsa-65", "UnsupportedAlgorithm", ML_DSA),
    ("rfc9881::ml-dsa-87", "UnsupportedAlgorithm", ML_DSA),
];

const ML_DSA: &str = "ML-DSA is not among the signature algorithms of docs/design/tls.md item 14";

fn limbo_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/data/x509-limbo")
}

fn points(value: &Value) -> Option<&[u32]> {
    match value {
        Value::String(points) => Some(points),
        _ => None,
    }
}

fn text(points: &[u32]) -> String {
    points
        .iter()
        .map(|&p| char::from_u32(p).unwrap_or(char::REPLACEMENT_CHARACTER))
        .collect()
}

/// The PEM texts of one case, each with the field it stands in: trusted certificates,
/// intermediates, the peer's certificate.
fn pem_texts(case: &Value) -> Vec<(&'static str, &[u32])> {
    let mut out = Vec::new();
    for key in ["trusted_certs", "untrusted_intermediates"] {
        if let Some(Value::Array(items)) = case.get(key) {
            out.extend(items.iter().filter_map(points).map(|p| (key, p)));
        }
    }
    out.extend(
        case.get("peer_certificate")
            .and_then(points)
            .map(|p| ("peer_certificate", p)),
    );
    out
}

/// The verdict on each certificate of one PEM text: `None` read, else the error's name. A PEM
/// text that does not decode, holds no block or a block that is not a certificate counts as
/// one certificate refused, under the PEM error's name, `NoBlock` or `NotACertificate`.
enum Decoded {
    Der(Vec<u8>),
    Refused(&'static str),
}

fn decode(pem_points: &[u32]) -> Vec<Decoded> {
    match pem::parse(text(pem_points).as_bytes()) {
        Ok(blocks) if blocks.is_empty() => vec![Decoded::Refused("NoBlock")],
        Ok(blocks) => blocks
            .into_iter()
            .map(|b| {
                if b.label == "CERTIFICATE" {
                    Decoded::Der(b.data)
                } else {
                    Decoded::Refused("NotACertificate")
                }
            })
            .collect(),
        Err(e) => vec![Decoded::Refused(e.name())],
    }
}

/// The counts of one file, each line starting with the file's name: cases, distinct
/// certificates, read, refused in all and per error kind, and one `finding` line per case
/// limbo expects to succeed that holds a refused certificate (its id and the error).
fn tally(file: &str) -> String {
    let started = Instant::now();
    let data = fs::read_to_string(limbo_dir().join(file)).expect("read the file");
    let root = json::parse(&data).expect("the file is JSON");
    let parsed = started.elapsed();
    let Some(Value::Array(cases)) = root.get("testcases") else {
        panic!("{file} has no testcases");
    };

    // Each distinct PEM text decoded once, each distinct certificate read once.
    let mut by_pem: HashMap<&[u32], Vec<Option<&'static str>>> = HashMap::new();
    let mut seen_der: HashSet<Vec<u8>> = HashSet::new();
    let (mut certificates, mut read) = (0usize, 0usize);
    let mut refused: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut findings = Vec::new();
    let mut refusal_lines = Vec::new();
    let mut leaf_checked = false;
    for case in cases {
        let id = case
            .get("id")
            .and_then(points)
            .map(text)
            .expect("a case id");
        if id == LIMBO_CASE {
            let peer = case.get("peer_certificate").and_then(points).map(text);
            assert_eq!(
                peer.as_deref(),
                Some(LIMBO_LEAF),
                "the leaf of {LIMBO_CASE}"
            );
            leaf_checked = true;
        }
        let expected_result = case
            .get("expected_result")
            .and_then(points)
            .map(text)
            .unwrap_or_default();
        let succeeds = expected_result == "SUCCESS";
        let mut refusals_of_case = Vec::new();
        for (field, pem_points) in pem_texts(case) {
            let verdicts = by_pem.entry(pem_points).or_insert_with(|| {
                decode(pem_points)
                    .into_iter()
                    .map(|decoded| {
                        let verdict = match &decoded {
                            Decoded::Der(der_bytes) => {
                                Certificate::from_der(der_bytes).err().map(|e| e.name())
                            }
                            Decoded::Refused(kind) => Some(*kind),
                        };
                        let new = match decoded {
                            Decoded::Der(der_bytes) => seen_der.insert(der_bytes),
                            Decoded::Refused(_) => true,
                        };
                        if new {
                            certificates += 1;
                            match verdict {
                                None => read += 1,
                                Some(kind) => *refused.entry(kind).or_default() += 1,
                            }
                        }
                        verdict
                    })
                    .collect()
            });
            for kind in verdicts.iter().flatten() {
                refusal_lines.push(format!("{id} {expected_result} {field} {kind}\n"));
            }
            refusals_of_case.extend(verdicts.iter().flatten().copied());
        }
        if succeeds {
            refusals_of_case.sort_unstable();
            refusals_of_case.dedup();
            for kind in &refusals_of_case {
                let line = match EXPECTED_REFUSALS
                    .iter()
                    .find(|(case_id, error, _)| *case_id == id && error == kind)
                {
                    Some((_, _, why)) => format!("expected-refusal {id} {kind}: {why}"),
                    None => format!("finding {id} {kind}"),
                };
                findings.push(line);
            }
        }
    }
    findings.sort();
    // Every refusal with its case, the case's expected result and the field, for a reader of
    // the run (not compared): `target/tmp/limbo-refusals-<file>.txt`.
    let list = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("limbo-refusals-{file}.txt"));
    fs::write(&list, refusal_lines.concat()).expect("write the refusals of this run");

    let total: usize = refused.values().sum();
    let mut out = format!(
        "{file} cases {}\n{file} certificates {certificates}\n{file} read {read}\n\
         {file} refused {total}\n",
        cases.len()
    );
    for (kind, n) in &refused {
        out.push_str(&format!("{file} refused {kind} {n}\n"));
    }
    for finding in &findings {
        out.push_str(&format!("{file} {finding}\n"));
    }
    if leaf_checked {
        out.push_str(&format!("{file} acceptance-1 {LIMBO_CASE}\n"));
    }
    println!(
        "limbo_parse: {file}: JSON read in {:.3} s, all in {:.3} s",
        parsed.as_secs_f64(),
        started.elapsed().as_secs_f64()
    );
    print!("{out}");
    out
}

/// The lines of `parse-counts.txt` for `file`.
fn expected(file: &str) -> String {
    let counts =
        fs::read_to_string(limbo_dir().join("parse-counts.txt")).expect("read parse-counts.txt");
    let prefix = format!("{file} ");
    counts
        .lines()
        .filter(|line| line.starts_with(&prefix))
        .map(|line| format!("{line}\n"))
        .collect()
}

fn check(file: &str) {
    let actual = tally(file);
    if actual != expected(file) {
        let out = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("parse-counts-{file}.txt"));
        fs::write(&out, &actual).expect("write the counts of this run");
        panic!(
            "the counts of {file} differ from parse-counts.txt; this run's are in {}",
            out.display()
        );
    }
}

#[test]
fn other_groups() {
    check(FILES[0]);
    assert!(
        expected(FILES[0]).contains(&format!(" acceptance-1 {LIMBO_CASE}\n")),
        "the leaf of {LIMBO_CASE} was compared with the one of x509_parse"
    );
    // Each expected refusal was met, under its error and reason.
    for (id, error, why) in EXPECTED_REFUSALS {
        let line = format!("{} expected-refusal {id} {error}: {why}\n", FILES[0]);
        assert!(expected(FILES[0]).contains(&line), "{line}");
    }
}

#[test]
fn bettertls_1() {
    check(FILES[1]);
}

#[test]
fn bettertls_2() {
    check(FILES[2]);
}

#[test]
fn bettertls_3() {
    check(FILES[3]);
}

#[test]
fn bettertls_4() {
    check(FILES[4]);
}

/// Every line of `parse-counts.txt` belongs to one of the files.
#[test]
fn parse_counts_name_only_the_files() {
    let counts =
        fs::read_to_string(limbo_dir().join("parse-counts.txt")).expect("read parse-counts.txt");
    for line in counts.lines() {
        assert!(
            FILES.iter().any(|f| line.starts_with(&format!("{f} "))),
            "{line}"
        );
    }
    for file in FILES {
        assert!(expected(file).contains(" cases "), "{file} has its counts");
    }
}

// ---------------------------------------------------------------------------------------------
// Acceptance 6: the digests of the vendored files.

/// Every file of `dir` but `SOURCE` and `parse-counts.txt` (written here, not vendored) against
/// the sha256 its `SOURCE` lists, hashed through the seam: the number of files compared, or one
/// line per file that differs, is missing or is not listed.
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
                let actual: String = crypto::hash(HashAlgorithm::Sha256, &content)
                    .as_ref()
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect();
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
        let own = name == "SOURCE" || name == "parse-counts.txt";
        if !own && !listed.iter().any(|(_, n)| *n == name) {
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
    let dir = limbo_dir();
    assert_eq!(
        compare_digests(&dir),
        Ok(6),
        "the five data files and LICENSE"
    );
    // A copy of SOURCE and LICENSE alone, LICENSE changed, one file more: three kinds of
    // problem, and the five data files missing.
    let copy = Path::new(env!("CARGO_TARGET_TMPDIR")).join("x509-limbo-digests");
    let _ = fs::remove_dir_all(&copy);
    fs::create_dir_all(&copy).expect("create the copy");
    for name in ["SOURCE", "LICENSE"] {
        fs::copy(dir.join(name), copy.join(name)).expect("copy a file");
    }
    let mut license = fs::read(copy.join("LICENSE")).expect("read the copy");
    license[100] ^= 0x01;
    fs::write(copy.join("LICENSE"), license).expect("write the copy");
    fs::write(copy.join("extra.json"), b"{}").expect("write a file SOURCE does not list");
    let problems = compare_digests(&copy).expect_err("a changed byte fails");
    assert_eq!(problems.len(), 7, "{problems:?}");
    assert!(
        problems.iter().any(|p| p.starts_with("LICENSE: sha256 ")),
        "{problems:?}"
    );
    assert!(
        problems.contains(&"extra.json: not listed in SOURCE".to_string()),
        "{problems:?}"
    );
    for file in FILES {
        assert!(
            problems.iter().any(|p| p.starts_with(&format!("{file}: "))),
            "{problems:?}"
        );
    }
    let _ = fs::remove_dir_all(&copy);
}
