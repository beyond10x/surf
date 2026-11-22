//! The integration tests of `surf-tls-test-server`, one test binary: one module per file of
//! `tests/` (AGENTS.md, Layout). Each test starts its own server on 127.0.0.1 port 0 and drives
//! the client of `surf-tls` (`surf_tls::connection::Connection`) against it, with a verifier
//! double that answers the key of the case's leaf. Every client socket has a read timeout, so no
//! test waits longer than its bound; no test reaches the network.

#![forbid(unsafe_code)]

mod adversary_server;
mod common;
mod handshake;
mod hostile;
mod hundred;
mod modes;
