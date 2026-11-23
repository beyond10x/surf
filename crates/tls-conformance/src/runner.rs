//! Reads the limbo files, each once and side by side, and runs their cases.

use std::fs;
use std::path::Path;

use surf_html_conformance::json::{self, Value};
use surf_tls::der::Time;
use surf_tls::path::{INTERMEDIATES_MAX, Limits, PathVerifier};
use surf_tls::pem;
use surf_tls::verify::{CertificateRejection, ServerCertVerifier};
use surf_tls::x509::LeafKey;

use crate::LIMBO_TIME;

/// The data files, by case group (`SOURCE` says how they are cut).
pub const FILES: [&str; 5] = [
    "limbo-other.json",
    "limbo-bettertls-1.json",
    "limbo-bettertls-2.json",
    "limbo-bettertls-3.json",
    "limbo-bettertls-4.json",
];

/// What limbo expects of a case.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Expected {
    /// The leaf key.
    Success,
    /// A refusal.
    Failure,
}

/// How a case ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// Not run, for the reason given.
    NotRun(&'static str),
    /// Run: `ok`, or the refusal as [`outcome_text`] writes it.
    Ran(String),
}

/// A case and how it ended.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CaseResult {
    /// The limbo id.
    pub id: String,
    /// What limbo expects.
    pub expected: Expected,
    /// How it ended.
    pub outcome: Outcome,
    /// The refusal in full (its `Debug` form) when the case ran and was refused, else empty.
    pub detail: String,
}

/// A verifier's answer as a line of `INDEX` and of `limbo-kinds.txt` writes it: `ok`,
/// `CertificateExpired`, `CertificateNameMismatch` or `CertificateNotTrusted <check>`.
pub fn outcome_text(result: &Result<LeafKey, CertificateRejection>) -> String {
    match result {
        Ok(_) => "ok".into(),
        Err(CertificateRejection::Expired { .. }) => "CertificateExpired".into(),
        Err(CertificateRejection::NameMismatch { .. }) => "CertificateNameMismatch".into(),
        Err(CertificateRejection::NotTrusted(check)) => {
            format!("CertificateNotTrusted {}", check.name())
        }
    }
}

/// Runs every case of the files of [`FILES`] in `data`, one thread per file, in file order.
pub fn run_all(data: &Path) -> Result<Vec<CaseResult>, String> {
    let per_file: Vec<Result<Vec<CaseResult>, String>> = std::thread::scope(|scope| {
        let handles: Vec<_> = FILES
            .iter()
            .map(|file| scope.spawn(move || run_file(&data.join(file))))
            .collect();
        handles
            .into_iter()
            .map(|h| {
                h.join()
                    .unwrap_or_else(|_| Err("a runner thread panicked".into()))
            })
            .collect()
    });
    let mut out = Vec::new();
    for file in per_file {
        out.extend(file?);
    }
    Ok(out)
}

/// Reads the file at `path` once and runs its cases.
pub fn run_file(path: &Path) -> Result<Vec<CaseResult>, String> {
    let at = path.display();
    let text = fs::read_to_string(path).map_err(|e| format!("{at}: {e}"))?;
    let value = json::parse(&text).map_err(|e| format!("{at}: {e}"))?;
    let Some(Value::Array(cases)) = value.get("testcases") else {
        return Err(format!("{at}: no array testcases"));
    };
    cases
        .iter()
        .map(|case| run_case(case).map_err(|e| format!("{at}: {e}")))
        .collect()
}

fn text(points: &[u32]) -> String {
    points
        .iter()
        .map(|&p| char::from_u32(p).unwrap_or(char::REPLACEMENT_CHARACTER))
        .collect()
}

fn string(case: &Value, key: &str) -> Option<String> {
    match case.get(key)? {
        Value::String(points) => Some(text(points)),
        _ => None,
    }
}

fn strings(case: &Value, key: &str) -> Vec<String> {
    match case.get(key) {
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(|v| match v {
                Value::String(points) => Some(text(points)),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// Why the case does not run, or `None`.
fn not_run(case: &Value) -> Option<&'static str> {
    if string(case, "validation_kind").as_deref() != Some("SERVER") {
        return Some("client");
    }
    let features = strings(case, "features");
    if features.iter().any(|f| f == "has-crl") {
        return Some("crl");
    }
    if features.iter().any(|f| f.starts_with("pedantic-")) {
        return Some("pedantic");
    }
    if features.iter().any(|f| f == "has-policy-constraints") {
        return Some("policy");
    }
    let named = case.get("expected_peer_name").is_some_and(|name| {
        matches!(string(name, "kind").as_deref(), Some("DNS" | "IP"))
            && string(name, "value").is_some()
    });
    (!named).then_some("no-peer-name")
}

/// `YYYY-MM-DDTHH:MM:SS`, then optional fractional seconds (dropped), then `Z` or `+00:00`.
pub fn parse_time(text: &str) -> Option<Time> {
    let b = text.as_bytes();
    if b.len() < 20 || b[4] != b'-' || b[7] != b'-' || b[10] != b'T' || b[13] != b':' {
        return None;
    }
    let n = |range: std::ops::Range<usize>| -> Option<u32> {
        let field = text.get(range)?;
        field
            .bytes()
            .all(|c| c.is_ascii_digit())
            .then(|| field.parse().ok())?
    };
    let mut rest = text.get(19..)?;
    if let Some(fraction) = rest.strip_prefix('.') {
        let digits = fraction.bytes().take_while(u8::is_ascii_digit).count();
        if digits == 0 {
            return None;
        }
        rest = &fraction[digits..];
    }
    if rest != "Z" && rest != "+00:00" {
        return None;
    }
    Time::from_utc(
        n(0..4)?,
        n(5..7)?,
        n(8..10)?,
        n(11..13)?,
        n(14..16)?,
        n(17..19)?,
    )
}

/// The DER of each certificate of a PEM text, or the reason it does not decode.
fn certificates(pem_text: &str) -> Result<Vec<Vec<u8>>, String> {
    let blocks = pem::parse(pem_text.as_bytes()).map_err(|e| format!("PEM: {e:?}"))?;
    Ok(blocks.into_iter().map(|b| b.data).collect())
}

/// Runs one case: `Err` only when the case is not of limbo's form.
pub fn run_case(case: &Value) -> Result<CaseResult, String> {
    let id = string(case, "id").ok_or("a case without an id")?;
    let expected = match string(case, "expected_result").as_deref() {
        Some("SUCCESS") => Expected::Success,
        Some("FAILURE") => Expected::Failure,
        _ => {
            return Err(format!(
                "{id}: expected_result is neither SUCCESS nor FAILURE"
            ));
        }
    };
    let (outcome, detail) = match not_run(case) {
        Some(reason) => (Outcome::NotRun(reason), String::new()),
        None => {
            let (outcome, detail) = run(case).map_err(|e| format!("{id}: {e}"))?;
            (Outcome::Ran(outcome), detail)
        }
    };
    Ok(CaseResult {
        id,
        expected,
        outcome,
        detail,
    })
}

fn run(case: &Value) -> Result<(String, String), String> {
    let time_text = match case.get("validation_time") {
        Some(Value::String(points)) => text(points),
        _ => LIMBO_TIME.to_string(),
    };
    let now = parse_time(&time_text).ok_or(format!("validation_time {time_text}"))?;
    let host = case
        .get("expected_peer_name")
        .and_then(|name| string(name, "value"))
        .ok_or("no expected_peer_name")?;
    let mut verifier = PathVerifier::new(Vec::new());
    if let Some(Value::Number(depth)) = case.get("max_chain_depth") {
        let depth: usize = depth
            .parse()
            .map_err(|_| format!("max_chain_depth {depth}"))?;
        if depth < INTERMEDIATES_MAX {
            let limits = Limits {
                intermediates: depth,
                ..verifier.limits()
            };
            verifier = verifier.with_limits(limits);
        }
    }
    for root in strings(case, "trusted_certs") {
        // A root the reader refuses is no anchor; its case then has no path.
        for der in certificates(&root).unwrap_or_default() {
            let _ = verifier.add_root(&der);
        }
    }
    let peer = string(case, "peer_certificate").ok_or("no peer_certificate")?;
    let mut chain = match certificates(&peer) {
        Ok(der) => der,
        Err(e) => return Ok(("unreadable-pem".into(), e)),
    };
    for intermediate in strings(case, "untrusted_intermediates") {
        match certificates(&intermediate) {
            Ok(der) => chain.extend(der),
            Err(e) => return Ok(("unreadable-pem".into(), e)),
        }
    }
    let refs: Vec<&[u8]> = chain.iter().map(Vec::as_slice).collect();
    let result = verifier.verify(&refs, &host, now);
    let detail = match &result {
        Ok(_) => String::new(),
        Err(rejection) => format!("{rejection:?}"),
    };
    Ok((outcome_text(&result), detail))
}
