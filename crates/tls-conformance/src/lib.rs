//! surf-tls-conformance: runs the vendored x509-limbo cases
//! (`crates/tls/tests/data/x509-limbo/`) through the path validation of surf-tls
//! ([`surf_tls::path::PathVerifier`]) and counts the cases that exist, run and pass
//! (`story:tls-x509-path`; docs/design/tls.md, items 9, 10 and 13). Test tooling, not shipped.
//!
//! Every SERVER case runs at its `validation_time`, or at [`LIMBO_TIME`] when it has none, never
//! at the wall clock, so the counts do not move with the clock. A case does not run when it is a
//! CLIENT case (`client`), holds CRLs (`crl`), has a feature limbo marks pedantic (`pedantic`),
//! constrains certificate policies (`policy`) or names no peer (`no-peer-name`); each such case
//! is printed with its reason.
//!
//! A SUCCESS case passes when the verifier returns the leaf key. A FAILURE case passes when the
//! verifier refuses it with the kind and check of its line of `expected/limbo-kinds.txt`; one
//! that ends with another kind is printed (`wrong-kind`) and does not pass, and one the verifier
//! accepts is printed (`accepted`) and does not pass.
//!
//! The binary prints, in order: `x509-limbo: <exist> cases, <run> run, <pass> passed`; a line
//! `group <name> <pass>/<run>/<exist>` per limbo group (the id before its first `::`); a line
//! `section <number> <pass>/<run>/<exist> complete|incomplete` per RFC 5280 section of
//! [`SECTIONS`] (`complete` only when every case of it runs and passes); then `not-run <id>
//! <reason>`, `refused <id> <outcome> (<refusal>)`, `wrong-kind <id> <outcome> expected <outcome>
//! (<refusal>)` and `accepted <id> expected <outcome>` lines. `--check` adds what [`check`] finds.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod check;
pub mod runner;

use std::path::PathBuf;

/// The time a case without `validation_time` runs at: the `commit-date` of the pinned limbo
/// commit, as `crates/tls/tests/data/x509-limbo/SOURCE` records it (docs/design/tls.md, item 13).
pub const LIMBO_TIME: &str = "2026-11-17T15:57:29Z";

/// The RFC 5280 sections the cases map to: a prefix of case ids and the section. A case maps to
/// the section of the first prefix it starts with, or to none.
pub const SECTIONS: &[(&str, &str)] = &[
    ("rfc5280::mismatching-signature-algorithm", "4.1.1.2"),
    ("rfc5280::ee-empty-issuer", "4.1.2.4"),
    ("rfc5280::validity::", "4.1.2.5"),
    ("rfc5280::ca-empty-subject", "4.1.2.6"),
    ("rfc5280::unknown-critical-extension-", "4.2"),
    ("rfc5280::duplicate-extensions", "4.2"),
    ("rfc5280::aki::", "4.2.1.1"),
    ("webpki::aki::", "4.2.1.1"),
    ("rfc5280::ski::", "4.2.1.2"),
    ("rfc5280::ica-ku-keycertsign", "4.2.1.3"),
    ("rfc5280::leaf-ku-keycertsign", "4.2.1.3"),
    ("rfc5280::no-keyusage", "4.2.1.3"),
    ("rfc5280::san::", "4.2.1.6"),
    ("rfc5280::intermediate-ca-without-ca-bit", "4.2.1.9"),
    (
        "rfc5280::intermediate-ca-missing-basic-constraints",
        "4.2.1.9",
    ),
    ("rfc5280::root-missing-basic-constraints", "4.2.1.9"),
    ("rfc5280::root-non-critical-basic-constraints", "4.2.1.9"),
    ("rfc5280::no-basicconstraints", "4.2.1.9"),
    ("pathlen::", "4.2.1.9"),
    ("rfc5280::nc::", "4.2.1.10"),
    ("webpki::nc::", "4.2.1.10"),
    ("bettertls::nameconstraints::", "4.2.1.10"),
    ("rfc5280::eku::", "4.2.1.12"),
    ("rfc5280::chain-untrusted-root", "6.1"),
    ("rfc5280::root-and-intermediate-swapped", "6.1"),
    ("pathological::", "6.1"),
];

/// Where the files lie, relative to the repository root.
pub mod paths {
    use super::PathBuf;

    /// The limbo data.
    pub const DATA: &str = "crates/tls/tests/data/x509-limbo";
    /// The expected counts and open cases.
    pub const EXPECTED: &str = "crates/tls-conformance/expected/limbo.txt";
    /// The kinds the FAILURE cases end with.
    pub const KINDS: &str = "crates/tls-conformance/expected/limbo-kinds.txt";
    /// The negative suite of `story:tls-test-certs`, whose outcomes no accepted case may have.
    pub const INDEX: &str = "crates/tls-test-certs/certs/INDEX";

    /// The repository root this crate was built in.
    pub fn root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
    }
}

/// The section of [`SECTIONS`] the case `id` maps to.
pub fn section_of(id: &str) -> Option<&'static str> {
    SECTIONS
        .iter()
        .find(|(prefix, _)| id.starts_with(prefix))
        .map(|(_, section)| *section)
}

/// The group of the case `id`: the id before its first `::`.
pub fn group_of(id: &str) -> &str {
    id.split_once("::").map_or(id, |(group, _)| group)
}
