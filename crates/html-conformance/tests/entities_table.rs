//! The named character reference table of `surf-html` against the vendored `entities.json`:
//! the committed `crates/html/src/charref_table.rs` is what the binary `entities-table` writes,
//! and it carries the BSD 3-Clause notice of the WHATWG portion.

use surf_html_conformance::{entities_table, paths};

fn committed_table() -> String {
    let path = paths::root().join(entities_table::TABLE);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

#[test]
fn the_named_reference_table_matches_entities_json() {
    let path = paths::data().join(entities_table::JSON);
    let json = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let written = entities_table::table_text(&json).unwrap_or_else(|e| panic!("{e}"));
    assert!(
        written == committed_table(),
        "{} is not the text `cargo run -p surf-html-conformance --bin entities-table` writes \
         from {}",
        entities_table::TABLE,
        entities_table::JSON
    );
}

#[test]
fn the_named_reference_table_carries_the_bsd_3_clause_notice() {
    let table = committed_table();
    let copyright = format!("// {}\n", entities_table::COPYRIGHT);
    assert!(
        table.starts_with(&copyright),
        "{} does not start with {copyright:?}",
        entities_table::TABLE
    );
    let comments: String = table
        .lines()
        .take_while(|line| line.starts_with("//") && !line.starts_with("//!"))
        .map(|line| {
            let text = line.strip_prefix("//").unwrap_or(line);
            format!("{}\n", text.strip_prefix(' ').unwrap_or(text))
        })
        .collect();
    let notice = format!(
        "{}\n\n{}",
        entities_table::COPYRIGHT,
        entities_table::BSD_3_CLAUSE
    );
    assert!(
        comments == notice,
        "the header of {} is not the BSD 3-Clause notice:\n{comments}",
        entities_table::TABLE
    );
    for part in [
        "1. Redistributions of source code must retain the above copyright notice",
        "2. Redistributions in binary form must reproduce the above copyright notice",
        "3. Neither the name of the copyright holder nor the names of its",
        "THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS \"AS IS\"",
    ] {
        assert!(comments.contains(part), "the header lacks {part:?}");
    }
}
