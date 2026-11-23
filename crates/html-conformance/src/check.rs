//! `--check --base <revision>`: holds the counts of the expected files so that they can only
//! rise, and refuses a `complete` mark the rule of `AGENTS.md` ("Goals") does not allow.
//!
//! The expected files are `crates/html-conformance/expected/<group>.txt`, one per group of the
//! inventory (`tokenizer`, `tree-construction`, `encoding`), plain text, one record per line,
//! fields separated by one space:
//!
//! - `file <path> <pass>/<run>/<exist>`, followed by ` not-run:<reason>=<n>` for each reason
//!   cases of that file are not run: the line the harness prints for each data file of the
//!   group, in the order of the inventory;
//! - `tag <name> <pass>/<run>/<exist>`: one per tag that has a case in the group, counted over
//!   the group's cases, in the order of the inventory's `tag` lines;
//! - `section <name> complete` or `section <name> incomplete`: one per section of the note's
//!   `html-sections` block whose `data` files are in the group, in the note's order.
//!
//! The check fails, naming the file, tag or section and both numbers, when the harness's case
//! count per file or per tag differs from the `html-inventory` block; when a run or pass count is
//! below the expected one; when an expected count is below its value in the same file at the
//! base revision (the three files read with one `git cat-file --batch`, an object that is no
//! blob with `git show`; a file that does not exist there is not compared, a revision git cannot
//! resolve fails); when the sections of an expected file differ from the
//! note's, a data file or a case tagged `not-run` belongs to no section, or a section with no
//! `data` line has neither an `open` nor a `wpt` line; and when a section marked `complete` has
//! a case that does not pass or does not run, an `open` line or a `wpt` line. A count above the
//! expected one passes and is printed as one that the expected file can raise.
//!
//! It prints one line per section of the note, in the note's order: `section <name> complete
//! <n>/<n>`, or `section <name> incomplete <pass>/<exist> <reason>`, the reason being the fields
//! `failing:<n>` (cases that run and do not pass), `not-run:<n>`, `open:<id>,...` and
//! `wpt:<n>` (web-platform-tests tests that do not run) that apply, or `marked-incomplete` when
//! none does; then the harness's `tag` lines and its total line; then a `raise:` line per count
//! above the expected one and a `fail:` line per failure. A `wpt` line for
//! `html5lib_write.html` or `html5lib_write_single.html` counts one test per `data` file of its
//! section (its variants `?file=<f>`); a `wpt` path ending in `/` counts as one.

use std::fmt::Write as _;
use std::io::Write as _;
use std::path::Path;
use std::process::{Command, Stdio};

use crate::inventory::{self, Group, Inventory};
use crate::runner::{self, Count, Report};

/// The note, relative to the repository root.
pub const NOTE: &str = "docs/design/html-parser.md";
/// The data directory, relative to the repository root.
pub const DATA: &str = "crates/html-conformance/data";

/// The expected file of `group`, relative to the repository root.
pub fn expected_path(group: Group) -> String {
    format!("crates/html-conformance/expected/{}.txt", group.name())
}

/// A section of the `html-sections` block.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Section {
    /// The standard's anchor.
    pub name: String,
    /// The `<id>` of each `open` line.
    pub open: Vec<String>,
    /// The `<path>` of each `wpt` line.
    pub wpt: Vec<String>,
    /// The `<path>` of each `data` line.
    pub data: Vec<String>,
}

impl Section {
    /// The web-platform-tests tests of the section that do not run: one per `wpt` line, but a
    /// `wpt` line of a `.dat` wrapper counts one per `data` file of the section.
    pub fn wpt_count(&self) -> usize {
        self.wpt
            .iter()
            .map(|path| {
                let name = path.rsplit('/').next().unwrap_or(path);
                if matches!(name, "html5lib_write.html" | "html5lib_write_single.html") {
                    self.data.len()
                } else {
                    1
                }
            })
            .sum()
    }
}

/// Reads the lines of an `html-sections` block: `section <name> <address>`, `open <name> <id>
/// <text>`, `wpt <name> <path> <needs>`, `data <name> <path>`.
pub fn parse_sections(lines: &[&str]) -> Result<Vec<Section>, inventory::Error> {
    let mut sections: Vec<Section> = Vec::new();
    for &line in lines {
        let fields: Vec<&str> = line.split(' ').collect();
        let err = |what: &str| inventory::Error(format!("{what}: `{line}`"));
        let (kind, name) = match fields.as_slice() {
            [kind, name, ..] => (*kind, *name),
            _ => return Err(err("not a record")),
        };
        if kind == "section" {
            if fields.len() != 3 {
                return Err(err("a section line is not `section <name> <address>`"));
            }
            if sections.iter().any(|s| s.name == name) {
                return Err(err("a section declared twice"));
            }
            sections.push(Section {
                name: name.to_string(),
                open: Vec::new(),
                wpt: Vec::new(),
                data: Vec::new(),
            });
            continue;
        }
        let section = sections
            .iter_mut()
            .find(|s| s.name == name)
            .ok_or_else(|| err("a section no `section` line declares"))?;
        match (kind, &fields[2..]) {
            ("open", [id, _, ..])
                if !id.is_empty()
                    && id
                        .bytes()
                        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-') =>
            {
                section.open.push(id.to_string());
            }
            ("wpt", [path, _needs]) => section.wpt.push(path.to_string()),
            ("data", [path]) => section.data.push(path.to_string()),
            _ => return Err(err("not a record")),
        }
    }
    Ok(sections)
}

/// A section's mark in an expected file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mark {
    /// `complete`.
    Complete,
    /// `incomplete`.
    Incomplete,
}

/// One expected file.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Expected {
    /// The `file` lines: path, counts and the `not-run:<reason>=<n>` fields as written.
    pub files: Vec<(String, Count, Vec<String>)>,
    /// The `tag` lines: name and counts.
    pub tags: Vec<(String, Count)>,
    /// The `section` lines: name and mark.
    pub sections: Vec<(String, Mark)>,
}

fn number(field: &str) -> Option<usize> {
    if field.is_empty() || !field.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    field.parse().ok()
}

fn fraction(field: &str) -> Option<Count> {
    let mut parts = field.split('/');
    let pass = number(parts.next()?)?;
    let run = number(parts.next()?)?;
    let exist = number(parts.next()?)?;
    if parts.next().is_some() || pass > run || run > exist {
        return None;
    }
    Some(Count { exist, run, pass })
}

/// Reads an expected file.
pub fn parse_expected(text: &str) -> Result<Expected, String> {
    let mut expected = Expected::default();
    for (index, line) in text.lines().enumerate() {
        let fields: Vec<&str> = line.split(' ').collect();
        let err = || format!("line {}: not a record: `{line}`", index + 1);
        match fields.as_slice() {
            ["file", path, count, not_run @ ..] => {
                let count = fraction(count).ok_or_else(err)?;
                for field in not_run {
                    let ok = field
                        .strip_prefix("not-run:")
                        .and_then(|rest| rest.split_once('='))
                        .is_some_and(|(reason, n)| !reason.is_empty() && number(n).is_some());
                    if !ok {
                        return Err(err());
                    }
                }
                expected.files.push((
                    path.to_string(),
                    count,
                    not_run.iter().map(|f| f.to_string()).collect(),
                ));
            }
            ["tag", name, count] => {
                expected
                    .tags
                    .push((name.to_string(), fraction(count).ok_or_else(err)?));
            }
            ["section", name, "complete"] => {
                expected.sections.push((name.to_string(), Mark::Complete))
            }
            ["section", name, "incomplete"] => {
                expected.sections.push((name.to_string(), Mark::Incomplete))
            }
            _ => return Err(err()),
        }
    }
    Ok(expected)
}

/// What a check found.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Outcome {
    /// The `section` lines, the `tag` lines and the total line.
    pub lines: Vec<String>,
    /// Counts above the expected ones, each a sentence without the `raise: ` prefix.
    pub raise: Vec<String>,
    /// The failures, each a sentence without the `fail: ` prefix.
    pub failures: Vec<String>,
}

impl Outcome {
    /// Everything to print, in order: the lines, the `raise:` lines, the `fail:` lines.
    pub fn printed(&self) -> Vec<String> {
        let mut out = self.lines.clone();
        out.extend(self.raise.iter().map(|r| format!("raise: {r}")));
        out.extend(self.failures.iter().map(|f| format!("fail: {f}")));
        out
    }
}

/// Runs the harness over the repository at `root` and checks it against the expected files,
/// the note and the expected files at `base`. `Err` when the note or a data file cannot be
/// read.
pub fn check(root: &Path, base: &str) -> Result<Outcome, String> {
    let note = root.join(NOTE);
    let inventory = inventory::load(&note).map_err(|e| e.to_string())?;
    let text = std::fs::read_to_string(&note).map_err(|e| format!("{NOTE}: {e}"))?;
    let block = inventory::block(&text, "html-sections").map_err(|e| format!("{NOTE}: {e}"))?;
    let sections = parse_sections(&block).map_err(|e| format!("{NOTE}: html-sections: {e}"))?;
    let report = runner::run_all(&root.join(DATA), &inventory).map_err(|e| e.to_string())?;
    let mut out = Outcome::default();
    against_inventory(&inventory, &report, &mut out.failures);
    let groups = section_groups(&inventory, &report, &sections, &mut out.failures);
    let base = resolve(root, base, &mut out.failures);
    let at_base = base.as_ref().map(|base| read_at_base(root, base));
    let mut marks: Vec<(String, Mark)> = Vec::new();
    let mut expected_tags: Vec<(Group, Vec<(String, Count)>)> = Vec::new();
    for (index, group) in Group::ALL.into_iter().enumerate() {
        let path = expected_path(group);
        let files: Vec<&runner::FileReport> = report
            .files
            .iter()
            .zip(&inventory.files)
            .filter(|(_, f)| f.group == group)
            .map(|(r, _)| r)
            .collect();
        let measured_tags = report
            .groups
            .iter()
            .find(|(g, _)| *g == group)
            .map(|(_, tags)| tags.clone())
            .unwrap_or_default();
        let note_sections: Vec<&str> = groups
            .iter()
            .filter(|(_, g)| *g == Some(group))
            .map(|(name, _)| name.as_str())
            .collect();
        let text = match std::fs::read_to_string(root.join(&path)) {
            Ok(text) => text,
            Err(e) => {
                if !files.is_empty() || !note_sections.is_empty() {
                    out.failures.push(format!("{path}: {e}"));
                }
                continue;
            }
        };
        let expected = match parse_expected(&text) {
            Ok(expected) => expected,
            Err(e) => {
                out.failures.push(format!("{path}: {e}"));
                continue;
            }
        };
        compare_files(&path, &files, &expected, &mut out);
        compare_tags(&path, &measured_tags, &expected.tags, &mut out);
        let written: Vec<&str> = expected.sections.iter().map(|(n, _)| n.as_str()).collect();
        if written != note_sections {
            out.failures.push(format!(
                "{path}: the sections are [{}], the note's sections with data in {} are [{}]",
                written.join(", "),
                group.name(),
                note_sections.join(", ")
            ));
        }
        marks.extend(expected.sections.iter().cloned());
        expected_tags.push((group, expected.tags.clone()));
        if let (Some(base), Some(at_base)) = (&base, &at_base) {
            let answer = match at_base {
                Ok(answers) => Ok(&answers[index]),
                Err(e) => Err(e.as_str()),
            };
            against_base(root, base, &path, answer, &expected, &mut out.failures);
        }
    }
    tag_sums(&report, &expected_tags, &mut out.failures);
    for section in &sections {
        section_line(
            &path_of(&groups, section),
            section,
            &inventory,
            &report,
            &marks,
            &mut out,
        );
    }
    for (name, count) in &report.tags {
        out.lines.push(format!("tag {name} {}", count.fraction()));
    }
    out.lines.push(format!(
        "html-conformance: {} cases, {} run, {} passed",
        report.total.exist, report.total.run, report.total.pass
    ));
    Ok(out)
}

/// The expected file that holds a section's mark, or the note for a section without data.
fn path_of(groups: &[(String, Option<Group>)], section: &Section) -> String {
    groups
        .iter()
        .find(|(name, _)| *name == section.name)
        .and_then(|(_, g)| *g)
        .map_or_else(|| NOTE.to_string(), expected_path)
}

fn against_inventory(inventory: &Inventory, report: &Report, failures: &mut Vec<String>) {
    for (file, measured) in inventory.files.iter().zip(&report.files) {
        if measured.count.exist != file.cases {
            failures.push(format!(
                "{NOTE}: file {}: the harness reads {} cases, the inventory {}",
                file.path, measured.count.exist, file.cases
            ));
        }
        let mut written = file.tags.clone();
        written.sort();
        let mut read = measured.tags.clone();
        read.sort();
        if written != read {
            failures.push(format!(
                "{NOTE}: file {}: the harness counts {} by tag, the inventory {}",
                file.path,
                pairs(&read),
                pairs(&written)
            ));
        }
    }
    for (name, count) in &report.tags {
        match inventory.tags.iter().find(|(n, _)| n == name) {
            Some((_, n)) if *n == count.exist => {}
            Some((_, n)) => failures.push(format!(
                "{NOTE}: tag {name}: the harness reads {} cases, the inventory {n}",
                count.exist
            )),
            None => failures.push(format!(
                "{NOTE}: tag {name}: the harness reads {} cases, the inventory has no `tag` line",
                count.exist
            )),
        }
    }
}

fn pairs(pairs: &[(String, usize)]) -> String {
    let fields: Vec<String> = pairs.iter().map(|(k, n)| format!("{k}={n}")).collect();
    fields.join(" ")
}

/// Each section's group, `None` for a section with no `data` line; checks that every data
/// file and every case tagged `not-run` is in a section and that a section without data has
/// an `open` or a `wpt` line.
fn section_groups(
    inventory: &Inventory,
    report: &Report,
    sections: &[Section],
    failures: &mut Vec<String>,
) -> Vec<(String, Option<Group>)> {
    let mut out = Vec::new();
    let mut seen: Vec<&str> = Vec::new();
    for section in sections {
        let mut group = None;
        for path in &section.data {
            if seen.contains(&path.as_str()) {
                failures.push(format!(
                    "{NOTE}: html-sections: data {path} is in two sections"
                ));
            }
            seen.push(path);
            match inventory.files.iter().find(|f| f.path == *path) {
                None => failures.push(format!(
                    "{NOTE}: html-sections: data {path} of section {} is no file of the inventory",
                    section.name
                )),
                Some(file) => match group {
                    None => group = Some(file.group),
                    Some(g) if g != file.group => failures.push(format!(
                        "{NOTE}: html-sections: section {} has data files in {} and {}",
                        section.name,
                        g.name(),
                        file.group.name()
                    )),
                    Some(_) => {}
                },
            }
        }
        if section.data.is_empty() && section.open.is_empty() && section.wpt.is_empty() {
            failures.push(format!(
                "{NOTE}: html-sections: section {} has no data line and neither an open nor a \
                 wpt line: nothing tests it and nothing keeps it incomplete",
                section.name
            ));
        }
        out.push((section.name.clone(), group));
    }
    for (file, measured) in inventory.files.iter().zip(&report.files) {
        if seen.contains(&file.path.as_str()) {
            continue;
        }
        failures.push(format!(
            "{NOTE}: html-sections: file {} is in no section",
            file.path
        ));
        let not_run: usize = measured.not_run.iter().map(|(_, n)| n).sum();
        if not_run > 0 {
            failures.push(format!(
                "{NOTE}: html-sections: {not_run} cases of {} tagged not-run are in no section",
                file.path
            ));
        }
    }
    out
}

/// Compares the measured counts of one record with its expected counts.
fn compare(path: &str, record: &str, measured: Count, expected: Count, out: &mut Outcome) {
    if measured.exist != expected.exist {
        out.failures.push(format!(
            "{path}: {record}: {} cases, expected {}",
            measured.exist, expected.exist
        ));
    }
    if measured.run < expected.run {
        out.failures.push(format!(
            "{path}: {record}: run {}, expected {}",
            measured.run, expected.run
        ));
    }
    if measured.pass < expected.pass {
        out.failures.push(format!(
            "{path}: {record}: passed {}, expected {}",
            measured.pass, expected.pass
        ));
    }
    let mut above = Vec::new();
    if measured.run > expected.run {
        above.push(format!("run {} above {}", measured.run, expected.run));
    }
    if measured.pass > expected.pass {
        above.push(format!("passed {} above {}", measured.pass, expected.pass));
    }
    if !above.is_empty() && measured.exist == expected.exist {
        out.raise.push(format!(
            "{path}: {record}: {}: the expected file can be raised to {}",
            above.join(", "),
            measured.fraction()
        ));
    }
}

/// Names that occur more than once, and the order: a record list against the measured one.
fn sequence(
    path: &str,
    kind: &str,
    written: &[&str],
    measured: &[&str],
    failures: &mut Vec<String>,
) {
    for name in measured {
        if !written.contains(name) {
            failures.push(format!("{path}: {kind} {name} has no line"));
        }
    }
    for (i, name) in written.iter().enumerate() {
        if !measured.contains(name) {
            failures.push(format!(
                "{path}: {kind} {name} is not measured in this group"
            ));
        } else if written[..i].contains(name) {
            failures.push(format!("{path}: {kind} {name} has two lines"));
        }
    }
    let common: Vec<&&str> = written.iter().filter(|n| measured.contains(n)).collect();
    let order: Vec<&&str> = measured.iter().filter(|n| written.contains(n)).collect();
    if common.len() == order.len() && common != order {
        failures.push(format!(
            "{path}: the {kind} lines are not in the order of the inventory"
        ));
    }
}

fn compare_files(
    path: &str,
    files: &[&runner::FileReport],
    expected: &Expected,
    out: &mut Outcome,
) {
    let written: Vec<&str> = expected.files.iter().map(|(p, _, _)| p.as_str()).collect();
    let measured: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();
    sequence(path, "file", &written, &measured, &mut out.failures);
    for file in files {
        let Some((_, count, not_run)) = expected.files.iter().find(|(p, _, _)| *p == file.path)
        else {
            continue;
        };
        let record = format!("file {}", file.path);
        compare(path, &record, file.count, *count, out);
        let read: Vec<String> = file
            .not_run
            .iter()
            .map(|(reason, n)| format!("not-run:{reason}={n}"))
            .collect();
        if read != *not_run {
            out.failures.push(format!(
                "{path}: {record}: the harness prints [{}], expected [{}]",
                read.join(" "),
                not_run.join(" ")
            ));
        }
    }
}

fn compare_tags(
    path: &str,
    measured: &[(String, Count)],
    expected: &[(String, Count)],
    out: &mut Outcome,
) {
    let written: Vec<&str> = expected.iter().map(|(n, _)| n.as_str()).collect();
    let names: Vec<&str> = measured.iter().map(|(n, _)| n.as_str()).collect();
    sequence(path, "tag", &written, &names, &mut out.failures);
    for (name, count) in measured {
        if let Some((_, e)) = expected.iter().find(|(n, _)| n == name) {
            compare(path, &format!("tag {name}"), *count, *e, out);
        }
    }
}

/// The tag lines the harness prints against the per-group counts and the expected files.
fn tag_sums(
    report: &Report,
    expected: &[(Group, Vec<(String, Count)>)],
    failures: &mut Vec<String>,
) {
    for (name, count) in &report.tags {
        let mut groups = Count::default();
        for (_, tags) in &report.groups {
            for (n, c) in tags {
                if n == name {
                    groups.exist += c.exist;
                    groups.run += c.run;
                    groups.pass += c.pass;
                }
            }
        }
        if groups != *count {
            failures.push(format!(
                "tag {name}: the harness prints {}, its groups sum to {}",
                count.fraction(),
                groups.fraction()
            ));
        }
        if expected.len() == report.groups.len() {
            let mut files = Count::default();
            for (_, tags) in expected {
                for (n, c) in tags {
                    if n == name {
                        files.exist += c.exist;
                        files.run += c.run;
                        files.pass += c.pass;
                    }
                }
            }
            if files.exist != count.exist || files.run > count.run || files.pass > count.pass {
                failures.push(format!(
                    "tag {name}: the harness prints {}, the expected files sum to {}",
                    count.fraction(),
                    files.fraction()
                ));
            }
        }
    }
}

fn git(root: &Path, args: &[&str]) -> Result<std::process::Output, String> {
    Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .map_err(|e| format!("git could not run: {e}"))
}

/// The commit `base` names, or a failure naming it.
fn resolve(root: &Path, base: &str, failures: &mut Vec<String>) -> Option<String> {
    let refused = |failures: &mut Vec<String>, why: String| {
        failures.push(format!(
            "--base {base}: git cannot resolve the revision: {why}"
        ));
        None
    };
    if base.is_empty() || base.starts_with('-') {
        return refused(failures, "not a revision".to_string());
    }
    let spec = format!("{base}^{{commit}}");
    match git(root, &["rev-parse", "--verify", "--quiet", &spec]) {
        Err(e) => refused(failures, e),
        Ok(output) if !output.status.success() => {
            refused(failures, "no commit of that name".to_string())
        }
        Ok(output) => Some(String::from_utf8_lossy(&output.stdout).trim().to_string()),
    }
}

/// What `git cat-file --batch` answered for one expected file at the base.
#[derive(Debug)]
enum AtBase {
    /// No object at that path (`missing`, or another answer that names no object): the file is
    /// not compared.
    Missing,
    /// A blob: its bytes.
    Blob(Vec<u8>),
    /// An object of another type: read with `git show`, whose text the parser refuses.
    Other,
    /// The batch ended before it answered this file: its standard error.
    Unanswered(String),
}

/// The expected files of [`Group::ALL`] at `base`, in that order, read with one `git cat-file
/// --batch`; `Err` when git cannot run.
fn read_at_base(root: &Path, base: &str) -> Result<Vec<AtBase>, String> {
    let paths: Vec<String> = Group::ALL.into_iter().map(expected_path).collect();
    let mut child = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["cat-file", "--batch"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("git could not run: {e}"))?;
    let mut input = String::new();
    for path in &paths {
        let _ = writeln!(input, "{base}:{path}");
    }
    // A few hundred bytes, below a pipe's buffer: written whole before the answer is read. A
    // git that ended early makes this fail, and its answers end early.
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(input.as_bytes());
    }
    let output = child
        .wait_with_output()
        .map_err(|e| format!("git could not run: {e}"))?;
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let mut rest: &[u8] = &output.stdout;
    let mut answers = Vec::new();
    for _ in &paths {
        let Some(end) = rest.iter().position(|&b| b == b'\n') else {
            answers.push(AtBase::Unanswered(stderr.clone()));
            rest = &[];
            continue;
        };
        let header = String::from_utf8_lossy(&rest[..end]).into_owned();
        rest = &rest[end + 1..];
        let fields: Vec<&str> = header.split(' ').collect();
        let answer = match fields.as_slice() {
            [_, kind, size] => match size.parse::<usize>() {
                Ok(size) if rest.len() > size => {
                    let content = rest[..size].to_vec();
                    rest = &rest[size + 1..];
                    if *kind == "blob" {
                        AtBase::Blob(content)
                    } else {
                        AtBase::Other
                    }
                }
                Ok(_) => {
                    rest = &[];
                    AtBase::Unanswered(stderr.clone())
                }
                Err(_) => AtBase::Missing,
            },
            _ => AtBase::Missing,
        };
        answers.push(answer);
    }
    Ok(answers)
}

/// The expected file `path` at `base`, as `at_base` answered it, against its counts now.
fn against_base(
    root: &Path,
    base: &str,
    path: &str,
    at_base: Result<&AtBase, &str>,
    expected: &Expected,
    failures: &mut Vec<String>,
) {
    let text = match at_base {
        Err(e) => {
            failures.push(format!("{path} at {base}: {e}"));
            return;
        }
        Ok(AtBase::Missing) => return,
        Ok(AtBase::Unanswered(stderr)) => {
            failures.push(format!("{path} at {base}: git cat-file: {stderr}"));
            return;
        }
        Ok(AtBase::Blob(bytes)) => String::from_utf8_lossy(bytes).into_owned(),
        Ok(AtBase::Other) => match git(root, &["show", &format!("{base}:{path}")]) {
            Ok(output) if output.status.success() => {
                String::from_utf8_lossy(&output.stdout).into_owned()
            }
            Ok(output) => {
                failures.push(format!(
                    "{path} at {base}: git show: {}",
                    String::from_utf8_lossy(&output.stderr).trim()
                ));
                return;
            }
            Err(e) => {
                failures.push(format!("{path} at {base}: {e}"));
                return;
            }
        },
    };
    let then = match parse_expected(&text) {
        Ok(then) => then,
        Err(e) => {
            failures.push(format!("{path} at {base}: {e}"));
            return;
        }
    };
    let mut records: Vec<(String, Count, Count)> = Vec::new();
    for (p, now, _) in &expected.files {
        if let Some((_, was, _)) = then.files.iter().find(|(q, _, _)| q == p) {
            records.push((format!("file {p}"), *now, *was));
        }
    }
    for (n, now) in &expected.tags {
        if let Some((_, was)) = then.tags.iter().find(|(m, _)| m == n) {
            records.push((format!("tag {n}"), *now, *was));
        }
    }
    for (record, now, was) in records {
        for (what, n, w) in [
            ("cases", now.exist, was.exist),
            ("run", now.run, was.run),
            ("passed", now.pass, was.pass),
        ] {
            if n < w {
                failures.push(format!(
                    "{path}: {record}: expected {what} {n}, at {base} {w}: an expected count \
                     is lowered"
                ));
            }
        }
    }
}

/// The section's line, and a failure for each reason a `complete` mark is refused.
fn section_line(
    path: &str,
    section: &Section,
    inventory: &Inventory,
    report: &Report,
    marks: &[(String, Mark)],
    out: &mut Outcome,
) {
    let mut count = Count::default();
    for (file, measured) in inventory.files.iter().zip(&report.files) {
        if section.data.contains(&file.path) {
            count.exist += measured.count.exist;
            count.run += measured.count.run;
            count.pass += measured.count.pass;
        }
    }
    let failing = count.run - count.pass;
    let not_run = count.exist - count.run;
    let wpt = section.wpt_count();
    let mut reason = String::new();
    if failing > 0 {
        let _ = write!(reason, " failing:{failing}");
    }
    if not_run > 0 {
        let _ = write!(reason, " not-run:{not_run}");
    }
    if !section.open.is_empty() {
        let _ = write!(reason, " open:{}", section.open.join(","));
    }
    if !section.wpt.is_empty() {
        let _ = write!(reason, " wpt:{wpt}");
    }
    let name = &section.name;
    let marked = marks.iter().find(|(n, _)| n == name).map(|(_, m)| *m);
    if marked == Some(Mark::Complete) {
        let mut refused = Vec::new();
        if failing > 0 {
            refused.push(format!("{failing} failing"));
        }
        if not_run > 0 {
            refused.push(format!("{not_run} not run"));
        }
        for id in &section.open {
            refused.push(format!("open {id}"));
        }
        for wpt in &section.wpt {
            refused.push(format!("wpt {wpt}"));
        }
        if refused.is_empty() {
            out.lines.push(format!(
                "section {name} complete {}/{}",
                count.pass, count.exist
            ));
            return;
        }
        out.failures.push(format!(
            "{path}: section {name} is marked complete with {}",
            refused.join(", ")
        ));
    }
    if reason.is_empty() {
        reason = " marked-incomplete".to_string();
        if marked == Some(Mark::Incomplete) {
            out.raise.push(format!(
                "{path}: section {name}: every case passes and nothing is open: it can be marked \
                 complete"
            ));
        }
    }
    out.lines.push(format!(
        "section {name} incomplete {}/{}{reason}",
        count.pass, count.exist
    ));
}
