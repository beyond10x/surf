//! The live view's HTTP server, with the standard library and `surf-secret` only.
//!
//! [`serve`] answers what `docs/view-stream.md` states: HTTP/1.1, `GET` only, one request per
//! connection, a fixed route table (the page's files, `/api/sessions`, `/api/sessions/<id>` and
//! `/events`), the `Host` check against DNS rebinding, the view token compared in constant time
//! by `surf-secret`, the head lines and refusal bodies byte for byte, and server-sent events with
//! a bounded queue per reader. The limits of item 8 of `docs/design/live-view.md` are the fields
//! of [`Limits`].
//!
//! The data behind the routes comes from a [`Source`] the caller implements; the server keeps
//! no data of its own. [`page_files`] gives the page itself, the files of `crates/view/web/`
//! compiled in; the example `prototype` serves it over scripted sessions.
//!
//! What it does not do: TLS, IPv6, `HEAD`, compression, ranges, persistent connections, a
//! request body, or any route that changes something.

#![forbid(unsafe_code)]

mod assets;
mod http;
mod sse;

pub use assets::page_files;
pub use http::{Config, Limits, PageFile, Server, Source, serve};
pub use sse::{EventKind, Item, Next, Reader, TooOld};
