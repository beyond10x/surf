//! The vendored data against the `html-inventory` block of `docs/design/html-parser.md`: every
//! data file reads, and the cases the harness finds and the tags it gives them are the ones the
//! note counted.

use std::collections::BTreeMap;

use surf_html_conformance::cases::{self, Case};
use surf_html_conformance::inventory::{self, Inventory};
use surf_html_conformance::paths;

fn inventory() -> Inventory {
    inventory::load(&paths::note()).unwrap_or_else(|e| panic!("{e}"))
}

fn read_all(inventory: &Inventory) -> Vec<(String, Vec<Case>)> {
    inventory
        .files
        .iter()
        .map(|file| {
            let read = cases::read(&paths::data(), file.group, &file.path)
                .unwrap_or_else(|e| panic!("{e}"));
            (file.path.clone(), read)
        })
        .collect()
}

#[test]
fn every_data_file_is_read_without_a_reader_error() {
    let inventory = inventory();
    let mut listed: Vec<String> = inventory.files.iter().map(|f| f.path.clone()).collect();
    listed.sort();
    let mut found = paths::data_files(&paths::data(), &inventory).expect("the data directory");
    found.sort();
    assert_eq!(
        found, listed,
        "the files under data/<source>/ are the files of the inventory"
    );
    assert!(!listed.is_empty(), "the inventory lists data files");

    let mut errors = Vec::new();
    for file in &inventory.files {
        if let Err(e) = cases::read(&paths::data(), file.group, &file.path) {
            errors.push(e.to_string());
        }
    }
    assert!(errors.is_empty(), "reader errors:\n{}", errors.join("\n"));
}

#[test]
fn the_case_count_per_file_equals_the_inventory() {
    let inventory = inventory();
    let mut differences = Vec::new();
    let mut sum = 0;
    for (file, (path, read)) in inventory.files.iter().zip(read_all(&inventory)) {
        sum += read.len();
        if read.len() != file.cases {
            differences.push(format!(
                "{path}: inventory {} cases, harness {}",
                file.cases,
                read.len()
            ));
        }
    }
    assert!(differences.is_empty(), "{}", differences.join("\n"));
    assert_eq!(
        sum, inventory.total,
        "the cases of all files sum to `total`"
    );
    assert!(inventory.total > 0);
}

#[test]
fn the_case_count_per_tag_equals_the_inventory() {
    let inventory = inventory();
    let mut per_tag: BTreeMap<String, usize> = BTreeMap::new();
    let mut differences = Vec::new();
    for (file, (path, read)) in inventory.files.iter().zip(read_all(&inventory)) {
        let mut in_file: Vec<(String, usize)> = Vec::new();
        for case in &read {
            *per_tag.entry(case.tag.name().to_string()).or_default() += 1;
            let key = case.tag.inventory_key();
            match in_file.iter_mut().find(|(k, _)| *k == key) {
                Some((_, n)) => *n += 1,
                None => in_file.push((key, 1)),
            }
        }
        let mut expected = file.tags.clone();
        expected.sort();
        in_file.sort();
        if in_file != expected {
            differences.push(format!(
                "{path}: inventory {expected:?}, harness {in_file:?}"
            ));
        }
    }
    for (name, n) in &inventory.tags {
        let measured = per_tag.remove(name).unwrap_or(0);
        if measured != *n {
            differences.push(format!("tag {name}: inventory {n}, harness {measured}"));
        }
    }
    for (name, n) in per_tag {
        differences.push(format!("tag {name}: in no `tag` line, harness {n}"));
    }
    assert!(differences.is_empty(), "{}", differences.join("\n"));
    assert!(!inventory.tags.is_empty(), "the inventory has `tag` lines");
}

#[test]
fn every_case_has_exactly_one_tag() {
    let inventory = inventory();
    let declared: Vec<&str> = inventory
        .tags
        .iter()
        .map(|(name, _)| name.as_str())
        .collect();
    let mut cases_seen = 0;
    let mut tagged = 0;
    let mut problems = Vec::new();
    for (path, read) in read_all(&inventory) {
        for case in &read {
            cases_seen += 1;
            let name = case.tag.name();
            if !declared.contains(&name) {
                problems.push(format!(
                    "{path} {}: tag {name} has no `tag` line",
                    case.number
                ));
                continue;
            }
            if name == "not-run" && case.tag.reason().is_none_or(str::is_empty) {
                problems.push(format!("{path} {}: not-run without a reason", case.number));
                continue;
            }
            tagged += 1;
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
    assert_eq!(tagged, cases_seen, "each case is counted under one tag");
    let tag_sum: usize = inventory.tags.iter().map(|(_, n)| n).sum();
    assert_eq!(
        tagged, tag_sum,
        "the tagged cases equal the sum of the `tag` lines"
    );
}
