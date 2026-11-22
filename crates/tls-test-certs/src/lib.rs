//! The test certificates of surf-tls (test tooling, not shipped; docs/design/tls.md, items 5,
//! 6 and 10): a DER writer of its own and a certificate builder that signs through the seam of
//! `surf-tls` (feature `test-tooling`) under a fixed random source, so that every certificate
//! it makes is reproducible byte for byte.
//!
//! The cases are made once, by the lane `slow` (`task tls-certs-slow`), into `certs/`, which is
//! committed with `certs/DIGEST`, the sha256 of the sources it was made from. The gate's check
//! (`task tls-certs-check`) only compares that digest; tests read the committed files. Three
//! cases ring cannot sign (a SHA-1 and an MD5 signature, an RSA key of 1024 bits) are made by
//! `openssl/make.sh` in the same lane, from the same keys.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod cases;
pub mod cert;
pub mod der;
pub mod inspect;
pub mod keys;
pub mod output;
pub mod pem;

use std::fmt;

/// The time every case of `INDEX` is validated at (docs/design/tls.md, item 13).
pub const VALIDATION_TIME: &str = "2026-11-22T00:00:00Z";

/// The cases `openssl/make.sh` makes, which ring cannot sign; it appends their lines to `INDEX`.
pub const OPENSSL_CASES: [&str; 3] = ["sha1", "md5", "rsa-1024"];

/// Why a case could not be made.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Error(pub String);

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}
