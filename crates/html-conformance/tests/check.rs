//! `--check --base <revision>`: the gate that holds the counts of the expected files
//! (`crates/html-conformance/expected/<group>.txt`) against the harness, the `html-inventory` and
//! `html-sections` blocks of the note and the same files at a base revision.
//!
//! Each case but the last runs the binary with `--root` over a small repository the test writes
//! under `CARGO_TARGET_TMPDIR`: a note with both blocks, a few data files and the three expected
//! files, committed with git so that a base revision exists. Over that data the empty parser
//! passes one case: the tokenizer test with an empty input. The expected values below are
//! written out here, never taken from the crate.

use std::path::{Path, PathBuf};
use std::process::Command;

use surf_html_conformance::inventory::{self, Group};
use surf_html_conformance::{paths, runner};

const TOKENIZER: &str = "crates/html-conformance/expected/tokenizer.txt";
const TREE: &str = "crates/html-conformance/expected/tree-construction.txt";
const ENCODING: &str = "crates/html-conformance/expected/encoding.txt";
const NOTE: &str = "docs/design/html-parser.md";

const TEST1: &str = "html5lib-tests/tokenizer/test1.test";
const XML: &str = "html5lib-tests/tokenizer/xmlViolation.test";
const TREE1: &str = "wpt/html/syntax/parsing/resources/tests1.dat";
const TREE2: &str = "wpt/html/syntax/parsing/resources/tests2.dat";
const ENC1: &str = "html5lib-tests/encoding/tests1.dat";

/// One test that passes with the empty parser (no input, no token) and one that does not. The
/// failing one expects `b` for the input `a`: it fails for any tokenizer, not only an empty one.
const TEST1_DATA: &str = r#"{"tests": [
{"description": "passes", "input": "", "output": []},
{"description": "fails", "input": "a", "output": [["Character", "b"]]}
]}
"#;

/// Only the test that passes.
const TEST1_PASSING: &str = r#"{"tests": [
{"description": "passes", "input": "", "output": []}
]}
"#;

const XML_DATA: &str = r#"{"xmlViolationTests": [
{"description": "not run", "input": "a", "output": [["Character", "a"]]}
]}
"#;

/// The expected tree holds a `q` where the input has a `p`, so the case fails for any tree
/// builder, not only an empty one.
const TREE_DATA: &str =
    "#data\n<p>\n#errors\n#document\n| <html>\n|   <head>\n|   <body>\n|     <q>\n";

/// The expected label is no encoding name, so the case fails for any sniffer, not only an empty
/// one.
const ENC_DATA: &str = "#data\n<meta charset=\"utf-8\">\n#encoding\nx-no-such-encoding\n";

const INVENTORY: &str = "\
source html5lib-tests https://example.invalid/html5lib-tests 0
source wpt https://example.invalid/wpt 0
file tokenizer html5lib-tests/tokenizer/test1.test 2 markup=2
file tokenizer html5lib-tests/tokenizer/xmlViolation.test 1 not-run:xml-infoset=1
file tree-construction wpt/html/syntax/parsing/resources/tests1.dat 1 mixed=1
file tree-construction wpt/html/syntax/parsing/resources/tests2.dat 1 mixed=1
file encoding html5lib-tests/encoding/tests1.dat 1 encoding=1
tag markup 2
tag mixed 2
tag encoding 1
tag not-run 1
total 6
";

const INVENTORY_PASSING: &str = "\
source html5lib-tests https://example.invalid/html5lib-tests 0
source wpt https://example.invalid/wpt 0
file tokenizer html5lib-tests/tokenizer/test1.test 1 markup=1
file tokenizer html5lib-tests/tokenizer/xmlViolation.test 1 not-run:xml-infoset=1
file tree-construction wpt/html/syntax/parsing/resources/tests1.dat 1 mixed=1
file tree-construction wpt/html/syntax/parsing/resources/tests2.dat 1 mixed=1
file encoding html5lib-tests/encoding/tests1.dat 1 encoding=1
tag markup 1
tag mixed 2
tag encoding 1
tag not-run 1
total 5
";

const SECTIONS: &str = "\
section tokenization https://example.invalid/#tokenization
wpt tokenization html/syntax/parsing/zero.html script
data tokenization html5lib-tests/tokenizer/test1.test
section coercing https://example.invalid/#coercing
open coercing not-taken No infoset.
data coercing html5lib-tests/tokenizer/xmlViolation.test
section tree-construction https://example.invalid/#tree-construction
open tree-construction script-processing Scripts.
wpt tree-construction html/syntax/parsing/html5lib_write.html script
data tree-construction wpt/html/syntax/parsing/resources/tests1.dat
data tree-construction wpt/html/syntax/parsing/resources/tests2.dat
section the-input-byte-stream https://example.invalid/#the-input-byte-stream
wpt the-input-byte-stream encoding/ script
data the-input-byte-stream html5lib-tests/encoding/tests1.dat
section the-end https://example.invalid/#the-end
open the-end scripts-and-events Events.
wpt the-end html/syntax/parsing/the-end.html script
";

const TOKENIZER_EXPECTED: &str = "\
file html5lib-tests/tokenizer/test1.test 1/2/2
file html5lib-tests/tokenizer/xmlViolation.test 0/0/1 not-run:xml-infoset=1
tag markup 1/2/2
tag not-run 0/0/1
section tokenization incomplete
section coercing incomplete
";

const TREE_EXPECTED: &str = "\
file wpt/html/syntax/parsing/resources/tests1.dat 0/1/1
file wpt/html/syntax/parsing/resources/tests2.dat 0/1/1
tag mixed 0/2/2
section tree-construction incomplete
";

const ENCODING_EXPECTED: &str = "\
file html5lib-tests/encoding/tests1.dat 0/1/1
tag encoding 0/1/1
section the-input-byte-stream incomplete
";

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    /// The fixture repository with its data, note and expected files, not committed yet.
    fn new(name: &str) -> Fixture {
        let root = Path::new(env!("CARGO_TARGET_TMPDIR"))
            .join("check")
            .join(name);
        if root.exists() {
            std::fs::remove_dir_all(&root).expect("remove the old fixture");
        }
        std::fs::create_dir_all(&root).expect("create the fixture");
        let fixture = Fixture { root };
        fixture.data(TEST1, TEST1_DATA);
        fixture.data(XML, XML_DATA);
        fixture.data(TREE1, TREE_DATA);
        fixture.data(TREE2, TREE_DATA);
        fixture.data(ENC1, ENC_DATA);
        fixture.note(INVENTORY, SECTIONS);
        fixture.write(TOKENIZER, TOKENIZER_EXPECTED);
        fixture.write(TREE, TREE_EXPECTED);
        fixture.write(ENCODING, ENCODING_EXPECTED);
        fixture.git(&["init", "--quiet", "--initial-branch", "main"]);
        fixture
    }

    fn write(&self, path: &str, content: &str) {
        let path = self.root.join(path);
        std::fs::create_dir_all(path.parent().expect("a parent")).expect("create a directory");
        std::fs::write(&path, content).expect("write a fixture file");
    }

    fn read(&self, path: &str) -> String {
        std::fs::read_to_string(self.root.join(path)).expect("read a fixture file")
    }

    /// Replaces `from`, which must occur, with `to` in the file `path`.
    fn edit(&self, path: &str, from: &str, to: &str) {
        let text = self.read(path);
        assert!(text.contains(from), "{path} holds {from:?}");
        self.write(path, &text.replacen(from, to, 1));
    }

    fn data(&self, path: &str, content: &str) {
        self.write(&format!("crates/html-conformance/data/{path}"), content);
    }

    fn note(&self, inventory: &str, sections: &str) {
        self.write(
            NOTE,
            &format!(
                "# A note\n\n```html-sections\n{sections}```\n\n```html-inventory\n{inventory}```\n"
            ),
        );
    }

    fn git(&self, args: &[&str]) -> String {
        let output = Command::new("git")
            .arg("-C")
            .arg(&self.root)
            .args([
                "-c",
                "user.name=fixture",
                "-c",
                "user.email=fixture@example.invalid",
                "-c",
                "commit.gpgsign=false",
                "-c",
                "core.hooksPath=/dev/null",
            ])
            .args(args)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .output()
            .expect("run git");
        assert!(
            output.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    }

    /// Commits everything and answers the commit's name.
    fn commit(&self) -> String {
        self.git(&["add", "--all"]);
        self.git(&["commit", "--quiet", "--allow-empty", "--message", "fixture"]);
        self.git(&["rev-parse", "HEAD"])
    }

    /// Runs `--check --base <base>` over the fixture: the exit status and what it printed.
    fn check(&self, base: &str) -> (Option<i32>, String) {
        let output = Command::new(env!("CARGO_BIN_EXE_surf-html-conformance"))
            .arg("--root")
            .arg(&self.root)
            .args(["--check", "--base", base])
            .output()
            .expect("run the harness");
        let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
        text.push_str(&String::from_utf8_lossy(&output.stderr));
        (output.status.code(), text)
    }
}

/// Asserts the check failed with exit status 1 and printed a line that holds every one of
/// `parts`.
fn assert_fails_with(result: &(Option<i32>, String), parts: &[&str]) {
    let (status, text) = result;
    assert_eq!(*status, Some(1), "exit status 1 expected; output:\n{text}");
    assert!(
        text.lines()
            .any(|line| parts.iter().all(|part| line.contains(part))),
        "no line holds all of {parts:?}; output:\n{text}"
    );
}

fn assert_passes(result: &(Option<i32>, String)) {
    let (status, text) = result;
    assert_eq!(*status, Some(0), "exit status 0 expected; output:\n{text}");
}

#[test]
fn a_pass_count_below_the_expected_count_fails() {
    let fixture = Fixture::new("pass-below-expected");
    let base = fixture.commit();
    assert_passes(&fixture.check(&base));
    fixture.edit(
        TOKENIZER,
        &format!("{TEST1} 1/2/2"),
        &format!("{TEST1} 2/2/2"),
    );
    assert_fails_with(
        &fixture.check(&base),
        &[
            TOKENIZER,
            &format!("file {TEST1}"),
            "passed 1",
            "expected 2",
        ],
    );
    // A run count raised above what runs: the case of xmlViolation.test is not run.
    fixture.write(TOKENIZER, TOKENIZER_EXPECTED);
    fixture.edit(TOKENIZER, &format!("{XML} 0/0/1"), &format!("{XML} 0/1/1"));
    assert_fails_with(
        &fixture.check(&base),
        &[TOKENIZER, &format!("file {XML}"), "run 0", "expected 1"],
    );
}

#[test]
fn a_tag_pass_count_below_the_expected_count_fails() {
    let fixture = Fixture::new("tag-pass-below-expected");
    let base = fixture.commit();
    fixture.edit(TOKENIZER, "tag markup 1/2/2", "tag markup 2/2/2");
    assert_fails_with(
        &fixture.check(&base),
        &[TOKENIZER, "tag markup", "passed 1", "expected 2"],
    );
}

#[test]
fn a_case_count_that_differs_from_the_inventory_fails() {
    let fixture = Fixture::new("case-count-differs");
    let base = fixture.commit();
    fixture.data(
        TEST1,
        &TEST1_DATA.replace(
            "\n]}",
            ",\n{\"description\": \"added\", \"input\": \"b\", \"output\": [[\"Character\", \"b\"]]}\n]}",
        ),
    );
    let result = fixture.check(&base);
    assert_fails_with(
        &result,
        &[NOTE, &format!("file {TEST1}"), "3 cases", "inventory 2"],
    );
    assert_fails_with(&result, &[NOTE, "tag markup", "3 cases", "inventory 2"]);
}

#[test]
fn an_expected_count_below_its_value_at_the_base_fails() {
    let fixture = Fixture::new("expected-below-base");
    let base = fixture.commit();
    fixture.edit(
        TOKENIZER,
        &format!("{TEST1} 1/2/2"),
        &format!("{TEST1} 0/2/2"),
    );
    fixture.edit(TOKENIZER, "tag markup 1/2/2", "tag markup 0/2/2");
    let result = fixture.check(&base);
    let was = format!("at {base} 1:");
    assert_fails_with(
        &result,
        &[
            TOKENIZER,
            &format!("file {TEST1}"),
            "expected passed 0",
            &was,
        ],
    );
    assert_fails_with(
        &result,
        &[TOKENIZER, "tag markup", "expected passed 0", &was],
    );
}

#[test]
fn a_base_git_cannot_resolve_fails() {
    let fixture = Fixture::new("base-unresolved");
    fixture.commit();
    assert_passes(&fixture.check("HEAD"));
    assert_fails_with(&fixture.check("no-such-revision"), &["no-such-revision"]);
}

#[test]
fn a_section_map_that_differs_from_the_note_fails() {
    let fixture = Fixture::new("section-map-differs");
    let base = fixture.commit();
    fixture.edit(TOKENIZER, "section coercing incomplete\n", "");
    assert_fails_with(&fixture.check(&base), &[TOKENIZER, "coercing"]);
    fixture.write(TOKENIZER, TOKENIZER_EXPECTED);
    fixture.edit(
        TOKENIZER,
        "section tokenization incomplete\nsection coercing incomplete\n",
        "section coercing incomplete\nsection tokenization incomplete\n",
    );
    assert_fails_with(
        &fixture.check(&base),
        &[TOKENIZER, "coercing", "tokenization"],
    );
}

#[test]
fn a_data_file_in_no_section_fails() {
    let fixture = Fixture::new("data-file-in-no-section");
    let base = fixture.commit();
    fixture.note(
        INVENTORY,
        &SECTIONS.replace(&format!("data tree-construction {TREE2}\n"), ""),
    );
    assert_fails_with(&fixture.check(&base), &[TREE2, "in no section"]);
}

#[test]
fn a_case_not_run_in_no_section_fails() {
    let fixture = Fixture::new("not-run-in-no-section");
    let base = fixture.commit();
    fixture.note(
        INVENTORY,
        &SECTIONS.replace(&format!("data coercing {XML}\n"), ""),
    );
    fixture.edit(TOKENIZER, "section coercing incomplete\n", "");
    assert_fails_with(&fixture.check(&base), &[XML, "not-run", "in no section"]);
}

#[test]
fn a_section_marked_complete_with_a_failing_case_fails() {
    let fixture = Fixture::new("complete-with-failing-case");
    let base = fixture.commit();
    fixture.note(
        INVENTORY,
        &SECTIONS.replace(
            "wpt tokenization html/syntax/parsing/zero.html script\n",
            "",
        ),
    );
    fixture.edit(
        TOKENIZER,
        "section tokenization incomplete",
        "section tokenization complete",
    );
    assert_fails_with(
        &fixture.check(&base),
        &["tokenization", "complete", "1 failing"],
    );
}

#[test]
fn a_section_marked_complete_with_a_case_not_run_fails() {
    let fixture = Fixture::new("complete-with-case-not-run");
    let base = fixture.commit();
    fixture.note(
        INVENTORY,
        &SECTIONS.replace("open coercing not-taken No infoset.\n", ""),
    );
    fixture.edit(
        TOKENIZER,
        "section coercing incomplete",
        "section coercing complete",
    );
    assert_fails_with(
        &fixture.check(&base),
        &["coercing", "complete", "1 not run"],
    );
}

/// The fixture with `test1.test` holding only the passing case, so that the section
/// `tokenization` has every case passing; its `wpt` line replaced by `lines`.
fn all_passing(name: &str, lines: &str) -> (Fixture, String) {
    let fixture = Fixture::new(name);
    fixture.data(TEST1, TEST1_PASSING);
    fixture.note(
        INVENTORY_PASSING,
        &SECTIONS.replace(
            "wpt tokenization html/syntax/parsing/zero.html script\n",
            lines,
        ),
    );
    fixture.edit(
        TOKENIZER,
        &format!("{TEST1} 1/2/2"),
        &format!("{TEST1} 1/1/1"),
    );
    fixture.edit(TOKENIZER, "tag markup 1/2/2", "tag markup 1/1/1");
    let base = fixture.commit();
    assert_passes(&fixture.check(&base));
    fixture.edit(
        TOKENIZER,
        "section tokenization incomplete",
        "section tokenization complete",
    );
    (fixture, base)
}

#[test]
fn a_section_marked_complete_with_an_open_requirement_fails() {
    let (fixture, base) = all_passing(
        "complete-with-open-requirement",
        "open tokenization left-out A requirement left out.\n",
    );
    assert_fails_with(
        &fixture.check(&base),
        &["tokenization", "complete", "open", "left-out"],
    );
    let (fixture, base) = all_passing("complete-with-nothing-open", "");
    let result = fixture.check(&base);
    assert_passes(&result);
    assert!(
        result
            .1
            .lines()
            .any(|l| l == "section tokenization complete 1/1"),
        "{}",
        result.1
    );
}

#[test]
fn a_section_marked_complete_with_a_web_platform_test_not_run_fails() {
    let (fixture, base) = all_passing(
        "complete-with-wpt-not-run",
        "wpt tokenization html/syntax/parsing/zero.html script\n",
    );
    assert_fails_with(
        &fixture.check(&base),
        &["tokenization", "complete", "html/syntax/parsing/zero.html"],
    );
}

#[test]
fn a_count_above_the_expected_count_passes() {
    let fixture = Fixture::new("count-above-expected");
    fixture.edit(
        TOKENIZER,
        &format!("{TEST1} 1/2/2"),
        &format!("{TEST1} 0/2/2"),
    );
    fixture.edit(TOKENIZER, "tag markup 1/2/2", "tag markup 0/2/2");
    let base = fixture.commit();
    let result = fixture.check(&base);
    assert_passes(&result);
    let raised: Vec<&str> = result
        .1
        .lines()
        .filter(|l| l.contains("can be raised"))
        .collect();
    assert!(
        raised
            .iter()
            .any(|l| l.contains(TOKENIZER) && l.contains(&format!("file {TEST1}"))),
        "{}",
        result.1
    );
    assert!(
        raised
            .iter()
            .any(|l| l.contains(TOKENIZER) && l.contains("tag markup")),
        "{}",
        result.1
    );
    assert!(
        result
            .1
            .lines()
            .any(|l| l == "html-conformance: 6 cases, 5 run, 1 passed"),
        "{}",
        result.1
    );
}

#[test]
fn a_section_without_a_data_file_is_printed_incomplete_with_its_reasons() {
    let fixture = Fixture::new("section-without-data");
    std::fs::remove_dir_all(fixture.root.join("crates/html-conformance/expected"))
        .expect("remove the expected files");
    let base = fixture.commit();
    fixture.write(TOKENIZER, TOKENIZER_EXPECTED);
    fixture.write(TREE, TREE_EXPECTED);
    fixture.write(ENCODING, ENCODING_EXPECTED);
    let result = fixture.check(&base);
    assert_passes(&result);
    let printed: Vec<&str> = result
        .1
        .lines()
        .filter(|l| l.starts_with("section ") || l.starts_with("tag ") || l.starts_with("html-"))
        .collect();
    assert_eq!(
        printed,
        [
            "section tokenization incomplete 1/2 failing:1 wpt:1",
            "section coercing incomplete 0/1 not-run:1 open:not-taken",
            "section tree-construction incomplete 0/2 failing:2 open:script-processing wpt:2",
            "section the-input-byte-stream incomplete 0/1 failing:1 wpt:1",
            "section the-end incomplete 0/0 open:scripts-and-events wpt:1",
            "tag markup 1/2/2",
            "tag mixed 0/2/2",
            "tag encoding 0/1/1",
            "tag not-run 0/0/1",
            "html-conformance: 6 cases, 5 run, 1 passed",
        ],
        "{}",
        result.1
    );
}

#[test]
fn a_section_without_a_data_file_and_without_open_or_wpt_lines_fails() {
    let fixture = Fixture::new("section-without-anything");
    let base = fixture.commit();
    fixture.note(
        INVENTORY,
        &format!("{SECTIONS}section speculative https://example.invalid/#speculative\n"),
    );
    assert_fails_with(&fixture.check(&base), &["speculative", "no data"]);
}

#[test]
fn a_write_wrapper_line_counts_one_test_per_data_file_of_its_section() {
    let fixture = Fixture::new("write-wrapper");
    let base = fixture.commit();
    let wrapper = "wpt tree-construction html/syntax/parsing/html5lib_write.html script\n";
    fixture.note(
        INVENTORY,
        &SECTIONS.replace(
            wrapper,
            &format!(
                "{wrapper}wpt tree-construction html/syntax/parsing/html5lib_write_single.html \
                 script\nwpt tree-construction html/syntax/parsing/template/ script\n"
            ),
        ),
    );
    let result = fixture.check(&base);
    assert_passes(&result);
    assert!(
        result.1.lines().any(|l| l
            == "section tree-construction incomplete 0/2 failing:2 open:script-processing wpt:5"),
        "two wrappers over two data files and one directory are 5 tests; output:\n{}",
        result.1
    );
}

#[test]
fn the_tag_counts_of_the_groups_sum_to_the_tag_lines() {
    let inventory = inventory::load(&paths::note()).unwrap_or_else(|e| panic!("{e}"));
    let report = runner::run_all(&paths::data(), &inventory).unwrap_or_else(|e| panic!("{e}"));
    let not_run = |group: Group| {
        report
            .groups
            .iter()
            .find(|(g, _)| *g == group)
            .and_then(|(_, tags)| tags.iter().find(|(name, _)| name == "not-run"))
            .map(|(_, count)| count.exist)
    };
    assert_eq!(not_run(Group::Tokenizer), Some(4));
    assert_eq!(not_run(Group::TreeConstruction), Some(6));
    assert_eq!(not_run(Group::Encoding), Some(2));
    assert_eq!(report.groups.len(), 3, "three groups");
    for (name, count) in &report.tags {
        let mut sum = runner::Count::default();
        for (_, tags) in &report.groups {
            for (n, c) in tags {
                if n == name {
                    sum.exist += c.exist;
                    sum.run += c.run;
                    sum.pass += c.pass;
                }
            }
        }
        assert_eq!(sum, *count, "tag {name}: the groups sum to the tag line");
    }
    for (group, tags) in &report.groups {
        for (name, count) in tags {
            assert!(
                count.exist > 0,
                "{group:?}: tag {name} with no case is listed"
            );
        }
    }
    assert_eq!(report.tags.len(), 11);
}
