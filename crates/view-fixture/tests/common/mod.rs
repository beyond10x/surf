//! What the tests of `surf-view-fixture` share: the directory of `surf-agent` and
//! `surf-session`.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// The directory of `surf-agent` and `surf-session`: `SURF_BIN_DIR` when it is set (nothing is
/// built), as `bin_dir()` of `crates/session/tests/common/mod.rs` reads it; otherwise
/// `target/<profile>/` of this test, where they are built with the cargo running these tests
/// (nothing is built when they are fresh). Asserts that both programs are in it.
pub fn bin_dir() -> PathBuf {
    let dir = match std::env::var_os("SURF_BIN_DIR") {
        Some(dir) => PathBuf::from(dir),
        None => built(),
    };
    for program in ["surf-agent", "surf-session"] {
        assert!(
            dir.join(program).is_file(),
            "{program} is not in {}",
            dir.display()
        );
    }
    dir
}

/// `target/<profile>/` of this test, after `cargo build -p surf-agent -p surf-session`.
fn built() -> PathBuf {
    let exe = std::env::current_exe().expect("the test's executable");
    let profile_dir = exe
        .parent()
        .and_then(Path::parent)
        .expect("target/<profile>")
        .to_path_buf();
    let profile = match profile_dir.file_name().and_then(|name| name.to_str()) {
        Some("debug") => "dev".to_string(),
        Some(other) => other.to_string(),
        None => panic!("no profile directory above {}", exe.display()),
    };
    let status = Command::new(env!("CARGO"))
        .args([
            "build",
            "--locked",
            "--quiet",
            "-p",
            "surf-agent",
            "-p",
            "surf-session",
            "--profile",
            &profile,
        ])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .stdin(Stdio::null())
        .status()
        .expect("cargo runs");
    assert!(
        status.success(),
        "building surf-agent and surf-session: {status}"
    );
    profile_dir
}
