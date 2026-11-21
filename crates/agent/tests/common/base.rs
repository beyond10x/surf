//! The base directory of the tests of `surf-agent` and `surfd`, one definition: the tests of
//! `surfd` include this file with `#[path]`.

use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// Where the test directories go: the test process's own `XDG_RUNTIME_DIR` when it is set,
/// absolute, and an existing directory owned by the caller (a short per-user directory, so
/// that the socket paths do not depend on where the tree is checked out); otherwise cargo's
/// temporary directory of the target. Never `/tmp`. The tests give `surf-agent` an
/// `XDG_RUNTIME_DIR` inside their own directory, so nothing touches the real
/// `$XDG_RUNTIME_DIR/surf`.
pub fn test_base() -> &'static Path {
    static BASE: OnceLock<PathBuf> = OnceLock::new();
    BASE.get_or_init(|| {
        let caller = surf_agent::wire::caller_uid().ok();
        std::env::var_os("XDG_RUNTIME_DIR")
            .map(PathBuf::from)
            .filter(|dir| dir.is_absolute())
            .filter(|dir| {
                fs::symlink_metadata(dir).is_ok_and(|m| m.is_dir() && Some(m.uid()) == caller)
            })
            .unwrap_or_else(|| PathBuf::from(env!("CARGO_TARGET_TMPDIR")))
    })
}
