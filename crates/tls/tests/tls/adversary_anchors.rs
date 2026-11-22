//! Adversary cases of `story:tls-anchors`: what the unit's own tests of `surf_tls::anchors` and
//! `taskfiles/tls-anchors.sh` do not say. The script runs with `sh` on local files under
//! `CARGO_TARGET_TMPDIR`; no case reaches the network.

#![forbid(unsafe_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use surf_tls::anchors::{AnchorError, Anchors};
use surf_tls::crypto::{self, HashAlgorithm};

const SOURCE: &str = include_str!("../../src/anchors/SOURCE");
const BUNDLE: &[u8] = include_bytes!("../../src/anchors/cacert.pem");

fn test_certs(path: &str) -> Vec<u8> {
    let file = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../tls-test-certs/certs")
        .join(path);
    fs::read(&file).unwrap_or_else(|e| panic!("{}: {e}", file.display()))
}

fn field<'a>(source: &'a str, key: &str) -> &'a str {
    let prefix = format!("{key}: ");
    let found: Vec<&str> = source
        .lines()
        .filter_map(|l| l.strip_prefix(prefix.as_str()))
        .collect();
    assert_eq!(found.len(), 1, "one line `{key}:`");
    found[0]
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn refused(text: &[u8]) -> AnchorError {
    let mut anchors = Anchors::bundled();
    let before = anchors.as_slice().to_vec();
    let error = anchors.with_extra_pem(text).expect_err("refused");
    assert_eq!(anchors.as_slice(), before.as_slice(), "nothing added");
    error
}

// ---------------------------------------------------------------------------------------------
// The bundle against SOURCE.

/// `SOURCE`'s `date:` is the date of the header line of the list, not only a date: the unit's
/// `the_bundled_date_is_the_date_of_the_header_line` compares `bundled_date()` with `SOURCE`
/// itself, so `date: 2026-11-04` in `SOURCE` passed every test.
#[test]
fn adv_the_bundled_date_is_computed_from_the_header_line_of_the_bundle() {
    let text = std::str::from_utf8(BUNDLE).expect("UTF-8");
    let headers: Vec<&str> = text
        .lines()
        .filter_map(|l| l.strip_prefix("## Certificate data from Mozilla as of: "))
        .collect();
    assert_eq!(headers.len(), 1, "one header line");
    let f: Vec<&str> = headers[0].split_whitespace().collect();
    assert_eq!(f.len(), 6, "{:?}", headers[0]);
    let months = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let month = months.iter().position(|m| *m == f[1]).expect("a month") + 1;
    let day: u32 = f[2].parse().expect("a day");
    let date = format!("{}-{month:02}-{day:02}", f[4]);
    assert_eq!(Anchors::bundled_date(), date);
    assert_eq!(field(SOURCE, "date"), date);
}

// ---------------------------------------------------------------------------------------------
// The extra file: hostile bytes the unit's cases do not hold.

/// A BEGIN line of another label with spaces and a tab after its dashes is still refused by its
/// label, before its content is decoded (the PEM reader strips them too).
#[test]
fn adv_a_private_key_whose_begin_line_ends_in_blanks_is_refused_by_its_label() {
    let label = ["PRIVATE", "KEY"].join(" ");
    let text = format!("-----BEGIN {label}----- \t \nZm9v\n-----END {label}-----\n");
    assert_eq!(
        refused(text.as_bytes()),
        AnchorError::NotACertificate { index: 0, label }
    );
}

/// A comment line that starts with `-----BEGIN` but not `-----BEGIN ` is text outside the
/// blocks, as the PEM reader reads it.
#[test]
fn adv_a_comment_line_starting_with_dashes_begin_is_text() {
    let mut text = b"-----BEGINNING of the roots of the lab\n".to_vec();
    text.extend_from_slice(&test_certs("ok-ed25519/root.pem"));
    let mut anchors = Anchors::bundled();
    assert_eq!(anchors.with_extra_pem(&text), Ok(1));
}

/// A BEGIN line that is the last line of the file, with no line feed after it, is a block.
#[test]
fn adv_a_begin_line_without_a_line_feed_at_the_end_is_a_block() {
    assert_eq!(
        refused(b"notes\n-----BEGIN X509 CRL-----"),
        AnchorError::NotACertificate {
            index: 0,
            label: "X509 CRL".to_string()
        }
    );
}

// ---------------------------------------------------------------------------------------------
// taskfiles/tls-anchors.sh.

fn script() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../taskfiles/tls-anchors.sh")
}

fn dir_with_bundle(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("adv-anchors-{name}"));
    if dir.exists() {
        let _ = Command::new("chmod")
            .arg("-R")
            .arg("u+w")
            .arg(&dir)
            .status();
    }
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create");
    fs::write(dir.join("cacert.pem"), BUNDLE).expect("write");
    fs::write(dir.join("SOURCE"), SOURCE).expect("write");
    dir
}

fn sh(args: &[&Path], env: &[(&str, &Path)], date: Option<&str>) -> Output {
    let mut command = Command::new("sh");
    command.arg(script()).args(args);
    for var in ["FILE", "SHA256", "DATE"] {
        command.env_remove(var);
    }
    for (k, v) in env {
        command.env(k, v);
    }
    if let Some(d) = date {
        command.env("DATE", d);
    }
    command.output().expect("run sh")
}

fn text(out: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

fn sum_line(data: &[u8]) -> String {
    format!(
        "{}  cacert.pem\n",
        hex(crypto::hash(HashAlgorithm::Sha256, data).as_ref())
    )
}

const HEADER: &str = "## Certificate data from Mozilla as of: Mon Feb  2 10:00:00 2026 GMT\n";

/// Runs `refresh` on `list` with its matching `.sha256` (or `sums`), returns the output and
/// whether the committed pair is still in place.
fn refresh(name: &str, list: &[u8], sums: Option<&str>) -> (Output, bool, PathBuf) {
    let dir = dir_with_bundle(name);
    let input = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("adv-anchors-{name}-in"));
    let _ = fs::remove_dir_all(&input);
    fs::create_dir_all(&input).expect("create");
    let file = input.join("new.pem");
    let sum = input.join("new.sha256");
    fs::write(&file, list).expect("write");
    let line = sums.map_or_else(|| sum_line(list), str::to_string);
    fs::write(&sum, line).expect("write");
    // The lists here have no titles, so each removes the 118 of the bundle: the bound of the
    // fix round (ANCHORS_REMOVED_MAX) is named, so that a refusal comes from the form checked.
    let out = sh(
        &[Path::new("refresh"), &dir],
        &[
            ("FILE", &file),
            ("SHA256", &sum),
            ("ANCHORS_REMOVED_MAX", Path::new("1000")),
        ],
        None,
    );
    let unchanged = fs::read(dir.join("cacert.pem")).expect("read") == BUNDLE
        && fs::read_to_string(dir.join("SOURCE")).expect("read") == SOURCE;
    (out, unchanged, dir)
}

/// A list whose sha256 matches but whose last block is cut in its middle: the script counts its
/// BEGIN lines and writes it, and the crate reads the whole written list as no anchor at all
/// (`read_list` takes `pem::parse(..).unwrap_or_default()`). The refresh is the one place that
/// writes the trust store of the next release; a list the crate's reader cannot read is refused
/// there, not left to a test run after it.
#[test]
fn adv_refresh_refuses_a_list_cut_in_the_middle_of_a_block() {
    let mut list = HEADER.as_bytes().to_vec();
    list.extend_from_slice(&test_certs("ok-rsa2048/root.pem"));
    let ed = test_certs("ok-ed25519/root.pem");
    list.extend_from_slice(&ed[..300]);
    let (out, unchanged, dir) = refresh("cut-block", &list, None);
    let written = fs::read(dir.join("cacert.pem")).expect("read");
    assert!(
        out.status.code() == Some(1) && unchanged,
        "exit {:?}, committed pair unchanged {unchanged}, anchors of the written list {}: {}",
        out.status.code(),
        Anchors::from_list(&written).count(),
        text(&out).trim_end()
    );
}

/// The script's head: it refuses a list without exactly one header line `... as of: <weekday>
/// <month> <day> <hh:mm:ss> <year> GMT`. It checks month, day, year and GMT and takes any
/// weekday and time, and writes the header into SOURCE with `echo`: under dash (the `sh` of
/// the forge and of Debian, which `task anchors-refresh` runs) `echo` reads `\n` and `\c`, so a
/// weekday `Mon\nsha256:\c` writes a line `sha256:date: ...` and SOURCE loses its `date:`.
#[test]
fn adv_refresh_refuses_a_header_whose_weekday_or_time_is_not_in_curls_form() {
    let root = test_certs("ok-rsa2048/root.pem");
    for (case, header) in [
        (
            "a weekday Xyz and a time 99:99:99",
            "## Certificate data from Mozilla as of: Xyz Feb  2 99:99:99 2026 GMT\n",
        ),
        (
            "a weekday holding echo escapes",
            "## Certificate data from Mozilla as of: Mon\\nsha256:\\c Feb  2 10:00:00 2026 GMT\n",
        ),
    ] {
        let mut list = header.as_bytes().to_vec();
        list.extend_from_slice(&root);
        let (out, unchanged, _) = refresh("header-form", &list, None);
        assert!(
            out.status.code() == Some(1) && unchanged,
            "{case}: exit {:?}, committed pair unchanged {unchanged}: {}",
            out.status.code(),
            text(&out).trim_end()
        );
    }
}

/// The script's head: "A refusal is one line on standard error". A directory it cannot write
/// is refused with cp's message and its own: two lines.
#[test]
fn adv_refresh_into_a_read_only_directory_is_one_line() {
    let mut list = HEADER.as_bytes().to_vec();
    list.extend_from_slice(&test_certs("ok-rsa2048/root.pem"));
    let dir = dir_with_bundle("read-only");
    let input = Path::new(env!("CARGO_TARGET_TMPDIR")).join("adv-anchors-read-only-in");
    let _ = fs::remove_dir_all(&input);
    fs::create_dir_all(&input).expect("create");
    let file = input.join("new.pem");
    let sum = input.join("new.sha256");
    fs::write(&file, &list).expect("write");
    fs::write(&sum, sum_line(&list)).expect("write");
    let status = Command::new("chmod")
        .arg("a-w")
        .arg(&dir)
        .status()
        .expect("chmod");
    assert!(status.success());
    let out = sh(
        &[Path::new("refresh"), &dir],
        &[("FILE", &file), ("SHA256", &sum)],
        None,
    );
    let _ = Command::new("chmod").arg("u+w").arg(&dir).status();
    assert_eq!(out.status.code(), Some(1), "{}", text(&out));
    assert_eq!(fs::read(dir.join("cacert.pem")).expect("read"), BUNDLE);
    assert_eq!(
        text(&out).lines().count(),
        1,
        "one line: {}",
        text(&out).trim_end()
    );
}

/// Forms of the `.sha256` file and of the list that the unit's cases do not hold: a CRLF
/// `.sha256` line is taken (its name is cut at the two spaces); a sum with one space, a CRLF
/// list (its header ends in `GMT\r`) and a header of seven fields are refused.
#[test]
fn adv_refresh_takes_and_refuses_these_forms() {
    let mut list = HEADER.as_bytes().to_vec();
    list.extend_from_slice(&test_certs("ok-rsa2048/root.pem"));
    let digest = hex(crypto::hash(HashAlgorithm::Sha256, &list).as_ref());
    let (out, unchanged, _) = refresh(
        "crlf-sum",
        &list,
        Some(&format!("{digest}  cacert.pem\r\n")),
    );
    assert!(out.status.success() && !unchanged, "{}", text(&out));
    let (out, unchanged, _) = refresh("no-final-lf", &list, Some(&format!("{digest}  cacert.pem")));
    assert!(out.status.success() && !unchanged, "{}", text(&out));
    let (out, unchanged, _) = refresh("one-space", &list, Some(&format!("{digest} cacert.pem\n")));
    assert!(out.status.code() == Some(1) && unchanged, "{}", text(&out));
    let crlf: Vec<u8> = String::from_utf8(list.clone())
        .expect("ASCII")
        .replace('\n', "\r\n")
        .into_bytes();
    let (out, unchanged, _) = refresh("crlf-list", &crlf, None);
    assert!(out.status.code() == Some(1) && unchanged, "{}", text(&out));
    let mut seven =
        b"## Certificate data from Mozilla as of: Mon Feb  2 10:00:00 2026 GMT x\n".to_vec();
    seven.extend_from_slice(&test_certs("ok-rsa2048/root.pem"));
    let (out, unchanged, _) = refresh("seven-fields", &seven, None);
    assert!(out.status.code() == Some(1) && unchanged, "{}", text(&out));
    let mut utc =
        b"## Certificate data from Mozilla as of: Mon Feb  2 10:00:00 2026 UTC\n".to_vec();
    utc.extend_from_slice(&test_certs("ok-rsa2048/root.pem"));
    let (out, unchanged, _) = refresh("utc", &utc, None);
    assert!(out.status.code() == Some(1) && unchanged, "{}", text(&out));
}

/// `age` refusals the unit's cases do not hold, each exit 1 and one line.
#[test]
fn adv_age_refuses_these_with_one_line() {
    let base = Path::new(env!("CARGO_TARGET_TMPDIR")).join("adv-anchors-age");
    let _ = fs::remove_dir_all(&base);
    let source = |name: &str, body: String| -> PathBuf {
        let dir = base.join(name);
        fs::create_dir_all(&dir).expect("create");
        fs::write(dir.join("SOURCE"), body).expect("write");
        dir
    };
    let good = source("good", SOURCE.to_string());
    let two_dates = source("two-dates", format!("{SOURCE}date: 2020-01-01\n"));
    let bad_count = source("bad-count", SOURCE.replace("count: ", "count: x"));
    let age = Path::new("age");
    for (case, dir, date) in [
        ("a month 00", &good, "2027-00-10"),
        ("a day 00", &good, "2027-01-00"),
        ("a year 0999", &good, "0999-01-01"),
        ("two date lines", &two_dates, "2027-01-01"),
        ("a count not a number", &bad_count, "2027-01-01"),
    ] {
        let out = sh(&[age, dir.as_path()], &[], Some(date));
        let t = text(&out);
        assert_eq!(out.status.code(), Some(1), "{case}: {t}");
        assert_eq!(t.lines().count(), 1, "{case}: {t}");
        assert!(!t.contains("days old"), "{case}: {t}");
    }
}
