//! The integration tests of `surf-fetch`, one test binary: one module per file of `tests/`.
//! A new test file gets its `mod` line here (AGENTS.md, Layout).

mod adversary_redirect_memory;
mod adversary_tls_fetch;
mod cookies;
mod fetch;
mod fetch_bounds;
mod https;
mod post;
mod redirects;
mod review1_fetch;
mod review1_security_fetch;
mod review2_fetch_limits;
mod secreview_tls_fetch;
