//! Review cases of `task lock-check` for `story:scripts-engine-dependency`: locks, lists and
//! manifests of the test's own, each in a directory of its own under `CARGO_TARGET_TMPDIR`, run
//! through the task's line as `lock_check` runs it. The first three cases show what the check
//! lets through; the cases marked "Held" record what it refused.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The source cargo writes for the six packages of swell at the tag the root manifest names.
const SWELL: &str =
    "git+https://github.com/beyond10x/swell?tag=0.10.0#b9d3d46fa7ac4abc982f2248e903e6301e277a9f";

/// The version of the six packages of swell at that tag.
const SWELL_VERSION: &str = "0.10.0";

/// The commit `SWELL` names.
fn swell_commit() -> &'static str {
    SWELL.rsplit_once('#').expect("a commit in SWELL").1
}

/// The listed version with its patch number raised by one: a version beside the listed one.
fn next_patch() -> String {
    let (head, patch) = SWELL_VERSION.rsplit_once('.').expect("a patch number");
    let patch: u32 = patch.parse().expect("a numeric patch number");
    format!("{head}.{}", patch + 1)
}

const SWELL_PACKAGES: [&str; 6] = [
    "swell",
    "swell-compiler",
    "swell-num",
    "swell-regexp",
    "swell-syntax",
    "swell-unicode",
];

const LINE: &str = "sh taskfiles/lock-check.sh Cargo.lock taskfiles/lock-allow.txt taskfiles/lock-owners.txt Cargo.toml";

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(path: &str) -> String {
    fs::read_to_string(repository().join(path)).unwrap_or_else(|e| panic!("read {path}: {e}"))
}

/// The files of one copy; `None` leaves the file out.
#[derive(Clone)]
struct Files {
    lock: Option<String>,
    allow: Option<String>,
    owners: Option<String>,
    manifest: Option<String>,
    cargo_config: Option<String>,
}

impl Files {
    fn committed() -> Self {
        Self {
            lock: Some(read("Cargo.lock")),
            allow: Some(read("taskfiles/lock-allow.txt")),
            owners: Some(read("taskfiles/lock-owners.txt")),
            manifest: Some(read("Cargo.toml")),
            cargo_config: Some(read(".cargo/config.toml")),
        }
    }

    fn lock(&self) -> &str {
        self.lock.as_deref().expect("a lock")
    }
}

fn write(dir: &Path, path: &str, text: &Option<String>) {
    if let Some(text) = text {
        let file = dir.join(path);
        fs::create_dir_all(file.parent().expect("a parent")).expect("create dir");
        fs::write(&file, text).unwrap_or_else(|e| panic!("write {path}: {e}"));
    }
}

/// Runs the task's line in a fresh copy named `name` holding `files`: exit status and output.
fn check(name: &str, files: &Files) -> (Option<i32>, String) {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("lock-check-adversary")
        .join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("taskfiles")).expect("create copy");
    fs::copy(
        repository().join("taskfiles/lock-check.sh"),
        dir.join("taskfiles/lock-check.sh"),
    )
    .expect("copy lock-check.sh");
    write(&dir, "Cargo.lock", &files.lock);
    write(&dir, "taskfiles/lock-allow.txt", &files.allow);
    write(&dir, "taskfiles/lock-owners.txt", &files.owners);
    write(&dir, "Cargo.toml", &files.manifest);
    write(&dir, ".cargo/config.toml", &files.cargo_config);
    let mut words = LINE.split(' ');
    let output = Command::new(words.next().expect("a program"))
        .args(words)
        .current_dir(&dir)
        .output()
        .expect("run sh");
    let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&output.stderr));
    let _ = fs::remove_dir_all(&dir);
    (output.status.code(), text)
}

/// Asserts the check fails on `files` with a line for which `wanted` holds.
fn assert_fails(name: &str, files: &Files, what: &str, wanted: impl Fn(&str) -> bool) {
    let (status, text) = check(name, files);
    assert_ne!(status, Some(0), "{name}: the check passed:\n{text}");
    assert!(
        text.lines().any(wanted),
        "{name}: no line {what} in\n{text}"
    );
}

/// `lock` with the block of the package `name` (the first of that name) passed through `edit`.
fn edit_package(lock: &str, name: &str, edit: impl Fn(&str) -> String) -> String {
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

/// The section of the root manifest that turns overflow checks on for `name` in release.
fn release_section(name: &str) -> String {
    format!("[profile.release.package.{name}]\noverflow-checks = true\n")
}

/// A path package (no `source` line) that bears the name of a listed package from outside, here
/// `swell` of the listed version with its patch number raised by one beside the git `swell` of
/// the listed version, used by `surf-script`: cargo writes it so for a dependency
/// `swell2 = { package = "swell", path = "<a directory outside the tree>" }`. The check reads a
/// package without `source` as a crate of this repository, and `surf-script` may depend on a
/// package named `swell`, so the copy passes.
#[test]
fn a_path_package_named_like_a_listed_one_fails() {
    let mut files = Files::committed();
    let beside = next_patch();
    let lock = edit_package(files.lock(), "surf-script", |b| {
        b.replace(
            " \"swell\",\n",
            &format!(" \"swell {SWELL_VERSION}\",\n \"swell {beside}\",\n"),
        )
    });
    let shadow = format!("[[package]]\nname = \"swell\"\nversion = \"{beside}\"\n\n");
    let at = lock
        .find("[[package]]\nname = \"swell-compiler\"\n")
        .expect("swell-compiler in the lock");
    files.lock = Some(format!("{}{shadow}{}", &lock[..at], &lock[at..]));
    let named = format!("swell {beside}");
    assert_fails("path-shadow", &files, &format!("naming {named}"), |l| {
        l.contains(&named)
    });
}

/// The repository's `.cargo/config.toml` turning overflow checks off for a swell package in the
/// release profile. cargo reads `[profile]` from config files too, and a config file's profile
/// overrides the manifest's (The Cargo Book, "Configuration", `[profile]`: "can be used to
/// globally change profile settings, and override settings specified in Cargo.toml"); the file
/// already holds `[profile.dev.package.ring]`. The check reads the root manifest only and
/// passes, so a release build compiles swell-num without overflow checks.
#[test]
fn a_cargo_config_turning_overflow_checks_off_fails() {
    let mut files = Files::committed();
    let mut config = files.cargo_config.take().expect("a config");
    config.push_str("\n[profile.release.package.swell-num]\noverflow-checks = false\n");
    files.cargo_config = Some(config);
    assert_fails("config-overflow-off", &files, "naming swell-num", |l| {
        l.contains("swell-num")
    });
}

/// The six sections removed from the root manifest and written inside a multi-line string of
/// `[workspace.metadata]`, which cargo does not read. TOML reads no table there, so the release
/// profile checks no overflow in swell; the check reads the lines as tables and passes.
#[test]
fn profile_sections_inside_a_string_fail() {
    let mut files = Files::committed();
    let mut manifest = files.manifest.take().expect("a manifest");
    for name in SWELL_PACKAGES {
        let section = release_section(name);
        assert!(manifest.contains(&section), "no {section:?} in Cargo.toml");
        manifest = manifest.replace(&section, "");
    }
    manifest.push_str("\n[workspace.metadata]\nnote = \"\"\"\n");
    for name in SWELL_PACKAGES {
        manifest.push_str(&release_section(name));
    }
    manifest.push_str("\"\"\"\n");
    files.manifest = Some(manifest);
    assert_fails(
        "overflow-in-string",
        &files,
        "naming swell-num and overflow-checks",
        |l| l.contains("swell-num") && l.contains("overflow-checks"),
    );
}

/// Held: the section of another profile, or a nested table of the release section, does not
/// stand for the release section of swell-num.
#[test]
fn a_section_of_another_profile_does_not_count() {
    for (i, other) in [
        "[profile.dev.package.swell-num]\noverflow-checks = true\n",
        "[profile.bench.package.swell-num]\noverflow-checks = true\n",
        "[profile.release.package.swell-num.build-override]\noverflow-checks = true\n",
        "[profile.release.package.\"*\"]\noverflow-checks = true\n",
    ]
    .iter()
    .enumerate()
    {
        let mut files = Files::committed();
        let manifest = files.manifest.take().expect("a manifest");
        files.manifest = Some(manifest.replace(&release_section("swell-num"), other));
        let line = format!(
            "Cargo.toml: swell-num {SWELL_VERSION} from {SWELL}: no overflow-checks = true under [profile.release.package.swell-num]"
        );
        assert_fails(&format!("other-profile-{i}"), &files, &line, |l| l == line);
    }
}

/// Held: swell of the listed version from every other spelling of a source fails, naming it.
#[test]
fn swell_from_any_other_source_fails_naming_it() {
    let commit = swell_commit();
    let sources = [
        format!("git+https://github.com/beyond10x/swell?branch=main#{commit}"),
        format!("git+https://github.com/beyond10x/swell?rev={commit}#{commit}"),
        format!("git+https://github.com/beyond10x/swell#{commit}"),
        format!("git+https://github.com/someone/swell?tag={SWELL_VERSION}#{commit}"),
        format!("git+https://github.com/Beyond10x/swell?tag={SWELL_VERSION}#{commit}"),
        format!("git+https://github.com/beyond10x/swell.git?tag={SWELL_VERSION}#{commit}"),
        format!("git+https://github.com/beyond10x/swell/?tag={SWELL_VERSION}#{commit}"),
        format!("git+http://github.com/beyond10x/swell?tag={SWELL_VERSION}#{commit}"),
        format!("git+ssh://git@github.com/beyond10x/swell?tag={SWELL_VERSION}#{commit}"),
        format!(
            "git+https://github.com/beyond10x/swell?tag={SWELL_VERSION}#{}",
            &commit[..8]
        ),
    ];
    for (i, source) in sources.iter().enumerate() {
        let mut files = Files::committed();
        files.lock = Some(edit_package(files.lock(), "swell", |b| {
            b.replace(SWELL, source)
        }));
        let start =
            format!("Cargo.lock: swell {SWELL_VERSION} from {source}: not on the allow-list");
        assert_fails(&format!("source-{i}"), &files, &start, |l| {
            l.starts_with(&start)
        });
    }
    // A registry package named swell, with a checksum.
    let mut files = Files::committed();
    files.lock = Some(edit_package(files.lock(), "swell", |b| {
        b.replace(
            &format!("source = \"{SWELL}\"\n"),
            &format!(
                "source = \"registry+https://github.com/rust-lang/crates.io-index\"\nchecksum = \"{}\"\n",
                "ab".repeat(32)
            ),
        )
    }));
    assert_fails(
        "source-registry",
        &files,
        "naming swell from the registry",
        |l| l.starts_with(&format!("Cargo.lock: swell {SWELL_VERSION} from registry+")),
    );
}

/// Held: the lock writes a renamed, target-specific, build, dev or workspace-inherited
/// dependency as one more entry of `dependencies`, also as `"<name> <version> (<source>)"`
/// when two packages share the name; a crate without a line of the owners file fails.
#[test]
fn a_dependency_entry_with_version_and_source_fails_naming_the_crate() {
    for (i, entry) in [
        &format!("swell {SWELL_VERSION}"),
        &format!("swell {SWELL_VERSION} ({SWELL})"),
        "swell-syntax",
    ]
    .iter()
    .enumerate()
    {
        let mut files = Files::committed();
        files.lock = Some(edit_package(files.lock(), "surf-fetch", |b| {
            b.replace(
                " \"surf-url\",\n]",
                &format!(" \"surf-url\",\n \"{entry}\",\n]"),
            )
        }));
        let name = entry.split(' ').next().expect("a name");
        let line =
            format!("Cargo.lock: surf-fetch depends on {name}; lock-owners.txt does not allow it");
        assert_fails(&format!("entry-{i}"), &files, &line, |l| l == line);
    }
}

/// Held: `[patch]` of swell to a path copy leaves swell without `source`; the six listed
/// lines then match nothing and the check fails naming them. An old `replace` line is not in
/// the form cargo writes.
#[test]
fn a_patch_or_replace_of_swell_fails() {
    let mut files = Files::committed();
    let mut lock = files.lock().to_string();
    for name in SWELL_PACKAGES {
        lock = edit_package(&lock, name, |b| {
            b.replace(&format!("source = \"{SWELL}\"\n"), "")
        });
    }
    files.lock = Some(lock);
    let line = format!(
        "taskfiles/lock-allow.txt: swell-num {SWELL_VERSION} is in no package of Cargo.lock"
    );
    assert_fails("patch-path", &files, &line, |l| l == line);
    let mut files = Files::committed();
    files.lock = Some(edit_package(files.lock(), "swell", |b| {
        b.replace(
            &format!("version = \"{SWELL_VERSION}\"\n"),
            &format!(
                "version = \"{SWELL_VERSION}\"\nreplace = \"swell {SWELL_VERSION} ({SWELL})\"\n"
            ),
        )
    }));
    assert_fails("replace", &files, "naming the replace line", |l| {
        l.contains("not in the form cargo writes: replace = ")
    });
}

/// Held: an empty or missing list, owners file or manifest fails; so does a line with a
/// trailing space or a carriage return, and a listed line written twice.
#[test]
fn empty_missing_or_malformed_lists_fail() {
    let committed = Files::committed();
    let allow = committed.allow.clone().expect("allow");
    let owners = committed.owners.clone().expect("owners");
    let mut cases: Vec<(&str, Files)> = Vec::new();
    let with = |f: &dyn Fn(&mut Files)| {
        let mut files = committed.clone();
        f(&mut files);
        files
    };
    cases.push(("allow-empty", with(&|f| f.allow = Some(String::new()))));
    cases.push(("owners-empty", with(&|f| f.owners = Some(String::new()))));
    cases.push(("allow-missing", with(&|f| f.allow = None)));
    cases.push(("owners-missing", with(&|f| f.owners = None)));
    cases.push(("manifest-missing", with(&|f| f.manifest = None)));
    cases.push((
        "manifest-empty",
        with(&|f| f.manifest = Some(String::new())),
    ));
    cases.push((
        "allow-trailing-space",
        with(&|f| f.allow = Some(allow.replace(" none\n", " none \n"))),
    ));
    cases.push((
        "owners-trailing-space",
        with(&|f| f.owners = Some(owners.replace("swell surf-script\n", "swell surf-script \n"))),
    ));
    cases.push((
        "allow-crlf",
        with(&|f| f.allow = Some(allow.replace('\n', "\r\n"))),
    ));
    cases.push((
        "owners-crlf",
        with(&|f| f.owners = Some(owners.replace('\n', "\r\n"))),
    ));
    let swell_line = allow
        .lines()
        .find(|l| l.starts_with("swell "))
        .expect("swell listed")
        .to_string();
    cases.push((
        "allow-duplicate",
        with(&|f| {
            f.allow = Some(allow.replace(&swell_line, &format!("{swell_line}\n{swell_line}")))
        }),
    ));
    for (name, files) in &cases {
        let (status, text) = check(name, files);
        assert_ne!(status, Some(0), "{name}: the check passed:\n{text}");
    }
}

/// Held: a seventh package from swell's address, listed, without its section of the release
/// profile fails naming it; with the section it passes.
#[test]
fn a_seventh_swell_package_needs_its_profile_section() {
    let mut files = Files::committed();
    let block = format!(
        "[[package]]\nname = \"swell-extra\"\nversion = \"{SWELL_VERSION}\"\nsource = \"{SWELL}\"\n\n"
    );
    let lock = files.lock().to_string();
    let at = lock
        .find("[[package]]\nname = \"swell-num\"\n")
        .expect("swell-num");
    let lock = format!("{}{block}{}", &lock[..at], &lock[at..]);
    let lock = edit_package(&lock, "swell", |b| {
        b.replace(
            " \"swell-compiler\",\n",
            " \"swell-compiler\",\n \"swell-extra\",\n",
        )
    });
    files.lock = Some(lock);
    let mut allow = files.allow.take().expect("allow");
    allow = allow.replace(
        &format!("swell-num {SWELL_VERSION} {SWELL} none\n"),
        &format!(
            "swell-extra {SWELL_VERSION} {SWELL} none\nswell-num {SWELL_VERSION} {SWELL} none\n"
        ),
    );
    files.allow = Some(allow);
    let line = format!(
        "Cargo.toml: swell-extra {SWELL_VERSION} from {SWELL}: no overflow-checks = true under [profile.release.package.swell-extra]"
    );
    assert_fails("seventh", &files, &line, |l| l == line);
    let mut manifest = files.manifest.take().expect("manifest");
    manifest.push_str(&format!("\n{}", release_section("swell-extra")));
    files.manifest = Some(manifest);
    let (status, text) = check("seventh-with-section", &files);
    assert_eq!(status, Some(0), "{text}");
}
