//! TLS 1.3 for surf's `https:` (RFC 8446), the client side, with the server authenticated
//! against trust anchors. Design: `docs/design/tls.md`.
//!
//! The cryptographic primitives come from ring 0.17.14, the one third-party crate in what
//! surf ships (README.md, principle 4). The module [`crypto`], the file `crypto.rs`, is the
//! seam: it alone names ring, and every other module reaches the primitives through it. The
//! protocol code on top of it is written here.
//!
//! Not done: TLS 1.2, which a later epic adds (a server limited to it fails with one of the
//! kinds of [`error::Error`]). Not checked: revocation; a revoked certificate that has not
//! expired loads.
//!
//! The cargo feature `test-tooling`, off by default and never on in a release, adds signing
//! from PKCS#8 and the fixed random sources to [`crypto`], and `TEST_TOOLING_MARKER`.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod base64;
pub mod connection;
pub mod crypto;
pub mod der;
pub mod error;
#[cfg(not(feature = "test-tooling"))]
mod handshake;
#[cfg(feature = "test-tooling")]
pub mod handshake;
pub mod key_schedule;
pub mod pem;
pub mod record;
pub mod transcript;
pub mod verify;
pub mod x509;

/// A fixed string that a binary built with the feature `test-tooling` carries, so that a
/// release can be checked for it.
#[cfg(feature = "test-tooling")]
pub const TEST_TOOLING_MARKER: &str = "surf-tls test-tooling, never in a release";
