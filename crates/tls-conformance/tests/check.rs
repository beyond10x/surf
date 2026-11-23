//! The check of `surf-tls-conformance` (`story:tls-x509-path`, acceptance 6, 7 and 9): counts
//! held to `expected/limbo.txt` and to it at the base, the kinds of `limbo-kinds.txt`, the time
//! of the cases. Every test here builds its report from a few made-up cases or a case of
//! `crates/tls-test-certs`; none reads the limbo data, whose run is the task's.

#![forbid(unsafe_code)]

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use surf_html_conformance::json;
use surf_tls_conformance::check::{self, ExpectedFile, Outcome, Report};
use surf_tls_conformance::runner::{self, CaseResult, Expected, Outcome as Ended};
use surf_tls_conformance::{LIMBO_TIME, group_of, paths, section_of};

fn root() -> PathBuf {
    paths::root()
}

fn case(id: &str, expected: Expected, outcome: &str) -> CaseResult {
    CaseResult {
        id: id.into(),
        expected,
        outcome: Ended::Ran(outcome.into()),
        detail: String::new(),
    }
}

fn kinds(text: &str) -> Vec<check::KindLine> {
    check::parse_kinds(text).expect("kinds")
}

/// Two groups: `a` with a SUCCESS case that passes and a FAILURE case of the kind its line
/// names, `b` with one FAILURE case refused with another kind.
fn sample() -> Report {
    let results = [
        case("a::one", Expected::Success, "ok"),
        case(
            "a::two",
            Expected::Failure,
            "CertificateNotTrusted NameConstraint",
        ),
        case("b::three", Expected::Failure, "CertificateExpired"),
    ];
    check::report(
        &results,
        &kinds("a::* CertificateNotTrusted NameConstraint\nb::three CertificateNameMismatch\n"),
    )
}

fn expected(text: &str) -> ExpectedFile {
    check::parse_expected(text).expect("an expected file")
}

const SAMPLE_EXPECTED: &str = "x509-limbo: 3 cases, 3 run, 2 passed\n\
                               group a 2/2/2\n\
                               group b 0/1/1\n\
                               open b::three ends Expired, the file wants NameMismatch\n";

fn compare(report: &Report, file: &str, base: Option<&str>) -> Outcome {
    let then = base.map(expected);
    check::compare(
        report,
        &expected(file),
        then.as_ref().map(|t| ("base", t)),
        &BTreeSet::from(["CertificateNotTrusted NoPathToAnchor".to_string()]),
    )
}

#[test]
fn the_sample_holds_its_expected_file() {
    let report = sample();
    assert_eq!(
        report.count_lines(),
        [
            "x509-limbo: 3 cases, 3 run, 2 passed",
            "group a 2/2/2",
            "group b 0/1/1"
        ]
    );
    assert_eq!(
        report.lines,
        ["wrong-kind b::three CertificateExpired expected CertificateNameMismatch ()"]
    );
    let out = compare(&report, SAMPLE_EXPECTED, Some(SAMPLE_EXPECTED));
    assert_eq!(out.failures, Vec::<String>::new());
    assert_eq!(out.raise, Vec::<String>::new());
}

#[test]
fn a_pass_count_below_the_expected_one_fails() {
    let file = SAMPLE_EXPECTED.replace("3 run, 2 passed", "3 run, 3 passed");
    let out = compare(&sample(), &file, None);
    assert_eq!(
        out.failures,
        [format!(
            "{}: x509-limbo: 2 passed, below the expected 3",
            paths::EXPECTED
        )]
    );
    let file = SAMPLE_EXPECTED.replace("group a 2/2/2", "group a 2/2/3");
    let out = compare(&sample(), &file, None);
    assert_eq!(
        out.failures,
        [format!(
            "{}: group a: 2 cases, the file says 3",
            paths::EXPECTED
        )]
    );
}

#[test]
fn an_expected_count_below_its_value_at_the_base_fails() {
    let base = SAMPLE_EXPECTED.replace("group b 0/1/1", "group b 1/1/1");
    let out = compare(&sample(), SAMPLE_EXPECTED, Some(&base));
    assert_eq!(
        out.failures,
        [format!(
            "{}: group b: 0 passed, below its 1 at base",
            paths::EXPECTED
        )]
    );
}

#[test]
fn a_base_git_cannot_resolve_fails() {
    for base in ["no-such-revision-of-surf", "", "--all"] {
        let error = check::resolve(&root(), base).expect_err(base);
        assert!(
            error.starts_with(&format!("--base {base}: git cannot resolve the revision")),
            "{error}"
        );
    }
    assert!(check::resolve(&root(), "HEAD").is_ok());
    assert_eq!(
        check::file_at(&root(), "HEAD", "no/such/file.txt"),
        Ok(None)
    );
    assert!(
        check::file_at(&root(), "HEAD", "Cargo.toml")
            .expect("git show")
            .is_some_and(|text| text.contains("[workspace]"))
    );
}

#[test]
fn a_count_above_the_expected_one_passes_and_prints_a_raise_line() {
    let file = SAMPLE_EXPECTED.replace("3 run, 2 passed", "3 run, 1 passed");
    let out = compare(&sample(), &file, None);
    assert!(out.failures.is_empty(), "{:?}", out.failures);
    assert_eq!(
        out.raise,
        [format!(
            "{}: x509-limbo: 2 passed, the file says 1",
            paths::EXPECTED
        )]
    );
    assert!(out.printed().contains(&format!("raise: {}", out.raise[0])));
}

#[test]
fn a_failure_case_matched_by_no_kinds_line_fails_naming_it() {
    let results = [case("c::lost", Expected::Failure, "CertificateExpired")];
    let report = check::report(&results, &kinds("a::* CertificateExpired\n"));
    assert_eq!(report.unmatched, [("c::lost".to_string(), Vec::new())]);
    let file = "x509-limbo: 1 cases, 1 run, 0 passed\ngroup c 0/1/1\nopen c::lost no kind\n";
    let out = compare(&report, file, None);
    assert_eq!(
        out.failures,
        [
            format!(
                "c::lost: a FAILURE case that no line of {} matches",
                paths::KINDS
            ),
            format!("{}: a::* matches no FAILURE case that runs", paths::KINDS),
        ]
    );
    // Two lines that match one case fail as well, naming both.
    let report = check::report(
        &results,
        &kinds("c::* CertificateExpired\nc::lost CertificateExpired\n"),
    );
    assert_eq!(
        report.unmatched,
        [(
            "c::lost".to_string(),
            vec!["c::*".to_string(), "c::lost".to_string()]
        )]
    );
}

#[test]
fn a_case_that_does_not_pass_needs_an_open_line_and_a_passing_one_loses_it() {
    let file = SAMPLE_EXPECTED.replace(
        "open b::three ends Expired, the file wants NameMismatch\n",
        "",
    );
    let out = compare(&sample(), &file, None);
    assert_eq!(
        out.failures,
        [format!(
            "b::three does not pass (CertificateExpired, expected CertificateNameMismatch) and has \
             no open line in {}",
            paths::EXPECTED
        )]
    );
    let file = format!("{SAMPLE_EXPECTED}open a::one was refused once\nopen z::gone gone\n");
    let out = compare(&sample(), &file, None);
    assert_eq!(
        out.raise,
        [format!(
            "{}: a::one passes; its open line can go",
            paths::EXPECTED
        )]
    );
    assert_eq!(
        out.failures,
        [format!(
            "{}: open z::gone: no case of that id runs",
            paths::EXPECTED
        )]
    );
}

#[test]
fn a_failure_case_ends_with_one_of_the_kinds_of_its_line() {
    let lines = kinds("n::* CertificateNotTrusted NameConstraint | CertificateNameMismatch\n");
    let results = [
        case(
            "n::1",
            Expected::Failure,
            "CertificateNotTrusted NameConstraint",
        ),
        case("n::2", Expected::Failure, "CertificateNameMismatch"),
        case("n::3", Expected::Failure, "CertificateExpired"),
        case("n::4", Expected::Failure, "ok"),
    ];
    let report = check::report(&results, &lines);
    assert_eq!(report.total.pass, 2);
    assert_eq!(
        report.passing,
        BTreeSet::from(["n::1".to_string(), "n::2".to_string()])
    );
    assert_eq!(
        report.lines,
        [
            "wrong-kind n::3 CertificateExpired expected CertificateNotTrusted NameConstraint | \
             CertificateNameMismatch ()",
            "accepted n::4 expected CertificateNotTrusted NameConstraint | CertificateNameMismatch",
        ]
    );
}

#[test]
fn an_accepted_case_of_a_kind_index_names_fails() {
    let results = [
        case("x::checked", Expected::Failure, "ok"),
        case("y::unchecked", Expected::Failure, "ok"),
    ];
    let report = check::report(
        &results,
        &kinds(
            "x::checked CertificateNotTrusted NoPathToAnchor\n\
             y::unchecked CertificateNotTrusted SomethingSurfDoesNotCheck\n",
        ),
    );
    let file = "x509-limbo: 2 cases, 2 run, 0 passed\ngroup x 0/1/1\ngroup y 0/1/1\n\
                open x::checked accepted\nopen y::unchecked accepted\n";
    let out = compare(&report, file, None);
    assert_eq!(
        out.failures,
        [format!(
            "x::checked: accepted, and its kind CertificateNotTrusted NoPathToAnchor is one {} \
             names",
            paths::INDEX
        )]
    );
    assert!(check::unchecked(
        "CertificateNotTrusted SomethingSurfDoesNotCheck"
    ));
    assert!(!check::unchecked("CertificateNotTrusted NoPathToAnchor"));
    assert!(!check::unchecked("CertificateExpired"));
}

#[test]
fn a_section_marked_complete_must_be() {
    let results = [
        case("rfc5280::validity::a", Expected::Success, "ok"),
        case(
            "rfc5280::validity::b",
            Expected::Success,
            "CertificateExpired",
        ),
    ];
    let report = check::report(&results, &[]);
    assert_eq!(report.sections, [("4.1.2.5".to_string(), report.total)]);
    let file = "x509-limbo: 2 cases, 2 run, 1 passed\ngroup rfc5280 1/2/2\n\
                section 4.1.2.5 1/2/2 complete\nopen rfc5280::validity::b refused\n";
    let out = compare(&report, file, None);
    assert_eq!(
        out.failures,
        [format!(
            "{}: section 4.1.2.5 is marked complete and 1 of its 2 cases pass",
            paths::EXPECTED
        )]
    );
}

#[test]
fn kinds_lines_and_patterns_read_as_written() {
    for bad in [
        "a::b",
        "a::b CertificateNotTrusted",
        "a::b CertificateExpired extra",
        "a::b Unknown",
        "a::b CertificateExpired | CertificateExpired",
        " CertificateExpired",
    ] {
        assert!(check::parse_kinds(bad).is_err(), "{bad}");
    }
    assert!(check::glob("a::*", "a::b"));
    assert!(check::glob("*::expired*", "rfc5280::expired-leaf"));
    assert!(check::glob("a*b*c", "abc"));
    assert!(!check::glob("a*b*c", "acb"));
    assert!(!check::glob("ab*ba", "aba"));
    assert!(!check::glob("a::b", "a::bc"));
    assert_eq!(group_of("rfc5280::nc::x"), "rfc5280");
    assert_eq!(section_of("rfc5280::nc::x"), Some("4.2.1.10"));
    assert_eq!(section_of("online::x"), None);
}

#[test]
fn the_committed_files_read() {
    let read = |p: &str| fs::read_to_string(root().join(p)).expect(p);
    let lines = check::parse_kinds(&read(paths::KINDS)).expect("limbo-kinds.txt reads");
    let index = check::index_outcomes(&read(paths::INDEX));
    for (pattern, outcomes) in &lines {
        for outcome in outcomes {
            assert!(
                check::unchecked(outcome) || index.contains(outcome),
                "{pattern}: {outcome} is neither a check of item 7 nor marked unchecked"
            );
        }
    }
    let file = check::parse_expected(&read(paths::EXPECTED)).expect("limbo.txt reads");
    assert!(file.total.exist > 5000);
}

/// Acceptance 9: the time of a case without one is the commit date `SOURCE` records, and the
/// runner reads no clock.
#[test]
fn limbo_time() {
    let source = fs::read_to_string(root().join(paths::DATA).join("SOURCE")).expect("SOURCE");
    let date = source
        .lines()
        .find_map(|l| l.strip_prefix("commit-date: "))
        .expect("a commit-date line");
    assert_eq!(LIMBO_TIME, date);
    assert!(runner::parse_time(LIMBO_TIME).is_some());
    let src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = 0;
    for entry in fs::read_dir(&src).expect("src") {
        let path = entry.expect("an entry").path();
        let text = fs::read_to_string(&path).expect("a source file");
        assert!(!text.contains("SystemTime"), "{}", path.display());
        files += 1;
    }
    assert!(files >= 3);
}

#[test]
fn times_of_cases_read_in_the_forms_limbo_writes() {
    let t = |s: &str| runner::parse_time(s).map(|t| t.unix());
    assert_eq!(t("1970-01-01T00:00:00Z"), Some(0));
    assert_eq!(t("1970-01-01T00:00:01+00:00"), Some(1));
    assert_eq!(t("1970-01-01T00:00:01.999+00:00"), Some(1));
    for bad in [
        "1970-01-01T00:00:00+01:00",
        "1970-01-01 00:00:00Z",
        "1970-01-01T00:00:00",
        "1970-01-01T00:00:00.Z",
        "1970-13-01T00:00:00Z",
    ] {
        assert_eq!(t(bad), None, "{bad}");
    }
}

fn pem(case: &str, file: &str) -> String {
    let path = root()
        .join("crates/tls-test-certs/certs")
        .join(case)
        .join(file);
    fs::read_to_string(&path).expect("a PEM file")
}

fn json_string(text: &str) -> String {
    let mut out = String::from("\"");
    for c in text.chars() {
        match c {
            '\n' => out.push_str("\\n"),
            '"' => out.push_str("\\\""),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// A case in limbo's form made of `ok-rsa2048` of `crates/tls-test-certs` (root, then the
/// chain the server sends, leaf first), with `extra` members.
fn made_case(extra: &str) -> json::Value {
    let chain = pem("ok-rsa2048", "chain.pem");
    let split = chain.find("-----END CERTIFICATE-----").expect("a leaf") + 26;
    let (leaf, rest) = chain.split_at(split);
    let text = format!(
        "{{\"id\":\"made::ok\",\"expected_result\":\"SUCCESS\",\"trusted_certs\":[{}],\
         \"untrusted_intermediates\":[{}],\"peer_certificate\":{},\
         \"validation_time\":\"2026-11-22T00:00:00+00:00\",\
         \"expected_peer_name\":{{\"kind\":\"DNS\",\"value\":\"localhost\"}}{extra}}}",
        json_string(&pem("ok-rsa2048", "root.pem")),
        json_string(rest.trim_start()),
        json_string(leaf),
    );
    json::parse(&text).expect("JSON")
}

#[test]
fn a_case_runs_through_the_path_verifier_or_says_why_not() {
    let ran = runner::run_case(&made_case(
        ",\"validation_kind\":\"SERVER\",\"features\":[]",
    ))
    .expect("a case");
    assert_eq!(ran.outcome, Ended::Ran("ok".into()), "{}", ran.detail);
    for (extra, reason) in [
        (",\"validation_kind\":\"CLIENT\",\"features\":[]", "client"),
        (
            ",\"validation_kind\":\"SERVER\",\"features\":[\"has-crl\"]",
            "crl",
        ),
        (
            ",\"validation_kind\":\"SERVER\",\"features\":[\"pedantic-rfc5280\"]",
            "pedantic",
        ),
        (
            ",\"validation_kind\":\"SERVER\",\"features\":[\"has-policy-constraints\"]",
            "policy",
        ),
    ] {
        let result = runner::run_case(&made_case(extra)).expect("a case");
        assert_eq!(result.outcome, Ended::NotRun(reason), "{extra}");
    }
    // A max_chain_depth below the bound of item 9 is the bound of the case.
    let cut = runner::run_case(&made_case(
        ",\"validation_kind\":\"SERVER\",\"features\":[],\"max_chain_depth\":0",
    ))
    .expect("a case");
    assert_eq!(
        cut.outcome,
        Ended::Ran("CertificateNotTrusted PathTooLong".into())
    );
}

#[test]
fn the_data_files_are_where_the_runner_reads_them() {
    let data: &Path = &root().join(paths::DATA);
    for file in runner::FILES {
        assert!(data.join(file).is_file(), "{file}");
    }
}
