//! The client's handshake of TLS 1.3 (RFC 8446 section 4; `story:tls-handshake`): the
//! ClientHello surf builds, the HelloRetryRequest with its cookie and a second share for
//! secp256r1, the middlebox compatibility mode of appendix D.4, EncryptedExtensions, a
//! CertificateRequest answered with an empty Certificate, the server's Certificate handed to a
//! [`crate::verify::ServerCertVerifier`], CertificateVerify and Finished; each deviation of the
//! server ends with the alert of RFC 8446 section 6.2 and the `Error` of docs/design/tls.md
//! item 6. After the handshake [`crate::connection::Connection`] reads tickets, key updates and
//! alerts.
//!
//! Private but for [`trace`], the test mode of the feature `test-tooling`, which runs the RFC
//! 8448 traces with their own ClientHellos over given bytes.

pub(crate) mod client;
pub(crate) mod codec;
pub(crate) mod messages;
#[cfg(feature = "test-tooling")]
pub mod trace;
