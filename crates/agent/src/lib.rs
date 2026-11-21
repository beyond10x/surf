//! `surf-agent`, the command line an AI agent types to drive a surf session: a stateless client
//! of `surfd` (design section 3 of `docs/design/agent-interface.md`). It parses its words
//! ([`cli`], held to `cli/surf-agent.yaml`), reads the token, sends one request to `surfd` over
//! [`wire`] and prints the answer ([`client`]). `open` starts `surfd` when none is running.
//!
//! The commands of slice 1: `open [<url>]`, `go <url>`, `look`, `close`. The session comes from
//! `--session <id>` or `SURF_SESSION`, the token from `--token-file <path>` or `SURF_TOKEN`;
//! `open` takes neither. The socket is `$XDG_RUNTIME_DIR/surf/surfd.sock`.
//!
//! Standard library, `surf-answer` (its quoting function and `unquote`) and `surf-secret` (the
//! token) only.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod cli;
pub mod client;
pub mod wire;
