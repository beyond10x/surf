//! `task lock-check` (Taskfile.yml, which runs `taskfiles/lock-check.sh` over the root
//! `Cargo.lock`, `taskfiles/lock-allow.txt`, `taskfiles/lock-owners.txt` and the root
//! `Cargo.toml`): nothing that ships uses a third-party crate but the packages of ring and of
//! the script engine swell, which the allow-list pins by name, version, source and checksum
//! (`none` for a package from git); a crate of this repository depends on one of them, or on
//! `surf-script`, only by a line of the owners file; and each package from git has
//! `overflow-checks = true` in the root manifest's release profile. Each case copies
//! `Cargo.lock`, `Cargo.toml`, `taskfiles/lock-check.sh`, `taskfiles/lock-allow.txt` and
//! `taskfiles/lock-owners.txt` into a directory of its own under `CARGO_TARGET_TMPDIR`, changes
//! one of them or none, and runs the task's line there, [`LINE`], without `task`.
//! `the_unchanged_lock_passes` holds the task to that line with `task --dry --verbose
//! lock-check` in the repository, so it needs `task` on `PATH`, as `task check` does.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const CRATES_IO: &str = "registry+https://github.com/rust-lang/crates.io-index";

/// The source cargo writes for the six packages of swell at the tag the root manifest names.
const SWELL: &str =
    "git+https://github.com/beyond10x/swell?tag=0.8.0#0c1827128b57845e203713e87a0b0949fc8d63be";

/// The six packages a host takes from swell (`docs/design/scripts.md`, Q13).
const SWELL_PACKAGES: [&str; 6] = [
    "swell",
    "swell-compiler",
    "swell-num",
    "swell-regexp",
    "swell-syntax",
    "swell-unicode",
];

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The root `Cargo.lock` as committed.
fn lock() -> String {
    fs::read_to_string(repository().join("Cargo.lock")).expect("read Cargo.lock")
}

/// The allow-list as committed.
fn allow_list() -> String {
    fs::read_to_string(repository().join("taskfiles/lock-allow.txt"))
        .expect("read taskfiles/lock-allow.txt")
}

/// The owners file as committed.
fn owners() -> String {
    fs::read_to_string(repository().join("taskfiles/lock-owners.txt"))
        .expect("read taskfiles/lock-owners.txt")
}

/// The root `Cargo.toml` as committed.
fn manifest() -> String {
    fs::read_to_string(repository().join("Cargo.toml")).expect("read Cargo.toml")
}

/// The line of `task lock-check` (Taskfile.yml), which `check` runs in a copy.
const LINE: &str = "sh taskfiles/lock-check.sh Cargo.lock taskfiles/lock-allow.txt taskfiles/lock-owners.txt Cargo.toml";

/// The files of one copy: the lock, the allow-list, the owners file and the root manifest.
struct Files {
    lock: String,
    allow: String,
    owners: String,
    manifest: String,
}

impl Files {
    /// The four files as committed.
    fn committed() -> Self {
        Self {
            lock: lock(),
            allow: allow_list(),
            owners: owners(),
            manifest: manifest(),
        }
    }
}

/// A fresh copy named `name` of the check's script with `files`.
fn copy_files(name: &str, files: &Files) -> PathBuf {
    let root = repository();
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("lock-check")
        .join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("taskfiles")).expect("create copy");
    fs::copy(
        root.join("taskfiles/lock-check.sh"),
        dir.join("taskfiles/lock-check.sh"),
    )
    .expect("copy taskfiles/lock-check.sh");
    fs::write(dir.join("taskfiles/lock-allow.txt"), &files.allow).expect("write lock-allow.txt");
    fs::write(dir.join("taskfiles/lock-owners.txt"), &files.owners).expect("write lock-owners.txt");
    fs::write(dir.join("Cargo.lock"), &files.lock).expect("write Cargo.lock");
    fs::write(dir.join("Cargo.toml"), &files.manifest).expect("write Cargo.toml");
    dir
}

/// A fresh copy named `name` of the check's script, `allow` as its allow-list and `lock` as the
/// root `Cargo.lock`, with the owners file and the root manifest as committed.
fn copy_with(name: &str, lock: &str, allow: &str) -> PathBuf {
    copy_files(
        name,
        &Files {
            lock: lock.to_string(),
            allow: allow.to_string(),
            ..Files::committed()
        },
    )
}

/// A fresh copy named `name` with `extra` appended to the lock.
fn copy(name: &str, extra: &str) -> PathBuf {
    let mut lock = lock();
    lock.push_str(extra);
    copy_with(name, &lock, &allow_list())
}

/// The line of `task lock-check` in `dir`, [`LINE`]: its exit status and everything it printed.
fn check(dir: &Path) -> (Option<i32>, String) {
    let mut words = LINE.split(' ');
    let output = Command::new(words.next().expect("a program"))
        .args(words)
        .current_dir(dir)
        .output()
        .expect("run sh");
    let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&output.stderr));
    (output.status.code(), text)
}

/// Runs the check in `dir` and asserts it fails with a line for which `wanted` holds.
fn assert_fails_with(case: &str, dir: &Path, what: &str, wanted: impl Fn(&str) -> bool) {
    let (status, text) = check(dir);
    assert_ne!(status, Some(0), "{case}: {text}");
    assert!(
        text.lines().any(wanted),
        "{case}: no line {what} in\n{text}"
    );
    let _ = fs::remove_dir_all(dir);
}

/// Appends `package` to a copy of the lock, runs the check and asserts it fails with `line`.
fn assert_fails_naming(case: &str, package: &str, line: &str) {
    let dir = copy(case, package);
    assert_fails_with(case, &dir, &format!("{line:?}"), |l| l == line);
}

/// The lock with the block of the package `name` passed through `edit`, which must change it.
fn lock_with_package_edited(name: &str, edit: impl Fn(&str) -> String) -> String {
    package_edited(&lock(), name, edit)
}

/// `lock` with the block of the package `name` passed through `edit`, which must change it.
fn package_edited(lock: &str, name: &str, edit: impl Fn(&str) -> String) -> String {
    let head = format!("[[package]]\nname = \"{name}\"\n");
    let start = lock
        .find(&head)
        .unwrap_or_else(|| panic!("no package {name}"));
    let end = lock[start + head.len()..]
        .find("\n[[package]]")
        .map_or(lock.len(), |i| start + head.len() + i + 1);
    let block = &lock[start..end];
    let edited = edit(block);
    assert_ne!(block, edited, "the edit of {name} changed nothing");
    format!("{}{}{}", &lock[..start], edited, &lock[end..])
}

/// The allow-list's line of ring.
fn listed_ring() -> String {
    allow_list()
        .lines()
        .find(|l| l.starts_with("ring "))
        .expect("ring on the allow-list")
        .to_string()
}

/// The lock as committed passes, and names the packages from outside this repository, each on
/// the allow-list. `task lock-check` runs the line the other cases run: the task is `silent`, so
/// `task --dry --verbose` prints its line, on standard error. `--color=false`, because task
/// colours its lines when `CI=true` (as the gate's runner form sets it), even into a pipe.
#[test]
fn the_unchanged_lock_passes() {
    let dry = Command::new("task")
        .args(["--color=false", "--dry", "--verbose", "lock-check"])
        .current_dir(repository())
        .output()
        .expect("run task");
    let stderr = String::from_utf8_lossy(&dry.stderr);
    let wanted = format!("task: [lock-check] {LINE}");
    assert!(
        dry.status.success() && stderr.lines().any(|l| l == wanted),
        "task --color=false --dry --verbose lock-check: no line {wanted:?} in\n{stderr}{}",
        String::from_utf8_lossy(&dry.stdout)
    );
    let lock = lock();
    let packages = lock.lines().filter(|l| *l == "[[package]]").count();
    let dir = copy("unchanged", "");
    let (status, text) = check(&dir);
    assert_eq!(status, Some(0), "{text}");
    let line = format!(
        "Cargo.lock: {packages} package(s), 25 from outside this repository, each on the allow-list"
    );
    assert!(
        text.lines().any(|l| l == line),
        "no line {line:?} in\n{text}"
    );
    let _ = fs::remove_dir_all(&dir);
}

/// A crate from crates.io that is on no list, as `cargo generate-lockfile` writes it.
#[test]
fn a_registry_package_fails_naming_it() {
    assert_fails_naming(
        "registry",
        concat!(
            "\n[[package]]\n",
            "name = \"serde\"\n",
            "version = \"1.0.0\"\n",
            "source = \"registry+https://github.com/rust-lang/crates.io-index\"\n",
            "checksum = \"0000000000000000000000000000000000000000000000000000000000000000\"\n",
        ),
        "Cargo.lock: serde 1.0.0 from registry+https://github.com/rust-lang/crates.io-index: not on the allow-list",
    );
}

/// A crate from a git repository is from outside this one too.
#[test]
fn a_git_package_fails_naming_it() {
    assert_fails_naming(
        "git",
        concat!(
            "\n[[package]]\n",
            "name = \"ess-conformance\"\n",
            "version = \"0.96.0\"\n",
            "source = \"git+https://github.com/beyond10x/ess?tag=0.96.0#9e91e8b5\"\n",
        ),
        "Cargo.lock: ess-conformance 0.96.0 from git+https://github.com/beyond10x/ess?tag=0.96.0#9e91e8b5: not on the allow-list",
    );
}

/// ring of another version than the listed one fails, naming it and the listed line.
#[test]
fn another_version_of_ring_fails_naming_both() {
    let lock = lock_with_package_edited("ring", |b| {
        b.replace("version = \"0.17.14\"", "version = \"0.17.13\"")
    });
    let dir = copy_with("ring-version", &lock, &allow_list());
    let start = format!("Cargo.lock: ring 0.17.13 from {CRATES_IO}: not on the allow-list");
    let listed = listed_ring();
    assert_fails_with(
        "ring-version",
        &dir,
        &format!("{start:?} naming {listed:?}"),
        |l| l.starts_with(&start) && l.contains(&listed),
    );
}

/// ring of the listed version with one hex digit of its checksum changed fails, naming it.
#[test]
fn a_changed_checksum_of_ring_fails_naming_it() {
    let listed = listed_ring();
    let checksum = listed
        .split(' ')
        .nth(3)
        .expect("checksum field")
        .to_string();
    let flipped = format!(
        "{}{}",
        if checksum.starts_with('0') { '1' } else { '0' },
        &checksum[1..]
    );
    let lock = lock_with_package_edited("ring", |b| b.replace(&checksum, &flipped));
    let dir = copy_with("ring-checksum", &lock, &allow_list());
    let start = format!("Cargo.lock: ring 0.17.14 from {CRATES_IO}: not on the allow-list");
    assert_fails_with(
        "ring-checksum",
        &dir,
        &format!("{start:?} naming {flipped} and {listed:?}"),
        |l| l.starts_with(&start) && l.contains(&flipped) && l.contains(&listed),
    );
}

/// ring of the listed version from a git repository fails, naming it.
#[test]
fn ring_from_git_fails_naming_it() {
    let git = "git+https://github.com/briansmith/ring?tag=0.17.14#a4689e6c";
    let lock = lock_with_package_edited("ring", |b| {
        b.lines()
            .filter(|l| !l.starts_with("checksum = "))
            .map(|l| {
                if l.starts_with("source = ") {
                    format!("source = \"{git}\"\n")
                } else {
                    format!("{l}\n")
                }
            })
            .collect()
    });
    let dir = copy_with("ring-git", &lock, &allow_list());
    let start = format!("Cargo.lock: ring 0.17.14 from {git}: not on the allow-list");
    assert_fails_with("ring-git", &dir, &format!("{start:?}"), |l| {
        l.starts_with(&start)
    });
}

/// A crate of this repository other than surf-tls that depends on ring fails, naming it: only
/// the line `ring surf-tls` of the owners file allows a crate to.
#[test]
fn another_crate_depending_on_ring_fails_naming_it() {
    let lock = lock_with_package_edited("surf-fetch", |b| {
        b.replace("dependencies = [\n", "dependencies = [\n \"ring\",\n")
    });
    let dir = copy_with("fetch-ring", &lock, &allow_list());
    let line = "Cargo.lock: surf-fetch depends on ring; lock-owners.txt does not allow it";
    assert_fails_with("fetch-ring", &dir, &format!("{line:?}"), |l| l == line);
}

/// A line of the allow-list that no package of the lock matches fails, naming the line.
#[test]
fn a_listed_package_the_lock_lacks_fails_naming_it() {
    let mut allow = allow_list();
    allow.push_str(&format!("zlib-rs 0.5.2 {CRATES_IO} {}\n", "ab".repeat(32)));
    let dir = copy_with("listed-missing", &lock(), &allow);
    let line = "taskfiles/lock-allow.txt: zlib-rs 0.5.2 is in no package of Cargo.lock";
    assert_fails_with("listed-missing", &dir, &format!("{line:?}"), |l| l == line);
}

/// A line in no form cargo writes fails, named with its line number, also where cargo would
/// read it.
#[test]
fn a_line_cargo_does_not_write_fails_naming_it() {
    let n = lock().lines().count() + 1;
    for (case, extra) in [
        ("foreign-metadata", "[metadata]\n"),
        ("foreign-patch", "[[patch.unused]]\n"),
        ("foreign-spaces", "name  = \"surf\"\n"),
    ] {
        let dir = copy(case, extra);
        let line = format!(
            "Cargo.lock:{n}: not in the form cargo writes: {}",
            extra.trim_end()
        );
        assert_fails_with(case, &dir, &format!("{line:?}"), |l| l == line);
    }
}

/// The lock with the `source` of the package `name` replaced by `source`.
fn lock_with_source(name: &str, source: &str) -> String {
    lock_with_package_edited(name, |b| {
        b.replace(
            &format!("source = \"{SWELL}\""),
            &format!("source = \"{source}\""),
        )
    })
}

/// The allow-list's line of `name`.
fn listed(name: &str) -> String {
    allow_list()
        .lines()
        .find(|l| l.starts_with(&format!("{name} ")))
        .unwrap_or_else(|| panic!("{name} on the allow-list"))
        .to_string()
}

/// Runs the check on `lock` and asserts it fails with the line of the allow-list for swell
/// 0.8.0 from `source`, naming the listed line.
fn assert_swell_from_fails(case: &str, lock: &str, source: &str) {
    let dir = copy_with(case, lock, &allow_list());
    let start = format!("Cargo.lock: swell 0.8.0 from {source}: not on the allow-list");
    let listed = listed("swell");
    assert_fails_with(case, &dir, &format!("{start:?} naming {listed:?}"), |l| {
        l.starts_with(&start) && l.contains(&listed)
    });
}

/// swell of the listed version and commit under another tag fails, naming it and the listed line.
#[test]
fn a_swell_package_of_another_tag_fails_naming_it() {
    let source = SWELL.replace("?tag=0.8.0#", "?tag=0.7.0#");
    assert_swell_from_fails("swell-tag", &lock_with_source("swell", &source), &source);
}

/// swell of the listed tag at another commit fails, naming it and the listed line.
#[test]
fn a_swell_package_at_another_commit_fails_naming_it() {
    let source = SWELL.replace("#0c18", "#1c18");
    assert_swell_from_fails("swell-commit", &lock_with_source("swell", &source), &source);
}

/// swell of the listed tag and commit from another address fails, naming it and the listed line.
#[test]
fn a_swell_package_from_another_address_fails_naming_it() {
    let source = SWELL.replace("github.com/beyond10x/swell", "github.com/beyond11x/swell");
    assert_swell_from_fails(
        "swell-address",
        &lock_with_source("swell", &source),
        &source,
    );
}

/// A swell line of the allow-list with a checksum, where the lock (a package from git) has
/// none, fails, naming the package with its checksum `none` and the listed line.
#[test]
fn a_listed_swell_line_with_a_checksum_fails_naming_it() {
    let listed = listed("swell");
    let with_checksum = format!(
        "{} {}",
        listed
            .strip_suffix(" none")
            .expect("swell listed with none"),
        "ab".repeat(32)
    );
    let allow = allow_list().replace(&listed, &with_checksum);
    let dir = copy_with("swell-checksum", &lock(), &allow);
    let start =
        format!("Cargo.lock: swell 0.8.0 from {SWELL}: not on the allow-list; its checksum none");
    assert_fails_with(
        "swell-checksum",
        &dir,
        &format!("{start:?} naming {with_checksum:?}"),
        |l| l.starts_with(&start) && l.contains(&with_checksum),
    );
}

/// A crate of this repository other than surf-script that depends on swell fails, naming it.
#[test]
fn another_crate_depending_on_swell_fails_naming_it() {
    let lock = lock_with_package_edited("surf-fetch", |b| {
        b.replace(" \"surf-url\",\n]", " \"surf-url\",\n \"swell\",\n]")
    });
    let dir = copy_with("fetch-swell", &lock, &allow_list());
    let line = "Cargo.lock: surf-fetch depends on swell; lock-owners.txt does not allow it";
    assert_fails_with("fetch-swell", &dir, &format!("{line:?}"), |l| l == line);
}

/// `surf` depending on `surf-script` fails, naming it: the owners file names no crate that may
/// (`docs/design/scripts.md`, Q14), so `surf` links no swell.
#[test]
fn surf_depending_on_surf_script_fails_naming_it() {
    let lock = lock_with_package_edited("surf", |b| {
        b.replace(
            "dependencies = [\n",
            "dependencies = [\n \"surf-script\",\n",
        )
    });
    let dir = copy_with("surf-script", &lock, &allow_list());
    let line = "Cargo.lock: surf depends on surf-script; lock-owners.txt does not allow it";
    assert_fails_with("surf-script", &dir, &format!("{line:?}"), |l| l == line);
}

/// `surf-script` depending on ring fails, naming it: its line of the owners file is for swell.
#[test]
fn surf_script_depending_on_ring_fails_naming_it() {
    let lock = lock_with_package_edited("surf-script", |b| {
        b.replace("dependencies = [\n", "dependencies = [\n \"ring\",\n")
    });
    let dir = copy_with("script-ring", &lock, &allow_list());
    let line = "Cargo.lock: surf-script depends on ring; lock-owners.txt does not allow it";
    assert_fails_with("script-ring", &dir, &format!("{line:?}"), |l| l == line);
}

/// A crate given the line `surf-script <crate>` in an owners file written for the test may
/// depend on `surf-script`, and the check passes.
#[test]
fn a_crate_with_a_line_of_the_owners_file_may_depend_on_surf_script() {
    let lock = lock_with_package_edited("surf-session", |b| {
        b.replace(
            " \"surf-answer\",\n",
            " \"surf-answer\",\n \"surf-script\",\n",
        )
    });
    let mut owners = owners();
    owners.push_str("surf-script surf-session\n");
    let dir = copy_files(
        "session-script",
        &Files {
            lock,
            owners,
            ..Files::committed()
        },
    );
    let (status, text) = check(&dir);
    assert_eq!(status, Some(0), "{text}");
    assert!(
        text.lines()
            .any(|l| l
                .ends_with(" package(s), 25 from outside this repository, each on the allow-list")),
        "{text}"
    );
    let _ = fs::remove_dir_all(&dir);
}

/// The root manifest's section `[profile.release.package.<name>]` of a package from git.
fn release_section(name: &str) -> String {
    format!("[profile.release.package.{name}]\n")
}

/// A swell package of the lock without `overflow-checks = true` in its section of the root
/// manifest's release profile fails, naming the package: cargo reads profiles from the root
/// manifest only (`docs/design/scripts.md`, Q17). Two forms: the section gone, and the setting
/// `false`.
#[test]
fn a_swell_package_without_overflow_checks_fails_naming_it() {
    let manifest = manifest();
    let section = format!("{}overflow-checks = true\n", release_section("swell-num"));
    assert!(manifest.contains(&section), "no {section:?} in Cargo.toml");
    for (case, edited) in [
        ("overflow-gone", manifest.replace(&section, "")),
        (
            "overflow-false",
            manifest.replace(
                &section,
                &format!("{}overflow-checks = false\n", release_section("swell-num")),
            ),
        ),
    ] {
        let dir = copy_files(
            case,
            &Files {
                manifest: edited,
                ..Files::committed()
            },
        );
        let line = format!(
            "Cargo.toml: swell-num 0.8.0 from {SWELL}: no overflow-checks = true under [profile.release.package.swell-num]"
        );
        assert_fails_with(case, &dir, &format!("{line:?}"), |l| l == line);
    }
}

/// A root manifest that holds the six sections and nothing else passes: the check reads the
/// sections, not the rest of the manifest.
#[test]
fn the_root_manifest_with_every_entry_passes() {
    let manifest: String = SWELL_PACKAGES
        .iter()
        .map(|name| format!("{}overflow-checks = true\n\n", release_section(name)))
        .collect();
    let dir = copy_files(
        "manifest-entries",
        &Files {
            manifest,
            ..Files::committed()
        },
    );
    let (status, text) = check(&dir);
    assert_eq!(status, Some(0), "{text}");
    let _ = fs::remove_dir_all(&dir);
}

/// A line of the owners file that is not two fields fails, naming it.
#[test]
fn an_owners_line_not_two_fields_fails_naming_it() {
    let mut owners = owners();
    owners.push_str("surf-script\n");
    let n = owners.lines().count();
    let dir = copy_files(
        "owners-form",
        &Files {
            owners,
            ..Files::committed()
        },
    );
    let line =
        format!("taskfiles/lock-owners.txt: line {n} is not \"<package> <crate>\": surf-script");
    assert_fails_with("owners-form", &dir, &format!("{line:?}"), |l| l == line);
}

/// No root lock at all proves nothing, so it fails too.
#[test]
fn a_missing_lock_fails() {
    let dir = copy("missing", "");
    fs::remove_file(dir.join("Cargo.lock")).expect("remove Cargo.lock");
    let (status, text) = check(&dir);
    assert_ne!(status, Some(0), "{text}");
    let _ = fs::remove_dir_all(&dir);
}

// Adversary cases: cargo reads Cargo.lock as TOML, and under `--locked` (the gate's build and
// test) it compares the lock it would write with the one on disk as parsed values, so a lock
// that differs from cargo's own layout but means the same is accepted and not rewritten. Each
// lock below was shown to pass `cargo tree --locked --offline` unchanged in a copy of the
// workspace whose manifest asked for the package. The check must read these as cargo does.

/// An unlisted registry crate whose `source` and `checksum` keys have no spaces around `=`.
#[test]
fn adversary_an_unlisted_package_with_keys_without_spaces_fails() {
    assert_fails_naming(
        "adversary-nospace",
        concat!(
            "\n[[package]]\n",
            "name = \"either\"\n",
            "version = \"1.19.0\"\n",
            "source=\"registry+https://github.com/rust-lang/crates.io-index\"\n",
            "checksum=\"0e9c71c2167ca323c882b99918929403426e2373ea17242ff5653e0d5e1058be\"\n",
        ),
        "Cargo.lock: either 1.19.0 from registry+https://github.com/rust-lang/crates.io-index: not on the allow-list",
    );
}

/// An unlisted registry crate after a `[[package]]` header with a comment.
#[test]
fn adversary_an_unlisted_package_under_a_commented_header_fails() {
    assert_fails_naming(
        "adversary-header-comment",
        concat!(
            "\n[[package]] # x\n",
            "name = \"either\"\n",
            "version = \"1.19.0\"\n",
            "source = \"registry+https://github.com/rust-lang/crates.io-index\"\n",
            "checksum = \"0e9c71c2167ca323c882b99918929403426e2373ea17242ff5653e0d5e1058be\"\n",
        ),
        "Cargo.lock: either 1.19.0 from registry+https://github.com/rust-lang/crates.io-index: not on the allow-list",
    );
}

/// surf-fetch with ring in a dependency array whose first entry stands on its opening line.
#[test]
fn adversary_another_crate_depending_on_ring_in_an_inline_array_fails() {
    let lock = lock_with_package_edited("surf-fetch", |b| {
        b.replace("dependencies = [\n", "dependencies = [\"ring\",\n")
    });
    let dir = copy_with("adversary-fetch-ring-inline", &lock, &allow_list());
    let line = "Cargo.lock: surf-fetch depends on ring; lock-owners.txt does not allow it";
    assert_fails_with(
        "adversary-fetch-ring-inline",
        &dir,
        &format!("{line:?}"),
        |l| l == line,
    );
}

/// surf-fetch with untrusted, a package of ring, as the last of its dependencies, where cargo's
/// sorting puts it. Green today; red when the check reads only the first dependency of a crate
/// (the case above with ring first stays green under that change).
#[test]
fn adversary_another_crate_depending_on_a_package_of_ring_last_fails() {
    let lock = lock_with_package_edited("surf-fetch", |b| {
        b.replace(" \"surf-url\",\n]", " \"surf-url\",\n \"untrusted\",\n]")
    });
    let dir = copy_with("adversary-fetch-untrusted-last", &lock, &allow_list());
    let line = "Cargo.lock: surf-fetch depends on untrusted; lock-owners.txt does not allow it";
    assert_fails_with(
        "adversary-fetch-untrusted-last",
        &dir,
        &format!("{line:?}"),
        |l| l == line,
    );
}

/// surf-fetch with ring as a TOML literal string, `'ring'`.
#[test]
fn adversary_another_crate_depending_on_ring_as_a_literal_string_fails() {
    let lock = lock_with_package_edited("surf-fetch", |b| {
        b.replace("dependencies = [\n", "dependencies = [\n 'ring',\n")
    });
    let dir = copy_with("adversary-fetch-ring-literal", &lock, &allow_list());
    let line = "Cargo.lock: surf-fetch depends on ring; lock-owners.txt does not allow it";
    assert_fails_with(
        "adversary-fetch-ring-literal",
        &dir,
        &format!("{line:?}"),
        |l| l == line,
    );
}

/// surf-fetch with ring spelled with a TOML escape, `"ring"`: cargo reads `ring`, accepts
/// the lock unchanged under `--locked` and builds surf-fetch with ring, while the check's reader
/// takes the escape for `u0069`. Only the form of an entry, which holds no backslash, stops it:
/// green today, red when that form admits a backslash.
#[test]
fn adversary_another_crate_depending_on_ring_spelled_with_an_escape_fails() {
    let entry = " \"r\\u0069ng\",";
    let lock = lock_with_package_edited("surf-fetch", |b| {
        b.replace(
            "dependencies = [\n",
            &format!("dependencies = [\n{entry}\n"),
        )
    });
    let n = lock
        .lines()
        .position(|l| l == entry)
        .expect("the planted entry")
        + 1;
    let dir = copy_with("adversary-fetch-ring-escape", &lock, &allow_list());
    let line = format!("Cargo.lock:{n}: not in the form cargo writes: {entry}");
    assert_fails_with(
        "adversary-fetch-ring-escape",
        &dir,
        &format!("{line:?}"),
        |l| l == line,
    );
}
