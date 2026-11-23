//! The counts of a run, `expected/limbo-kinds.txt`, `expected/limbo.txt`, and the check that
//! holds a run to them with the rules of `surf-html-conformance --check`.
//!
//! `limbo-kinds.txt` has one line per pattern of case ids: `<pattern> <kind> [<check>]`, the
//! pattern a case id in which `*` stands for any run of characters, the kind
//! `CertificateNotTrusted` with a check, `CertificateExpired` or `CertificateNameMismatch`. A
//! line may name more than one outcome, joined by ` | `, for a family whose cases fail for one
//! of them (bettertls: a name outside the constraints or no name for the host). A check that is
//! not one of item 7 ([`unchecked`]) names a property surf does not check: such a case can only
//! be accepted. Every FAILURE case that runs matches exactly one line, and every line matches a
//! FAILURE case that runs.
//!
//! `limbo.txt` holds the lines the run prints for the counts (`x509-limbo:`, `group`,
//! `section`) and one line `open <id> <reason>` for each case that runs and does not pass.
//!
//! The check fails when: a pass or run count is below the expected one, or a case count differs
//! from it; a group or section is in the run and not in the file or the other way round; a
//! section the file marks complete is not; an expected count is below its value in the file at
//! the base revision, or a section complete there is not now; the base is no revision git can
//! resolve; a case that runs and does not pass has no `open` line, or an `open` line names a case
//! that does not run; a FAILURE case that runs matches no line or two lines of
//! `limbo-kinds.txt`, or a line matches none; a FAILURE case is accepted whose kind is one an
//! outcome of `INDEX` names. A count above the expected one, a section complete that the file
//! marks incomplete and an `open` line of a case that passes each print a `raise:` line.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::process::Command;

use surf_tls::verify::Check;

use crate::runner::{self, CaseResult, Expected as Want, Outcome as Ended};
use crate::{group_of, paths, section_of};

/// Cases that exist, run and pass.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Count {
    /// Passed.
    pub pass: usize,
    /// Run.
    pub run: usize,
    /// Exist.
    pub exist: usize,
}

impl Count {
    /// `<pass>/<run>/<exist>`.
    pub fn fraction(&self) -> String {
        format!("{}/{}/{}", self.pass, self.run, self.exist)
    }

    fn add(&mut self, ran: bool, passed: bool) {
        self.exist += 1;
        self.run += usize::from(ran);
        self.pass += usize::from(passed);
    }

    /// Whether every case runs and passes.
    pub fn complete(&self) -> bool {
        self.pass == self.exist
    }
}

/// A line of `limbo-kinds.txt`: the pattern and the outcome a case it matches ends with.
pub type KindLine = (String, Vec<String>);

/// What a run counted, measured against `limbo-kinds.txt`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Report {
    /// All cases.
    pub total: Count,
    /// Per group, by name.
    pub groups: BTreeMap<String, Count>,
    /// Per section of [`crate::SECTIONS`], in its order.
    pub sections: Vec<(String, Count)>,
    /// The `not-run`, `refused`, `wrong-kind` and `accepted` lines.
    pub lines: Vec<String>,
    /// The cases that run and do not pass, each with what happened.
    pub failing: BTreeMap<String, String>,
    /// The cases that run and pass.
    pub passing: BTreeSet<String>,
    /// The cases that do not run.
    pub not_run: BTreeSet<String>,
    /// FAILURE cases accepted, each with the outcome of its kinds line.
    pub accepted: Vec<(String, Vec<String>)>,
    /// FAILURE cases that run and match no kinds line, or more than one (with the patterns).
    pub unmatched: Vec<(String, Vec<String>)>,
    /// Kinds lines that match no FAILURE case that runs.
    pub unused: Vec<String>,
}

impl Report {
    /// The lines of the counts, as `limbo.txt` writes them.
    pub fn count_lines(&self) -> Vec<String> {
        let mut out = vec![total_line(&self.total)];
        for (name, count) in &self.groups {
            out.push(format!("group {name} {}", count.fraction()));
        }
        for (name, count) in &self.sections {
            let mark = if count.complete() {
                "complete"
            } else {
                "incomplete"
            };
            out.push(format!("section {name} {} {mark}", count.fraction()));
        }
        out
    }
}

fn total_line(count: &Count) -> String {
    format!(
        "x509-limbo: {} cases, {} run, {} passed",
        count.exist, count.run, count.pass
    )
}

/// Whether `pattern` (`*` for any run of characters) matches all of `text`.
pub fn glob(pattern: &str, text: &str) -> bool {
    let parts: Vec<&str> = pattern.split('*').collect();
    if parts.len() == 1 {
        return pattern == text;
    }
    let (first, last) = (parts[0], parts[parts.len() - 1]);
    if !text.starts_with(first) || !text.ends_with(last) || text.len() < first.len() + last.len() {
        return false;
    }
    let mut rest = &text[first.len()..text.len() - last.len()];
    for middle in &parts[1..parts.len() - 1] {
        match rest.find(middle) {
            Some(at) => rest = &rest[at + middle.len()..],
            None => return false,
        }
    }
    true
}

/// Reads `limbo-kinds.txt`.
pub fn parse_kinds(text: &str) -> Result<Vec<KindLine>, String> {
    let mut out = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let err = || format!("line {}: not `<pattern> <kind> [<check>] [| ...]`", i + 1);
        let (pattern, rest) = line.split_once(' ').ok_or_else(err)?;
        let mut outcomes = Vec::new();
        for outcome in rest.split(" | ") {
            let fields: Vec<&str> = outcome.split(' ').collect();
            let ok = match fields.as_slice() {
                ["CertificateExpired" | "CertificateNameMismatch"] => true,
                ["CertificateNotTrusted", check] => {
                    !check.is_empty() && check.bytes().all(|b| b.is_ascii_alphanumeric())
                }
                _ => false,
            };
            if !ok || outcomes.iter().any(|o| o == outcome) {
                return Err(err());
            }
            outcomes.push(outcome.to_string());
        }
        if pattern.is_empty() {
            return Err(err());
        }
        out.push((pattern.to_string(), outcomes));
    }
    Ok(out)
}

/// Whether `outcome` (`CertificateNotTrusted <check>` and the like) names a check that is not
/// one of [`Check::ALL`]: a property surf does not check.
pub fn unchecked(outcome: &str) -> bool {
    outcome
        .strip_prefix("CertificateNotTrusted ")
        .is_some_and(|check| !Check::ALL.iter().any(|c| c.name() == check))
}

/// Counts `results` and holds each FAILURE case that runs to its line of `kinds`.
pub fn report(results: &[CaseResult], kinds: &[KindLine]) -> Report {
    let mut out = Report::default();
    let mut sections: Vec<(String, Count)> = Vec::new();
    let mut used = vec![false; kinds.len()];
    for case in results {
        let id = &case.id;
        let (ran, passed) = match (&case.outcome, case.expected) {
            (Ended::NotRun(reason), _) => {
                out.lines.push(format!("not-run {id} {reason}"));
                out.not_run.insert(id.clone());
                (false, false)
            }
            (Ended::Ran(got), Want::Success) => {
                let passed = got == "ok";
                if !passed {
                    out.lines
                        .push(format!("refused {id} {got} ({})", case.detail));
                    out.failing.insert(id.clone(), format!("refused: {got}"));
                }
                (true, passed)
            }
            (Ended::Ran(got), Want::Failure) => {
                let matched: Vec<usize> = (0..kinds.len())
                    .filter(|&k| glob(&kinds[k].0, id))
                    .collect();
                matched.iter().for_each(|&k| used[k] = true);
                let passed = match matched.as_slice() {
                    [k] => {
                        let wants = &kinds[*k].1;
                        let want = wants.join(" | ");
                        if got == "ok" {
                            out.lines.push(format!("accepted {id} expected {want}"));
                            out.failing
                                .insert(id.clone(), format!("accepted, expected {want}"));
                            out.accepted.push((id.clone(), wants.clone()));
                            false
                        } else if !wants.contains(got) {
                            out.lines.push(format!(
                                "wrong-kind {id} {got} expected {want} ({})",
                                case.detail
                            ));
                            out.failing
                                .insert(id.clone(), format!("{got}, expected {want}"));
                            false
                        } else {
                            true
                        }
                    }
                    _ => {
                        let patterns = matched.iter().map(|&k| kinds[k].0.clone()).collect();
                        out.unmatched.push((id.clone(), patterns));
                        out.failing.insert(id.clone(), format!("{got}, no kind"));
                        false
                    }
                };
                (true, passed)
            }
        };
        if passed {
            out.passing.insert(id.clone());
        }
        out.total.add(ran, passed);
        out.groups
            .entry(group_of(id).to_string())
            .or_default()
            .add(ran, passed);
        if let Some(section) = section_of(id) {
            match sections.iter_mut().find(|(name, _)| name == section) {
                Some((_, count)) => count.add(ran, passed),
                None => {
                    let mut count = Count::default();
                    count.add(ran, passed);
                    sections.push((section.to_string(), count));
                }
            }
        }
    }
    let order = |name: &str| {
        crate::SECTIONS
            .iter()
            .position(|(_, s)| *s == name)
            .unwrap_or(usize::MAX)
    };
    sections.sort_by_key(|(name, _)| order(name));
    out.sections = sections;
    out.unused = kinds
        .iter()
        .zip(&used)
        .filter(|(_, used)| !**used)
        .map(|((pattern, _), _)| pattern.clone())
        .collect();
    out
}

/// The content of `limbo.txt`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ExpectedFile {
    /// The `x509-limbo:` line.
    pub total: Count,
    /// The `group` lines.
    pub groups: BTreeMap<String, Count>,
    /// The `section` lines: name, counts, marked complete.
    pub sections: Vec<(String, Count, bool)>,
    /// The `open` lines: case id and reason.
    pub open: BTreeMap<String, String>,
}

fn fraction(field: &str) -> Option<Count> {
    let mut parts = field.split('/').map(|p| {
        (!p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
            .then(|| p.parse::<usize>().ok())
            .flatten()
    });
    let count = Count {
        pass: parts.next()??,
        run: parts.next()??,
        exist: parts.next()??,
    };
    (parts.next().is_none() && count.pass <= count.run && count.run <= count.exist).then_some(count)
}

/// Reads `limbo.txt`.
pub fn parse_expected(text: &str) -> Result<ExpectedFile, String> {
    let mut out = ExpectedFile::default();
    let mut total = None;
    for (i, line) in text.lines().enumerate() {
        let err = || format!("line {}: `{line}` is not a line of limbo.txt", i + 1);
        let fields: Vec<&str> = line.split(' ').collect();
        match fields.as_slice() {
            ["x509-limbo:", exist, "cases,", run, "run,", pass, "passed"] if total.is_none() => {
                let count = fraction(&format!("{pass}/{run}/{exist}")).ok_or_else(err)?;
                total = Some(count);
            }
            ["group", name, counts] if !out.groups.contains_key(*name) => {
                out.groups
                    .insert(name.to_string(), fraction(counts).ok_or_else(err)?);
            }
            ["section", name, counts, mark @ ("complete" | "incomplete")]
                if !out.sections.iter().any(|(n, _, _)| n == name) =>
            {
                let count = fraction(counts).ok_or_else(err)?;
                out.sections
                    .push((name.to_string(), count, *mark == "complete"));
            }
            ["open", id, reason @ ..] if !reason.is_empty() && !out.open.contains_key(*id) => {
                out.open.insert(id.to_string(), reason.join(" "));
            }
            _ => return Err(err()),
        }
    }
    out.total = total.ok_or("no x509-limbo: line")?;
    Ok(out)
}

/// What a check found.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Outcome {
    /// The lines of the run.
    pub lines: Vec<String>,
    /// Counts that can be raised, each without its `raise: ` prefix.
    pub raise: Vec<String>,
    /// The failures, each without its `fail: ` prefix.
    pub failures: Vec<String>,
}

impl Outcome {
    /// Everything to print: the lines, the `raise:` lines, the `fail:` lines.
    pub fn printed(&self) -> Vec<String> {
        let mut out = self.lines.clone();
        out.extend(self.raise.iter().map(|r| format!("raise: {r}")));
        out.extend(self.failures.iter().map(|f| format!("fail: {f}")));
        out
    }
}

fn hold(record: &str, now: Count, expected: Count, out: &mut Outcome) {
    let path = paths::EXPECTED;
    if now.exist != expected.exist {
        out.failures.push(format!(
            "{path}: {record}: {} cases, the file says {}",
            now.exist, expected.exist
        ));
    }
    for (what, n, e) in [
        ("run", now.run, expected.run),
        ("passed", now.pass, expected.pass),
    ] {
        if n < e {
            out.failures.push(format!(
                "{path}: {record}: {n} {what}, below the expected {e}"
            ));
        } else if n > e {
            out.raise
                .push(format!("{path}: {record}: {n} {what}, the file says {e}"));
        }
    }
}

fn not_below(record: &str, now: Count, then: Count, base: &str, out: &mut Outcome) {
    for (what, n, t) in [("run", now.run, then.run), ("passed", now.pass, then.pass)] {
        if n < t {
            out.failures.push(format!(
                "{}: {record}: {n} {what}, below its {t} at {base}",
                paths::EXPECTED
            ));
        }
    }
}

/// Holds `report` to `expected`, to the same file at the base (`base`: its name and content,
/// when the file exists there), and to the outcomes `INDEX` names.
pub fn compare(
    report: &Report,
    expected: &ExpectedFile,
    base: Option<(&str, &ExpectedFile)>,
    index_outcomes: &BTreeSet<String>,
) -> Outcome {
    let mut out = Outcome::default();
    let path = paths::EXPECTED;
    out.lines = report.count_lines();
    out.lines.extend(report.lines.iter().cloned());
    hold("x509-limbo", report.total, expected.total, &mut out);
    let names = |a: Vec<&String>, b: Vec<&String>, what: &str, out: &mut Outcome| {
        for name in &a {
            if !b.contains(name) {
                out.failures.push(format!(
                    "{path}: no {what} line for {name}, which the run has"
                ));
            }
        }
        for name in &b {
            if !a.contains(name) {
                out.failures.push(format!(
                    "{path}: a {what} line for {name}, which the run has not"
                ));
            }
        }
    };
    names(
        report.groups.keys().collect(),
        expected.groups.keys().collect(),
        "group",
        &mut out,
    );
    names(
        report.sections.iter().map(|(n, _)| n).collect(),
        expected.sections.iter().map(|(n, _, _)| n).collect(),
        "section",
        &mut out,
    );
    for (name, count) in &report.groups {
        if let Some(e) = expected.groups.get(name) {
            hold(&format!("group {name}"), *count, *e, &mut out);
        }
    }
    for (name, count) in &report.sections {
        if let Some((_, e, complete)) = expected.sections.iter().find(|(n, _, _)| n == name) {
            hold(&format!("section {name}"), *count, *e, &mut out);
            if *complete && !count.complete() {
                out.failures.push(format!(
                    "{path}: section {name} is marked complete and {} of its {} cases pass",
                    count.pass, count.exist
                ));
            } else if !*complete && count.complete() {
                out.raise.push(format!(
                    "{path}: section {name} is complete, the file says incomplete"
                ));
            }
        }
    }
    if let Some((name, then)) = base {
        not_below("x509-limbo", expected.total, then.total, name, &mut out);
        for (group, count) in &expected.groups {
            if let Some(t) = then.groups.get(group) {
                not_below(&format!("group {group}"), *count, *t, name, &mut out);
            }
        }
        for (section, count, complete) in &expected.sections {
            if let Some((_, t, was)) = then.sections.iter().find(|(n, _, _)| n == section) {
                not_below(&format!("section {section}"), *count, *t, name, &mut out);
                if *was && !complete {
                    out.failures.push(format!(
                        "{path}: section {section} is complete at {name} and marked incomplete"
                    ));
                }
            }
        }
    }
    for (id, what) in &report.failing {
        if !expected.open.contains_key(id) {
            out.failures.push(format!(
                "{id} does not pass ({what}) and has no open line in {path}"
            ));
        }
    }
    for id in expected.open.keys() {
        if report.passing.contains(id) {
            out.raise
                .push(format!("{path}: {id} passes; its open line can go"));
        } else if !report.failing.contains_key(id) {
            out.failures
                .push(format!("{path}: open {id}: no case of that id runs"));
        }
    }
    for (id, patterns) in &report.unmatched {
        if patterns.is_empty() {
            out.failures.push(format!(
                "{id}: a FAILURE case that no line of {} matches",
                paths::KINDS
            ));
        } else {
            out.failures.push(format!(
                "{id}: a FAILURE case that lines {} of {} match",
                patterns.join(", "),
                paths::KINDS
            ));
        }
    }
    for pattern in &report.unused {
        out.failures.push(format!(
            "{}: {pattern} matches no FAILURE case that runs",
            paths::KINDS
        ));
    }
    for (id, wants) in &report.accepted {
        for want in wants.iter().filter(|w| index_outcomes.contains(*w)) {
            out.failures.push(format!(
                "{id}: accepted, and its kind {want} is one {} names",
                paths::INDEX
            ));
        }
    }
    out
}

/// The outcomes of the lines of `INDEX` (the fourth field on), `ok` left out.
pub fn index_outcomes(text: &str) -> BTreeSet<String> {
    text.lines()
        .filter_map(|line| {
            let fields: Vec<&str> = line.split(' ').collect();
            (fields.len() >= 4 && fields[3] != "ok").then(|| fields[3..].join(" "))
        })
        .collect()
}

fn git(root: &Path, args: &[&str]) -> Result<std::process::Output, String> {
    Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .map_err(|e| format!("git could not run: {e}"))
}

/// The commit `base` names in the repository at `root`.
pub fn resolve(root: &Path, base: &str) -> Result<String, String> {
    let refused = |why: &str| {
        Err(format!(
            "--base {base}: git cannot resolve the revision: {why}"
        ))
    };
    if base.is_empty() || base.starts_with('-') {
        return refused("not a revision");
    }
    match git(
        root,
        &[
            "rev-parse",
            "--verify",
            "--quiet",
            &format!("{base}^{{commit}}"),
        ],
    ) {
        Err(e) => refused(&e),
        Ok(output) if !output.status.success() => refused("no commit of that name"),
        Ok(output) => Ok(String::from_utf8_lossy(&output.stdout).trim().to_string()),
    }
}

/// The file `path` at the commit `commit`, or `None` when it does not exist there.
pub fn file_at(root: &Path, commit: &str, path: &str) -> Result<Option<String>, String> {
    let object = format!("{commit}:{path}");
    match git(root, &["cat-file", "-e", &object])? {
        output if !output.status.success() => return Ok(None),
        _ => {}
    }
    let output = git(root, &["show", &object])?;
    if !output.status.success() {
        return Err(format!(
            "{path} at {commit}: git show: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(Some(String::from_utf8_lossy(&output.stdout).into_owned()))
}

fn read(root: &Path, path: &str) -> Result<String, String> {
    std::fs::read_to_string(root.join(path)).map_err(|e| format!("{path}: {e}"))
}

/// The run of the repository at `root`, counted against its `limbo-kinds.txt`.
pub fn run(root: &Path) -> Result<Report, String> {
    let kinds =
        parse_kinds(&read(root, paths::KINDS)?).map_err(|e| format!("{}: {e}", paths::KINDS))?;
    let results = runner::run_all(&root.join(paths::DATA))?;
    Ok(report(&results, &kinds))
}

/// Runs the cases of the repository at `root` and holds them to its expected files and to
/// those at `base`. `Err` when a file cannot be read.
pub fn check(root: &Path, base: &str) -> Result<Outcome, String> {
    let report = run(root)?;
    let expected = parse_expected(&read(root, paths::EXPECTED)?)
        .map_err(|e| format!("{}: {e}", paths::EXPECTED))?;
    let index = index_outcomes(&read(root, paths::INDEX)?);
    let mut failures = Vec::new();
    let then = match resolve(root, base) {
        Err(e) => {
            failures.push(e);
            None
        }
        Ok(commit) => match file_at(root, &commit, paths::EXPECTED)? {
            None => None,
            Some(text) => Some(
                parse_expected(&text).map_err(|e| format!("{} at {base}: {e}", paths::EXPECTED))?,
            ),
        },
    };
    let mut out = compare(&report, &expected, then.as_ref().map(|t| (base, t)), &index);
    out.failures.extend(failures);
    Ok(out)
}
