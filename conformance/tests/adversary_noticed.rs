//! Adversary cases on `tests/noticed.txt` and `surf_conformance::listed_suite`: a planted-defect
//! test is green when one listed scenario does not pass over its defect, so each listed suite
//! must pass over the unplanted system, and the list must name real scenarios, once each.

use std::collections::BTreeSet;

use surf_conformance::{NOTICED, listed, listed_suite, memory_target, run, suite_scenarios};

/// The lines of `tests/noticed.txt` after the first: (label, ids).
fn lines() -> Vec<(String, Vec<String>)> {
    let list = std::fs::read_to_string(NOTICED).unwrap_or_else(|error| panic!("{error}"));
    list.lines()
        .skip(1)
        .map(|line| {
            let (label, ids) = line
                .split_once(": ")
                .unwrap_or_else(|| panic!("no `: ` in {line:?}"));
            (
                label.to_owned(),
                ids.split(' ').map(str::to_owned).collect(),
            )
        })
        .collect()
}

/// A listed scenario that fails over the unplanted system would make its planted-defect test
/// green whatever the defect does: the controls run the whole suite, never a listed one. Each
/// listed suite is the narrowed file the setup step wrote, which the planted tests read.
#[test]
fn adversary_each_listed_suite_passes_over_the_unplanted_system() {
    let target = memory_target();
    let mut wrong = Vec::new();
    for (label, ids) in lines() {
        let narrowed = listed_suite(&label).unwrap_or_else(|error| panic!("{error}"));
        let conformance = run(&narrowed, &target).expect("the suite runs");
        let counts = conformance.report.counts();
        if counts.total as usize != ids.len() || counts.passed != counts.total {
            wrong.push(format!(
                "{label}: {} of {} passed, {} listed",
                counts.passed,
                counts.total,
                ids.len()
            ));
        }
    }
    assert!(wrong.is_empty(), "{wrong:?}");
}

/// Each line: a label once in the file, 1 to 3 ids, each once, each a scenario of the suite, in
/// the suite's order (as the setup step wrote it, from the one admission of the whole suite).
#[test]
fn adversary_each_line_names_one_to_three_scenarios_of_the_suite_in_order() {
    let order: Vec<String> = suite_scenarios().unwrap_or_else(|error| panic!("{error}"));
    let mut labels = BTreeSet::new();
    let mut wrong = Vec::new();
    for (label, ids) in lines() {
        if !labels.insert(label.clone()) {
            wrong.push(format!("{label}: a second line"));
        }
        if ids.is_empty() || ids.len() > 3 {
            wrong.push(format!("{label}: {} ids", ids.len()));
        }
        let positions: Vec<Option<usize>> = ids
            .iter()
            .map(|id| order.iter().position(|known| known == id))
            .collect();
        if positions.iter().any(Option::is_none) {
            wrong.push(format!("{label}: an id the suite does not hold in {ids:?}"));
        } else if positions.windows(2).any(|pair| pair[0] >= pair[1]) {
            wrong.push(format!(
                "{label}: not once each in the suite's order: {ids:?}"
            ));
        }
    }
    assert!(wrong.is_empty(), "{wrong:?}");
}

/// Each line belongs to a planted defect a test plants: a line no test reads is dead.
#[test]
fn adversary_each_line_is_read_by_a_planted_test() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests");
    let sources: String = [
        "planted_defect.rs",
        "review2_planted_defects.rs",
        "two_of_each_entity.rs",
    ]
    .iter()
    .map(|file| std::fs::read_to_string(format!("{dir}/{file}")).expect("a test file"))
    .collect();
    let mut wrong = Vec::new();
    for (label, _) in lines() {
        let planted = match label.split_once('(') {
            Some((defect, entity)) => {
                let entity = entity.trim_end_matches(')');
                sources.contains(&format!("Defect::{defect}(Entity::{entity}),"))
            }
            None => sources.contains(&format!("\"{label}\"")),
        };
        if !planted {
            wrong.push(label);
        }
    }
    assert!(wrong.is_empty(), "no planted test reads {wrong:?}");
}

/// A label without a line, or a prefix of a label, is refused, not run over nothing.
#[test]
fn adversary_a_label_without_its_own_line_is_refused() {
    let list = std::fs::read_to_string(NOTICED).unwrap_or_else(|error| panic!("{error}"));
    for label in [
        "no such defect",
        "",
        "FirstRowOnly(Action",
        "navigation storage",
    ] {
        let narrowed = listed(&list, label);
        assert!(narrowed.is_err(), "{label:?}: {narrowed:?}");
    }
}
