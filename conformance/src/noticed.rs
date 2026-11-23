//! The list of scenarios that notice each planted defect, `tests/noticed.txt`, and the narrowed
//! suites made from it.
//!
//! The list's first line is `sources <sha256>`, the digest of its sources, which
//! `taskfiles/conform-noticed.sh check` compares before the tests run; then one line per planted
//! defect, `<label>: <id> <id> <id>`, the first [`NOTICED_PER_DEFECT`] scenarios, in the suite's
//! order, that do not pass over it. `task conform-slow` writes it again: with [`NOTICED_OUT`]
//! set every planted defect runs over the whole suite and [`note_noticed`] writes its line.
//!
//! Admitting the whole suite is the expensive part of a run (seconds under load, against
//! milliseconds for 3 scenarios), so it is done once, not in each test process: the setup step
//! of `task conform-rust` (`surf-conformance --narrow`, [`write_narrowed`]) writes the narrowed
//! suite of every listed defect under [`NARROWED`], in a directory named by the list's digest,
//! and each planted-defect test reads its own file ([`listed_suite`]). A missing file is an
//! error that names the step; nothing falls back to the whole suite.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use ess_conformance::{AdmittedSuite, ConformanceSuite, CountReport};

/// The suite `task suite` writes.
pub const SUITE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../.out/suite.json");

/// The committed list every planted-defect test of `tests/` reads.
pub const NOTICED: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/noticed.txt");

/// The variable `task conform-slow` sets to a directory: each planted defect then runs over the
/// whole suite, and [`note_noticed`] writes its line of [`NOTICED`] and its count line there.
pub const NOTICED_OUT: &str = "SURF_NOTICED_OUT";

/// How many scenarios a line of [`NOTICED`] names at most.
pub const NOTICED_PER_DEFECT: usize = 3;

/// Where the setup step writes the narrowed suites: one directory per digest of the list's
/// sources, holding `<label>.json` per defect, `scenarios.txt` (every scenario of the suite, in
/// its order) and a copy of the list it was made from.
pub const NARROWED: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/target/noticed");

/// The end of every message a stale list can cause.
pub const STALE: &str = "the list tests/noticed.txt may be stale (a change under crates/ or a \
line edited by hand); `task conform-slow WRITE=1` regenerates it";

/// The step that writes [`NARROWED`], as a message names it.
const SETUP: &str = "the setup step of `task conform-rust` (`surf-conformance --narrow \
.out/suite.json conformance/tests/noticed.txt`) writes it";

/// Whether [`NOTICED_OUT`] is set: the planted defects run over the whole suite.
pub fn writing_noticed() -> bool {
    std::env::var_os(NOTICED_OUT).is_some()
}

/// Admits `suite`, the bytes of a suite.
pub fn admit(suite: &str) -> Result<AdmittedSuite, String> {
    AdmittedSuite::from_json(suite).map_err(|error| error.to_string())
}

/// The bytes of [`SUITE`].
pub fn whole_suite() -> Result<String, String> {
    std::fs::read_to_string(SUITE)
        .map_err(|error| format!("{SUITE}: {error}; run `task suite` first"))
}

/// The committed list, [`NOTICED`].
fn committed_list() -> Result<String, String> {
    std::fs::read_to_string(NOTICED).map_err(|error| format!("{NOTICED}: {error}"))
}

/// The scenarios the line of `label` in `list` names.
pub fn listed<'a>(list: &'a str, label: &str) -> Result<Vec<&'a str>, String> {
    list.lines()
        .skip(1)
        .find_map(|line| line.strip_prefix(label)?.strip_prefix(": "))
        .map(|ids| ids.split(' ').collect())
        .ok_or_else(|| format!("{label}: tests/noticed.txt has no line for this defect; {STALE}"))
}

/// `admitted` narrowed to the scenarios `ids`, as canonical JSON. An id the suite does not hold
/// is an error naming `label`.
pub fn narrowed(admitted: &AdmittedSuite, label: &str, ids: &[&str]) -> Result<String, String> {
    let whole = admitted.suite();
    let scenarios: BTreeMap<_, _> = whole
        .scenarios
        .iter()
        .filter(|(id, _)| ids.contains(&id.to_string().as_str()))
        .map(|(id, scenario)| (id.clone(), scenario.clone()))
        .collect();
    if scenarios.len() != ids.len() {
        let held: Vec<String> = scenarios.keys().map(ToString::to_string).collect();
        let missing: Vec<&&str> = ids
            .iter()
            .filter(|id| !held.iter().any(|held| held == **id))
            .collect();
        return Err(format!(
            "{label}: tests/noticed.txt names {missing:?}, which the suite does not hold; {STALE}"
        ));
    }
    let suite = ConformanceSuite {
        provenance: whole.provenance.clone(),
        scenarios,
    };
    suite.to_canonical_json().map_err(|error| error.to_string())
}

/// The digest of the list's sources, from its first line.
fn digest_of(list: &str) -> Result<&str, String> {
    list.lines()
        .next()
        .and_then(|line| line.strip_prefix("sources "))
        .filter(|digest| !digest.is_empty() && digest.chars().all(|c| c.is_ascii_hexdigit()))
        .ok_or_else(|| "tests/noticed.txt: its first line is not `sources <sha256>`".to_owned())
}

/// The file name a defect's label gives: its ASCII letters and digits, `-` for every other
/// character.
fn file_name(label: &str) -> String {
    label
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect()
}

/// Writes the narrowed suite of every line of `list` from `suite` into
/// `<NARROWED>/<digest>/`, unless that directory holds a copy of exactly `list`; removes the
/// directories of other digests. Returns the line the step prints.
pub fn write_narrowed(suite: &str, list: &str) -> Result<String, String> {
    let digest = digest_of(list)?;
    let root = Path::new(NARROWED);
    let shown = format!("conformance/target/noticed/{digest}");
    let dir = root.join(digest);
    let labels = list
        .lines()
        .skip(1)
        .map(|line| {
            line.split_once(": ")
                .map(|(label, _)| label)
                .ok_or_else(|| format!("tests/noticed.txt: no `: ` in {line:?}"))
        })
        .collect::<Result<Vec<&str>, String>>()?;
    if std::fs::read_to_string(dir.join("noticed.txt")).is_ok_and(|held| held == list) {
        return Ok(format!(
            "conform-rust: the narrowed suites of {} planted defects in {shown} are current",
            labels.len()
        ));
    }
    let admitted = admit(suite)?;
    let io = |path: &Path, error: std::io::Error| format!("{}: {error}", path.display());
    if root.is_dir() {
        for entry in std::fs::read_dir(root).map_err(|error| io(root, error))? {
            let path = entry.map_err(|error| io(root, error))?.path();
            std::fs::remove_dir_all(&path).map_err(|error| io(&path, error))?;
        }
    }
    let partial = root.join(format!("{digest}.partial"));
    std::fs::create_dir_all(&partial).map_err(|error| io(&partial, error))?;
    for label in &labels {
        let ids = listed(list, label)?;
        let path = partial.join(format!("{}.json", file_name(label)));
        let text = narrowed(&admitted, label, &ids)?;
        std::fs::write(&path, text).map_err(|error| io(&path, error))?;
    }
    let order: Vec<String> = admitted
        .suite()
        .scenarios
        .keys()
        .map(ToString::to_string)
        .collect();
    let path = partial.join("scenarios.txt");
    std::fs::write(&path, order.join("\n") + "\n").map_err(|error| io(&path, error))?;
    let path = partial.join("noticed.txt");
    std::fs::write(&path, list).map_err(|error| io(&path, error))?;
    std::fs::rename(&partial, &dir).map_err(|error| io(&dir, error))?;
    Ok(format!(
        "conform-rust: the narrowed suites of {} planted defects written to {shown}",
        labels.len()
    ))
}

/// The directory of the narrowed suites made from the committed list's sources.
fn narrowed_dir(list: &str) -> Result<PathBuf, String> {
    Ok(Path::new(NARROWED).join(digest_of(list)?))
}

/// The suite the planted defect `label` runs over: the whole suite when [`NOTICED_OUT`] is set;
/// else the narrowed suite the setup step wrote for the sources of the committed list. A label
/// without a line, and a missing file, are errors; neither falls back to the whole suite.
pub fn listed_suite(label: &str) -> Result<String, String> {
    if writing_noticed() {
        return whole_suite();
    }
    let list = committed_list()?;
    listed(&list, label)?;
    let path = narrowed_dir(&list)?.join(format!("{}.json", file_name(label)));
    std::fs::read_to_string(&path).map_err(|error| {
        format!(
            "{label}: {}: {error}: no narrowed suite made from the sources of tests/noticed.txt; \
             {SETUP}",
            path.display()
        )
    })
}

/// Every scenario of the suite, in its order, as the setup step wrote them beside the narrowed
/// suites.
pub fn suite_scenarios() -> Result<Vec<String>, String> {
    let list = committed_list()?;
    let path = narrowed_dir(&list)?.join("scenarios.txt");
    let text = std::fs::read_to_string(&path)
        .map_err(|error| format!("{}: {error}; {SETUP}", path.display()))?;
    Ok(text.lines().map(str::to_owned).collect())
}

/// After the planted defect `label` ran. When [`NOTICED_OUT`] is set, writes into that directory
/// its line of [`NOTICED`] (`<name>.line`: the first [`NOTICED_PER_DEFECT`] scenarios of
/// `report`, in the suite's order, that did not pass) and its count line (`<name>.count`).
/// Otherwise checks that the scenarios that ran are those its line names, so a narrowed suite
/// made from another list is refused.
pub fn note_noticed(label: &str, report: &CountReport) -> Result<(), String> {
    let ran: Vec<String> = report.statuses().into_keys().collect();
    let Some(directory) = std::env::var_os(NOTICED_OUT) else {
        let list = committed_list()?;
        let ids = listed(&list, label)?;
        if ran != ids {
            return Err(format!(
                "{label}: the narrowed suite ran {ran:?}, not the scenarios tests/noticed.txt \
                 lists, {ids:?}; it was made from another list; {SETUP}"
            ));
        }
        return Ok(());
    };
    let ids: Vec<String> = report
        .statuses()
        .into_iter()
        .filter(|(_, status)| *status != "passed")
        .map(|(scenario, _)| scenario)
        .take(NOTICED_PER_DEFECT)
        .collect();
    let counts = report.counts();
    let directory = Path::new(&directory);
    let name = file_name(label);
    let path = directory.join(format!("{name}.line"));
    std::fs::write(&path, format!("{label}: {}\n", ids.join(" ")))
        .map_err(|error| format!("{}: {error}", path.display()))?;
    let path = directory.join(format!("{name}.count"));
    std::fs::write(
        &path,
        format!(
            "{label}: {} scenarios: {} passed, {} failed, {} error, {} unsupported\n",
            counts.total, counts.passed, counts.failed, counts.error, counts.unsupported
        ),
    )
    .map_err(|error| format!("{}: {error}", path.display()))
}

/// The message of a planted defect `label` that no scenario it ran noticed: `what` happened,
/// over the whole suite, or over its listed scenarios, which may be stale.
pub fn not_noticed(label: &str, what: &str) -> String {
    if writing_noticed() {
        format!("{label}: {what}, over the whole suite")
    } else {
        format!("{label}: {what}, of the scenarios tests/noticed.txt lists for it; {STALE}")
    }
}

/// `run` over each of `items`, in order: one after another, or each on a thread of its own when
/// [`NOTICED_OUT`] is set, since then each runs the whole suite.
pub fn each_planted<T: Sync, R: Send>(items: &[T], run: impl Fn(&T) -> R + Sync) -> Vec<R> {
    if !writing_noticed() {
        return items.iter().map(&run).collect();
    }
    std::thread::scope(|scope| {
        let handles: Vec<_> = items.iter().map(|item| scope.spawn(|| run(item))).collect();
        handles
            .into_iter()
            .map(|handle| {
                handle
                    .join()
                    .unwrap_or_else(|panic| std::panic::resume_unwind(panic))
            })
            .collect()
    })
}
